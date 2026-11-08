//! Fachada de render 3D (docs/tecnologia-3d.md §7.2). El resto del reproductor habla solo con este módulo:
//! crear / actualizar / quitar el dibujo de una instancia por id, cámara, entorno y dibujar el cuadro.
//! Toda la API de kiss3d queda aquí adentro; para pasar a Bevy se reescribe solo este archivo.
//!
//! Convenciones (las mismas que `runtime.rs`):
//! - Metros, Y hacia arriba, adelante = -Z. La posición de una instancia es su CENTRO.
//! - Giros en grados, aplicados como Godot con `Basis.from_euler(YXZ)`: primero `roll` alrededor de Z,
//!   después `pitch` alrededor de X y por último `angle` (yaw) alrededor de Y. Es decir R = Ry(yaw)·Rx(pitch)·Rz(roll),
//!   que es lo que da `move_forward` del runtime: adelante = (-sin(yaw)·cos(pitch), sin(pitch), -cos(yaw)·cos(pitch)).
//! - Primitivas: su tamaño es (w, h, d) de la instancia. Esfera, cilindro y cono se estiran a la caja w×h×d;
//!   la cápsula usa radio = min(w, d)/2 y alto total = h; el plano es w×d (horizontal, mira hacia +Y).
//! - Modelos glTF: se dibujan a su escala propia multiplicada por (w/base_w, h/base_h, d/base_d), así que una
//!   instancia sin escalar se ve del tamaño que tiene el archivo (el `size` del objeto queda para los choques).
//!   Limitación de kiss3d: la escala no se propaga a las traslaciones de los nodos hijos del glTF, así que una
//!   escala no uniforme en un modelo con varias piezas es aproximada. Si el glTF trae animaciones, la primera
//!   se reproduce en bucle.
//!
//! Qué se usa de cada campo del modelo (etapa F2):
//! - Environment: sky_top/horizon/ground → cielo procedural generado como imagen HDR (fondo + IBL); sky_image → HDR
//!   equirectangular (fondo + IBL); ambient → intensidad del IBL (y de la ambiente plana); fog_* → niebla lineal
//!   (fog_end <= fog_start o 0 = sin niebla); glow → bloom; ssao → SSAO; tonemap linear/filmic/aces/agx (filmic =
//!   Khronos PBR Neutral); exposure. Falta: brillo del fondo separado de la luz ambiente, ajustes finos de glow.
//! - Mesh: todas las formas, color, metallic, roughness, texture (envuelve la forma), emission (×2, para que brille
//!   con glow), tint y opacity de la instancia (opacidad < 255 = transparente). En modelos glTF se respetan sus
//!   materiales; tint/opacity reemplazan el color base de todas sus piezas (y no vuelven atrás).
//! - Light: sun (sombras en 4 cascadas), point y spot (con sombras), color, energy (×3 = intensidad de kiss3d),
//!   range, spot_angle (= ángulo del borde del cono, como Godot), shadows. Máximo 8 luces principales (kiss3d agrupa
//!   el resto sin sombras).
//! - Camera3d: la primera instancia (posición + giros, fov vertical, near, far).

use kiss3d::glamx::glam::camera::rh::proj::opengl;
use kiss3d::glamx::{EulerRot, Mat4};
use kiss3d::post_processing::Tonemap;
use kiss3d::prelude::*;
use nicostation::model::{Environment, Kind, LightKind, Shape3};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

/// Posición, giros y tamaño de una instancia, en las unidades del runtime.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Pose {
    pub pos: [f32; 3],
    /// Grados.
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    /// Tamaño actual (w, h, d) y el original del objeto (para la escala de los modelos).
    pub size: [f32; 3],
    pub base: [f32; 3],
}

/// Cómo se ve la instancia: visible, color que multiplica y opacidad (0..255).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aspecto {
    pub visible: bool,
    pub tint: [u8; 3],
    pub opacity: f32,
}

