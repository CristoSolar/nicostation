//! Vista previa dentro del editor («Jugar aquí»): corre el motor (`World`) con paso fijo de 60 Hz y lo dibuja con egui.
//! Lo puro (teclas, marco con franjas, reloj de paso fijo) tiene pruebas; el dibujo imita al reproductor (`src/bin/nicostation.rs`).

use eframe::egui::{self, Color32, FontId, Key, Pos2, Rect, Stroke, Vec2};
use nicostation::model::{Kind, Project};
use nicostation::runtime::{Cmd, Input, World};
use std::collections::{HashMap, HashSet, VecDeque};

/// Duración de un paso del motor (60 por segundo).
pub const PASO: f64 = 1. / 60.;
/// Pasos como máximo por cuadro del editor (si el editor se traba, el juego va más lento en vez de saltar).
pub const MAX_PASOS: u32 = 5;
/// Cuadros que recuerda el gráfico de rendimiento.
pub const HISTORIA: usize = 120;
pub const VELOCIDADES: [f64; 4] = [0.25, 0.5, 1., 2.];

// ---------- partes puras ----------

/// Nombre de tecla del motor (igual al `Debug` de `KeyCode` de macroquad, que usa el reproductor).
pub fn tecla(k: Key) -> Option<&'static str> {
    use Key::*;
    Some(match k {
        ArrowLeft => "Left",
        ArrowRight => "Right",
        ArrowUp => "Up",
        ArrowDown => "Down",
        Space => "Space",
        Enter => "Enter",
        Escape => "Escape",
        Tab => "Tab",
        Backspace => "Backspace",
        Delete => "Delete",
        Insert => "Insert",
        Home => "Home",
        End => "End",
        PageUp => "PageUp",
        PageDown => "PageDown",
        Minus => "Minus",
        Equals => "Equal",
        Comma => "Comma",
        Period => "Period",
        Slash => "Slash",
        Semicolon => "Semicolon",
        Num0 => "Key0",
        Num1 => "Key1",
        Num2 => "Key2",
        Num3 => "Key3",
        Num4 => "Key4",
        Num5 => "Key5",
        Num6 => "Key6",
        Num7 => "Key7",
        Num8 => "Key8",
        Num9 => "Key9",
        A => "A",
        B => "B",
        C => "C",
        D => "D",
        E => "E",
        F => "F",
        G => "G",
        H => "H",
        I => "I",
        J => "J",
        K => "K",
        L => "L",
        M => "M",
        N => "N",
        O => "O",
        P => "P",
        Q => "Q",
        R => "R",
        S => "S",
        T => "T",
        U => "U",
        V => "V",
        W => "W",
        X => "X",
        Y => "Y",
        Z => "Z",
        _ => return None,
    })
}

/// Dónde cae el área de juego dentro de un rectángulo (con franjas negras) y a qué escala.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Marco {
    /// Área de juego en pantalla.
    pub rect: Rect,
    /// Puntos de pantalla por unidad del juego.
    pub k: f32,
}

impl Marco {
    pub fn nuevo(area: Rect, w: f32, h: f32) -> Marco {
        let k = (area.width() / w.max(1.)).min(area.height() / h.max(1.)).max(0.0001);
        Marco { rect: Rect::from_center_size(area.center(), Vec2::new(w * k, h * k)), k }
    }
    /// Pantalla → coordenadas del juego (sin cámara: lo que el motor recibe como mouse).
    pub fn a_juego(&self, p: Pos2) -> (f32, f32) {
        ((p.x - self.rect.left()) / self.k, (p.y - self.rect.top()) / self.k)
    }
    pub fn a_pantalla(&self, x: f32, y: f32) -> Pos2 {
        Pos2::new(self.rect.left() + x * self.k, self.rect.top() + y * self.k)
    }
}

/// Acumula tiempo real y dice cuántos pasos fijos tocan.
#[derive(Default, Debug)]
pub struct Reloj {
    acc: f64,
}

impl Reloj {
    pub fn avanzar(&mut self, dt: f64, vel: f64) -> u32 {
        self.acc += dt.clamp(0., 0.25) * vel;
        let n = ((self.acc + 1e-9) / PASO).floor().max(0.);
        self.acc = (self.acc - n * PASO).max(0.);
        (n as u32).min(MAX_PASOS)
    }
    pub fn reiniciar(&mut self) {
        self.acc = 0.;
    }
}

/// Teclado y mouse que van al juego. Las teclas apretadas entre dos pasos se entregan en el siguiente paso.
#[derive(Default)]
pub struct Entrada {
    pub down: HashSet<String>,
    pressed: HashSet<String>,
    released: HashSet<String>,
    pub mouse: (f32, f32),
    pub mouse_down: [bool; 3],
    mouse_pressed: [bool; 3],
}

impl Entrada {
    /// Una tecla de egui (sin repeticiones).
    pub fn tecla(&mut self, k: Key, apretada: bool) {
        let Some(n) = tecla(k) else { return };
        self.cambiar(n, apretada);
    }
    fn cambiar(&mut self, n: &str, apretada: bool) {
        if apretada {
            if self.down.insert(n.to_string()) {
                self.pressed.insert(n.to_string());
            }
        } else if self.down.remove(n) {
            self.released.insert(n.to_string());
        }
    }
    /// Shift y Control no llegan como teclas en egui: se miran los modificadores.
    pub fn modificadores(&mut self, shift: bool, ctrl: bool) {
        self.cambiar("LeftShift", shift);
        self.cambiar("LeftControl", ctrl);
    }
    pub fn botones(&mut self, abajo: [bool; 3]) {
        for b in 0..3 {
            self.mouse_pressed[b] |= abajo[b] && !self.mouse_down[b];
        }
        self.mouse_down = abajo;
    }
    /// Suelta todo (se perdió el foco).
    pub fn soltar(&mut self) {
        for k in std::mem::take(&mut self.down) {
            self.released.insert(k);
        }
        self.mouse_down = [false; 3];
    }
    /// Entrada para un paso; lo «recién apretado» se consume.
    /// Un toque más corto que un paso cuenta como tecla apretada durante ese paso (si no, el motor no lo vería abajo).
    pub fn tomar(&mut self) -> Input {
        Input {
            down: self.down.union(&self.pressed).cloned().collect(),
            pressed: std::mem::take(&mut self.pressed),
            released: std::mem::take(&mut self.released),
            mouse: self.mouse,
            mouse_down: self.mouse_down,
            mouse_pressed: std::mem::take(&mut self.mouse_pressed), axes: Default::default() }
    }
}

