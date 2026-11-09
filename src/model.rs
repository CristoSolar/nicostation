//! Formato del proyecto NicoStation (JSON). Lo leen el reproductor y el editor, y el editor lo escribe.
//! Modelo inspirado en GDevelop (MIT): escenas, capas, objetos, comportamientos, eventos.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type Vars = BTreeMap<String, Value>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum Value {
    Bool(bool),
    Num(f64),
    Str(String),
    /// Lista (inventario, puntajes...). En JSON, un arreglo.
    List(Vec<Value>),
    /// Estructura con campos (en JSON, un objeto).
    Map(BTreeMap<String, Value>),
}

impl Value {
    pub fn num(&self) -> f64 {
        match self {
            Value::Num(n) => *n,
            Value::Bool(b) => *b as i32 as f64,
            Value::Str(s) => s.trim().parse().unwrap_or(0.),
            // Una colección como número vale su cantidad de elementos.
            Value::List(l) => l.len() as f64,
            Value::Map(m) => m.len() as f64,
        }
    }
    pub fn text(&self) -> String {
        match self {
            Value::Num(n) if n.fract() == 0. && n.abs() < 1e15 => format!("{}", *n as i64),
            Value::Num(n) => format!("{n}"),
            Value::Bool(b) => b.to_string(),
            Value::Str(s) => s.clone(),
            Value::List(l) => format!("[{}]", l.iter().map(Value::text).collect::<Vec<_>>().join(", ")),
            Value::Map(m) => format!("{{{}}}", m.iter().map(|(k, v)| format!("{k}: {}", v.text())).collect::<Vec<_>>().join(", ")),
        }
    }
    pub fn truthy(&self) -> bool {
        match self {
            Value::Num(n) => *n != 0.,
            Value::Bool(b) => *b,
            Value::Str(s) => !matches!(s.as_str(), "" | "0" | "false"),
            Value::List(l) => !l.is_empty(),
            Value::Map(m) => !m.is_empty(),
        }
    }
    /// Elemento de una lista (por número, desde 0) o campo de una estructura (por nombre).
    pub fn at(&self, k: &Value) -> Value {
        match self {
            Value::List(l) => l.get(k.num().max(0.) as usize).cloned(),
            Value::Map(m) => m.get(&k.text()).cloned(),
            _ => None,
        }
        .unwrap_or(Value::Num(0.))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Project {
    pub name: String,
    pub width: f32,
    pub height: f32,
    pub first_scene: String,
    #[serde(default)]
    pub globals: Vars,
    /// Objetos globales: disponibles en todas las escenas.
    #[serde(default)]
    pub objects: Vec<ObjectDef>,
    pub scenes: Vec<Scene>,
    #[serde(default)]
    pub functions: Vec<Function>,
    /// Versión del formato (ver `migrate`). Proyectos sin versión son 1.
    #[serde(default = "v1")]
    pub version: u32,
    #[serde(default, skip_serializing_if = "is_default")]
    pub settings: Settings,
    /// Plantilla de la que salió el proyecto (carpeta), para ofrecer sus misiones.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub plantilla: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Scene {
    pub name: String,
    #[serde(default)]
    pub background: [u8; 3],
    #[serde(default = "default_layers")]
    pub layers: Vec<Layer>,
    #[serde(default)]
    pub objects: Vec<ObjectDef>,
    #[serde(default)]
    pub groups: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub instances: Vec<Instance>,
    #[serde(default)]
    pub vars: Vars,
    #[serde(default)]
    pub events: Vec<Event>,
    /// Lógica en nodos (ver `nodos.rs`); se compila a eventos que van después de `events`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub graphs: Vec<crate::nodos::Graph>,
    /// Escena 3D: se juega con el reproductor 3D. Las instancias usan x, y, z como posición
    /// (Y hacia arriba, en metros) y `angle`/`pitch`/`roll` como giros en grados.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub dim3: bool,
    /// Gravedad de la física 2D en px/s² (x, y). Por defecto hacia abajo.
    #[serde(default = "gravity2", skip_serializing_if = "is_gravity2")]
    pub gravity: [f32; 2],
    /// Cielo, luz ambiente, niebla y post-proceso de una escena 3D.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<Environment>,
}

/// Entorno de una escena 3D (como WorldEnvironment de Godot).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Environment {
    /// Cielo procedural: color arriba, en el horizonte y del suelo.
    #[serde(default = "sky_top")]
    pub sky_top: [u8; 3],
    #[serde(default = "sky_horizon")]
    pub sky_horizon: [u8; 3],
    #[serde(default = "sky_ground")]
    pub sky_ground: [u8; 3],
    /// Imagen HDR del cielo (opcional): ilumina y se refleja en los materiales.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sky_image: String,
    /// Cuánta luz da el cielo a toda la escena.
    #[serde(default = "one")]
    pub ambient: f32,
    /// Niebla: color y distancias donde empieza y donde tapa todo (0 = sin niebla).
    #[serde(default)]
    pub fog_color: [u8; 3],
    #[serde(default)]
    pub fog_start: f32,
    #[serde(default)]
    pub fog_end: f32,
    #[serde(default = "yes")]
    pub glow: bool,
    #[serde(default = "yes")]
    pub ssao: bool,
    /// linear, filmic, aces o agx.
    #[serde(default = "filmic")]
    pub tonemap: String,
    /// Brillo general (exposición).
    #[serde(default = "one")]
    pub exposure: f32,
}

impl Default for Environment {
    fn default() -> Self {
        serde_json::from_str("{}").expect("entorno por defecto")
    }
}

fn gravity2() -> [f32; 2] {
    [0., 980.]
}
fn is_gravity2(g: &[f32; 2]) -> bool {
    *g == [0., 980.]
}

fn sky_top() -> [u8; 3] {
    [92, 140, 214]
}
fn sky_horizon() -> [u8; 3] {
    [200, 214, 230]
}
fn sky_ground() -> [u8; 3] {
    [60, 56, 52]
}
fn one() -> f32 {
    1.
}
fn filmic() -> String {
    "filmic".into()
}

fn default_layers() -> Vec<Layer> {
    vec![Layer { name: String::new(), visible: true, fixed: false, parallax: [1., 1.] }]
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Layer {
    /// "" = capa base.
    pub name: String,
    #[serde(default = "yes")]
    pub visible: bool,
    /// No sigue a la cámara (útil para HUD).
    #[serde(default)]
    pub fixed: bool,
    /// Cuánto sigue a la cámara: [1, 1] normal; [0.5, 0.5] se mueve a la mitad (fondos lejanos).
    #[serde(default = "unit2", skip_serializing_if = "is_unit2")]
    pub parallax: [f32; 2],
}

fn unit2() -> [f32; 2] {
    [1., 1.]
}
fn is_unit2(v: &[f32; 2]) -> bool {
    *v == [1., 1.]
}

fn v1() -> u32 {
    1
}

/// Versión actual del formato del proyecto.
pub const FORMAT_VERSION: u32 = 2;

/// Ajustes del proyecto.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Settings {
    /// Cómo se adapta el juego a la ventana: fit (con franjas), stretch (estira) o integer (múltiplos enteros, pixel perfect).
    #[serde(default = "fit")]
    pub scale_mode: String,
    /// Dibujo nítido sin suavizar (pixel art).
    #[serde(default = "yes")]
    pub pixel_art: bool,
    /// Cuadros por segundo máximos (0 = los de la pantalla).
    #[serde(default)]
    pub max_fps: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub icon: String,
    /// Imagen mostrada mientras carga.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub loading_image: String,
}

impl Default for Settings {
    fn default() -> Self {
        serde_json::from_str("{}").expect("ajustes por defecto")
    }
}

fn fit() -> String {
    "fit".into()
}

/// Lleva un proyecto de una versión anterior del formato a la actual.
pub fn migrate(p: &mut Project) {
    // 1 → 2: aparecen `version` y `settings` (con valores por defecto); nada que convertir.
    p.version = FORMAT_VERSION;
}

fn yes() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[derive(Default)]
pub struct Instance {
    pub obj: String,
    pub x: f32,
    pub y: f32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub layer: String,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub angle: f32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub z: f32,
    /// Tamaño propio; si no, el del objeto.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub w: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h: Option<f32>,
    /// Profundidad propia (3D).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub d: Option<f32>,
    /// Giros 3D en grados: `angle` es alrededor del eje vertical; pitch inclina hacia arriba/abajo; roll de costado.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub pitch: f32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub roll: f32,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vars: Vars,
}

fn is_zero(v: &f32) -> bool {
    *v == 0.
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ObjectDef {
    pub name: String,
    #[serde(flatten)]
    pub kind: Kind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub behaviors: Vec<Behavior>,
    /// Efecto visual: none, flash (blanco), outline (silueta), grayscale, wave (ondas). Se cambia con la propiedad «effect».
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub effect: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vars: Vars,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Kind {
    /// Imagen con animaciones. w/h = 0 → tamaño de la primera imagen.
    Sprite {
        #[serde(default)]
        w: f32,
        #[serde(default)]
        h: f32,
        animations: Vec<Animation>,
        /// Punto de origen en píxeles de la imagen (x, y de la instancia y centro de giro). Sin él: esquina superior izquierda.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        origin: Option<[f32; 2]>,
        /// Máscara de choque: polígono convexo en píxeles de la imagen. Vacía = el rectángulo completo.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        mask: Vec<[f32; 2]>,
    },
    Text {
        text: String,
        #[serde(default = "text_size")]
        size: f32,
        #[serde(default = "white")]
        color: [u8; 3],
        /// Fuente TTF/OTF (ruta relativa); vacía = la del motor.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        font: String,
        /// left, center o right.
        #[serde(default = "left", skip_serializing_if = "is_left")]
        align: String,
        /// Ancho para partir en líneas (0 = una sola línea).
        #[serde(default, skip_serializing_if = "is_zero")]
        wrap: f32,
    },
    /// Imagen que se repite (fondos, pasto, agua). Con capas de paralaje da profundidad.
    Background {
        image: String,
        w: f32,
        h: f32,
        /// Desplazamiento automático en px/s (nubes, agua).
        #[serde(default, skip_serializing_if = "is_zero2")]
        scroll: [f32; 2],
    },
    /// Emisor de partículas (humo, chispas, explosiones, lluvia).
    Particles(Box<ParticleDef>),
    /// Rectángulo de color (prototipos sin imágenes).
    Box { w: f32, h: f32, color: [u8; 3] },
    /// Objeto 3D con forma y material (escenas 3D). Su posición x, y, z es el centro.
    Mesh {
        #[serde(default)]
        shape: Shape3,
        /// Modelo glTF (.glb/.gltf) cuando shape = model.
        #[serde(default, skip_serializing_if = "String::is_empty")]
        model: String,
        /// Tamaño en metros (ancho, alto, profundidad).
        #[serde(default = "unit3")]
        size: [f32; 3],
        #[serde(default = "white")]
        color: [u8; 3],
        /// Material PBR, como StandardMaterial3D de Godot: 0 = no metálico, 1 = metal.
        #[serde(default)]
        metallic: f32,
        /// 0 = espejo, 1 = mate.
        #[serde(default = "half")]
        roughness: f32,
        /// Imagen que envuelve la forma (opcional).
        #[serde(default, skip_serializing_if = "String::is_empty")]
        texture: String,
        /// Color que brilla solo (con glow se ve como luz).
        #[serde(default, skip_serializing_if = "is_black")]
        emission: [u8; 3],
    },
    /// Luz 3D: sol (direccional), puntual o foco. Apunta según angle/pitch.
    Light {
        #[serde(default)]
        light: LightKind,
        #[serde(default = "white")]
        color: [u8; 3],
        #[serde(default = "one")]
        energy: f32,
        /// Alcance en metros (puntual y foco).
        #[serde(default = "ten")]
        range: f32,
        /// Apertura del foco en grados.
        #[serde(default = "spot_angle")]
        spot_angle: f32,
        #[serde(default = "yes")]
        shadows: bool,
    },
    /// Cámara 3D: la primera instancia es la que mira el juego.
    Camera3d {
        /// Campo de visión vertical en grados.
        #[serde(default = "fov")]
        fov: f32,
        #[serde(default = "near")]
        near: f32,
        #[serde(default = "far")]
        far: f32,
    },
    /// Mapa de baldosas: una grilla pintada con las baldosas de una imagen (tileset).
    /// Con el comportamiento `platform`, cada baldosa sólida es suelo o pared.
    Tilemap {
        /// Imagen con las baldosas en grilla, de izquierda a derecha y de arriba a abajo.
        tileset: String,
        /// Tamaño de cada baldosa en px (cuadradas).
        #[serde(default = "d32")]
        tile: f32,
        cols: usize,
        rows: usize,
        /// Índice de baldosa por celda, fila por fila; -1 = vacía.
        #[serde(default)]
        data: Vec<i32>,
        /// Baldosas de decoración que no chocan (pasto, nubes...).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pass: Vec<i32>,
    },
}

fn left() -> String {
    "left".into()
}
fn is_left(s: &String) -> bool {
    s == "left"
}
fn is_zero2(v: &[f32; 2]) -> bool {
    *v == [0., 0.]
}

/// Parámetros de un emisor de partículas (al estilo de los de GameMaker/GDevelop).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ParticleDef {
    /// Imagen de cada partícula; vacía = círculo.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub image: String,
    #[serde(default = "white")]
    pub color_start: [u8; 3],
    #[serde(default = "white")]
    pub color_end: [u8; 3],
    #[serde(default = "full_alpha")]
    pub alpha_start: f32,
    #[serde(default)]
    pub alpha_end: f32,
    #[serde(default = "d8")]
    pub size_start: f32,
    #[serde(default = "d2")]
    pub size_end: f32,
    /// Vida en segundos (mínimo, máximo).
    #[serde(default = "life")]
    pub life: [f32; 2],
    /// Velocidad en px/s (mínimo, máximo).
    #[serde(default = "pspeed")]
    pub speed: [f32; 2],
    /// Dirección en grados (0 = derecha, -90 = arriba) y apertura del cono.
    #[serde(default = "up")]
    pub angle: f32,
    #[serde(default = "spread")]
    pub spread: f32,
    /// Gravedad en px/s² (x, y).
    #[serde(default)]
    pub gravity: [f32; 2],
    /// Partículas por segundo (0 = solo ráfagas).
    #[serde(default = "rate")]
    pub rate: f32,
    /// Al crearse, suelta esta cantidad de golpe.
    #[serde(default)]
    pub burst: u32,
    #[serde(default = "pmax")]
    pub max: usize,
    /// Sumar colores (fuego, magia) en vez de taparse.
    #[serde(default)]
    pub additive: bool,
    /// Se borra solo cuando ya no emite ni le quedan partículas (para explosiones de una vez).
    #[serde(default)]
    pub auto_destroy: bool,
}

impl Default for ParticleDef {
    fn default() -> Self {
        serde_json::from_str("{}").expect("partículas por defecto")
    }
}

fn full_alpha() -> f32 {
    255.
}
fn d8() -> f32 {
    8.
}
fn d2() -> f32 {
    2.
}
fn life() -> [f32; 2] {
    [0.6, 1.2]
}
fn pspeed() -> [f32; 2] {
    [40., 120.]
}
fn up() -> f32 {
    -90.
}
fn spread() -> f32 {
    30.
}
fn rate() -> f32 {
    30.
}
fn pmax() -> usize {
    300
}

fn d32() -> f32 {
    32.
}
fn unit3() -> [f32; 3] {
    [1., 1., 1.]
}
fn half() -> f32 {
    0.5
}
fn ten() -> f32 {
    10.
}
fn spot_angle() -> f32 {
    45.
}
fn fov() -> f32 {
    70.
}
fn near() -> f32 {
    0.05
}
fn far() -> f32 {
    4000.
}
fn is_black(c: &[u8; 3]) -> bool {
    *c == [0, 0, 0]
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Shape3 {
    #[default]
    Cube,
    Sphere,
    Plane,
    Capsule,
    Cylinder,
    Cone,
    /// Modelo glTF.
    Model,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LightKind {
    /// Luz del sol: llega paralela desde lejos, ilumina toda la escena.
    #[default]
    Sun,
    Point,
    Spot,
}

impl Kind {
    /// Baldosa en (columna, fila), o -1 si está vacía o fuera del mapa.
    pub fn tile_at(&self, c: i64, r: i64) -> i32 {
        match self {
            Kind::Tilemap { cols, rows, data, .. } if c >= 0 && r >= 0 && (c as usize) < *cols && (r as usize) < *rows => {
                data.get(r as usize * cols + c as usize).copied().unwrap_or(-1)
            }
            _ => -1,
        }
    }
}

fn text_size() -> f32 {
    24.
}
fn white() -> [u8; 3] {
    [255, 255, 255]
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Animation {
    pub name: String,
    /// Rutas de imagen relativas al archivo del proyecto.
    pub frames: Vec<String>,
    #[serde(default = "fps")]
    pub fps: f32,
    #[serde(default = "yes")]
    pub looping: bool,
}

fn fps() -> f32 {
    10.
}

/// Comportamientos con los valores por defecto de GDevelop.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Behavior {
    /// Personaje de plataformas (PlatformerObject).
    Platformer {
        #[serde(default = "d250")]
        max_speed: f32,
        #[serde(default = "d1500")]
        acceleration: f32,
        #[serde(default = "d1500")]
        deceleration: f32,
        #[serde(default = "d1000")]
        gravity: f32,
        #[serde(default = "d700")]
        max_falling_speed: f32,
        #[serde(default = "d600")]
        jump_speed: f32,
        #[serde(default = "d0_2")]
        jump_sustain_time: f32,
        #[serde(default = "d150")]
        ladder_speed: f32,
        /// Pendiente máxima que puede subir, en grados.
        #[serde(default = "d60")]
        slope_max_angle: f32,
        /// Se agarra de los bordes de plataformas marcadas como `grabbable`.
        #[serde(default)]
        can_grab_platforms: bool,
        /// Flecha abajo atraviesa las plataformas `jumpthru`.
        #[serde(default = "yes")]
        can_go_down_from_jumpthru: bool,
    },
    /// Suelo o pared para el Platformer. Rotada sirve de pendiente.
    Platform {
        /// Se atraviesa desde abajo.
        #[serde(default)]
        jumpthru: bool,
        /// Escalera: se sube y baja con las flechas.
        #[serde(default)]
        ladder: bool,
        /// El personaje puede agarrarse de su borde.
        #[serde(default)]
        grabbable: bool,
    },
    /// Movimiento de vista superior con flechas (TopDownMovement).
    TopDown {
        #[serde(default = "d200")]
        max_speed: f32,
        #[serde(default = "d400")]
        acceleration: f32,
        #[serde(default = "d800")]
        deceleration: f32,
        #[serde(default = "yes")]
        allow_diagonals: bool,
        #[serde(default)]
        rotate: bool,
    },
    /// Avanza en la dirección de su ángulo.
    Bullet { speed: f32 },
    /// Física 2D realista (rapier2d): cae, rebota, empuja y se apila.
    Physics {
        /// dynamic (se mueve con fuerzas), static (no se mueve) o kinematic (lo mueven los eventos).
        #[serde(default = "dynamic")]
        body: String,
        #[serde(default = "one")]
        density: f32,
        #[serde(default = "friction")]
        friction: f32,
        /// Rebote: 0 = nada, 1 = rebota igual de alto.
        #[serde(default)]
        restitution: f32,
        #[serde(default)]
        fixed_rotation: bool,
        #[serde(default = "one")]
        gravity_scale: f32,
        /// circle o box.
        #[serde(default = "box_shape")]
        shape: String,
        /// Detección continua para cosas muy rápidas (balas).
        #[serde(default)]
        bullet: bool,
    },
    /// Va hasta un punto esquivando obstáculos (A* en grilla).
    Pathfinder {
        #[serde(default = "d200")]
        speed: f32,
        /// Tamaño de celda de la grilla en px.
        #[serde(default = "d32")]
        cell: f32,
        #[serde(default = "yes")]
        diagonals: bool,
        /// Girar hacia donde camina.
        #[serde(default)]
        rotate: bool,
    },
    /// Obstáculo que esquivan los objetos con Pathfinder.
    Obstacle,
    Rotate { speed: f32 },
    DestroyOutside,
    BoundToLayout,
    ScreenWrap,
    Draggable,
}

fn d60() -> f32 { 60. }
fn d150() -> f32 { 150. }
fn d200() -> f32 { 200. }
fn dynamic() -> String { "dynamic".into() }
fn friction() -> f32 { 0.5 }
fn box_shape() -> String { "box".into() }
fn d250() -> f32 { 250. }
fn d400() -> f32 { 400. }
fn d600() -> f32 { 600. }
fn d700() -> f32 { 700. }
fn d800() -> f32 { 800. }
fn d1000() -> f32 { 1000. }
fn d1500() -> f32 { 1500. }
fn d0_2() -> f32 { 0.2 }

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    #[default]
    Standard,
    /// «Si no»: se ejecuta solo si el evento anterior (normal o «si no») no se cumplió, y sus condiciones sí.
    Else,
    /// `text` = cantidad (expresión).
    Repeat,
    /// `text` = objeto; cada iteración elige una sola instancia.
    ForEach,
    /// Repite mientras `conds` se cumplan (máx. 100000 vueltas).
    While,
    /// Solo agrupa sub-eventos. `text` = título.
    Group,
    Comment,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Event {
    #[serde(default, skip_serializing_if = "is_default")]
    pub kind: EventKind,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conds: Vec<Instr>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acts: Vec<Instr>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sub: Vec<Event>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub disabled: bool,
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

/// Condición o acción: `op` del catálogo (ver `catalog.rs`) + argumentos como texto.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Instr {
    pub op: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Condición invertida (por instancia, como en GDevelop).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub not: bool,
    /// Sub-condiciones de `or` / `and`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sub: Vec<Instr>,
}

impl Instr {
    pub fn new(op: &str, args: &[&str]) -> Self {
        Instr { op: op.into(), args: args.iter().map(|s| s.to_string()).collect(), ..Default::default() }
    }
}

/// Acción o condición propia hecha con eventos. Los parámetros llegan como variables locales.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Function {
    pub name: String,
    /// true = se usa como condición; devuelve con la acción `return`.
    #[serde(default)]
    pub is_condition: bool,
    #[serde(default)]
    pub params: Vec<String>,
    pub events: Vec<Event>,
    /// Lógica en nodos; se compila y se agrega después de `events`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph: Option<crate::nodos::Graph>,
}

impl Project {
    pub fn scene(&self, name: &str) -> Option<&Scene> {
        self.scenes.iter().find(|s| s.name == name)
    }
    /// Objetos visibles desde una escena: los suyos y los globales.
    pub fn objects_of<'a>(&'a self, scene: &'a Scene) -> impl Iterator<Item = &'a ObjectDef> {
        scene.objects.iter().chain(self.objects.iter())
    }
}