/// Cámara del juego: posición, giros en grados y lente.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camara {
    pub pos: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    /// Campo de visión vertical en grados.
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

/// Lo que pasó en la ventana desde el último cuadro.
#[derive(Clone, Debug, PartialEq)]
pub enum Evento {
    /// Tecla con el nombre de `nombre_tecla` (como macroquad): true = apretada, false = soltada.
    Tecla(&'static str, bool),
}

/// Giro de una instancia: R = Ry(yaw)·Rx(pitch)·Rz(roll), en grados.
pub fn rotacion(yaw: f32, pitch: f32, roll: f32) -> Quat {
    Quat::from_euler(EulerRot::YXZ, yaw.to_radians(), pitch.to_radians(), roll.to_radians())
}

/// Hacia dónde mira algo con estos giros (-Z girado).
#[cfg(test)]
pub fn adelante(yaw: f32, pitch: f32, roll: f32) -> Vec3 {
    rotacion(yaw, pitch, roll) * Vec3::NEG_Z
}

/// Color de 8 bits en sRGB a lineal (lo que usan los materiales PBR y las luces).
pub fn lineal(c: [u8; 3]) -> [f32; 3] {
    c.map(|v| {
        let v = v as f32 / 255.;
        if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    })
}

/// Color del cielo procedural (lineal) para una dirección con elevación `elev` (radianes, + = arriba).
/// Como ProceduralSkyMaterial de Godot: del horizonte al cénit con una curva suave; hacia abajo llega rápido al suelo.
pub fn cielo(top: [f32; 3], horizon: [f32; 3], ground: [f32; 3], elev: f32) -> [f32; 3] {
    let half_pi = std::f32::consts::FRAC_PI_2;
    let (to, t) = if elev >= 0. {
        (top, (elev / half_pi).clamp(0., 1.).powf(0.5))
    } else {
        (ground, (-elev / half_pi * 6.).clamp(0., 1.))
    };
    [0, 1, 2].map(|k| horizon[k] + (to[k] - horizon[k]) * t)
}

/// Imagen equirectangular (lineal, HDR) con el cielo procedural: fondo y luz ambiente (IBL).
fn imagen_cielo(env: &Environment) -> image::DynamicImage {
    let (w, h) = (256u32, 128u32);
    let (top, hor, gnd) = (lineal(env.sky_top), lineal(env.sky_horizon), lineal(env.sky_ground));
    let img = image::Rgb32FImage::from_fn(w, h, |_, y| {
        // Fila 0 = arriba (elevación +90°), última = abajo.
        let elev = std::f32::consts::FRAC_PI_2 - (y as f32 + 0.5) / h as f32 * std::f32::consts::PI;
        image::Rgb(cielo(top, hor, gnd, elev))
    });
    image::DynamicImage::ImageRgb32F(img)
}

fn tonemap(nombre: &str) -> Tonemap {
    match nombre {
        "linear" => Tonemap::None,
        "aces" => Tonemap::Aces,
        "agx" => Tonemap::AgX,
        "reinhard" => Tonemap::Reinhard,
        // «filmic» de Godot: el más parecido en kiss3d es Khronos PBR Neutral (suave, sin cambiar los tonos).
        _ => Tonemap::Neutral,
    }
}

/// Cámara que mira desde una pose fija (la que manda el juego).
struct CamaraJuego {
    eye: Vec3,
    rot: Quat,
    fov: f32,
    near: f32,
    far: f32,
    aspect: f32,
}

impl CamaraJuego {
    fn view(&self) -> Pose3 {
        Pose3::from_parts(self.eye, self.rot).inverse()
    }
    fn proj(&self) -> Mat4 {
        opengl::perspective(self.fov, self.aspect, self.near, self.far)
    }
}

impl Camera3d for CamaraJuego {
    fn handle_event(&mut self, _: &Canvas, event: &WindowEvent) {
        if let WindowEvent::FramebufferSize(w, h) = *event {
            self.aspect = w as f32 / h.max(1) as f32;
        }
    }
    fn eye(&self) -> Vec3 {
        self.eye
    }
    fn view_transform(&self) -> Pose3 {
        self.view()
    }
    fn transformation(&self) -> Mat4 {
        self.proj() * self.view().to_mat4()
    }
    fn inverse_transformation(&self) -> Mat4 {
        self.transformation().inverse()
    }
    fn clip_planes(&self) -> (f32, f32) {
        (self.near, self.far)
    }
    fn update(&mut self, canvas: &Canvas) {
        let (w, h) = canvas.size();
        self.aspect = w as f32 / h.max(1) as f32;
    }
    fn view_transform_pair(&self, _pass: usize) -> (Pose3, Mat4) {
        (self.view(), self.proj())
    }
}

/// Material aplicado por última vez (para no tocar la GPU si no cambió).
#[derive(Clone, Copy, PartialEq)]
struct Material {
    color: [f32; 4],
    metallic: f32,
    roughness: f32,
    emission: [f32; 3],
}

struct Visual {
    nodo: SceneNode3d,
    /// Qué objeto lo creó: si cambia (otro tipo o forma) se vuelve a crear.
    firma: String,
    /// Tamaño con el que se creó la geometría (la escala es tamaño / base).
    base: Vec3,
    /// Modelo glTF: la raíz del archivo (lleva la escala) y su animación.
    modelo: Option<(SceneNode3d, AnimationPlayer)>,
    material: Option<Material>,
    visible: bool,
}

/// Plano horizontal de w×d centrado en el origen, mirando hacia +Y (con UV).
fn plano(w: f32, d: f32) -> SceneNode3d {
    let (x, z) = (w / 2., d / 2.);
    let coords = vec![Vec3::new(-x, 0., -z), Vec3::new(x, 0., -z), Vec3::new(x, 0., z), Vec3::new(-x, 0., z)];
    let normals = vec![Vec3::Y; 4];
    let uvs = vec![Vec2::new(0., 0.), Vec2::new(1., 0.), Vec2::new(1., 1.), Vec2::new(0., 1.)];
    let idx = kiss3d::procedural::IndexBuffer::Unified(vec![[0, 2, 1].into(), [0, 3, 2].into()]);
    SceneNode3d::render_mesh(kiss3d::procedural::RenderMesh::new(coords, Some(normals), Some(uvs), Some(idx)), Vec3::ONE)
}

pub struct Escena {
    ventana: Window,
    raiz: SceneNode3d,
    visuales: HashMap<u64, Visual>,
    cam: CamaraJuego,
    /// Cámara orbital de desarrollo (`--libre`): si está, manda ella.
    libre: Option<OrbitCamera3d>,
    entorno: Option<Environment>,
    fuente: Arc<Font>,
}

impl Escena {
    /// Ventana visible de `w`×`h`.
    pub async fn ventana(titulo: &str, w: u32, h: u32) -> Escena {
        Self::con(Window::new_with_size(titulo, w, h).await)
    }