/// Los sprites sin tamaño toman el de su primera imagen (como hace el reproductor antes de crear el mundo).
pub fn preparar(p: &mut Project, tam: impl Fn(&str) -> Option<[usize; 2]>) {
    for o in p.objects.iter_mut().chain(p.scenes.iter_mut().flat_map(|s| s.objects.iter_mut())) {
        if let Kind::Sprite { w, h, animations, .. } = &mut o.kind {
            if let Some([tw, th]) = animations.first().and_then(|a| a.frames.first()).and_then(|f| tam(f)) {
                if *w == 0. {
                    *w = tw as f32;
                }
                if *h == 0. {
                    *h = th as f32;
                }
            }
        }
    }
}

/// Imágenes que usa el proyecto (cuadros de animación y baldosas).
pub fn imagenes(p: &Project) -> Vec<String> {
    let mut v: Vec<String> = vec![];
    for o in p.objects.iter().chain(p.scenes.iter().flat_map(|s| s.objects.iter())) {
        match &o.kind {
            Kind::Sprite { animations, .. } => v.extend(animations.iter().flat_map(|a| a.frames.iter().cloned())),
            Kind::Tilemap { tileset, .. } if !tileset.is_empty() => v.push(tileset.clone()),
            Kind::Background { image, .. } if !image.is_empty() => v.push(image.clone()),
            Kind::Particles(d) if !d.image.is_empty() => v.push(d.image.clone()),
            _ => {}
        }
    }
    v.sort();
    v.dedup();
    v
}

/// ¿Está `p` dentro del cuadrilátero convexo `q`?
pub fn dentro(q: &[Pos2; 4], p: Pos2) -> bool {
    let mut signo = 0.;
    for i in 0..4 {
        let (a, b) = (q[i], q[(i + 1) % 4]);
        let c = (b - a).x * (p - a).y - (b - a).y * (p - a).x;
        if c.abs() < 1e-6 {
            continue;
        }
        if signo == 0. {
            signo = c.signum();
        } else if c.signum() != signo {
            return false;
        }
    }
    true
}

// ---------- sesión ----------

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tipo {
    Escena,
    Sonido,
    Guardado,
    Salir,
    Aviso,
    Info,
}

pub struct Linea {
    pub t: f64,
    pub tipo: Tipo,
    pub texto: String,
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Pestana {
    #[default]
    Cosas,
    Cajitas,
    Registro,
    Rendimiento,
    Perfil,
}

/// Preferencias de la vista previa (sobreviven a detener y volver a jugar).
pub struct Prefs {
    pub vel: f64,
    pub recargar: bool,
    /// Ocupa toda el área central (si no, queda abajo).
    pub grande: bool,
    /// Empieza en la escena que se está editando (si no, en la escena inicial).
    pub desde_aqui: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs { vel: 1., recargar: true, grande: true, desde_aqui: false }
    }
}

pub struct Sesion {
    pub world: World,
    pub tex: HashMap<String, egui::TextureHandle>,
    /// JSON del proyecto con que se armó el mundo (para recargar en caliente).
    pub json: String,
    pub reloj: Reloj,
    pub pausa: bool,
    pub entrada: Entrada,
    pub foco: bool,
    /// Marco del último cuadro (para pasar el mouse a coordenadas del juego).
    pub marco: Option<Marco>,
    /// Instancia elegida en el depurador (por id).
    pub sel: Option<u64>,
    pub registro: Vec<Linea>,
    /// Avisos del motor ya copiados al registro.
    avisos_vistos: usize,
    pub pasos_ms: VecDeque<f32>,
    pub fps: f32,
    t_prev: Option<f64>,
    /// Mensaje flotante y hasta cuándo (hora de egui).
    pub aviso: Option<(String, f64)>,
    pub ver_todo: bool,
    /// El juego pidió terminar.
    pub fin: bool,
    pub pestana: Pestana,
    pub pasos: u64,
    /// Cajitas en edición (nombre → texto), para no pisar lo que se escribe.
    pub editando: HashMap<String, String>,
    /// Darle el teclado al juego en el próximo cuadro (al empezar).
    pub pedir_foco: bool,
    /// Perfilador: (qué, ms promedio, ms máximo reciente, ms del último paso), suavizado entre pasos.
    pub perfil: Vec<(String, f32, f32, f32)>,
}

impl Sesion {
    pub fn nueva(world: World, tex: HashMap<String, egui::TextureHandle>, json: String) -> Sesion {
        let avisos_vistos = world.log.borrow().len();
        Sesion {
            world, tex, json, reloj: Reloj::default(), pausa: false, entrada: Entrada::default(), foco: false, marco: None, sel: None,
            registro: vec![], avisos_vistos, pasos_ms: VecDeque::new(), fps: 0., t_prev: None, aviso: None, ver_todo: false, fin: false,
            pestana: Pestana::default(), pasos: 0, editando: HashMap::new(), pedir_foco: true, perfil: vec![],
        }
    }

    pub fn anotar(&mut self, tipo: Tipo, texto: impl Into<String>) {
        self.registro.push(Linea { t: self.world.time, tipo, texto: texto.into() });
        if self.registro.len() > 500 {
            self.registro.remove(0);
        }
    }

    /// Lee teclado y mouse del cuadro. Si el juego tiene el foco, sus teclas no llegan al editor.
    pub fn leer(&mut self, ctx: &egui::Context) {
        let foco = self.foco;
        let marco = self.marco;
        ctx.input_mut(|i| {
            if foco {
                for e in &i.events {
                    if let egui::Event::Key { key, pressed, repeat: false, .. } = e {
                        self.entrada.tecla(*key, *pressed);
                    }
                }
                self.entrada.modificadores(i.modifiers.shift, i.modifiers.ctrl);
                i.events.retain(|e| match e {
                    egui::Event::Key { key, .. } => tecla(*key).is_none(),
                    egui::Event::Text(_) => false,
                    _ => true,
                });
            }
            if let (Some(m), Some(p)) = (marco, i.pointer.latest_pos()) {
                self.entrada.mouse = m.a_juego(p);
            }
            let sobre = marco.zip(i.pointer.latest_pos()).is_some_and(|(m, p)| m.rect.contains(p));
            let b = [egui::PointerButton::Primary, egui::PointerButton::Secondary, egui::PointerButton::Middle];
            let abajo = b.map(|b| i.pointer.button_down(b));
            // Clic sobre el juego: los botones van al juego (también el clic que le da el foco).
            if sobre || abajo.iter().zip(self.entrada.mouse_down).any(|(a, b)| *a && b) {
                self.entrada.botones(if self.pausa { [false; 3] } else { abajo });
            } else {
                self.entrada.botones([false; 3]);
            }
        });
        if !foco {
            self.entrada.soltar();
        }
    }

    /// Avanza el juego según el tiempo real del cuadro.
    pub fn avanzar(&mut self, ahora: f64, vel: f64) {
        let dt = self.t_prev.map_or(0., |t| ahora - t);
        self.t_prev = Some(ahora);
        if dt > 0. {
            let f = (1. / dt) as f32;
            self.fps = if self.fps == 0. { f } else { self.fps * 0.9 + f * 0.1 };
        }
        if self.pausa || self.fin {
            self.reloj.reiniciar();
            return;
        }
        for _ in 0..self.reloj.avanzar(dt, vel) {
            self.paso();
            if self.fin {
                break;
            }
        }
    }

    /// Un paso del motor, con medición y registro de lo que pidió el juego.
    pub fn paso(&mut self) {
        let input = self.entrada.tomar();
        let (escena, t_antes) = (self.world.scene, self.world.time);
        let t0 = std::time::Instant::now();
        self.world.step(PASO as f32, input);
        let ms = t0.elapsed().as_secs_f32() * 1000.;
        self.pasos += 1;
        self.pasos_ms.push_back(ms);
        while self.pasos_ms.len() > HISTORIA {
            self.pasos_ms.pop_front();
        }
        acumular_perfil(&mut self.perfil, &self.world.profile);
        let nombre = self.world.scene_def().name.clone();
        if self.world.scene != escena {
            self.anotar(Tipo::Escena, format!("Cambió a la escena «{nombre}»"));
            self.sel = None;
        } else if self.world.time < t_antes {
            self.anotar(Tipo::Escena, format!("La escena «{nombre}» empezó de nuevo"));
            self.sel = None;
        }
        for c in std::mem::take(&mut self.world.cmds) {
            match c {
                Cmd::ChannelVolume { channel, volume, .. } => self.anotar(Tipo::Sonido, format!("Volumen del canal «{channel}» a {volume}")),
                Cmd::StopChannel { channel } => self.anotar(Tipo::Sonido, format!("Se paró el canal «{channel}»")),
                Cmd::PlaySound { file, .. } => self.anotar(Tipo::Sonido, format!("Sonido «{file}»")),
                Cmd::PlayMusic { file, .. } => self.anotar(Tipo::Sonido, format!("Música «{file}»")),
                Cmd::StopMusic => self.anotar(Tipo::Sonido, "Se paró la música"),
                Cmd::StopSound { file } => self.anotar(Tipo::Sonido, format!("Se paró el sonido «{file}»")),
                Cmd::Persist(v) => self.anotar(Tipo::Guardado, format!("Partida guardada ({} dato{})", v.len(), if v.len() == 1 { "" } else { "s" })),
                Cmd::Quit => {
                    self.anotar(Tipo::Salir, "El juego pidió terminar");
                    self.fin = true;
                }
            }
        }
        let nuevos: Vec<String> = self.world.log.borrow().iter().skip(self.avisos_vistos).cloned().collect();
        self.avisos_vistos += nuevos.len();
        for a in nuevos {
            self.anotar(Tipo::Aviso, a);
        }
    }

    /// Recarga en caliente: el mundo nuevo sigue en la misma escena, con las globales y la partida guardada del anterior.
    pub fn cambiar_mundo(&mut self, mut w: World, tex: HashMap<String, egui::TextureHandle>, json: String) {
        conservar(&self.world, &mut w);
        let esc = self.world.scene_def().name.clone();
        w.load_scene(&esc);
        w.cmds.clear();
        self.avisos_vistos = 0;
        (self.world, self.tex, self.json) = (w, tex, json);
        self.sel = None;
        self.fin = false;
        self.reloj.reiniciar();
        self.anotar(Tipo::Info, "Cambios aplicados: la escena empezó de nuevo");
    }

    /// Índice de la instancia elegida (si sigue viva).
    pub fn elegida(&self) -> Option<usize> {
        let id = self.sel?;
        self.world.insts.iter().position(|i| i.id == id)
    }

    /// Reinicia la escena actual (las cajitas globales y la partida guardada se conservan).
    pub fn reiniciar_escena(&mut self) {
        let n = self.world.scene_def().name.clone();
        self.world.load_scene(&n);
        self.world.cmds.clear();
        self.fin = false;
        self.sel = None;
        self.reloj.reiniciar();
        self.anotar(Tipo::Escena, format!("Reiniciaste la escena «{n}»"));
    }