    /// Sin ventana: dibuja en una textura (para `--captura`; funciona sin monitor).
    pub async fn fuera_de_pantalla(w: u32, h: u32) -> Escena {
        let setup = CanvasSetup { samples: NumSamples::One, ..Default::default() };
        Self::con(Window::new_headless_with_setup(w, h, setup).await)
    }

    fn con(mut ventana: Window) -> Escena {
        ventana.set_shadows_enabled(true);
        let (w, h) = (ventana.width(), ventana.height());
        Escena {
            ventana,
            raiz: SceneNode3d::empty(),
            visuales: HashMap::new(),
            cam: CamaraJuego { eye: Vec3::new(0., 6., 12.), rot: Quat::IDENTITY, fov: 70f32.to_radians(), near: 0.05, far: 4000., aspect: w as f32 / h.max(1) as f32 },
            libre: None,
            entorno: None,
            fuente: Font::default(),
        }
    }

    pub fn tamano(&self) -> (u32, u32) {
        (self.ventana.width(), self.ventana.height())
    }

    /// Cámara orbital con el mouse (arrastrar = girar, rueda = acercar) mirando a `centro` desde `ojo`.
    pub fn set_libre(&mut self, ojo: [f32; 3], centro: [f32; 3]) {
        self.libre = Some(OrbitCamera3d::new(Vec3::from(ojo), Vec3::from(centro)));
    }

    pub fn set_camara(&mut self, c: &Camara) {
        self.cam.eye = Vec3::from(c.pos);
        self.cam.rot = rotacion(c.yaw, c.pitch, c.roll);
        self.cam.fov = c.fov.clamp(1., 179.).to_radians();
        self.cam.near = c.near.max(0.001);
        self.cam.far = c.far.max(self.cam.near + 0.01);
    }

    /// Cámara que mira desde `ojo` hacia `centro`.
    pub fn mirar(&mut self, ojo: [f32; 3], centro: [f32; 3]) {
        let (e, c) = (Vec3::from(ojo), Vec3::from(centro));
        self.cam.eye = e;
        self.cam.rot = Pose3::face_towards(e, e - (c - e), Vec3::Y).rotation;
    }

    /// Cielo, luz ambiente, niebla y post-proceso. `archivo` da la ruta local de un archivo del proyecto.
    pub fn set_entorno(&mut self, env: &Environment, archivo: &dyn Fn(&str) -> Option<std::path::PathBuf>) {
        if self.entorno.as_ref() == Some(env) {
            return;
        }
        let w = &mut self.ventana;
        let hdr = (!env.sky_image.is_empty()).then(|| archivo(&env.sky_image)).flatten();
        let ok = hdr.is_some_and(|p| w.set_skybox_from_file(&p));
        if !ok {
            if !env.sky_image.is_empty() {
                eprintln!("cielo {}: no pude leerlo; uso el cielo procedural", env.sky_image);
            }
            w.set_skybox_image(&imagen_cielo(env));
        }
        // Con cielo, la luz ambiente sale de él (IBL); `ambient` la escala. Sin IBL quedaría la ambiente plana.
        w.set_skybox_orientation(0., env.ambient.max(0.));
        w.set_ambient(0.2 * env.ambient.max(0.));
        let [r, g, b] = lineal(env.sky_horizon);
        w.set_ambient_color(Color::new(r, g, b, 1.));
        w.set_fog(if env.fog_end > env.fog_start && env.fog_end > 0. {
            let [r, g, b] = lineal(env.fog_color);
            Fog::linear(Color::new(r, g, b, 1.), env.fog_start, env.fog_end)
        } else {
            Fog::default()
        });
        w.set_bloom_enabled(env.glow);
        if env.glow {
            w.set_bloom(1.0, 0.08);
        }
        w.set_ssao_enabled(env.ssao);
        w.set_tonemap(tonemap(&env.tonemap));
        w.set_exposure(env.exposure.max(0.));
        self.entorno = Some(env.clone());
    }