    /// Cámara para las capas que la siguen: la del juego, o una que muestra toda la escena.
    pub fn camara(&self) -> ((f32, f32), f32) {
        let w = &self.world;
        if !self.ver_todo {
            return (w.camera, w.zoom);
        }
        let (pw, ph) = (w.project.width, w.project.height);
        let mut r = Rect::from_min_max(Pos2::ZERO, Pos2::new(pw, ph));
        for (i, o) in w.insts.iter().enumerate() {
            if w.layer_fixed(&o.layer) {
                continue;
            }
            for q in w.shapes(i).iter().flatten().chain(o.hitbox().iter()) {
                r.extend_with(Pos2::new(q.0, q.1));
            }
        }
        let r = r.expand(16.);
        (((r.center().x), (r.center().y)), (pw / r.width()).min(ph / r.height()))
    }
}

/// Suma un paso del perfilador del motor a los promedios (los que dejan de aparecer se apagan de a poco).
pub fn acumular_perfil(acc: &mut Vec<(String, f32, f32, f32)>, paso: &[(String, f32)]) {
    for e in acc.iter_mut() {
        e.3 = 0.;
    }
    for (k, ms) in paso {
        match acc.iter_mut().find(|e| &e.0 == k) {
            Some(e) => e.3 += ms,
            // Nuevo: el promedio empieza en lo que midió.
            None => acc.push((k.clone(), -1., 0., *ms)),
        }
    }
    for e in acc.iter_mut() {
        e.1 = if e.1 < 0. { e.3 } else { e.1 * 0.9 + e.3 * 0.1 };
        e.2 = (e.2 * 0.995).max(e.3);
    }
    acc.retain(|e| e.1 > 0.0005 || e.3 > 0.);
}

/// Pasa la partida guardada y las globales de un mundo al nuevo (recarga en caliente).
fn conservar(viejo: &World, nuevo: &mut World) {
    nuevo.globals = viejo.globals.clone();
    nuevo.storage = viejo.storage.clone();
}

// ---------- dibujo ----------

/// Fuentes del proyecto que egui ya tiene listas (ruta → estado: 0 falló, 1 cargando, 2 lista).
pub type Fuentes = HashMap<String, u8>;

/// Familia de letra para una fuente del proyecto (la del editor mientras no esté lista).
pub fn familia(f: &Fuentes, font: &str) -> egui::FontFamily {
    if !font.is_empty() && f.get(font) == Some(&2) { egui::FontFamily::Name(font.into()) } else { egui::FontFamily::Proportional }
}

/// Texto armado con fuente, alineación y ancho de renglón. Devuelve el texto y cuánto correrlo a la derecha
/// desde la x del objeto (con alineación al centro o a la derecha, egui arma las filas alrededor de 0).
pub fn texto(painter: &egui::Painter, t: String, size: f32, color: Color32, fam: egui::FontFamily, align: &str, wrap: f32) -> (std::sync::Arc<egui::Galley>, f32) {
    let mut job = egui::text::LayoutJob::simple(t, FontId::new(size, fam), color, if wrap > 0. { wrap } else { f32::INFINITY });
    job.halign = match align {
        "center" => egui::Align::Center,
        "right" => egui::Align::RIGHT,
        _ => egui::Align::LEFT,
    };
    let g = painter.layout_job(job);
    let ancho = if wrap > 0. { wrap } else { g.size().x };
    let dx = match align {
        "center" => ancho / 2.,
        "right" => ancho,
        _ => 0.,
    };
    (g, dx)
}

/// Cámara de una capa con paralaje: con [1, 1] sigue a la cámara; con [0, 0] queda quieta; en el medio, se mueve menos.
/// La referencia es la cámara al centro de la pantalla (donde empieza el juego).
pub fn camara_capa(cam: (f32, f32), parallax: [f32; 2], pw: f32, ph: f32) -> (f32, f32) {
    (pw / 2. + (cam.0 - pw / 2.) * parallax[0], ph / 2. + (cam.1 - ph / 2.) * parallax[1])
}

/// Cuadrilátero girado de un rectángulo (x, y, w, h) alrededor de `piv`.
pub fn cuad(x: f32, y: f32, w: f32, h: f32, ang: f32, piv: (f32, f32)) -> [(f32, f32); 4] {
    let (s, c) = ang.to_radians().sin_cos();
    [(x, y), (x + w, y), (x + w, y + h), (x, y + h)].map(|(px, py)| {
        let (dx, dy) = (px - piv.0, py - piv.1);
        (piv.0 + dx * c - dy * s, piv.1 + dx * s + dy * c)
    })
}

/// Mosaico de una imagen que se repite dentro de `r` (pantalla), corrido `off` píxeles de pantalla; `t` = tamaño de una baldosa en pantalla.
pub fn mosaico(painter: &egui::Painter, r: Rect, tex: &egui::TextureHandle, t: Vec2, off: Vec2, col: Color32) {
    let vis = r.intersect(painter.clip_rect());
    if vis.width() <= 0. || vis.height() <= 0. || t.x < 0.5 || t.y < 0.5 {
        return;
    }
    let p = painter.with_clip_rect(vis);
    let ox = off.x.rem_euclid(t.x);
    let oy = off.y.rem_euclid(t.y);
    let x0 = r.left() + ox - t.x * (((r.left() + ox - vis.left()) / t.x).ceil());
    let y0 = r.top() + oy - t.y * (((r.top() + oy - vis.top()) / t.y).ceil());
    let mut mesh = egui::Mesh::with_texture(tex.id());
    let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.));
    let mut y = y0;
    let mut n = 0;
    while y < vis.bottom() && n < 20000 {
        let mut x = x0;
        while x < vis.right() && n < 20000 {
            mesh.add_rect_with_uv(Rect::from_min_size(Pos2::new(x, y), t + Vec2::splat(0.3)), uv, col);
            x += t.x;
            n += 1;
        }
        y += t.y;
    }
    p.add(egui::Shape::mesh(mesh));
}