    /// Crea o actualiza el dibujo de la instancia `id` (Mesh o Light; lo demás se ignora).
    pub fn poner(&mut self, id: u64, def: &Kind, pose: &Pose, asp: &Aspecto, archivo: &dyn Fn(&str) -> Option<std::path::PathBuf>) {
        let firma = match def {
            Kind::Mesh { shape, model, texture, .. } => format!("mesh:{shape:?}:{model}:{texture}"),
            Kind::Light { light, .. } => format!("light:{light:?}"),
            _ => return,
        };
        if self.visuales.get(&id).is_some_and(|v| v.firma != firma) {
            self.quitar(id);
        }
        if !self.visuales.contains_key(&id) {
            let Some(v) = self.crear(def, pose, firma, archivo) else { return };
            self.visuales.insert(id, v);
        }
        let v = self.visuales.get_mut(&id).expect("recién creado");
        let rot = rotacion(pose.yaw, pose.pitch, pose.roll);
        v.nodo.set_pose(Pose3::from_parts(Vec3::from(pose.pos), rot));
        if v.visible != asp.visible {
            v.nodo.set_visible(asp.visible);
            v.visible = asp.visible;
        }
        match def {
            Kind::Mesh { color, metallic, roughness, emission, .. } => {
                let size = Vec3::from(pose.size);
                let alpha = (asp.opacity / 255.).clamp(0., 1.);
                let tint = lineal(asp.tint);
                if let Some((raiz, _)) = &mut v.modelo {
                    let base = Vec3::from(pose.base);
                    let k = Vec3::new(ratio(size.x, base.x), ratio(size.y, base.y), ratio(size.z, base.z));
                    raiz.set_local_scale(k.x * v.base.x, k.y * v.base.y, k.z * v.base.z);
                    // El modelo trae sus materiales: solo se tocan si la instancia tiene color u opacidad propios.
                    let m = Material { color: [tint[0], tint[1], tint[2], alpha], metallic: 0., roughness: 0., emission: [0.; 3] };
                    if (asp.tint != [255; 3] || alpha < 1.) && v.material != Some(m) {
                        raiz.set_color_recursive(Color::new(m.color[0], m.color[1], m.color[2], m.color[3]));
                        v.material = Some(m);
                    }
                } else {
                    v.nodo.set_local_scale(ratio(size.x, v.base.x), ratio(size.y, v.base.y), ratio(size.z, v.base.z));
                    let c = lineal(*color);
                    let m = Material {
                        color: [c[0] * tint[0], c[1] * tint[1], c[2] * tint[2], alpha],
                        metallic: metallic.clamp(0., 1.),
                        roughness: roughness.clamp(0., 1.),
                        emission: lineal(*emission).map(|e| e * 2.),
                    };
                    if v.material != Some(m) {
                        v.nodo.set_color(Color::new(m.color[0], m.color[1], m.color[2], m.color[3]));
                        v.nodo.set_metallic(m.metallic);
                        v.nodo.set_roughness(m.roughness);
                        v.nodo.set_emissive(Color::new(m.emission[0], m.emission[1], m.emission[2], 1.));
                        v.material = Some(m);
                    }
                }
            }
            Kind::Light { light, color, energy, range, spot_angle, shadows } => {
                let range = range.max(0.01);
                let mut l = match light {
                    // El sol alumbra hacia su -Z (girado por angle/pitch/roll); la posición no importa.
                    LightKind::Sun => Light::directional(Vec3::NEG_Z),
                    LightKind::Point => Light::point(range),
                    LightKind::Spot => {
                        let outer = spot_angle.clamp(1., 89.).to_radians();
                        Light::spot(outer * 0.8, outer, range)
                    }
                };
                let [r, g, b] = lineal(*color);
                let [tr, tg, tb] = lineal(asp.tint);
                l = l.with_color(Color::new(r * tr, g * tg, b * tb, 1.)).with_intensity(energy.max(0.) * 3.).with_casts_shadows(*shadows);
                v.nodo.set_light(Some(l));
            }
            _ => {}
        }
    }

    fn crear(&mut self, def: &Kind, pose: &Pose, firma: String, archivo: &dyn Fn(&str) -> Option<std::path::PathBuf>) -> Option<Visual> {
        let size = Vec3::from(pose.size).max(Vec3::splat(0.001));
        let mut modelo = None;
        let mut base = size;
        let nodo = match def {
            Kind::Light { .. } => self.raiz.add_light(Light::point(10.)),
            Kind::Mesh { shape: Shape3::Model, model, .. } => {
                let ruta = archivo(model).filter(|p| p.is_file());
                let Some(ruta) = ruta else {
                    eprintln!("modelo «{model}»: no está; dibujo una caja");
                    return self.crear(&Kind::Mesh { shape: Shape3::Cube, model: String::new(), size: pose.size, color: [255, 0, 255], metallic: 0., roughness: 0.5, texture: String::new(), emission: [0; 3] }, pose, firma, archivo);
                };
                let m = match kiss3d::loader::gltf::load(&ruta) {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("modelo «{model}»: {e}");
                        return None;
                    }
                };
                let mut grupo = self.raiz.add_group();
                grupo.add_child(m.root.clone());
                base = m.root.local_scale();
                let mut player = m.player;
                if player.clip_count() > 0 {
                    player.play_index(0);
                    player.set_looping(true);
                }
                modelo = Some((m.root, player));
                grupo
            }
            Kind::Mesh { shape, texture, .. } => {
                let mut n = match shape {
                    // Las geometrías con nombre de kiss3d (cubo, esfera, cilindro, cono) son de tamaño 1 y su tamaño
                    // va en la escala local, que se pone en cada cuadro.
                    Shape3::Cube | Shape3::Model => {
                        base = Vec3::ONE;
                        SceneNode3d::cube(1., 1., 1.)
                    }
                    Shape3::Sphere => {
                        base = Vec3::ONE;
                        SceneNode3d::sphere(0.5)
                    }
                    Shape3::Cylinder => {
                        base = Vec3::ONE;
                        SceneNode3d::cylinder(0.5, 1.)
                    }
                    Shape3::Cone => {
                        base = Vec3::ONE;
                        SceneNode3d::cone(0.5, 1.)
                    }
                    Shape3::Capsule => {
                        let r = size.x.min(size.z) / 2.;
                        SceneNode3d::capsule(r, (size.y - 2. * r).max(0.))
                    }
                    Shape3::Plane => plano(size.x, size.z),
                };
                if !texture.is_empty() {
                    match archivo(texture).filter(|p| p.is_file()) {
                        Some(p) => {
                            n.set_texture_from_file(&p, texture);
                        }
                        None => eprintln!("textura «{texture}»: no está"),
                    }
                }
                self.raiz.add_child(n.clone());
                n
            }
            _ => return None,
        };
        Some(Visual { nodo, firma, base, modelo, material: None, visible: true })
    }

    /// Avanza las animaciones de los modelos glTF.
    pub fn animar(&mut self, dt: f32) {
        for v in self.visuales.values_mut() {
            if let Some((_, p)) = &mut v.modelo {
                p.update(dt);
            }
        }
    }

    pub fn quitar(&mut self, id: u64) {
        if let Some(mut v) = self.visuales.remove(&id) {
            v.nodo.remove();
        }
    }

    /// Quita los dibujos de las instancias que ya no están.
    pub fn retener(&mut self, vivos: &HashSet<u64>) {
        let fuera: Vec<u64> = self.visuales.keys().filter(|id| !vivos.contains(id)).copied().collect();
        for id in fuera {
            self.quitar(id);
        }
    }

    pub fn cantidad(&self) -> usize {
        self.visuales.len()
    }

    /// Texto encima de la escena (HUD), en píxeles de la ventana desde arriba a la izquierda.
    pub fn texto(&mut self, t: &str, x: f32, y: f32, tam: f32, color: [u8; 3], alpha: f32) {
        let c = color.map(|v| v as f32 / 255.);
        self.ventana.draw_text(t, Vec2::new(x, y), tam, &self.fuente, Color::new(c[0], c[1], c[2], alpha));
    }

    /// Dibuja un cuadro. Devuelve false si se cerró la ventana.
    pub async fn dibujar(&mut self) -> bool {
        match &mut self.libre {
            Some(o) => self.ventana.render_3d(&mut self.raiz, o).await,
            None => self.ventana.render_3d(&mut self.raiz, &mut self.cam).await,
        }
    }