/// Texturas de un sprite: la normal y, si están, la gris y la silueta blanca (para los efectos).
pub struct Texs<'a> {
    pub n: &'a egui::TextureHandle,
    pub gris: Option<&'a egui::TextureHandle>,
    pub blanco: Option<&'a egui::TextureHandle>,
}

fn malla(painter: &egui::Painter, q: &[Pos2; 4], uv: &[Pos2; 4], tex: egui::TextureId, c: Color32) {
    let mut mesh = egui::Mesh::with_texture(tex);
    for (p, u) in q.iter().zip(uv) {
        mesh.vertices.push(egui::epaint::Vertex { pos: *p, uv: *u, color: c });
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
}

/// Dibuja un sprite con un efecto aproximado: flash (destello blanco), outline (silueta blanca alrededor),
/// grayscale (gris), wave (ondas). `amount` 0..1; `t` segundos (para las ondas).
pub fn sprite_fx(painter: &egui::Painter, q: [Pos2; 4], flip: bool, tx: &Texs, c: Color32, fx: &str, amount: f32, t: f32) {
    let (u0, u1) = if flip { (1., 0.) } else { (0., 1.) };
    let uv = [Pos2::new(u0, 0.), Pos2::new(u1, 0.), Pos2::new(u1, 1.), Pos2::new(u0, 1.)];
    let a = amount.clamp(0., 1.);
    match fx {
        "grayscale" if tx.gris.is_some() => {
            malla(painter, &q, &uv, tx.gris.unwrap().id(), c);
            if a < 1. {
                malla(painter, &q, &uv, tx.n.id(), c.gamma_multiply(1. - a));
            }
        }
        "outline" => {
            let w = (q[1] - q[0]).length().max((q[3] - q[0]).length());
            let d = (w * 0.04).clamp(1., 3.) * a.max(0.2);
            let sil = tx.blanco.unwrap_or(tx.n);
            let borde = Color32::from_rgba_unmultiplied(255, 255, 255, (c.a() as f32 * a.max(0.2)) as u8);
            for (dx, dy) in [(-1., 0.), (1., 0.), (0., -1.), (0., 1.), (-0.7, -0.7), (0.7, -0.7), (-0.7, 0.7), (0.7, 0.7)] {
                let o = Vec2::new(dx, dy) * d;
                malla(painter, &q.map(|p| p + o), &uv, sil.id(), borde);
            }
            malla(painter, &q, &uv, tx.n.id(), c);
        }
        "wave" => {
            let n = 12;
            let (izq, der) = (|v: f32| q[0] + (q[3] - q[0]) * v, |v: f32| q[1] + (q[2] - q[1]) * v);
            let dir = (q[1] - q[0]).normalized();
            let amp = ((q[3] - q[0]).length() * 0.06).clamp(1., 8.) * a;
            let off = |k: usize| dir * (t * 6. + k as f32 * 0.9).sin() * amp;
            for k in 0..n {
                let (v0, v1) = (k as f32 / n as f32, (k + 1) as f32 / n as f32);
                let sq = [izq(v0) + off(k), der(v0) + off(k), der(v1) + off(k + 1), izq(v1) + off(k + 1)];
                let su = [Pos2::new(u0, v0), Pos2::new(u1, v0), Pos2::new(u1, v1), Pos2::new(u0, v1)];
                malla(painter, &sq, &su, tx.n.id(), c);
            }
        }
        "flash" => {
            malla(painter, &q, &uv, tx.n.id(), c);
            if let Some(b) = tx.blanco {
                malla(painter, &q, &uv, b.id(), Color32::from_white_alpha((c.a() as f32 * a) as u8));
            }
        }
        _ => malla(painter, &q, &uv, tx.n.id(), c),
    }
}

/// Color con efecto para lo que no tiene imagen (cajas y textos): destello hacia blanco y gris.
pub fn color_fx(c: [u8; 3], fx: &str, amount: f32) -> [u8; 3] {
    let a = amount.clamp(0., 1.);
    match fx {
        "flash" => c.map(|v| (v as f32 + (255. - v as f32) * a).round() as u8),
        "grayscale" => {
            let l = 0.299 * c[0] as f32 + 0.587 * c[1] as f32 + 0.114 * c[2] as f32;
            c.map(|v| (v as f32 + (l - v as f32) * a).round() as u8)
        }
        _ => c,
    }
}

/// Versión gris (1) o silueta blanca (2) de una imagen, para los efectos.
pub fn variante(img: &image::RgbaImage, var: u8) -> egui::ColorImage {
    let mut v = img.clone();
    for px in v.pixels_mut() {
        let [r, g, b, a] = px.0;
        px.0 = if var == 1 {
            let l = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32).round() as u8;
            [l, l, l, a]
        } else {
            [255, 255, 255, a]
        };
    }
    egui::ColorImage::from_rgba_unmultiplied([v.width() as usize, v.height() as usize], v.as_raw())
}


fn rgba(c: [u8; 3], a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], a)
}

/// Dibuja el juego en `area`. Devuelve el cuadrilátero en pantalla de cada instancia dibujada (en orden de dibujo).
pub fn dibujar(painter: &egui::Painter, area: Rect, s: &Sesion, fuentes: &Fuentes) -> (Marco, Vec<(usize, [Pos2; 4])>) {
    let w = &s.world;
    let p = &w.project;
    let (pw, ph) = (p.width, p.height);
    let m = Marco::nuevo(area, pw, ph);
    let painter = painter.with_clip_rect(m.rect.intersect(painter.clip_rect()));
    let sc = w.scene_def();
    painter.rect_filled(m.rect, 0., rgba(sc.background, 255));
    let (cam_juego, zoom_juego) = s.camara();
    let tiempo = w.time as f32;
    let mut quads = vec![];
    for layer in sc.layers.iter().filter(|l| l.visible) {
        let (cam, z) = if layer.fixed { ((pw / 2., ph / 2.), 1.) } else { (camara_capa(cam_juego, layer.parallax, pw, ph), zoom_juego) };
        let pant = |x: f32, y: f32| m.a_pantalla((x - cam.0) * z + pw / 2., (y - cam.1) * z + ph / 2.);
        let k = z * m.k;
        let mut orden: Vec<usize> = (0..w.insts.len()).filter(|&i| w.insts[i].layer == layer.name).collect();
        orden.sort_by(|&a, &b| w.insts[a].z.total_cmp(&w.insts[b].z));
        for i in orden {
            let o = &w.insts[i];
            let Some(def) = w.object_def(&o.obj) else { continue };
            if !o.visible {
                continue;
            }
            let alpha = o.opacity.clamp(0., 255.) as u8;
            // El color de la instancia multiplica al del dibujo.
            let mul = |c: [u8; 3]| rgba([0, 1, 2].map(|k| (c[k] as u16 * o.tint[k] as u16 / 255) as u8), alpha);
            let quad = o.hitbox().map(|(x, y)| pant(x, y));
            let fx = o.effect.as_str();
            match &def.kind {
                Kind::Background { image, scroll, .. } => {
                    if let Some(t) = s.tex.get(image) {
                        let r = Rect::from_min_max(pant(o.x, o.y), pant(o.x + o.w, o.y + o.h));
                        let ts = t.size_vec2() * k;
                        let off = Vec2::new(scroll[0], scroll[1]) * tiempo * k;
                        mosaico(&painter, r, t, ts, off, mul(color_fx([255; 3], fx, o.effect_amount)));
                    }
                }
                Kind::Particles(_) => {} // las partículas vivas se dibujan después de la capa
                // ponytail: la vista previa 3D dentro del editor llega con el visor 3D (etapa F7).
                Kind::Mesh { .. } | Kind::Light { .. } | Kind::Camera3d { .. } => {}
                Kind::Box { color, .. } => {
                    painter.add(egui::Shape::convex_polygon(quad.to_vec(), mul(color_fx(*color, fx, o.effect_amount)), Stroke::NONE));
                    if fx == "outline" {
                        painter.add(egui::Shape::closed_line(quad.to_vec(), Stroke::new(2., Color32::WHITE)));
                    }
                }
                Kind::Sprite { animations, .. } => {
                    let f = animations.get(o.anim).and_then(|a| a.frames.get(o.frame));
                    match f.and_then(|f| s.tex.get(f).map(|t| (f, t))) {
                        Some((f, t)) => {
                            // Rectángulo de dibujo del motor (tiene en cuenta el origen).
                            let (x0, y0, ww, hh, px, py) = o.draw_rect();
                            let q = cuad(x0, y0, ww, hh, o.angle, (px, py)).map(|(x, y)| pant(x, y));
                            let tx = Texs { n: t, gris: s.tex.get(&format!("gris:{f}")), blanco: s.tex.get(&format!("blanco:{f}")) };
                            sprite_fx(&painter, q, o.flip_x, &tx, mul([255; 3]), fx, o.effect_amount, tiempo);
                            quads.push((i, q));
                            continue;
                        }
                        None => drop(painter.add(egui::Shape::convex_polygon(quad.to_vec(), Color32::from_rgb(255, 0, 255), Stroke::NONE))),
                    }
                }
                Kind::Tilemap { tileset, tile, cols, .. } => {
                    let Some(t) = s.tex.get(tileset) else { continue };
                    let [tw, th] = t.size().map(|v| v as f32);
                    let per_row = (tw / tile).floor().max(1.) as i32;
                    let (vw, vh) = (pw / z, ph / z);
                    let c0 = ((cam.0 - vw / 2. - o.x) / tile).floor().max(0.) as usize;
                    let c1 = ((cam.0 + vw / 2. - o.x) / tile).ceil().max(0.) as usize;
                    let r0 = ((cam.1 - vh / 2. - o.y) / tile).floor().max(0.) as usize;
                    let r1 = ((cam.1 + vh / 2. - o.y) / tile).ceil().max(0.) as usize;
                    let rows = o.tiles.len() / (*cols).max(1);
                    let mut mesh = egui::Mesh::with_texture(t.id());
                    let c = mul(color_fx([255; 3], fx, o.effect_amount));
                    for r in r0..r1.min(rows) {
                        for cc in c0..c1.min(*cols) {
                            let n = o.tiles[r * cols + cc];
                            if n < 0 {
                                continue;
                            }
                            let at = pant(o.x + cc as f32 * tile, o.y + r as f32 * tile);
                            let (sx, sy) = ((n % per_row) as f32 * tile, (n / per_row) as f32 * tile);
                            let uv = Rect::from_min_max(Pos2::new(sx / tw, sy / th), Pos2::new((sx + tile) / tw, (sy + tile) / th));
                            // +0.5 px para que no se vean rayas entre baldosas al escalar
                            mesh.add_rect_with_uv(Rect::from_min_size(at, Vec2::splat(tile * k + 0.5)), uv, c);
                        }
                    }
                    painter.add(egui::Shape::mesh(mesh));
                }
                Kind::Text { size, color, font, align, wrap, .. } => {
                    if size * k >= 1. {
                        let c = mul(color_fx(*color, fx, o.effect_amount));
                        let (g, dx) = texto(&painter, w.display_text(i), size * k, c, familia(fuentes, font), align, wrap * k);
                        let at = pant(o.x, o.y) + Vec2::new(dx, 0.);
                        let ts = egui::epaint::TextShape::new(at, g, c);
                        painter.add(if o.angle != 0. { ts.with_angle_and_anchor(o.angle.to_radians(), egui::Align2::LEFT_TOP) } else { ts });
                    }
                }
            }
            quads.push((i, quad));
        }
        // Partículas de esta capa.
        for q in w.particles.iter().filter(|q| q.layer == layer.name) {
            let Some(Kind::Particles(d)) = w.object_def(&q.obj).map(|d| &d.kind) else { continue };
            let (c, a, sz) = nicostation::particulas::particle_look(d, q);
            crate::particulas::dibujar(&painter, pant(q.x, q.y), sz * k, crate::particulas::color(c, a / 255., d.additive), s.tex.get(&d.image));
        }
    }
    // Con «ver toda la escena»: dónde está la cámara de verdad.
    if s.ver_todo {
        let (cam, z) = (cam_juego, zoom_juego);
        let pant = |x: f32, y: f32| m.a_pantalla((x - cam.0) * z + pw / 2., (y - cam.1) * z + ph / 2.);
        let (hw, hh) = (pw / 2. / w.zoom, ph / 2. / w.zoom);
        let r = Rect::from_min_max(pant(w.camera.0 - hw, w.camera.1 - hh), pant(w.camera.0 + hw, w.camera.1 + hh));
        painter.rect_stroke(r, 0., Stroke::new(2., Color32::from_rgb(0x4c, 0x97, 0xff)), egui::StrokeKind::Middle);
    }
    (m, quads)
}