    /// Teclas apretadas y soltadas desde el último cuadro.
    pub fn eventos(&mut self) -> Vec<Evento> {
        let mut out = vec![];
        for e in self.ventana.events().iter() {
            if let WindowEvent::Key(k, a, _) = e.value {
                if let Some(n) = nombre_tecla(k) {
                    match a {
                        Action::Press => out.push(Evento::Tecla(n, true)),
                        Action::Release => out.push(Evento::Tecla(n, false)),
                    }
                }
            }
        }
        out
    }

    /// Mouse: posición en píxeles de la ventana y botones (izquierdo, derecho, medio).
    pub fn mouse(&self) -> ((f32, f32), [bool; 3]) {
        let p = self.ventana.cursor_pos().map(|(x, y)| (x as f32, y as f32)).unwrap_or_default();
        let b = [MouseButton::Button1, MouseButton::Button2, MouseButton::Button3].map(|b| self.ventana.get_mouse_button(b) == Action::Press);
        (p, b)
    }

    /// Guarda el último cuadro dibujado como PNG.
    pub fn guardar_png(&self, ruta: &Path) -> Result<(), String> {
        self.ventana.snap_image().save(ruta).map_err(|e| e.to_string())
    }
}

fn ratio(a: f32, b: f32) -> f32 {
    if b.abs() < 1e-6 { 1. } else { a / b }
}