/// El tamaño real de los textos (para choques y «cursor sobre»), como hace el reproductor.
pub fn medir_textos(ctx: &egui::Context, w: &mut World, fuentes: &Fuentes) {
    for i in 0..w.insts.len() {
        if let Some(Kind::Text { size, font, wrap, .. }) = w.object_def(&w.insts[i].obj).map(|d| d.kind.clone()) {
            let t = w.display_text(i);
            let fam = familia(fuentes, &font);
            let job = egui::text::LayoutJob::simple(t, FontId::new(size, fam), Color32::WHITE, if wrap > 0. { wrap } else { f32::INFINITY });
            let sz = ctx.fonts_mut(|f| f.layout_job(job).size());
            w.insts[i].w = if wrap > 0. { wrap } else { sz.x };
            if wrap > 0. {
                w.insts[i].h = sz.y;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn teclas_como_macroquad() {
        assert_eq!(tecla(Key::ArrowLeft), Some("Left"));
        assert_eq!(tecla(Key::ArrowDown), Some("Down"));
        assert_eq!(tecla(Key::Space), Some("Space"));
        assert_eq!(tecla(Key::Enter), Some("Enter"));
        assert_eq!(tecla(Key::Escape), Some("Escape"));
        assert_eq!(tecla(Key::Num1), Some("Key1"));
        assert_eq!(tecla(Key::Num0), Some("Key0"));
        assert_eq!(tecla(Key::A), Some("A"));
        assert_eq!(tecla(Key::Z), Some("Z"));
        assert_eq!(tecla(Key::F6), None);
    }

    #[test]
    fn entrada_apretar_y_soltar() {
        let mut e = Entrada::default();
        e.tecla(Key::Space, true);
        e.tecla(Key::Space, true); // repetición: no vuelve a «apretar»
        let i = e.tomar();
        assert!(i.down.contains("Space") && i.pressed.contains("Space"));
        let i = e.tomar();
        assert!(i.down.contains("Space") && i.pressed.is_empty());
        e.tecla(Key::Space, false);
        let i = e.tomar();
        assert!(!i.down.contains("Space") && i.released.contains("Space"));
        // Toque rápido entre dos pasos: igual se entrega apretada y soltada.
        e.tecla(Key::A, true);
        e.tecla(Key::A, false);
        let i = e.tomar();
        assert!(i.pressed.contains("A") && i.released.contains("A") && i.down.contains("A"));
        assert!(!e.tomar().down.contains("A"));
        e.modificadores(true, false);
        assert!(e.tomar().pressed.contains("LeftShift"));
        e.botones([true, false, false]);
        e.botones([true, false, false]);
        let i = e.tomar();
        assert!(i.mouse_pressed[0] && i.mouse_down[0]);
        assert!(!e.tomar().mouse_pressed[0]);
    }

    #[test]
    fn marco_con_franjas() {
        // Área más ancha que el juego: franjas a los costados.
        let m = Marco::nuevo(Rect::from_min_size(Pos2::new(100., 50.), Vec2::new(1000., 300.)), 800., 600.);
        assert!((m.k - 0.5).abs() < 1e-5);
        assert_eq!(m.rect, Rect::from_min_size(Pos2::new(400., 50.), Vec2::new(400., 300.)));
        let (x, y) = m.a_juego(Pos2::new(600., 200.));
        assert!((x - 400.).abs() < 1e-3 && (y - 300.).abs() < 1e-3);
        let p = m.a_pantalla(x, y);
        assert!((p - Pos2::new(600., 200.)).length() < 1e-3);
        // Área más alta: franjas arriba y abajo.
        let m = Marco::nuevo(Rect::from_min_size(Pos2::ZERO, Vec2::new(400., 1000.)), 800., 400.);
        assert_eq!(m.rect, Rect::from_min_size(Pos2::new(0., 400.), Vec2::new(400., 200.)));
        assert_eq!(m.a_juego(Pos2::new(0., 400.)), (0., 0.));
    }

    #[test]
    fn reloj_de_paso_fijo() {
        let mut r = Reloj::default();
        assert_eq!(r.avanzar(1. / 60., 1.), 1);
        assert_eq!(r.avanzar(1. / 120., 1.), 0);
        assert_eq!(r.avanzar(1. / 120., 1.), 1);
        // A 30 cuadros por segundo: 2 pasos por cuadro.
        assert_eq!((0..30).map(|_| r.avanzar(1. / 30., 1.)).sum::<u32>(), 60);
        // Cámara lenta: un cuarto de los pasos.
        assert_eq!((0..60).map(|_| r.avanzar(1. / 60., 0.25)).sum::<u32>(), 15);
        // Al doble: dos por cuadro.
        assert_eq!(r.avanzar(1. / 60., 2.), 2);
        // Un tirón largo no da más de 5 pasos ni deja deuda.
        assert_eq!(r.avanzar(2., 1.), MAX_PASOS);
        assert_eq!(r.avanzar(0., 1.), 0);
    }

    #[test]
    fn punto_en_cuadrilatero() {
        let q = [Pos2::new(0., 0.), Pos2::new(10., 0.), Pos2::new(10., 10.), Pos2::new(0., 10.)];
        assert!(dentro(&q, Pos2::new(5., 5.)));
        assert!(!dentro(&q, Pos2::new(15., 5.)));
        let rombo = [Pos2::new(5., 0.), Pos2::new(10., 5.), Pos2::new(5., 10.), Pos2::new(0., 5.)];
        assert!(dentro(&rombo, Pos2::new(5., 5.)));
        assert!(!dentro(&rombo, Pos2::new(1., 1.)));
    }

    fn proyecto(fondo: u8) -> Project {
        serde_json::from_str(&format!(r#"{{"name":"t","width":100,"height":100,"first_scene":"A","globals":{{"vidas":3}},"scenes":[
            {{"name":"A","background":[{fondo},0,0],"objects":[{{"name":"C","type":"box","w":10,"h":10,"color":[1,2,3]}}],
              "instances":[{{"obj":"C","x":0,"y":0}}],
              "events":[{{"conds":[{{"op":"key_pressed","args":["Space"]}}],"acts":[{{"op":"change_scene","args":["B"]}}]}}]}},
            {{"name":"B","vars":{{"n":1}},"events":[{{"acts":[{{"op":"play_sound","args":["s.wav","100"]}}]}}]}}]}}"#)).unwrap()
    }

    #[test]
    fn sesion_registra_y_recarga_en_caliente() {
        let mut s = Sesion::nueva(World::new(proyecto(10)), HashMap::new(), "v1".into());
        s.paso();
        s.entrada.tecla(Key::Space, true);
        s.paso();
        assert_eq!(s.world.scene_def().name, "B");
        assert!(s.registro.iter().any(|l| l.tipo == Tipo::Escena && l.texto.contains("«B»")));
        s.paso();
        assert!(s.registro.iter().any(|l| l.tipo == Tipo::Sonido && l.texto.contains("s.wav")));
        // Cambios: el mundo nuevo queda en la escena B y conserva globales y partida guardada.
        s.world.globals.insert("vidas".into(), nicostation::model::Value::Num(1.));
        s.world.storage.insert("record".into(), nicostation::model::Value::Num(9.));
        s.cambiar_mundo(World::new(proyecto(200)), HashMap::new(), "v2".into());
        assert_eq!(s.json, "v2");
        assert_eq!(s.world.scene_def().name, "B");
        assert_eq!(s.world.globals["vidas"], nicostation::model::Value::Num(1.));
        assert_eq!(s.world.storage["record"], nicostation::model::Value::Num(9.));
        assert_eq!(s.world.project.scenes[0].background[0], 200);
        // Reiniciar la escena y la acción Salir.
        s.reiniciar_escena();
        assert!(!s.fin && s.world.time == 0.);
    }

    #[test]
    fn perfil_suavizado() {
        let mut a = vec![];
        acumular_perfil(&mut a, &[("Eventos".into(), 2.), ("Plataformas".into(), 1.), ("Eventos".into(), 1.)]);
        assert_eq!(a.len(), 2);
        assert_eq!((a[0].1, a[0].2, a[0].3), (3., 3., 3.));
        acumular_perfil(&mut a, &[("Plataformas".into(), 2.)]);
        assert!((a[0].1 - 2.7).abs() < 1e-4 && a[0].3 == 0.);
        assert!((a[1].1 - 1.1).abs() < 1e-4 && a[1].2 == 2.);
    }

    #[test]
    fn paralaje_origen_y_efectos() {
        // Paralaje: con la cámara al centro no se corre nada; a la mitad se mueve la mitad.
        assert_eq!(camara_capa((50., 50.), [0.5, 0.5], 100., 100.), (50., 50.));
        assert_eq!(camara_capa((150., 50.), [0.5, 0.], 100., 100.), (100., 50.));
        assert_eq!(camara_capa((150., 80.), [1., 1.], 100., 100.), (150., 80.));
        let q = cuad(0., 0., 10., 10., 90., (5., 5.));
        assert!((q[0].0 - 10.).abs() < 1e-4 && q[0].1.abs() < 1e-4);
        assert_eq!(color_fx([0, 100, 200], "flash", 1.), [255, 255, 255]);
        assert_eq!(color_fx([0, 100, 200], "flash", 0.), [0, 100, 200]);
        let g = color_fx([255, 0, 0], "grayscale", 1.);
        assert!(g[0] == g[1] && g[1] == g[2]);
        assert_eq!(color_fx([1, 2, 3], "wave", 1.), [1, 2, 3]);
        let img = image::RgbaImage::from_raw(1, 1, vec![10, 200, 30, 128]).unwrap();
        assert_eq!(variante(&img, 2).pixels[0], Color32::from_rgba_unmultiplied(255, 255, 255, 128));
    }

    #[test]
    fn sprites_toman_tamano_de_la_imagen() {
        let mut p: Project = serde_json::from_str(r#"{"name":"t","width":100,"height":100,"first_scene":"E","scenes":[{"name":"E",
            "objects":[{"name":"S","type":"sprite","animations":[{"name":"a","frames":["a.png"]}]},
                       {"name":"F","type":"sprite","w":5,"animations":[{"name":"a","frames":["a.png"]}]}]}]}"#).unwrap();
        preparar(&mut p, |f| (f == "a.png").then_some([12, 7]));
        let sz: Vec<(f32, f32)> = p.scenes[0].objects.iter().map(|o| match o.kind { Kind::Sprite { w, h, .. } => (w, h), _ => (0., 0.) }).collect();
        assert_eq!(sz, vec![(12., 7.), (5., 7.)]);
        assert_eq!(imagenes(&p), vec!["a.png".to_string()]);
    }
}