/// Nombre de tecla como lo escriben las hojas de eventos (los de `KeyCode` de macroquad que usa el reproductor 2D).
pub fn nombre_tecla(k: Key) -> Option<&'static str> {
    use Key::*;
    Some(match k {
        Key0 => "Key0", Key1 => "Key1", Key2 => "Key2", Key3 => "Key3", Key4 => "Key4",
        Key5 => "Key5", Key6 => "Key6", Key7 => "Key7", Key8 => "Key8", Key9 => "Key9",
        A => "A", B => "B", C => "C", D => "D", E => "E", F => "F", G => "G", H => "H", I => "I",
        J => "J", K => "K", L => "L", M => "M", N => "N", O => "O", P => "P", Q => "Q", R => "R",
        S => "S", T => "T", U => "U", V => "V", W => "W", X => "X", Y => "Y", Z => "Z",
        Left => "Left", Right => "Right", Up => "Up", Down => "Down",
        Space => "Space", Return => "Enter", NumpadEnter => "KpEnter", Escape => "Escape", Tab => "Tab", Back => "Backspace",
        Delete => "Delete", Insert => "Insert", Home => "Home", End => "End", PageUp => "PageUp", PageDown => "PageDown",
        LShift => "LeftShift", RShift => "RightShift", LControl => "LeftControl", RControl => "RightControl",
        LAlt => "LeftAlt", RAlt => "RightAlt", LWin => "LeftSuper", RWin => "RightSuper",
        F1 => "F1", F2 => "F2", F3 => "F3", F4 => "F4", F5 => "F5", F6 => "F6",
        F7 => "F7", F8 => "F8", F9 => "F9", F10 => "F10", F11 => "F11", F12 => "F12",
        Numpad0 => "Kp0", Numpad1 => "Kp1", Numpad2 => "Kp2", Numpad3 => "Kp3", Numpad4 => "Kp4",
        Numpad5 => "Kp5", Numpad6 => "Kp6", Numpad7 => "Kp7", Numpad8 => "Kp8", Numpad9 => "Kp9",
        Minus => "Minus", Equals => "Equal", Comma => "Comma", Period => "Period", Slash => "Slash",
        Semicolon => "Semicolon", Apostrophe => "Apostrophe", LBracket => "LeftBracket", RBracket => "RightBracket",
        Backslash => "Backslash", Grave => "GraveAccent",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cerca(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < 1e-4
    }

    #[test]
    fn adelante_igual_que_move_forward_del_runtime() {
        for (yaw, pitch) in [(0f32, 0f32), (90., 0.), (-45., 0.), (30., 20.), (180., -60.), (270., 10.)] {
            let (y, p) = (yaw.to_radians(), pitch.to_radians());
            // Fórmula de `move_forward` en runtime.rs.
            let rt = Vec3::new(-y.sin() * p.cos(), p.sin(), -y.cos() * p.cos());
            assert!(cerca(adelante(yaw, pitch, 0.), rt), "yaw {yaw} pitch {pitch}");
            // El roll gira alrededor del eje de adelante: no cambia hacia dónde se mira.
            assert!(cerca(adelante(yaw, pitch, 37.), rt));
        }
    }

    #[test]
    fn orden_de_los_giros() {
        // yaw 90 = mirar hacia -X (girar a la izquierda, como en Godot).
        assert!(cerca(rotacion(90., 0., 0.) * Vec3::NEG_Z, Vec3::NEG_X));
        // pitch 90 = mirar hacia arriba.
        assert!(cerca(rotacion(0., 90., 0.) * Vec3::NEG_Z, Vec3::Y));
        // roll se aplica primero (alrededor de Z local): con roll 90 la derecha local (+X) queda hacia arriba.
        assert!(cerca(rotacion(0., 0., 90.) * Vec3::X, Vec3::Y));
        // R = Ry · Rx · Rz
        let r = rotacion(30., 20., 10.);
        let m = Quat::from_rotation_y(30f32.to_radians()) * Quat::from_rotation_x(20f32.to_radians()) * Quat::from_rotation_z(10f32.to_radians());
        for v in [Vec3::X, Vec3::Y, Vec3::Z, Vec3::new(0.3, -0.5, 0.8)] {
            assert!(cerca(r * v, m * v));
        }
    }

    #[test]
    fn camara_mira_hacia_adelante() {
        let c = CamaraJuego { eye: Vec3::new(1., 2., 3.), rot: rotacion(90., 0., 0.), fov: 1., near: 0.1, far: 100., aspect: 1. };
        // Un punto delante de la cámara (hacia -X) queda en el centro de la pantalla y dentro del rango de profundidad.
        let p = c.transformation() * Vec3::new(-4., 2., 3.).extend(1.);
        let ndc = p.truncate() / p.w;
        assert!(ndc.x.abs() < 1e-4 && ndc.y.abs() < 1e-4 && ndc.z > -1. && ndc.z < 1., "{ndc:?}");
        // Uno detrás queda con w negativo (no se ve).
        assert!((c.transformation() * Vec3::new(6., 2., 3.).extend(1.)).w < 0.);
    }

    #[test]
    fn colores_y_cielo() {
        assert_eq!(lineal([0, 0, 0]), [0.; 3]);
        assert_eq!(lineal([255, 255, 255]), [1.; 3]);
        assert!((lineal([128, 128, 128])[0] - 0.2158).abs() < 1e-3);
        let (t, h, g) = ([0., 0., 1.], [1., 1., 1.], [0.2, 0.1, 0.]);
        assert_eq!(cielo(t, h, g, 0.), h);
        assert_eq!(cielo(t, h, g, std::f32::consts::FRAC_PI_2), t);
        assert!(cerca(Vec3::from(cielo(t, h, g, -std::f32::consts::FRAC_PI_2)), Vec3::from(g)));
        let mitad = cielo(t, h, g, 0.4);
        assert!(mitad[2] == 1. && mitad[0] < 1. && mitad[0] > 0.);
    }

    #[test]
    fn nombres_de_teclas_como_macroquad() {
        for (k, n) in [(Key::Left, "Left"), (Key::Space, "Space"), (Key::Return, "Enter"), (Key::Back, "Backspace"),
                       (Key::LShift, "LeftShift"), (Key::LControl, "LeftControl"), (Key::W, "W"), (Key::Key1, "Key1"),
                       (Key::Escape, "Escape"), (Key::Tab, "Tab")] {
            assert_eq!(nombre_tecla(k), Some(n));
        }
        assert_eq!(nombre_tecla(Key::Unknown), None);
    }
}
