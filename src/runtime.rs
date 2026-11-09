//! Motor en ejecución. No depende de ninguna ventana: recibe `Input` y `dt`, produce estado y `Cmd`.
//! Semántica de eventos y selección de objetos según GDevelop (ver docs/gdevelop-eventos-y-formato.md §2.4).

use crate::expr::{self, Ctx, Expr};
use crate::model::*;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Default, Clone)]
pub struct Input {
    pub down: HashSet<String>,
    pub pressed: HashSet<String>,
    pub released: HashSet<String>,
    /// Posición del mouse en pantalla.
    pub mouse: (f32, f32),
    /// Left, Right, Middle
    pub mouse_down: [bool; 3],
    pub mouse_pressed: [bool; 3],
    /// Ejes de mandos: "Pad1LX", "Pad1LY", "Pad1RX", "Pad1RY", "Pad1LT", "Pad1RT" de -1 a 1.
    /// Los botones de los mandos llegan como teclas: "Pad1A", "Pad1B", "Pad1X", "Pad1Y", "Pad1Start",
    /// "Pad1Select", "Pad1L", "Pad1R", "Pad1Up", "Pad1Down", "Pad1Left", "Pad1Right".
    pub axes: HashMap<String, f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Cmd {
    /// `channel` vacío = sin canal; con canal, reemplaza lo que sonaba en ese canal.
    PlaySound { file: String, volume: f32, channel: String },
    /// `fade`: segundos de fundido de entrada (y de salida de la música anterior).
    PlayMusic { file: String, volume: f32, fade: f32 },
    /// Cambiar el volumen de un canal ("musica" = la música) en `fade` segundos.
    ChannelVolume { channel: String, volume: f32, fade: f32 },
    StopChannel { channel: String },
    StopMusic,
    StopSound { file: String },
    /// Hay que guardar la partida: el reproductor escribe estos datos (archivo o localStorage).
    Persist(Vars),
    Quit,
}

#[derive(Clone, Default)]
pub struct Inst {
    pub id: u64,
    pub obj: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub angle: f32,
    pub z: f32,
    pub opacity: f32,
    pub layer: String,
    pub flip_x: bool,
    pub anim: usize,
    pub frame: usize,
    anim_time: f32,
    pub anim_done: bool,
    pub text: String,
    pub vars: Vars,
    pub dead: bool,
    // estado de comportamientos
    vx: f32,
    vy: f32,
    v_jump: f32,
    jump_time: f32,
    jump_held: bool,
    want_jump: bool,
    pub on_floor: bool,
    pub jumping: bool,
    last_axis_x: bool,
    /// Estado del Platformer: FALLING, ON_FLOOR, JUMPING, GRABBING, ON_LADDER.
    pstate: u8,
    /// Suelo actual (id) y su última posición, para que una plataforma móvil lo arrastre.
    floor: Option<(u64, f32, f32)>,
    /// Jumpthru que se está atravesando hacia abajo.
    ignore_jt: Option<u64>,
    /// Baldosas de un mapa (copia de las del objeto; las acciones pueden cambiarlas).
    pub tiles: Vec<i32>,
    /// Se dibuja (las invisibles igual chocan y corren sus eventos).
    pub visible: bool,
    /// 3D: profundidad y giros (pitch inclina, roll de costado; `angle` gira alrededor del eje vertical).
    pub d: f32,
    pub base_d: f32,
    pub pitch: f32,
    pub roll: f32,
    /// Color que multiplica al dibujo (blanco = sin cambio).
    pub tint: [u8; 3],
    /// Tamaño original, para scale_x / scale_y.
    pub base_w: f32,
    pub base_h: f32,
    /// Efecto visual: none, flash, outline, grayscale, wave.
    pub effect: String,
    /// Intensidad del efecto (0 a 1).
    pub effect_amount: f32,
    /// Alarmas en marcha: (nombre, segundos que faltan).
    pub alarms: Vec<(String, f32)>,
    /// Alarmas que sonaron en este frame.
    pub fired: Vec<String>,
    tweens: Vec<Tween>,
    /// Punto de origen de un sprite como fracción de w/h (0..1): x, y es ese punto, el giro y la escala pivotan ahí.
    /// None = x, y es la esquina superior izquierda y gira alrededor del centro (lo de siempre).
    pub origin: Option<(f32, f32)>,
    /// Máscara de choque convexa como fracciones de w/h (0..1). Vacía = el rectángulo completo.
    pub mask: Vec<(f32, f32)>,
    /// Emisor de partículas: está soltando partículas (prop «emitting»), resto acumulado y ráfaga pendiente.
    emitting: bool,
    emit_acc: f32,
    burst: u32,
    /// Velocidad de giro pedida antes de que exista su cuerpo físico (grados/s).
    angvel0: f32,
    /// Búsqueda de caminos en curso (Pathfinder).
    nav: Option<Box<crate::caminos::Camino>>,
}

/// Cambio suave de una propiedad numérica.
#[derive(Clone, Default)]
struct Tween {
    prop: String,
    from: f32,
    to: f32,
    t: f32,
    dur: f32,
    ease: String,
}

/// Curvas de suavizado (las clásicas de Robert Penner, como en GDevelop y GameMaker). t va de 0 a 1.
pub fn ease(name: &str, t: f32) -> f32 {
    use std::f32::consts::PI;
    let bounce = |t: f32| {
        let (n, d) = (7.5625, 2.75);
        if t < 1. / d { n * t * t } else if t < 2. / d { let t = t - 1.5 / d; n * t * t + 0.75 }
        else if t < 2.5 / d { let t = t - 2.25 / d; n * t * t + 0.9375 } else { let t = t - 2.625 / d; n * t * t + 0.984375 }
    };
    match name {
        "ease_in" => t * t,
        "ease_out" => 1. - (1. - t) * (1. - t),
        "ease_in_out" => if t < 0.5 { 2. * t * t } else { 1. - (-2. * t + 2.).powi(2) / 2. },
        "back" => { let c = 1.70158; 1. + (c + 1.) * (t - 1.).powi(3) + c * (t - 1.).powi(2) }
        "elastic" => if t <= 0. || t >= 1. { t } else { 2f32.powf(-10. * t) * ((t * 10. - 0.75) * (2. * PI / 3.)).sin() + 1. },
        "bounce" => bounce(t),
        _ => t,
    }
}

const FALLING: u8 = 0;
const ON_FLOOR: u8 = 1;
const JUMPING: u8 = 2;
const GRABBING: u8 = 3;
const ON_LADDER: u8 = 4;

#[derive(Clone, Copy, PartialEq)]
enum PKind {
    Solid,
    Jumpthru,
    Ladder,
}

/// Plataforma vista por el Platformer.
#[derive(Clone, Copy)]
struct Plat {
    /// Instancia dueña (un mapa de baldosas aporta varias formas).
    i: usize,
    poly: [(f32, f32); 4],
    kind: PKind,
    grabbable: bool,
}

/// Una partícula viva. La simula el motor; los reproductores la dibujan con los parámetros del emisor (`obj`).
#[derive(Clone, Debug, Default)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub age: f32,
    pub life: f32,
    /// Objeto emisor (para leer su ParticleDef) y capa donde se dibuja.
    pub obj: String,
    pub layer: String,
    /// Id de la instancia emisora.
    pub emitter: u64,
}

/// Instancias elegidas por objeto real (no por grupo). Objeto ausente = todas sus instancias.
pub type Sol = HashMap<String, Vec<usize>>;

/// Resto de un evento que quedó esperando («esperar N segundos»).
// ponytail: las instrucciones se copian; «solo una vez» y «cada X segundos» dentro de lo que sigue a un «esperar»
// cuentan como instrucciones nuevas en cada espera.
struct Task {
    at: f64,
    acts: Vec<Instr>,
    sub: Vec<Event>,
    /// Instancias elegidas, por id (los índices cambian entre frames).
    sol: HashMap<String, Vec<u64>>,
}

pub struct World {
    pub project: Rc<Project>,
    pub globals: Vars,
    pub scene: usize,
    pub insts: Vec<Inst>,
    pub vars: Vars,
    pub camera: (f32, f32),
    pub zoom: f32,
    /// Cámara que sigue a una instancia: (id, suavidad 0 = rígida .. 0.99 = muy suave).
    cam_follow: Option<(u64, f32)>,
    /// Límites de la cámara (x0, y0, x1, y1): no muestra nada fuera de este rectángulo.
    cam_bounds: Option<(f32, f32, f32, f32)>,
    /// Temblor: (intensidad en px, segundos que quedan) y el desplazamiento aplicado este frame.
    cam_shake: (f32, f32),
    cam_shake_off: (f32, f32),
    /// Zoom animado: (desde, hasta, t, duración).
    zoom_tween: Option<(f32, f32, f32, f32)>,
    pub time: f64,
    pub dt: f32,
    pub input: Input,
    pub cmds: Vec<Cmd>,
    /// Partida guardada (la carga el reproductor al empezar; sobrevive a cerrar el juego).
    pub storage: Vars,
    /// La escena actual es 3D.
    pub dim3: bool,
    /// Partículas vivas de todos los emisores.
    pub particles: Vec<Particle>,
    /// Perfilador: (qué, milisegundos) del último paso — eventos de primer nivel y comportamientos.
    pub profile: Vec<(String, f32)>,
    /// Punto donde pegó el último `raycast` (o el final del rayo si no pegó): expresiones ray_x, ray_y.
    pub ray_hit: (f32, f32),
    fisica: crate::fisica2d::Fisica,
    /// El próximo `run_events` es el de primer nivel del paso: mide cada evento para el perfilador.
    prof_top: bool,
    tasks: Vec<Task>,
    types: HashMap<String, ObjectDef>,
    timers: HashMap<String, f64>,
    every: HashMap<usize, f64>,
    once_now: HashSet<usize>,
    once_last: HashSet<usize>,
    locals: Vec<Vars>,
    ret: Option<bool>,
    first_frame: bool,
    pending: Option<String>,
    dragging: Option<(u64, f32, f32)>,
    next_id: u64,
    rng: Cell<u64>,
    cache: RefCell<HashMap<String, Rc<Expr>>>,
    /// Avisos del motor (instrucciones desconocidas, expresiones con error...), sin repetir.
    pub log: RefCell<Vec<String>>,
}

impl World {
    pub fn new(mut project: Project) -> Self {
        migrate(&mut project);
        // Los grafos de nodos se compilan a eventos: el resto del motor solo ve eventos.
        let project = project.compiled();
        let first = project.first_scene.clone();
        let mut w = World {
            globals: project.globals.clone(),
            project: Rc::new(project),
            scene: 0,
            insts: vec![],
            vars: Vars::new(),
            camera: (0., 0.),
            zoom: 1.,
            cam_follow: None,
            cam_bounds: None,
            cam_shake: (0., 0.),
            cam_shake_off: (0., 0.),
            zoom_tween: None,
            time: 0.,
            dt: 0.,
            input: Input::default(),
            cmds: vec![],
            storage: Vars::new(),
            dim3: false,
            particles: vec![],
            profile: vec![],
            ray_hit: (0., 0.),
            fisica: Default::default(),
            prof_top: false,
            tasks: vec![],
            types: HashMap::new(),
            timers: HashMap::new(),
            every: HashMap::new(),
            once_now: HashSet::new(),
            once_last: HashSet::new(),
            locals: vec![],
            ret: None,
            first_frame: true,
            pending: None,
            dragging: None,
            next_id: 0,
            rng: Cell::new(0x9E3779B97F4A7C15),
            cache: RefCell::new(HashMap::new()),
            log: RefCell::new(vec![]),
        };
        w.load_scene(&first);
        w
    }

    pub(crate) fn warn(&self, m: String) {
        let mut log = self.log.borrow_mut();
        if !log.contains(&m) {
            eprintln!("{m}");
            log.push(m);
        }
    }

    pub(crate) fn request_scene(&mut self, name: &str) {
        self.pending = Some(name.into());
    }

    pub fn scene_def(&self) -> &Scene {
        &self.project.scenes[self.scene]
    }

    pub fn object_def(&self, name: &str) -> Option<&ObjectDef> {
        self.types.get(name)
    }

    pub fn load_scene(&mut self, name: &str) {
        let p = self.project.clone();
        self.scene = p.scenes.iter().position(|s| s.name == name).unwrap_or(0);
        let sc = &p.scenes[self.scene];
        self.types = p.objects_of(sc).map(|o| (o.name.clone(), o.clone())).collect();
        self.dim3 = sc.dim3;
        self.insts.clear();
        self.vars = sc.vars.clone();
        self.timers.clear();
        self.tasks.clear();
        self.every.clear();
        self.once_now.clear();
        self.once_last.clear();
        self.camera = (p.width / 2., p.height / 2.);
        self.zoom = 1.;
        (self.cam_follow, self.cam_bounds, self.cam_shake, self.cam_shake_off, self.zoom_tween) = (None, None, (0., 0.), (0., 0.), None);
        self.time = 0.;
        self.first_frame = true;
        self.dragging = None;
        self.fisica = Default::default();
        self.particles.clear();
        self.profile.clear();
        self.ray_hit = (0., 0.);
        for i in &sc.instances {
            if !self.types.contains_key(&i.obj) {
                continue;
            }
            let k = self.spawn(&i.obj, i.x, i.y, &i.layer);
            let me = &mut self.insts[k];
            me.angle = i.angle;
            me.z = i.z;
            if let Some(w) = i.w { me.w = w }
            if let Some(h) = i.h { me.h = h }
            if let Some(d) = i.d { me.d = d }
            (me.pitch, me.roll) = (i.pitch, i.roll);
            me.vars.extend(i.vars.clone());
        }
    }

    pub fn spawn(&mut self, obj: &str, x: f32, y: f32, layer: &str) -> usize {
        let t = &self.types[obj];
        let (w, h, text) = match &t.kind {
            Kind::Sprite { w, h, .. } => (*w, *h, String::new()),
            Kind::Text { text, size, .. } => (text.chars().count() as f32 * size * 0.5, *size, text.clone()),
            Kind::Box { w, h, .. } => (*w, *h, String::new()),
            Kind::Tilemap { tile, cols, rows, .. } => (*cols as f32 * tile, *rows as f32 * tile, String::new()),
            Kind::Mesh { size, .. } => (size[0], size[1], String::new()),
            Kind::Light { .. } | Kind::Camera3d { .. } => (0.5, 0.5, String::new()),
            Kind::Background { w, h, .. } => (*w, *h, String::new()),
            Kind::Particles(_) => (16., 16., String::new()),
        };
        let d = match &t.kind {
            Kind::Mesh { size, .. } => size[2],
            _ => 0.5,
        };
        let tiles = match &t.kind {
            Kind::Tilemap { cols, rows, data, .. } => (0..cols * rows).map(|k| data.get(k).copied().unwrap_or(-1)).collect(),
            _ => vec![],
        };
        // Origen y máscara en píxeles del tamaño base → fracciones (siguen valiendo si la instancia cambia de tamaño).
        let (origin, mask) = match &t.kind {
            Kind::Sprite { origin, mask, .. } if w > 0. && h > 0. => (
                origin.map(|o| (o[0] / w, o[1] / h)),
                if mask.len() >= 3 { convex_hull(mask.iter().map(|p| (p[0] / w, p[1] / h)).collect()) } else { vec![] },
            ),
            _ => (None, vec![]),
        };
        let (emitting, burst) = match &t.kind {
            Kind::Particles(d) => (true, d.burst),
            _ => (false, 0),
        };
        self.next_id += 1;
        self.insts.push(Inst {
            origin,
            mask,
            emitting,
            burst,
            id: self.next_id,
            obj: obj.into(),
            x,
            y,
            w,
            h,
            opacity: 255.,
            layer: layer.into(),
            text,
            vars: t.vars.clone(),
            tiles,
            visible: true,
            effect: t.effect.clone(),
            effect_amount: 1.,
            tint: [255, 255, 255],
            base_w: w,
            base_h: h,
            d,
            base_d: d,
            ..Default::default()
        });
        self.insts.len() - 1
    }

    /// Formas de choque: el rectángulo rotado, o en un mapa de baldosas las tiras de baldosas sólidas de cada fila.
    pub fn shapes(&self, i: usize) -> Vec<[(f32, f32); 4]> {
        let me = &self.insts[i];
        let Some(Kind::Tilemap { tile, cols, pass, .. }) = self.types.get(&me.obj).map(|t| &t.kind) else {
            return vec![me.hitbox()];
        };
        let solid = |k: usize| me.tiles.get(k).is_some_and(|&t| t >= 0 && !pass.contains(&t));
        let mut out = vec![];
        for r in 0..me.tiles.len() / cols.max(&1) {
            let mut c = 0;
            while c < *cols {
                if !solid(r * cols + c) {
                    c += 1;
                    continue;
                }
                let start = c;
                while c < *cols && solid(r * cols + c) {
                    c += 1;
                }
                let (x0, x1, y0, y1) = (me.x + start as f32 * tile, me.x + c as f32 * tile, me.y + r as f32 * tile, me.y + (r + 1) as f32 * tile);
                out.push([(x0, y0), (x1, y0), (x1, y1), (x0, y1)]);
            }
        }
        out
    }

    /// Formas de choque reales: como `shapes`, pero un sprite con máscara usa su polígono convexo.
    /// Es lo que usan las colisiones, «cursor sobre», «separar», el raycast y los obstáculos de Pathfinder.
    pub fn polys(&self, i: usize) -> Vec<Vec<(f32, f32)>> {
        let me = &self.insts[i];
        if !me.mask.is_empty() {
            return vec![me.mask_poly()];
        }
        self.shapes(i).into_iter().map(|s| s.to_vec()).collect()
    }

    /// Choques 3D: cajas alineadas a los ejes, centradas en (x, y, z) con tamaño (w, h, d).
    // ponytail: AABB sin rotación hasta la física 3D (rapier3d, etapa F4).
    fn hits3(&self, mine: &[usize], others: &[usize], dist: f32) -> Vec<(usize, usize)> {
        let mut out = vec![];
        for &i in mine {
            let a = &self.insts[i];
            for &j in others {
                let b = &self.insts[j];
                if i == j {
                    continue;
                }
                let hit = if dist > 0. {
                    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)).sqrt() < dist
                } else {
                    (a.x - b.x).abs() * 2. < a.w + b.w && (a.y - b.y).abs() * 2. < a.h + b.h && (a.z - b.z).abs() * 2. < a.d + b.d
                };
                if hit {
                    out.push((i, j));
                }
            }
        }
        out
    }

    /// Pares (de `mine`, de `others`) que se tocan (o están a menos de `dist` entre centros si dist > 0).
    /// Grilla espacial de celdas de 128 px: cada instancia solo se compara con las de sus celdas.
    fn hits(&self, mine: &[usize], others: &[usize], dist: f32) -> Vec<(usize, usize)> {
        if self.dim3 {
            return self.hits3(mine, others, dist);
        }
        type Poly = Vec<(f32, f32)>;
        let bounds = |s: &[Poly]| s.iter().flatten().fold((f32::MAX, f32::MAX, f32::MIN, f32::MIN), |b, p| (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1)));
        // Dos cuerpos físicos apoyados uno en otro pueden quedar a una fracción de píxel: también vale el contacto de rapier.
        let phys = !self.fisica.is_empty();
        let touching = |i: usize, j: usize| phys && self.fisica.tocando(self.insts[i].id, self.insts[j].id);
        const CELL: f32 = 128.;
        let cells = |b: (f32, f32, f32, f32)| {
            let (x0, y0, x1, y1) = ((b.0 / CELL).floor() as i32, (b.1 / CELL).floor() as i32, (b.2 / CELL).floor() as i32, (b.3 / CELL).floor() as i32);
            (x0, y0, x1, y1, (x1 - x0 + 1) as i64 * (y1 - y0 + 1) as i64)
        };
        let oth: Vec<(usize, Vec<Poly>, (f32, f32, f32, f32))> = others.iter().map(|&j| {
            let s = self.polys(j);
            let b = bounds(&s);
            (j, s, b)
        }).filter(|o| !o.1.is_empty()).collect();
        let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        let mut big = vec![]; // instancias enormes (mapas): se prueban contra todas
        for (k, o) in oth.iter().enumerate() {
            let (x0, y0, x1, y1, n) = cells(o.2);
            if n > 256 {
                big.push(k);
                continue;
            }
            for cx in x0..=x1 {
                for cy in y0..=y1 {
                    grid.entry((cx, cy)).or_default().push(k);
                }
            }
        }
        let mut out = vec![];
        let mut seen = vec![usize::MAX; oth.len()];
        for &i in mine {
            let si = self.polys(i);
            if si.is_empty() {
                continue;
            }
            let bi = bounds(&si);
            let bq = (bi.0 - dist, bi.1 - dist, bi.2 + dist, bi.3 + dist);
            let (x0, y0, x1, y1, n) = cells(bq);
            let mut cand: Vec<usize> = big.clone();
            if n > 1024 {
                cand = (0..oth.len()).collect();
            } else {
                for cx in x0..=x1 {
                    for cy in y0..=y1 {
                        cand.extend(grid.get(&(cx, cy)).into_iter().flatten().copied());
                    }
                }
            }
            for k in cand {
                if seen[k] == i {
                    continue;
                }
                seen[k] = i;
                let (j, sj, bj) = &oth[k];
                if *j == i {
                    continue;
                }
                if bq.0 > bj.2 || bj.0 > bq.2 || bq.1 > bj.3 || bj.1 > bq.3 {
                    if dist <= 0. && touching(i, *j) {
                        out.push((i, *j));
                    }
                    continue;
                }
                let hit = if dist > 0. {
                    (self.insts[i].cx() - self.insts[*j].cx()).hypot(self.insts[i].cy() - self.insts[*j].cy()) < dist
                } else {
                    si.iter().any(|a| sj.iter().any(|b| sat(a, b).is_some())) || touching(i, *j)
                };
                if hit {
                    out.push((i, *j));
                }
            }
        }
        out
    }

    // ---------- frame ----------

    /// Avanza un frame. Orden como GDevelop: animaciones y comportamientos → eventos → post-eventos.
    pub fn step(&mut self, dt: f32, input: Input) {
        self.dt = dt.min(0.05);
        self.time += self.dt as f64;
        self.input = input;
        self.animate();
        self.tick_alarms();
        self.tick_tweens();
        self.run_tasks();
        self.profile.clear();
        let t = reloj::Reloj::ahora();
        self.behaviors_pre();
        let t_beh = t.ms();
        let t = reloj::Reloj::ahora();
        self.physics_step();
        let t_phys = t.ms();
        self.once_last = std::mem::take(&mut self.once_now);
        let p = self.project.clone();
        self.prof_top = reloj::ACTIVO;
        self.run_events(&p.scenes[self.scene].events, &Sol::new());
        self.prof_top = false;
        let t = reloj::Reloj::ahora();
        self.behaviors_post();
        let t_beh = t_beh + t.ms();
        let t = reloj::Reloj::ahora();
        self.particles_step();
        let t_part = t.ms();
        if reloj::ACTIVO {
            self.profile.push(("Comportamientos".into(), t_beh));
            if !self.fisica.is_empty() {
                self.profile.push(("Física".into(), t_phys));
            }
            if !self.particles.is_empty() {
                self.profile.push(("Partículas".into(), t_part));
            }
        }
        self.update_camera();
        self.insts.retain(|i| !i.dead);
        self.first_frame = false;
        if let Some(s) = self.pending.take() {
            self.load_scene(&s);
        }
    }

    fn tick_tweens(&mut self) {
        let dt = self.dt;
        for i in 0..self.insts.len() {
            if self.insts[i].tweens.is_empty() {
                continue;
            }
            let mut tw = std::mem::take(&mut self.insts[i].tweens);
            for t in &mut tw {
                t.t = (t.t + dt).min(t.dur);
                let k = if t.dur > 0. { ease(&t.ease, t.t / t.dur) } else { 1. };
                self.put(i, &t.prop, Value::Num((t.from + (t.to - t.from) * k) as f64));
            }
            tw.retain(|t| t.t < t.dur);
            self.insts[i].tweens = tw;
        }
    }

    /// Seguir, límites, temblor y zoom animado. Se aplica después de los eventos, como en GameMaker.
    fn update_camera(&mut self) {
        let dt = self.dt;
        // Quitar el temblor del frame anterior antes de mover la cámara.
        self.camera.0 -= self.cam_shake_off.0;
        self.camera.1 -= self.cam_shake_off.1;
        if let Some((from, to, t, dur)) = self.zoom_tween {
            let t = (t + dt).min(dur);
            self.zoom = from + (to - from) * ease("ease_in_out", if dur > 0. { t / dur } else { 1. });
            self.zoom_tween = (t < dur).then_some((from, to, t, dur));
        }
        if let Some((id, smooth)) = self.cam_follow {
            match self.insts.iter().find(|i| i.id == id && !i.dead) {
                Some(me) => {
                    // Suavidad independiente de los FPS: a 60 FPS se acerca (1 - smooth) del camino por frame.
                    let k = 1. - smooth.clamp(0., 0.99).powf(dt * 60.);
                    self.camera.0 += (me.cx() - self.camera.0) * k;
                    self.camera.1 += (me.cy() - self.camera.1) * k;
                }
                None => self.cam_follow = None,
            }
        }
        if let Some((x0, y0, x1, y1)) = self.cam_bounds {
            let (hw, hh) = (self.project.width / self.zoom / 2., self.project.height / self.zoom / 2.);
            let clamp = |v: f32, lo: f32, hi: f32| if hi - lo < 0. { (lo + hi) / 2. } else { v.clamp(lo, hi) };
            self.camera = (clamp(self.camera.0, x0 + hw, x1 - hw), clamp(self.camera.1, y0 + hh, y1 - hh));
        }
        self.cam_shake_off = (0., 0.);
        if self.cam_shake.1 > 0. {
            self.cam_shake.1 -= dt;
            let a = self.cam_shake.0;
            self.cam_shake_off = (((self.random() * 2. - 1.) as f32) * a, ((self.random() * 2. - 1.) as f32) * a);
            self.camera.0 += self.cam_shake_off.0;
            self.camera.1 += self.cam_shake_off.1;
        }
    }

    fn tick_alarms(&mut self) {
        let dt = self.dt;
        for me in &mut self.insts {
            me.fired.clear();
            for (name, t) in &mut me.alarms {
                *t -= dt;
                if *t <= 0. {
                    me.fired.push(name.clone());
                }
            }
            me.alarms.retain(|a| a.1 > 0.);
        }
    }

    /// Continúa los eventos que estaban esperando («esperar N segundos»), con sus mismas instancias.
    fn run_tasks(&mut self) {
        let now = self.time;
        let (ready, wait): (Vec<Task>, Vec<Task>) = std::mem::take(&mut self.tasks).into_iter().partition(|t| t.at <= now);
        self.tasks = wait;
        for t in ready {
            let mut sol = Sol::new();
            for (obj, ids) in &t.sol {
                sol.insert(obj.clone(), ids.iter().filter_map(|id| self.insts.iter().position(|i| i.id == *id && !i.dead)).collect());
            }
            self.body(&t.acts, &t.sub, &mut sol);
        }
    }

    /// Acciones y luego sub-eventos; si aparece «esperar», el resto queda para después.
    fn body(&mut self, acts: &[Instr], sub: &[Event], sol: &mut Sol) {
        match acts.iter().position(|a| a.op == "wait") {
            None => {
                self.acts(acts, sol);
                self.run_events(sub, sol);
            }
            Some(k) => {
                self.acts(&acts[..k], sol);
                let secs = self.ev(acts[k].args.first().map(String::as_str).unwrap_or("0"), sol, None).num().max(0.);
                let ids = sol.iter().map(|(o, l)| (o.clone(), l.iter().map(|&i| self.insts[i].id).collect())).collect();
                self.tasks.push(Task { at: self.time + secs, acts: acts[k + 1..].to_vec(), sub: sub.to_vec(), sol: ids });
            }
        }
    }

    fn run_events(&mut self, evs: &[Event], sol: &Sol) {
        // Cadena normal → «si no» → «si no»: ¿ya se cumplió algún eslabón? (None = no hay cadena abierta)
        let mut chain: Option<bool> = None;
        // Perfilador: solo los eventos de primer nivel de la escena (los anidados cuentan dentro de su padre).
        let top = std::mem::take(&mut self.prof_top);
        for (n, e) in evs.iter().enumerate() {
            if e.disabled || self.pending.is_some() {
                continue;
            }
            let t = top.then(reloj::Reloj::ahora);
            self.run_event(e, sol, &mut chain);
            if let Some(t) = t {
                let ms = t.ms();
                self.profile.push((event_label(n, e), ms));
            }
        }
    }

    fn run_event(&mut self, e: &Event, sol: &Sol, chain: &mut Option<bool>) {
        {
            match e.kind {
                EventKind::Comment => {}
                EventKind::Standard => *chain = Some(self.run_once(e, sol.clone())),
                EventKind::Else => {
                    if *chain != Some(true) {
                        *chain = Some(self.run_once(e, sol.clone()));
                    }
                }
                _ => *chain = None,
            }
            match e.kind {
                EventKind::Comment | EventKind::Standard | EventKind::Else => {}
                EventKind::Group => self.run_events(&e.sub, sol),
                EventKind::Repeat => {
                    let n = self.ev(&e.text, sol, None).num().max(0.) as usize;
                    for _ in 0..n {
                        self.run_once(e, sol.clone());
                    }
                }
                EventKind::ForEach => {
                    for i in self.pick(&e.text, sol) {
                        if !self.insts[i].dead {
                            let mut s = sol.clone();
                            self.set_picked(&e.text, &mut s, vec![i]);
                            self.run_once(e, s);
                        }
                    }
                }
                EventKind::While => {
                    for _ in 0..100_000 {
                        let mut s = sol.clone();
                        if !self.conds(&e.conds, &mut s) {
                            break;
                        }
                        self.acts(&e.acts, &mut s);
                        self.run_events(&e.sub, &s);
                    }
                }
            }
        }
    }

    fn run_once(&mut self, e: &Event, mut s: Sol) -> bool {
        let ok = self.conds(&e.conds, &mut s);
        if ok {
            self.body(&e.acts, &e.sub, &mut s);
        }
        ok
    }

    // ---------- selección ----------

    /// Grupo → sus objetos; objeto → él mismo.
    fn expand(&self, name: &str) -> Vec<String> {
        match self.scene_def().groups.get(name) {
            Some(g) => g.clone(),
            None => vec![name.to_string()],
        }
    }

    fn all_of(&self, obj: &str) -> impl Iterator<Item = usize> + '_ {
        let obj = obj.to_string();
        (0..self.insts.len()).filter(move |&i| self.insts[i].obj == obj && !self.insts[i].dead)
    }

    pub fn pick(&self, name: &str, sol: &Sol) -> Vec<usize> {
        let mut out = vec![];
        for o in self.expand(name) {
            match sol.get(&o) {
                Some(v) => out.extend(v.iter().copied().filter(|&i| !self.insts[i].dead)),
                None => out.extend(self.all_of(&o)),
            }
        }
        out
    }

    fn set_picked(&self, name: &str, sol: &mut Sol, list: Vec<usize>) {
        for o in self.expand(name) {
            sol.insert(o.clone(), list.iter().copied().filter(|&i| self.insts[i].obj == o).collect());
        }
    }

    /// Filtra las instancias de `name` por un predicado (negado si `not`). Verdadero si queda alguna.
    fn filter(&mut self, name: &str, not: bool, sol: &mut Sol, f: impl Fn(&World, usize) -> bool) -> bool {
        let keep: Vec<usize> = self.pick(name, sol).into_iter().filter(|&i| f(self, i) != not).collect();
        let ok = !keep.is_empty();
        self.set_picked(name, sol, keep);
        ok
    }

    // ---------- condiciones ----------

    fn conds(&mut self, cs: &[Instr], sol: &mut Sol) -> bool {
        cs.iter().all(|c| self.cond(c, sol))
    }

    fn cond(&mut self, c: &Instr, sol: &mut Sol) -> bool {
        let a = |n: usize| c.args.get(n).map(String::as_str).unwrap_or("");
        let id = c as *const Instr as usize;
        let free = |b: bool| b != c.not;
        match c.op.as_str() {
            "start" => free(self.first_frame),
            "trigger_once" => {
                self.once_now.insert(id);
                free(!self.once_last.contains(&id))
            }
            "every" => {
                let secs = self.ev(a(0), sol, None).num();
                let last = *self.every.entry(id).or_insert(self.time);
                let fire = self.time - last >= secs;
                if fire {
                    self.every.insert(id, self.time);
                }
                free(fire)
            }
            "timer" => {
                let start = *self.timers.entry(a(0).into()).or_insert(self.time);
                let secs = self.ev(a(1), sol, None).num();
                free(self.time - start >= secs)
            }
            "key_down" => free(self.input.down.contains(a(0))),
            "key_pressed" => free(self.input.pressed.contains(a(0))),
            "key_released" => free(self.input.released.contains(a(0))),
            "mouse_down" => free(self.input.mouse_down[button(a(0))]),
            "mouse_clicked" => free(self.input.mouse_pressed[button(a(0))]),
            "compare" => {
                let (l, r) = (self.ev(a(0), sol, None), self.ev(a(2), sol, None));
                free(compare(&l, a(1), &r))
            }
            "script_if" => {
                let r = crate::script::run(self, sol, a(0)).is_some_and(|v| v.truthy());
                free(r)
            }
            "call_cond" => {
                self.call(c, sol);
                let r = self.ret.take().unwrap_or(false);
                free(r)
            }
            "or" => {
                let mut any = false;
                let mut merged: Sol = HashMap::new();
                for s in &c.sub {
                    let mut copy = sol.clone();
                    if self.cond(s, &mut copy) {
                        any = true;
                        for (k, v) in copy {
                            let e = merged.entry(k).or_default();
                            for i in v {
                                if !e.contains(&i) {
                                    e.push(i);
                                }
                            }
                        }
                    }
                }
                sol.extend(merged);
                any != c.not
            }
            "and" => self.conds(&c.sub, sol) != c.not,
            "pick_all" => {
                for o in self.expand(a(0)) {
                    sol.remove(&o);
                }
                true
            }
            "pick_random" => {
                let l = self.pick(a(0), sol);
                if l.is_empty() {
                    return false;
                }
                let k = l[(self.random() * l.len() as f64) as usize % l.len()];
                self.set_picked(a(0), sol, vec![k]);
                true
            }
            "pick_nearest" => {
                let (x, y) = (self.ev(a(1), sol, None).num() as f32, self.ev(a(2), sol, None).num() as f32);
                let l = self.pick(a(0), sol);
                let best = l.into_iter().min_by(|&i, &j| {
                    let d = |k: usize| (self.insts[k].cx() - x).hypot(self.insts[k].cy() - y);
                    d(i).total_cmp(&d(j))
                });
                match best {
                    Some(k) => {
                        self.set_picked(a(0), sol, vec![k]);
                        true
                    }
                    None => false,
                }
            }
            "collides" | "near" => {
                let others = self.pick(a(1), sol);
                let dist = if c.op == "near" { self.ev(a(2), sol, None).num() as f32 } else { 0. };
                let mine = self.pick(a(0), sol);
                let pairs = self.hits(&mine, &others, dist);
                if c.not {
                    // Invertida: solo se filtra la primera lista (las que no tocan a ninguna).
                    let touching: HashSet<usize> = pairs.iter().map(|p| p.0).collect();
                    return self.filter(a(0), false, sol, |_, i| !touching.contains(&i));
                }
                let (mut ka, mut kb) = (vec![], vec![]);
                for (i, j) in pairs {
                    if !ka.contains(&i) { ka.push(i) }
                    if !kb.contains(&j) { kb.push(j) }
                }
                let ok = !ka.is_empty();
                self.set_picked(a(0), sol, ka);
                self.set_picked(a(1), sol, kb);
                ok
            }
            "alarm" => {
                let name = a(1).to_string();
                self.filter(a(0), c.not, sol, |w, i| w.insts[i].fired.contains(&name))
            }
            "cursor_on" => self.filter(a(0), c.not, sol, |w, i| {
                let (mx, my) = w.mouse_on(&w.insts[i].layer);
                w.polys(i).iter().any(|s| point_in(s, mx, my))
            }),
            "compare_prop" => {
                let (prop, op, val) = (a(1).to_string(), a(2).to_string(), a(3).to_string());
                let snapshot = sol.clone();
                self.filter(a(0), c.not, sol, |w, i| compare(&w.get(i, &prop), &op, &w.ev(&val, &snapshot, Some(i))))
            }
            "animation_is" => {
                let name = a(1).to_string();
                self.filter(a(0), c.not, sol, |w, i| w.anim_name(i) == name)
            }
            "animation_finished" => self.filter(a(0), c.not, sol, |w, i| w.insts[i].anim_done),
            "on_floor" => self.filter(a(0), c.not, sol, |w, i| w.insts[i].on_floor),
            "jumping" => self.filter(a(0), c.not, sol, |w, i| w.insts[i].jumping),
            "falling" => self.filter(a(0), c.not, sol, |w, i| w.insts[i].pstate == FALLING && w.has_platformer(i)),
            "on_ladder" => self.filter(a(0), c.not, sol, |w, i| w.insts[i].pstate == ON_LADDER),
            "grabbing" => self.filter(a(0), c.not, sol, |w, i| w.insts[i].pstate == GRABBING),
            "tweening" => self.filter(a(0), c.not, sol, |w, i| !w.insts[i].tweens.is_empty()),
            "is_moving" => self.filter(a(0), c.not, sol, |w, i| w.insts[i].vx != 0. || w.insts[i].vy != 0.),
            "path_found" => self.filter(a(0), c.not, sol, |w, i| w.insts[i].nav.as_ref().is_some_and(|n| n.encontrado)),
            "path_done" => self.filter(a(0), c.not, sol, |w, i| w.insts[i].nav.as_ref().is_some_and(|n| n.llego)),
            "raycast" => {
                // Rayo desde el centro de la primera instancia elegida de a(0); elige la instancia de a(3) que toca primero.
                let Some(&from) = self.pick(a(0), sol).first() else { return free(false) };
                let ang = (self.ev(a(1), sol, Some(from)).num() as f32).to_radians();
                let dist = (self.ev(a(2), sol, Some(from)).num() as f32).max(0.);
                let (ox, oy) = (self.insts[from].cx(), self.insts[from].cy());
                let (dx, dy) = (ang.cos(), ang.sin());
                let mut best: Option<(f32, usize)> = None;
                for j in self.pick(a(3), sol) {
                    if j == from {
                        continue;
                    }
                    for poly in self.polys(j) {
                        if let Some(t) = ray_poly((ox, oy), (dx, dy), dist, &poly) {
                            if best.is_none_or(|b| t < b.0) {
                                best = Some((t, j));
                            }
                        }
                    }
                }
                let t = best.map_or(dist, |b| b.0);
                self.ray_hit = (ox + dx * t, oy + dy * t);
                if c.not {
                    return best.is_none();
                }
                match best {
                    Some((_, j)) => {
                        self.set_picked(a(3), sol, vec![j]);
                        true
                    }
                    None => false,
                }
            }
            op => {
                self.warn(format!("condición desconocida: {op}"));
                false
            }
        }
    }

    // ---------- acciones ----------

    fn acts(&mut self, acts: &[Instr], sol: &mut Sol) {
        for a in acts {
            self.act(a, sol);
        }
    }

    fn act(&mut self, c: &Instr, sol: &mut Sol) {
        let a = |n: usize| c.args.get(n).map(String::as_str).unwrap_or("");
        let num = |w: &World, s: &str, sol: &Sol, cur: Option<usize>| w.ev(s, sol, cur).num() as f32;
        match c.op.as_str() {
            "create" => {
                if !self.types.contains_key(a(0)) {
                    return self.warn(format!("crear: objeto desconocido «{}»", a(0)));
                }
                let (x, y) = (num(self, a(1), sol, None), num(self, a(2), sol, None));
                let k = self.spawn(a(0), x, y, a(3));
                // Si el objeto no estaba elegido, la selección empieza vacía: queda solo el nuevo.
                sol.entry(a(0).into()).or_default().push(k);
            }
            "delete" => {
                for i in self.pick(a(0), sol) {
                    self.insts[i].dead = true;
                }
            }
            "set" | "add" => {
                for i in self.pick(a(0), sol) {
                    let v = self.ev(a(2), sol, Some(i));
                    let v = if c.op == "add" { add(&self.get(i, a(1)), &v) } else { v };
                    self.put(i, a(1), v);
                }
            }
            "set_text" => {
                for i in self.pick(a(0), sol) {
                    self.insts[i].text = self.ev(a(1), sol, Some(i)).text();
                }
            }
            "set_animation" => {
                for i in self.pick(a(0), sol) {
                    let idx = match &self.types[&self.insts[i].obj].kind {
                        Kind::Sprite { animations, .. } => animations.iter().position(|x| x.name == a(1)),
                        _ => None,
                    };
                    if let Some(idx) = idx.filter(|&k| k != self.insts[i].anim) {
                        let me = &mut self.insts[i];
                        (me.anim, me.frame, me.anim_time, me.anim_done) = (idx, 0, 0., false);
                    }
                }
            }
            "flip_x" => {
                for i in self.pick(a(0), sol) {
                    self.insts[i].flip_x = self.ev(a(1), sol, Some(i)).truthy();
                }
            }
            "look_at" => {
                for i in self.pick(a(0), sol) {
                    let (x, y) = (num(self, a(1), sol, Some(i)), num(self, a(2), sol, Some(i)));
                    let me = &mut self.insts[i];
                    me.angle = (y - me.cy()).atan2(x - me.cx()).to_degrees();
                }
            }
            "separate" => {
                let obstacles = self.pick(a(1), sol);
                for i in self.pick(a(0), sol) {
                    for &j in &obstacles {
                        if i != j {
                            for shape in self.polys(j) {
                                if let Some((dx, dy)) = sat(&self.insts[i].mask_poly(), &shape) {
                                    self.insts[i].x += dx;
                                    self.insts[i].y += dy;
                                }
                            }
                        }
                    }
                }
            }
            "set_tile" => {
                for i in self.pick(a(0), sol) {
                    let cols = match &self.types[&self.insts[i].obj].kind {
                        Kind::Tilemap { cols, .. } => *cols,
                        _ => continue,
                    };
                    let (c, r, t) = (num(self, a(1), sol, Some(i)), num(self, a(2), sol, Some(i)), num(self, a(3), sol, Some(i)));
                    let k = r as i64 * cols as i64 + c as i64;
                    if c >= 0. && (c as usize) < cols && k >= 0 && (k as usize) < self.insts[i].tiles.len() {
                        self.insts[i].tiles[k as usize] = t as i32;
                    }
                }
            }
            "jump" => {
                for i in self.pick(a(0), sol) {
                    self.insts[i].want_jump = true;
                }
            }
            "set_var" | "add_var" | "set_global" | "add_global" => {
                let v = self.ev(a(1), sol, None);
                let global = c.op.ends_with("global");
                let scope = match self.locals.last_mut() {
                    Some(l) if !global && l.contains_key(a(0)) => l,
                    _ if global => &mut self.globals,
                    _ => &mut self.vars,
                };
                let old = scope.get(a(0)).cloned().unwrap_or(Value::Num(0.));
                scope.insert(a(0).into(), if c.op.starts_with("add") { add(&old, &v) } else { v });
            }
            "list_add" | "list_set" | "list_remove" | "list_clear" => {
                let (v, k) = (self.ev(a(1), sol, None), self.ev(a(2), sol, None));
                let name = a(0).to_string();
                let global = !self.vars.contains_key(&name) && self.globals.contains_key(&name);
                let scope = if global { &mut self.globals } else { &mut self.vars };
                // Una cajita nueva es estructura si se le pone un campo con nombre, lista si no.
                let fresh = if c.op == "list_set" && matches!(v, Value::Str(_)) { Value::Map(Default::default()) } else { Value::List(vec![]) };
                let cur = scope.entry(name).or_insert(fresh);
                match (c.op.as_str(), cur) {
                    ("list_add", Value::List(l)) => l.push(v),
                    // list_set(var, índice o campo, valor)
                    ("list_set", Value::List(l)) => {
                        let i = v.num().max(0.) as usize;
                        if i >= l.len() {
                            l.resize(i + 1, Value::Num(0.));
                        }
                        l[i] = k;
                    }
                    ("list_set", Value::Map(m)) => drop(m.insert(v.text(), k)),
                    ("list_remove", Value::List(l)) => {
                        let i = v.num() as usize;
                        if v.num() >= 0. && i < l.len() {
                            l.remove(i);
                        }
                    }
                    ("list_remove", Value::Map(m)) => drop(m.remove(&v.text())),
                    ("list_clear", Value::List(l)) => l.clear(),
                    ("list_clear", Value::Map(m)) => m.clear(),
                    // Una cajita que no era colección pasa a serlo.
                    ("list_set", other) if !matches!(v, Value::Num(_)) => *other = Value::Map([(v.text(), k)].into()),
                    (op, other) => {
                        *other = Value::List(vec![]);
                        if op == "list_add" {
                            *other = Value::List(vec![v]);
                        }
                    }
                }
            }
            "reset_timer" => {
                self.timers.insert(a(0).into(), self.time);
            }
            "change_scene" => self.pending = Some(a(0).into()),
            "restart_scene" => self.pending = Some(self.scene_def().name.clone()),
            "camera" => {
                self.cam_follow = None;
                self.camera = (num(self, a(0), sol, None), num(self, a(1), sol, None));
            }
            "camera_zoom" => self.zoom = num(self, a(0), sol, None).max(0.01),
            "play_sound" | "play_music" => {
                let volume = if a(1).is_empty() { 100. } else { num(self, a(1), sol, None) };
                let file = a(0).to_string();
                self.cmds.push(if c.op == "play_sound" {
                    Cmd::PlaySound { file, volume, channel: a(2).to_string() }
                } else {
                    Cmd::PlayMusic { file, volume, fade: if a(2).is_empty() { 0. } else { num(self, a(2), sol, None) } }
                });
            }
            "channel_volume" => {
                let (volume, fade) = (num(self, a(1), sol, None), num(self, a(2), sol, None));
                self.cmds.push(Cmd::ChannelVolume { channel: a(0).into(), volume, fade });
            }
            "stop_channel" => self.cmds.push(Cmd::StopChannel { channel: a(0).into() }),
            "stop_music" => self.cmds.push(Cmd::StopMusic),
            "tween" => {
                for i in self.pick(a(0), sol) {
                    let to = num(self, a(2), sol, Some(i));
                    let dur = num(self, a(3), sol, Some(i)).max(0.);
                    let from = self.get(i, a(1)).num() as f32;
                    let me = &mut self.insts[i];
                    me.tweens.retain(|t| t.prop != a(1));
                    me.tweens.push(Tween { prop: a(1).into(), from, to, t: 0., dur, ease: a(4).into() });
                }
            }
            "camera_follow" => {
                let smooth = if a(1).is_empty() { 0.85 } else { num(self, a(1), sol, None) };
                self.cam_follow = self.pick(a(0), sol).first().map(|&i| (self.insts[i].id, smooth));
            }
            "camera_bounds" => {
                let v: Vec<f32> = (0..4).map(|k| num(self, a(k), sol, None)).collect();
                self.cam_bounds = Some((v[0], v[1], v[2], v[3]));
            }
            "camera_shake" => self.cam_shake = (num(self, a(0), sol, None), num(self, a(1), sol, None)),
            "camera_zoom_to" => {
                let (to, secs) = (num(self, a(0), sol, None).max(0.01), num(self, a(1), sol, None));
                self.zoom_tween = Some((self.zoom, to, 0., secs.max(0.)));
            }
            "move_forward" => {
                // Adelante = -Z girado por `angle` (como en Godot); con pitch también sube o baja.
                for i in self.pick(a(0), sol) {
                    let dist = num(self, a(1), sol, Some(i));
                    let me = &mut self.insts[i];
                    let (yaw, pitch) = (me.angle.to_radians(), me.pitch.to_radians());
                    me.x -= yaw.sin() * pitch.cos() * dist;
                    me.y += pitch.sin() * dist;
                    me.z -= yaw.cos() * pitch.cos() * dist;
                }
            }
            "move_right" => {
                for i in self.pick(a(0), sol) {
                    let dist = num(self, a(1), sol, Some(i));
                    let me = &mut self.insts[i];
                    let yaw = me.angle.to_radians();
                    me.x += yaw.cos() * dist;
                    me.z -= yaw.sin() * dist;
                }
            }
            "look_at3" => {
                for i in self.pick(a(0), sol) {
                    let (x, y, z) = (num(self, a(1), sol, Some(i)), num(self, a(2), sol, Some(i)), num(self, a(3), sol, Some(i)));
                    let me = &mut self.insts[i];
                    let (dx, dy, dz) = (x - me.x, y - me.y, z - me.z);
                    me.angle = (-dx).atan2(-dz).to_degrees();
                    me.pitch = dy.atan2(dx.hypot(dz)).to_degrees();
                }
            }
            "set_alarm" => {
                for i in self.pick(a(0), sol) {
                    let secs = num(self, a(2), sol, Some(i));
                    let me = &mut self.insts[i];
                    me.alarms.retain(|x| x.0 != a(1));
                    if secs > 0. {
                        me.alarms.push((a(1).into(), secs));
                    }
                }
            }
            "apply_force" | "apply_impulse" | "set_velocity" => {
                for i in self.pick(a(0), sol) {
                    let (x, y) = (num(self, a(1), sol, Some(i)), num(self, a(2), sol, Some(i)));
                    let id = self.insts[i].id;
                    match c.op.as_str() {
                        "apply_force" => self.fisica.fuerza(id, x, y),
                        "apply_impulse" => self.fisica.impulso(id, x, y),
                        _ => {
                            self.put(i, "vx", Value::Num(x as f64));
                            self.put(i, "vy", Value::Num(y as f64));
                        }
                    }
                }
            }
            "apply_torque" => {
                for i in self.pick(a(0), sol) {
                    let t = num(self, a(1), sol, Some(i));
                    self.fisica.torque(self.insts[i].id, t);
                }
            }
            "set_angular_velocity" => {
                for i in self.pick(a(0), sol) {
                    let v = self.ev(a(1), sol, Some(i));
                    self.put(i, "angular_velocity", v);
                }
            }
            "joint_distance" | "joint_revolute" => {
                // Cada instancia elegida de a(0) se une a la primera elegida de a(1) (que no sea ella misma).
                // Los cuerpos se crean en el paso de física: si son nuevos, se crean ya para poder unirlos.
                self.physics_sync();
                let others = self.pick(a(1), sol);
                for i in self.pick(a(0), sol) {
                    let Some(&j) = others.iter().find(|&&j| j != i) else { continue };
                    let (ia, ib) = (self.insts[i].id, self.insts[j].id);
                    let ok = if c.op == "joint_distance" {
                        let len = if a(2).trim().is_empty() { None } else { Some(num(self, a(2), sol, Some(i))) };
                        self.fisica.junta_distancia(ia, ib, len)
                    } else {
                        let (x, y) = (num(self, a(2), sol, Some(i)), num(self, a(3), sol, Some(i)));
                        self.fisica.junta_giro(ia, ib, x, y)
                    };
                    if !ok {
                        self.warn(format!("{}: «{}» y «{}» necesitan el comportamiento física", c.op, a(0), a(1)));
                    }
                }
            }
            "remove_joints" => {
                for i in self.pick(a(0), sol) {
                    self.fisica.sacar_juntas(self.insts[i].id);
                }
            }
            "move_to" => {
                for i in self.pick(a(0), sol) {
                    let (x, y) = (num(self, a(1), sol, Some(i)), num(self, a(2), sol, Some(i)));
                    self.path_to(i, x, y);
                }
            }
            "stop_path" => {
                for i in self.pick(a(0), sol) {
                    let me = &mut self.insts[i];
                    if let Some(n) = me.nav.as_mut() {
                        n.puntos.clear();
                    }
                    (me.vx, me.vy) = (0., 0.);
                }
            }
            "emit_burst" => {
                for i in self.pick(a(0), sol) {
                    let n = num(self, a(1), sol, Some(i)).max(0.);
                    self.insts[i].burst += n as u32;
                }
            }
            "set_emitting" => {
                for i in self.pick(a(0), sol) {
                    let v = self.ev(a(1), sol, Some(i));
                    self.put(i, "emitting", v);
                }
            }
            "set_effect" => {
                let amount = if a(2).trim().is_empty() { None } else { Some(num(self, a(2), sol, None)) };
                for i in self.pick(a(0), sol) {
                    let me = &mut self.insts[i];
                    me.effect = a(1).to_string();
                    if let Some(k) = amount {
                        me.effect_amount = k.clamp(0., 1.);
                    }
                }
            }
            "wait" => {} // lo maneja body()
            "script" => {
                crate::script::run(self, sol, a(0));
            }
            "stop_sound" => self.cmds.push(Cmd::StopSound { file: a(0).into() }),
            "save_value" | "erase_saved" => {
                if c.op == "save_value" {
                    let v = self.ev(a(1), sol, None);
                    self.storage.insert(a(0).into(), v);
                } else {
                    self.storage.remove(a(0));
                }
                self.cmds.push(Cmd::Persist(self.storage.clone()));
            }
            "quit" => self.cmds.push(Cmd::Quit),
            "call" => self.call(c, sol),
            "return" => self.ret = Some(self.ev(a(0), sol, None).truthy()),
            op => self.warn(format!("acción desconocida: {op}")),
        }
    }

    /// Ejecuta una función propia. Los parámetros llegan como variables locales;
    /// trabaja sobre una copia de la selección (como las funciones libres de GDevelop).
    fn call(&mut self, c: &Instr, sol: &Sol) {
        let p = self.project.clone();
        let Some(f) = p.functions.iter().find(|f| Some(&f.name) == c.args.first()) else {
            return self.warn(format!("función desconocida: {:?}", c.args.first()));
        };
        if self.locals.len() > 64 {
            return self.warn(format!("demasiadas llamadas anidadas en {}", f.name));
        }
        let mut locals = Vars::new();
        for (n, name) in f.params.iter().enumerate() {
            let src = c.args.get(n + 1).map(String::as_str).unwrap_or("0");
            locals.insert(name.clone(), self.ev(src, sol, None));
        }
        self.locals.push(locals);
        self.ret = None;
        self.run_events(&f.events, sol);
        self.locals.pop();
    }

    // ---------- propiedades ----------

    fn anim_name(&self, i: usize) -> String {
        match &self.types[&self.insts[i].obj].kind {
            Kind::Sprite { animations, .. } => animations.get(self.insts[i].anim).map(|a| a.name.clone()).unwrap_or_default(),
            _ => String::new(),
        }
    }

    pub fn get(&self, i: usize, p: &str) -> Value {
        let me = &self.insts[i];
        let n = |v: f32| Value::Num(v as f64);
        match p {
            "x" => n(me.x),
            "y" => n(me.y),
            "cx" => n(me.cx()),
            "cy" => n(me.cy()),
            "w" => n(me.w),
            "h" => n(me.h),
            "angle" => n(me.angle),
            "opacity" => n(me.opacity),
            "z" => n(me.z),
            "speed" => n(me.vx.hypot(me.vy)),
            "visible" => Value::Num(me.visible as i32 as f64),
            "effect" => Value::Str(me.effect.clone()),
            "effect_amount" => n(me.effect_amount),
            "d" => n(me.d),
            "pitch" => n(me.pitch),
            "roll" => n(me.roll),
            "scale_z" => n(if me.base_d != 0. { me.d / me.base_d } else { 1. }),
            "scale_x" => n(if me.base_w != 0. { me.w / me.base_w } else { 1. }),
            "scale_y" => n(if me.base_h != 0. { me.h / me.base_h } else { 1. }),
            "tint" => Value::Str(format!("#{:02x}{:02x}{:02x}", me.tint[0], me.tint[1], me.tint[2])),
            "anim" => Value::Str(self.anim_name(i)),
            "text" => Value::Str(me.text.clone()),
            // Velocidad (px/s): la del cuerpo físico si tiene; si no, la de su movimiento (top_down, pathfinder...).
            "vx" => n(self.fisica.velocidad(me.id).map_or(me.vx, |v| v.0)),
            "vy" => n(self.fisica.velocidad(me.id).map_or(me.vy, |v| v.1)),
            "angular_velocity" => n(self.fisica.giro(me.id).unwrap_or(me.angvel0)),
            "emitting" => Value::Num(me.emitting as i32 as f64),
            "path_length" => n(me.nav.as_ref().map_or(0., |c| c.largo(me.cx(), me.cy()))),
            _ => me.vars.get(p).cloned().unwrap_or(Value::Num(0.)),
        }
    }

    pub(crate) fn put(&mut self, i: usize, p: &str, v: Value) {
        let dim3 = self.dim3;
        let f = v.num() as f32;
        let id = self.insts[i].id;
        match p {
            "vx" | "vy" if self.fisica.tiene(id) => {
                let (mut vx, mut vy) = self.fisica.velocidad(id).unwrap_or_default();
                if p == "vx" { vx = f } else { vy = f }
                return self.fisica.poner_velocidad(id, vx, vy);
            }
            "angular_velocity" if self.fisica.tiene(id) => return self.fisica.poner_giro(id, f),
            _ => {}
        }
        let me = &mut self.insts[i];
        match p {
            "x" => me.x = f,
            "y" => me.y = f,
            "cx" if me.origin.is_none() => me.x = f - me.w / 2.,
            "cy" if me.origin.is_none() => me.y = f - me.h / 2.,
            "cx" => me.x += f - me.cx(),
            "cy" => me.y += f - me.cy(),
            // Antes de tener cuerpo físico (o sin física): la velocidad del movimiento.
            "vx" => me.vx = f,
            "vy" => me.vy = f,
            "angular_velocity" => me.angvel0 = f,
            "emitting" => me.emitting = v.truthy(),
            "w" => me.w = f,
            "h" => me.h = f,
            "angle" => me.angle = f,
            "opacity" => me.opacity = f.clamp(0., 255.),
            "z" => me.z = f,
            "text" => me.text = v.text(),
            "visible" => me.visible = v.truthy(),
            "effect" => me.effect = v.text(),
            "effect_amount" => me.effect_amount = f.clamp(0., 1.),
            "d" => me.d = f,
            "pitch" => me.pitch = f,
            "roll" => me.roll = f,
            "scale_z" => me.d = me.base_d * f,
            // Escala alrededor del centro, como image_xscale de GameMaker.
            "scale_x" if dim3 => me.w = me.base_w * f,
            "scale_y" if dim3 => me.h = me.base_h * f,
            // Con punto de origen, la escala es alrededor del origen (x, y no cambian).
            "scale_x" if me.origin.is_some() => me.w = me.base_w * f,
            "scale_y" if me.origin.is_some() => me.h = me.base_h * f,
            "scale_x" => {
                let c = me.cx();
                me.w = me.base_w * f;
                me.x = c - me.w / 2.;
            }
            "scale_y" => {
                let c = me.cy();
                me.h = me.base_h * f;
                me.y = c - me.h / 2.;
            }
            "tint" => me.tint = parse_color(&v.text()).unwrap_or([255, 255, 255]),
            _ => {
                me.vars.insert(p.into(), v);
            }
        }
    }

    // ---------- expresiones ----------

    pub fn ev(&self, src: &str, sol: &Sol, cur: Option<usize>) -> Value {
        let e = self.cache.borrow().get(src).cloned();
        let e = e.unwrap_or_else(|| {
            let e = Rc::new(expr::parse(src).unwrap_or_else(|err| {
                self.warn(format!("expresión «{src}»: {err}"));
                Expr::Num(0.)
            }));
            self.cache.borrow_mut().insert(src.into(), e.clone());
            e
        });
        expr::eval(&e, &Ev { w: self, sol, cur })
    }

    /// Texto a mostrar: `{expr}` dentro del texto se evalúa en cada frame.
    pub fn display_text(&self, i: usize) -> String {
        let tpl = &self.insts[i].text;
        if !tpl.contains('{') {
            return tpl.clone();
        }
        let (mut out, mut rest) = (String::new(), tpl.as_str());
        let sol = Sol::new();
        while let Some(a) = rest.find('{') {
            let Some(b) = rest[a..].find('}') else { break };
            out += &rest[..a];
            out += &self.ev(&rest[a + 1..a + b], &sol, Some(i)).text();
            rest = &rest[a + b + 1..];
        }
        out + rest
    }

    pub(crate) fn random(&self) -> f64 {
        next_random(&self.rng)
    }

    pub fn layer_fixed(&self, layer: &str) -> bool {
        self.scene_def().layers.iter().any(|l| l.name == layer && l.fixed)
    }

    /// Pantalla → mundo para una capa (las capas fijas usan coordenadas de pantalla).
    pub fn mouse_on(&self, layer: &str) -> (f32, f32) {
        let (mx, my) = self.input.mouse;
        if self.layer_fixed(layer) {
            return (mx, my);
        }
        let p = &self.project;
        ((mx - p.width / 2.) / self.zoom + self.camera.0, (my - p.height / 2.) / self.zoom + self.camera.1)
    }

    // ---------- animación y comportamientos ----------

    fn animate(&mut self) {
        let dt = self.dt;
        for me in &mut self.insts {
            let Some(Kind::Sprite { animations, .. }) = self.types.get(&me.obj).map(|t| &t.kind) else { continue };
            let Some(a) = animations.get(me.anim) else { continue };
            let n = a.frames.len().max(1);
            me.anim_time += dt;
            let f = (me.anim_time * a.fps) as usize;
            if a.looping {
                me.frame = if a.fps > 0. { f % n } else { 0 };
            } else {
                me.frame = f.min(n - 1);
                me.anim_done = me.anim_time * a.fps >= n as f32;
            }
        }
    }

    fn has_platformer(&self, i: usize) -> bool {
        self.types[&self.insts[i].obj].behaviors.iter().any(|b| matches!(b, Behavior::Platformer { .. }))
    }

    fn key(&self, k: &str) -> f32 {
        self.input.down.contains(k) as i32 as f32
    }

    fn behaviors_pre(&mut self) {
        let dt = self.dt;
        let mut plats: Vec<Plat> = vec![];
        for i in 0..self.insts.len() {
            let b = self.types[&self.insts[i].obj].behaviors.iter().find_map(|b| match b {
                Behavior::Platform { jumpthru, ladder, grabbable } => Some((*jumpthru, *ladder, *grabbable)),
                _ => None,
            });
            if let Some((jumpthru, ladder, grabbable)) = b.filter(|_| !self.insts[i].dead) {
                let kind = if ladder { PKind::Ladder } else if jumpthru { PKind::Jumpthru } else { PKind::Solid };
                plats.extend(self.shapes(i).into_iter().map(|poly| Plat { i, poly, kind, grabbable }));
            }
        }
        for i in 0..self.insts.len() {
            if self.insts[i].dead {
                continue;
            }
            for b in self.types[&self.insts[i].obj].behaviors.clone() {
                match b {
                    Behavior::Platformer { .. } => {
                        // El Platformer trabaja con la caja sin girar desde su esquina: con punto de origen, se corre.
                        let (ox, oy) = self.insts[i].origin_offset();
                        self.insts[i].x -= ox;
                        self.insts[i].y -= oy;
                        self.platformer(i, &b, &plats);
                        self.insts[i].x += ox;
                        self.insts[i].y += oy;
                    }
                    Behavior::Pathfinder { speed, rotate, .. } => self.follow_path(i, speed, rotate),
                    Behavior::TopDown { max_speed, acceleration, deceleration, allow_diagonals, rotate } => {
                        let (mut dx, mut dy) = (self.key("Right") - self.key("Left"), self.key("Down") - self.key("Up"));
                        let me = &mut self.insts[i];
                        if ["Left", "Right"].iter().any(|k| self.input.pressed.contains(*k)) {
                            me.last_axis_x = true;
                        } else if ["Up", "Down"].iter().any(|k| self.input.pressed.contains(*k)) {
                            me.last_axis_x = false;
                        }
                        if !allow_diagonals && dx != 0. && dy != 0. {
                            if me.last_axis_x { dy = 0. } else { dx = 0. }
                        }
                        let (pvx, pvy) = (me.vx, me.vy);
                        let len = dx.hypot(dy);
                        if len > 0. {
                            let (ux, uy) = (dx / len, dy / len);
                            let dot = me.vx * ux + me.vy * uy;
                            let speed = if dot < 0. { dot } else { me.vx.hypot(me.vy) };
                            let acc = if speed < 0. { acceleration.max(deceleration) } else { acceleration };
                            let s = (speed + acc * dt).min(max_speed);
                            (me.vx, me.vy) = (ux * s, uy * s);
                            if rotate {
                                me.angle = uy.atan2(ux).to_degrees();
                            }
                        } else {
                            let s = me.vx.hypot(me.vy);
                            let k = if s > 0. { (s - deceleration * dt).max(0.) / s } else { 0. };
                            (me.vx, me.vy) = (me.vx * k, me.vy * k);
                        }
                        me.x += (pvx + me.vx) / 2. * dt;
                        me.y += (pvy + me.vy) / 2. * dt;
                    }
                    Behavior::Bullet { speed } => {
                        let me = &mut self.insts[i];
                        let r = me.angle.to_radians();
                        me.x += r.cos() * speed * dt;
                        me.y += r.sin() * speed * dt;
                    }
                    Behavior::Rotate { speed } => self.insts[i].angle += speed * dt,
                    Behavior::Draggable => self.drag(i),
                    _ => {}
                }
            }
        }
    }

    /// Personaje de plataformas, según PlatformerObject de GDevelop (docs/gdevelop-runtime-y-editor.md §3.1).
    /// El personaje usa su caja sin rotar; las plataformas, su polígono (una plataforma rotada es una pendiente).
    fn platformer(&mut self, i: usize, b: &Behavior, plats: &[Plat]) {
        let Behavior::Platformer {
            max_speed, acceleration, deceleration, gravity, max_falling_speed, jump_speed, jump_sustain_time,
            ladder_speed, slope_max_angle, can_grab_platforms, can_go_down_from_jumpthru,
        } = *b
        else {
            return;
        };
        let dt = self.dt;
        let down = |k: &str| self.input.down.contains(k);
        let (left, right, up, dn) = (down("Left"), down("Right"), down("Up"), down("Down"));
        let in_ladder = plats.iter().any(|p| p.kind == PKind::Ladder && self.touches(i, &p.poly));
        // Arriba salta, salvo que esté en una escalera (ahí sirve para subir).
        let jump_key = down("Space") || down("LeftShift") || (up && !in_ladder);
        let slope = slope_max_angle.clamp(0., 89.).to_radians().tan();

        // Una plataforma móvil arrastra a quien está encima.
        if let Some((fid, fx, fy)) = self.insts[i].floor {
            if let Some(f) = self.insts.iter().find(|o| o.id == fid && !o.dead).map(|o| (o.x, o.y)) {
                let me = &mut self.insts[i];
                if me.pstate == ON_FLOOR {
                    me.x += f.0 - fx;
                    me.y += f.1 - fy;
                }
                me.floor = Some((fid, f.0, f.1));
            }
        }

        let me = &mut self.insts[i];
        let prev = me.vx;
        let dir = right as i32 as f32 - left as i32 as f32;
        me.vx = accelerate(me.vx, dir, acceleration, deceleration, dt).clamp(-max_speed, max_speed);
        let mut dx = (me.vx + prev) / 2. * dt;

        // Transiciones antes de mover.
        if in_ladder && (up || dn) && me.pstate != ON_LADDER {
            (me.pstate, me.vy, me.v_jump) = (ON_LADDER, 0., 0.);
        }
        if me.pstate == ON_LADDER && !in_ladder {
            me.pstate = FALLING;
        }
        if !jump_key {
            me.jump_held = false;
        }
        let can_jump = matches!(me.pstate, ON_FLOOR | ON_LADDER | GRABBING);
        if (can_jump && jump_key && !me.jump_held) || me.want_jump {
            (me.pstate, me.v_jump, me.vy, me.jump_time, me.jump_held) = (JUMPING, jump_speed, 0., 0., jump_key);
            me.floor = None;
        }
        me.want_jump = false;
        if me.pstate == GRABBING && dn {
            me.pstate = FALLING;
        }
        let floor_id = me.floor.map(|f| f.0);
        let on_jumpthru = floor_id.is_some_and(|f| plats.iter().any(|p| p.kind == PKind::Jumpthru && self.insts[p.i].id == f));
        let me = &mut self.insts[i];
        if me.pstate == ON_FLOOR && dn && can_go_down_from_jumpthru && on_jumpthru {
            (me.pstate, me.ignore_jt, me.floor) = (FALLING, floor_id, None);
        }

        // Cuánto bajar o subir según el estado.
        let mut dy = 0.;
        match me.pstate {
            JUMPING => {
                me.jump_time += dt;
                let pj = me.v_jump;
                if !(me.jump_held && me.jump_time < jump_sustain_time) {
                    me.v_jump -= gravity * dt;
                }
                dy += (-pj - me.v_jump) / 2. * dt;
                if me.v_jump < 0. {
                    me.pstate = FALLING;
                }
            }
            ON_LADDER => dy = (dn as i32 as f32 - up as i32 as f32) * ladder_speed * dt,
            GRABBING => dx = 0.,
            _ => {}
        }
        if matches!(me.pstate, FALLING | JUMPING) {
            let pf = me.vy;
            me.vy = (me.vy + gravity * dt).min(max_falling_speed);
            dy += (me.vy + pf) / 2. * dt;
        }
        let was = me.pstate;
        let (old_x, old_y) = (me.x, me.y);

        // Eje X: si choca, en el suelo intenta subir la pendiente; si no, retrocede de a 1 px.
        self.insts[i].x += dx;
        if self.blocked(i, plats) {
            let climbed = was == ON_FLOOR && {
                let rise = dx.abs() * slope + 1.;
                match self.floor_top(i, plats, rise, 0., true) {
                    Some((top, _)) => {
                        let saved = self.insts[i].y;
                        self.insts[i].y = top - self.insts[i].h;
                        let ok = !self.blocked(i, plats);
                        if !ok {
                            self.insts[i].y = saved;
                        }
                        ok
                    }
                    None => false,
                }
            };
            if !climbed {
                self.back_off(i, plats, old_x, true);
                self.insts[i].vx = 0.;
            }
        }

        // Eje Y.
        let state = self.insts[i].pstate;
        if state == ON_FLOOR {
            // Seguir el suelo: subir o bajar lo que permita la pendiente.
            let reach = dx.abs() * slope + 1.;
            match self.floor_top(i, plats, reach, reach, false) {
                Some((top, id)) => self.set_floor(i, top, id),
                None => {
                    let me = &mut self.insts[i];
                    (me.pstate, me.floor) = (FALLING, None);
                }
            }
        } else if dy > 0. {
            // Bajando: aterriza sobre el suelo más alto que cruce en este paso.
            match self.floor_top(i, plats, 0.5, dy, false) {
                Some((top, id)) => self.set_floor(i, top, id),
                None => {
                    self.insts[i].y += dy;
                    if self.blocked(i, plats) {
                        self.back_off(i, plats, old_y, false);
                    }
                }
            }
        } else if dy < 0. {
            self.insts[i].y += dy;
            if self.blocked(i, plats) {
                self.back_off(i, plats, old_y, false);
                let me = &mut self.insts[i];
                if me.pstate == JUMPING {
                    (me.pstate, me.v_jump) = (FALLING, 0.);
                }
            }
        }

        // Agarrarse de un borde: la parte de arriba del personaje cruzó el borde de la plataforma este frame.
        if can_grab_platforms && matches!(self.insts[i].pstate, FALLING | JUMPING) && self.insts[i].y > old_y {
            let tol = 10.; // xGrabTolerance de GDevelop
            let me = self.insts[i].clone();
            let edge = plats.iter().filter(|p| p.grabbable && p.kind != PKind::Ladder && p.i != i).find_map(|p| {
                let (ox, oy) = p.poly.iter().fold((f32::MAX, f32::MAX), |(x, y), q| (x.min(q.0), y.min(q.1)));
                let ox1 = p.poly.iter().fold(f32::MIN, |x, q| x.max(q.0));
                let side = (me.x + me.w - ox).abs() <= tol || (me.x - ox1).abs() <= tol;
                (side && old_y <= oy && me.y >= oy).then_some(oy)
            });
            if let Some(top) = edge {
                let me = &mut self.insts[i];
                (me.y, me.pstate, me.vy, me.v_jump, me.vx) = (top, GRABBING, 0., 0., 0.);
            }
        }

        // Dejar de ignorar el jumpthru cuando ya no lo toca.
        if let Some(jt) = self.insts[i].ignore_jt {
            let touching = plats.iter().any(|p| self.insts[p.i].id == jt && self.touches(i, &p.poly));
            if !touching {
                self.insts[i].ignore_jt = None;
            }
        }
        let me = &mut self.insts[i];
        me.on_floor = me.pstate == ON_FLOOR;
        me.jumping = me.pstate == JUMPING;
        if me.on_floor {
            me.vy = 0.;
        }
    }

    fn aabb(&self, i: usize) -> [(f32, f32); 4] {
        let o = &self.insts[i];
        [(o.x, o.y), (o.x + o.w, o.y), (o.x + o.w, o.y + o.h), (o.x, o.y + o.h)]
    }

    /// Se solapan de verdad (más de una centésima de píxel).
    fn touches(&self, i: usize, poly: &[(f32, f32); 4]) -> bool {
        sat(&self.aabb(i), poly).is_some_and(|(x, y)| x.hypot(y) > 0.01)
    }

    /// Choca con una plataforma sólida (las jumpthru y escaleras no bloquean).
    fn blocked(&self, i: usize, plats: &[Plat]) -> bool {
        plats.iter().any(|p| p.kind == PKind::Solid && p.i != i && self.touches(i, &p.poly))
    }

    /// Retrocede de a 1 px hacia `old` hasta salir del choque.
    fn back_off(&mut self, i: usize, plats: &[Plat], old: f32, x_axis: bool) {
        for _ in 0..2000 {
            if !self.blocked(i, plats) {
                return;
            }
            let me = &mut self.insts[i];
            let p = if x_axis { &mut me.x } else { &mut me.y };
            let d = old - *p;
            if d.abs() <= 1. {
                *p = old;
                return;
            }
            *p = p.round() + d.signum();
        }
    }

    /// Suelo más alto bajo el personaje entre `up` px por encima y `down` px por debajo de sus pies.
    /// `solid_only`: solo plataformas sólidas (para subir pendientes al chocar).
    fn floor_top(&self, i: usize, plats: &[Plat], up: f32, down: f32, solid_only: bool) -> Option<(f32, u64)> {
        let me = &self.insts[i];
        let (x0, x1, bottom) = (me.x, me.x + me.w, me.y + me.h);
        plats
            .iter()
            .filter(|p| p.i != i && p.kind != PKind::Ladder && !(solid_only && p.kind != PKind::Solid))
            .filter(|p| !(p.kind == PKind::Jumpthru && me.ignore_jt == Some(self.insts[p.i].id)))
            .filter_map(|p| top_between(&p.poly, x0, x1).map(|t| (t, p)))
            .filter(|(t, p)| {
                // Un jumpthru solo cuenta si los pies estaban por encima de él.
                *t >= bottom - up && *t <= bottom + down && (p.kind != PKind::Jumpthru || bottom <= t + up.max(0.5))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(t, p)| (t, self.insts[p.i].id))
    }

    fn set_floor(&mut self, i: usize, top: f32, id: u64) {
        let pos = self.insts.iter().find(|o| o.id == id).map(|o| (o.x, o.y)).unwrap_or_default();
        let me = &mut self.insts[i];
        me.y = top - me.h;
        (me.pstate, me.vy, me.v_jump) = (ON_FLOOR, 0., 0.);
        if me.floor.map(|f| f.0) != Some(id) {
            me.floor = Some((id, pos.0, pos.1));
        }
    }

    fn drag(&mut self, i: usize) {
        let (mx, my) = self.mouse_on(&self.insts[i].layer);
        let id = self.insts[i].id;
        match self.dragging {
            None if self.input.mouse_pressed[0] && point_in(&self.insts[i].hitbox(), mx, my) => {
                self.dragging = Some((id, self.insts[i].x - mx, self.insts[i].y - my));
            }
            Some((d, ox, oy)) if d == id => {
                if self.input.mouse_down[0] {
                    (self.insts[i].x, self.insts[i].y) = (mx + ox, my + oy);
                } else {
                    self.dragging = None;
                }
            }
            _ => {}
        }
    }

    // ---------- física 2D ----------

    /// Instancias con Physics → cuerpos de rapier (crea los nuevos, mueve los teletransportados, quita los muertos).
    fn physics_sync(&mut self) {
        use crate::fisica2d::{Def, Forma, Tipo};
        if self.dim3 {
            return;
        }
        let mut alive = HashSet::new();
        for i in 0..self.insts.len() {
            if self.insts[i].dead {
                continue;
            }
            let t = &self.types[&self.insts[i].obj];
            let Some((b, circle)) = t.behaviors.iter().find_map(|b| match b {
                Behavior::Physics { shape, .. } => Some((b, shape == "circle")),
                _ => None,
            }) else {
                continue;
            };
            let Some(mut def) = Def::de(b) else { continue };
            let me = &self.insts[i];
            let forma = if matches!(t.kind, Kind::Tilemap { .. }) {
                def.tipo = Tipo::Estatico; // un mapa no se cae
                Forma::Tiras(self.shapes(i))
            } else if !me.mask.is_empty() {
                Forma::Poligono(me.mask.iter().map(|&(mx, my)| ((if me.flip_x { 1. - mx } else { mx } - 0.5) * me.w, (my - 0.5) * me.h)).collect())
            } else if circle {
                Forma::Circulo((me.w + me.h) / 4.)
            } else {
                Forma::Caja(me.w / 2., me.h / 2.)
            };
            let (id, center, angle, vel, w0) = (me.id, (me.cx(), me.cy()), me.angle, (me.vx, me.vy), me.angvel0);
            let created = !self.fisica.tiene(id);
            self.fisica.entrar(id, def, center, angle, forma, vel);
            if created && w0 != 0. {
                self.fisica.poner_giro(id, w0);
            }
            alive.insert(id);
        }
        self.fisica.retener(|id| alive.contains(&id));
    }

    fn physics_step(&mut self) {
        self.physics_sync();
        if self.fisica.is_empty() {
            return;
        }
        let g = self.scene_def().gravity;
        self.fisica.paso(self.dt, g);
        for i in 0..self.insts.len() {
            let id = self.insts[i].id;
            let Some((x, y, a)) = self.fisica.pose(id) else { continue };
            let vel = self.fisica.velocidad(id).unwrap_or_default();
            let me = &mut self.insts[i];
            if let Some(a) = a {
                me.angle = a;
            }
            me.set_center(x, y);
            (me.vx, me.vy) = vel;
            let (c, a) = ((me.cx(), me.cy()), me.angle);
            self.fisica.anotar(id, c, a);
        }
    }

    // ---------- búsqueda de caminos ----------

    /// Pide ir a (x, y): busca un camino nuevo si el destino cambió de celda; si no, solo corrige el último punto.
    fn path_to(&mut self, i: usize, x: f32, y: f32) {
        let Some((cell, diagonals)) = self.types[&self.insts[i].obj].behaviors.iter().find_map(|b| match b {
            Behavior::Pathfinder { cell, diagonals, .. } => Some((*cell, *diagonals)),
            _ => None,
        }) else {
            return self.warn(format!("move_to: «{}» necesita el comportamiento pathfinder", self.insts[i].obj));
        };
        if let Some(n) = self.insts[i].nav.as_mut() {
            if n.destino == (x, y) {
                return;
            }
            let same_cell = |a: (f32, f32), b: (f32, f32)| (a.0 / n.celda).floor() == (b.0 / n.celda).floor() && (a.1 / n.celda).floor() == (b.1 / n.celda).floor();
            if n.encontrado && !n.puntos.is_empty() && n.celda > 0. && same_cell(n.destino, (x, y)) {
                n.destino = (x, y);
                *n.puntos.last_mut().expect("puntos") = (x, y);
                return;
            }
        }
        let me = &self.insts[i];
        let (cx, cy, hw, hh) = (me.cx(), me.cy(), me.w / 2., me.h / 2.);
        let mut obst: Vec<Vec<(f32, f32)>> = vec![];
        for j in 0..self.insts.len() {
            if j != i && !self.insts[j].dead && self.types[&self.insts[j].obj].behaviors.contains(&Behavior::Obstacle) {
                obst.extend(self.polys(j));
            }
        }
        let margin = 2. * cell + hw.max(hh) * 2.;
        let mut area = (cx.min(x) - margin, cy.min(y) - margin, cx.max(x) + margin, cy.max(y) + margin);
        for p in obst.iter().flatten() {
            area = (area.0.min(p.0 - margin), area.1.min(p.1 - margin), area.2.max(p.0 + margin), area.3.max(p.1 + margin));
        }
        let g = crate::caminos::Grilla::nueva(area, cell, &obst, (hw, hh));
        let found = g.buscar((cx, cy), (x, y), diagonals);
        let me = &mut self.insts[i];
        me.nav = Some(Box::new(crate::caminos::Camino {
            destino: (x, y),
            encontrado: found.is_some(),
            llego: false,
            puntos: found.unwrap_or_default(),
            celda: g.cell,
        }));
        if me.nav.as_ref().is_some_and(|n| n.puntos.is_empty()) {
            (me.vx, me.vy) = (0., 0.);
        }
    }

    /// Avanza por el camino a `speed` px/s (girando hacia donde camina si `rotate`).
    fn follow_path(&mut self, i: usize, speed: f32, rotate: bool) {
        let dt = self.dt;
        let me = &mut self.insts[i];
        let (cx, cy) = (me.cx(), me.cy());
        let Some(nav) = me.nav.as_mut() else { return };
        if nav.puntos.is_empty() {
            return;
        }
        let (mut x, mut y, mut left) = (cx, cy, speed.max(0.) * dt);
        while left > 0. && !nav.puntos.is_empty() {
            let p = nav.puntos[0];
            let d = (p.0 - x).hypot(p.1 - y);
            if d <= left {
                (x, y) = p;
                left -= d;
                nav.puntos.remove(0);
            } else {
                x += (p.0 - x) / d * left;
                y += (p.1 - y) / d * left;
                left = 0.;
            }
        }
        let done = nav.puntos.is_empty();
        nav.llego = done;
        let (dx, dy) = (x - cx, y - cy);
        if rotate && (dx != 0. || dy != 0.) {
            me.angle = dy.atan2(dx).to_degrees();
        }
        me.set_center(x, y);
        (me.vx, me.vy) = if done || dt <= 0. { (0., 0.) } else { (dx / dt, dy / dt) };
    }

    // ---------- partículas ----------

    /// Mueve las partículas y hace emitir a los emisores (ráfagas pendientes y `rate` por segundo).
    fn particles_step(&mut self) {
        let dt = self.dt;
        let types = &self.types;
        crate::particulas::mover(&mut self.particles, dt, |o| match types.get(o).map(|t| &t.kind) {
            Some(Kind::Particles(d)) => d.gravity,
            _ => [0., 0.],
        });
        let mut counts: HashMap<u64, usize> = HashMap::new();
        for p in &self.particles {
            *counts.entry(p.emitter).or_default() += 1;
        }
        let rng = &self.rng;
        let rnd = || next_random(rng) as f32;
        for me in self.insts.iter_mut().filter(|m| !m.dead) {
            let Some(Kind::Particles(def)) = types.get(&me.obj).map(|t| &t.kind) else { continue };
            let have = counts.get(&me.id).copied().unwrap_or(0);
            let mut n = std::mem::take(&mut me.burst) as usize;
            if me.emitting && def.rate > 0. {
                me.emit_acc += def.rate * dt;
                let k = me.emit_acc.floor();
                me.emit_acc -= k;
                n += k as usize;
            }
            let n = n.min(def.max.saturating_sub(have));
            let (cx, cy) = (me.cx(), me.cy());
            crate::particulas::emitir(def, n, cx, cy, me.angle, &me.obj, &me.layer, me.id, &rnd, &mut self.particles);
            if def.auto_destroy && (!me.emitting || def.rate <= 0.) && have + n == 0 {
                me.dead = true;
            }
        }
    }

    fn behaviors_post(&mut self) {
        let p = self.project.clone();
        let (vw, vh) = (p.width / self.zoom, p.height / self.zoom);
        for i in 0..self.insts.len() {
            for b in &self.types[&self.insts[i].obj].behaviors {
                let fixed = self.layer_fixed(&self.insts[i].layer);
                let me = &mut self.insts[i];
                // Estos trabajan con la esquina superior izquierda (con punto de origen, se corre y se devuelve).
                let (ox, oy) = me.origin_offset();
                me.x -= ox;
                me.y -= oy;
                match b {
                    Behavior::DestroyOutside => {
                        let (cx, cy) = if fixed { (p.width / 2., p.height / 2.) } else { self.camera };
                        let m = 200.; // extraBorder de GDevelop
                        if me.x + me.w < cx - vw / 2. - m || me.x > cx + vw / 2. + m || me.y + me.h < cy - vh / 2. - m || me.y > cy + vh / 2. + m {
                            me.dead = true;
                        }
                    }
                    Behavior::BoundToLayout => {
                        me.x = me.x.clamp(0., (p.width - me.w).max(0.));
                        me.y = me.y.clamp(0., (p.height - me.h).max(0.));
                    }
                    Behavior::ScreenWrap => {
                        if me.x > p.width { me.x = -me.w }
                        if me.x + me.w < 0. { me.x = p.width }
                        if me.y > p.height { me.y = -me.h }
                        if me.y + me.h < 0. { me.y = p.height }
                    }
                    _ => {}
                }
                me.x += ox;
                me.y += oy;
            }
        }
    }
}

impl Inst {
    /// Centro del rectángulo (ya girado).
    pub fn cx(&self) -> f32 {
        match self.origin {
            None => self.x + self.w / 2.,
            Some(_) => self.local_to_world(self.w / 2., self.h / 2.).0,
        }
    }
    pub fn cy(&self) -> f32 {
        match self.origin {
            None => self.y + self.h / 2.,
            Some(_) => self.local_to_world(self.w / 2., self.h / 2.).1,
        }
    }
    /// Punto de giro en el mundo: el origen (x, y) si tiene, si no el centro.
    pub fn pivot(&self) -> (f32, f32) {
        match self.origin {
            None => (self.x + self.w / 2., self.y + self.h / 2.),
            Some(_) => (self.x, self.y),
        }
    }
    /// Cuánto hay de la esquina superior izquierda (sin girar) a x, y: (0, 0) sin punto de origen.
    pub fn origin_offset(&self) -> (f32, f32) {
        self.origin.map_or((0., 0.), |(ox, oy)| (ox * self.w, oy * self.h))
    }
    /// Punto del rectángulo (en px desde su esquina superior izquierda, sin girar) → mundo.
    pub fn local_to_world(&self, lx: f32, ly: f32) -> (f32, f32) {
        let (px, py) = self.pivot();
        let (plx, ply) = match self.origin {
            None => (self.w / 2., self.h / 2.),
            Some((ox, oy)) => (ox * self.w, oy * self.h),
        };
        let (c, s) = (self.angle.to_radians().cos(), self.angle.to_radians().sin());
        let (x, y) = (lx - plx, ly - ply);
        (px + x * c - y * s, py + x * s + y * c)
    }
    /// Rectángulo girado (alrededor del centro, o del punto de origen si tiene).
    pub fn hitbox(&self) -> [(f32, f32); 4] {
        if self.origin.is_some() {
            return [(0., 0.), (self.w, 0.), (self.w, self.h), (0., self.h)].map(|(x, y)| self.local_to_world(x, y));
        }
        let (c, s) = (self.angle.to_radians().cos(), self.angle.to_radians().sin());
        let (hw, hh) = (self.w / 2., self.h / 2.);
        [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)].map(|(x, y)| (self.cx() + x * c - y * s, self.cy() + x * s + y * c))
    }
    /// Forma de choque: la máscara (polígono convexo, girado y escalado con la instancia; `flip_x` la espeja)
    /// o, sin máscara, el rectángulo de `hitbox`.
    pub fn mask_poly(&self) -> Vec<(f32, f32)> {
        if self.mask.is_empty() {
            return self.hitbox().to_vec();
        }
        self.mask.iter().map(|&(mx, my)| self.local_to_world(if self.flip_x { 1. - mx } else { mx } * self.w, my * self.h)).collect()
    }
    /// Para dibujar: (esquina superior izquierda sin girar x, y, ancho, alto, punto de giro x, y).
    /// Se dibuja el rectángulo (x, y, w, h) girado `angle` grados alrededor de (pivot_x, pivot_y).
    /// Equivale a dibujar centrado en (cx(), cy()) girando alrededor del centro.
    pub fn draw_rect(&self) -> (f32, f32, f32, f32, f32, f32) {
        let (ox, oy) = self.origin_offset();
        let (px, py) = self.pivot();
        match self.origin {
            None => (self.x, self.y, self.w, self.h, px, py),
            Some(_) => (self.x - ox, self.y - oy, self.w, self.h, px, py),
        }
    }
    /// Pone el centro del rectángulo (ya girado) en (cx, cy) moviendo x, y.
    pub fn set_center(&mut self, cx: f32, cy: f32) {
        let (dx, dy) = (cx - self.cx(), cy - self.cy());
        self.x += dx;
        self.y += dy;
    }
}

/// Número al azar en [0, 1) (xorshift64*).
fn next_random(rng: &Cell<u64>) -> f64 {
    let mut x = rng.get();
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    rng.set(x);
    (x.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64 / (1u64 << 53) as f64
}

/// Envoltura convexa (cadena monótona). Devuelve los puntos en orden; con menos de 3 distintos, vacío.
pub fn convex_hull(mut pts: Vec<(f32, f32)>) -> Vec<(f32, f32)> {
    pts.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    pts.dedup();
    if pts.len() < 3 {
        return vec![];
    }
    let cross = |o: (f32, f32), a: (f32, f32), b: (f32, f32)| (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0);
    let mut h: Vec<(f32, f32)> = vec![];
    for pass in 0..2 {
        let start = h.len();
        let iter: Box<dyn Iterator<Item = &(f32, f32)>> = if pass == 0 { Box::new(pts.iter()) } else { Box::new(pts.iter().rev()) };
        for &p in iter {
            while h.len() >= start + 2 && cross(h[h.len() - 2], h[h.len() - 1], p) <= 0. {
                h.pop();
            }
            h.push(p);
        }
        h.pop();
    }
    if h.len() < 3 { vec![] } else { h }
}

/// Distancia a la que un rayo (origen, dirección unitaria, largo máximo) entra en un polígono convexo.
/// 0 si el origen está adentro; None si no lo toca.
pub fn ray_poly(o: (f32, f32), d: (f32, f32), max: f32, poly: &[(f32, f32)]) -> Option<f32> {
    let n = poly.len();
    if n < 3 {
        return None;
    }
    let c = poly.iter().fold((0., 0.), |s, p| (s.0 + p.0 / n as f32, s.1 + p.1 / n as f32));
    let (mut t0, mut t1) = (0f32, max);
    for k in 0..n {
        let (p, q) = (poly[k], poly[(k + 1) % n]);
        let (mut nx, mut ny) = (q.1 - p.1, p.0 - q.0);
        if nx == 0. && ny == 0. {
            continue;
        }
        // Normal hacia afuera.
        if nx * (c.0 - p.0) + ny * (c.1 - p.1) > 0. {
            (nx, ny) = (-nx, -ny);
        }
        let num = nx * (p.0 - o.0) + ny * (p.1 - o.1);
        let den = nx * d.0 + ny * d.1;
        if den == 0. {
            if num < 0. {
                return None;
            }
            continue;
        }
        let t = num / den;
        if den < 0. { t0 = t0.max(t) } else { t1 = t1.min(t) }
        if t0 > t1 {
            return None;
        }
    }
    Some(t0)
}

/// Parte un texto en líneas que no pasen de `wrap_width` px. `measure(texto)` da el ancho en px con la fuente
/// y el tamaño `font_size_px` que usa el reproductor. Respeta los saltos de línea; wrap_width <= 0 = no parte.
/// Una palabra más larga que el ancho se corta por letras.
pub fn wrap_lines(text: &str, font_size_px: f32, wrap_width: f32, measure: impl Fn(&str) -> f32) -> Vec<String> {
    let _ = font_size_px; // lo usa `measure`; queda en la firma para que los reproductores pasen lo mismo
    let mut out = vec![];
    for para in text.split('\n') {
        if wrap_width <= 0. {
            out.push(para.to_string());
            continue;
        }
        let mut line = String::new();
        for word in para.split(' ') {
            let cand = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if measure(&cand) <= wrap_width {
                line = cand;
                continue;
            }
            if !line.is_empty() {
                out.push(std::mem::take(&mut line));
            }
            if measure(word) <= wrap_width {
                line = word.to_string();
                continue;
            }
            // La palabra sola no entra: cortarla por letras.
            for ch in word.chars() {
                let cand = format!("{line}{ch}");
                if !line.is_empty() && measure(&cand) > wrap_width {
                    out.push(std::mem::take(&mut line));
                    line.push(ch);
                } else {
                    line = cand;
                }
            }
        }
        out.push(line);
    }
    out
}

/// Nombre de un evento de primer nivel para el perfilador: «Evento 3: Tecla Space presionada».
fn event_label(n: usize, e: &Event) -> String {
    let what = match e.kind {
        EventKind::Group => format!("grupo «{}»", e.text),
        EventKind::Comment => "comentario".into(),
        EventKind::Repeat => format!("repetir {} veces", e.text),
        EventKind::ForEach => format!("para cada {}", e.text),
        EventKind::While => format!("mientras {}", e.conds.first().map(crate::catalog::sentence).unwrap_or_default()),
        _ => e.conds.first().map(crate::catalog::sentence).unwrap_or_else(|| "siempre".into()),
    };
    format!("Evento {}: {what}", n + 1)
}

/// Reloj del perfilador. En WebAssembly `Instant` no existe (entra en pánico): ahí no se mide.
#[cfg(not(target_arch = "wasm32"))]
mod reloj {
    pub const ACTIVO: bool = true;
    pub struct Reloj(std::time::Instant);
    impl Reloj {
        pub fn ahora() -> Self {
            Reloj(std::time::Instant::now())
        }
        pub fn ms(&self) -> f32 {
            self.0.elapsed().as_secs_f32() * 1000.
        }
    }
}
#[cfg(target_arch = "wasm32")]
mod reloj {
    pub const ACTIVO: bool = false;
    pub struct Reloj;
    impl Reloj {
        pub fn ahora() -> Self {
            Reloj
        }
        pub fn ms(&self) -> f32 {
            0.
        }
    }
}

/// Ejes separadores (SAT) entre polígonos convexos. Devuelve cuánto mover `a` para sacarlo de `b`.
pub fn sat(a: &[(f32, f32)], b: &[(f32, f32)]) -> Option<(f32, f32)> {
    let mut best = (f32::MAX, 0., 0.);
    for poly in [a, b] {
        for k in 0..poly.len() {
            let (p, q) = (poly[k], poly[(k + 1) % poly.len()]);
            let (nx, ny) = (q.1 - p.1, p.0 - q.0);
            let len = nx.hypot(ny);
            if len == 0. {
                continue;
            }
            let (nx, ny) = (nx / len, ny / len);
            let proj = |pts: &[(f32, f32)]| pts.iter().fold((f32::MAX, f32::MIN), |(lo, hi), &(x, y)| (lo.min(x * nx + y * ny), hi.max(x * nx + y * ny)));
            let ((amin, amax), (bmin, bmax)) = (proj(a), proj(b));
            let o = amax.min(bmax) - amin.max(bmin);
            if o <= 0. {
                return None;
            }
            if o < best.0 {
                let sign = if (amin + amax) < (bmin + bmax) { -1. } else { 1. };
                best = (o, nx * o * sign, ny * o * sign);
            }
        }
    }
    Some((best.1, best.2))
}

/// Punto más alto (menor y) de un polígono convexo entre x0 y x1: la superficie de una plataforma.
fn top_between(poly: &[(f32, f32)], x0: f32, x1: f32) -> Option<f32> {
    // Solo el interior: una pared que apenas roza el borde del personaje no es suelo.
    let (x0, x1) = (x0 + 0.01, x1 - 0.01);
    let mut best: Option<f32> = None;
    let mut put = |y: f32| best = Some(best.map_or(y, |b| b.min(y)));
    for k in 0..poly.len() {
        let (a, b) = (poly[k], poly[(k + 1) % poly.len()]);
        if a.0 >= x0 && a.0 <= x1 {
            put(a.1);
        }
        for x in [x0, x1] {
            if (a.0 - x) * (b.0 - x) < 0. {
                put(a.1 + (x - a.0) / (b.0 - a.0) * (b.1 - a.1));
            }
        }
    }
    best
}

/// Punto dentro de un polígono convexo (en cualquiera de los dos sentidos de giro).
fn point_in(poly: &[(f32, f32)], x: f32, y: f32) -> bool {
    let side = |k: usize| {
        let (p, q) = (poly[k], poly[(k + 1) % poly.len()]);
        (q.0 - p.0) * (y - p.1) - (q.1 - p.1) * (x - p.0)
    };
    (0..poly.len()).all(|k| side(k) >= 0.) || (0..poly.len()).all(|k| side(k) <= 0.)
}

/// Velocidad con aceleración/frenado (PlatformerObject de GDevelop).
fn accelerate(v: f32, dir: f32, acc: f32, dec: f32, dt: f32) -> f32 {
    if dir != 0. {
        let a = if v * dir < 0. { acc.max(dec) } else { acc };
        v + dir * a * dt
    } else if v > 0. {
        (v - dec * dt).max(0.)
    } else {
        (v + dec * dt).min(0.)
    }
}

/// "#rrggbb", "rrggbb" o nombres comunes en español/inglés.
pub fn parse_color(s: &str) -> Option<[u8; 3]> {
    let named = match s.trim().to_lowercase().as_str() {
        "blanco" | "white" => Some([255, 255, 255]),
        "negro" | "black" => Some([0, 0, 0]),
        "rojo" | "red" => Some([255, 60, 60]),
        "verde" | "green" => Some([70, 200, 90]),
        "azul" | "blue" => Some([70, 120, 255]),
        "amarillo" | "yellow" => Some([255, 220, 60]),
        "naranja" | "orange" => Some([255, 150, 50]),
        "gris" | "gray" | "grey" => Some([140, 140, 140]),
        _ => None,
    };
    if named.is_some() {
        return named;
    }
    let h = s.trim().trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    let b = |k: usize| u8::from_str_radix(&h[k..k + 2], 16).ok();
    Some([b(0)?, b(2)?, b(4)?])
}

fn button(name: &str) -> usize {
    match name {
        "Right" => 1,
        "Middle" => 2,
        _ => 0,
    }
}

fn compare(a: &Value, op: &str, b: &Value) -> bool {
    if let (Value::Str(x), Value::Str(y)) = (a, b) {
        return match op {
            "!=" => x != y,
            _ => x == y,
        };
    }
    let (a, b) = (a.num(), b.num());
    match op {
        "<" => a < b,
        "<=" => a <= b,
        ">" => a > b,
        ">=" => a >= b,
        "!=" => a != b,
        _ => a == b,
    }
}

fn add(old: &Value, v: &Value) -> Value {
    match (old, v) {
        (Value::Str(a), _) => Value::Str(format!("{a}{}", v.text())),
        _ => Value::Num(old.num() + v.num()),
    }
}

struct Ev<'a> {
    w: &'a World,
    sol: &'a Sol,
    cur: Option<usize>,
}

impl Ctx for Ev<'_> {
    fn var(&self, name: &str) -> Value {
        let w = self.w;
        if let Some(v) = w.locals.last().and_then(|l| l.get(name)).or_else(|| w.vars.get(name)).or_else(|| w.globals.get(name)) {
            return v.clone();
        }
        let (mx, my) = w.mouse_on("");
        Value::Num(match name {
            "dt" => w.dt as f64,
            "time" => w.time,
            "mousex" => mx as f64,
            "mousey" => my as f64,
            "screen_w" => w.project.width as f64,
            "screen_h" => w.project.height as f64,
            "ray_x" => w.ray_hit.0 as f64,
            "ray_y" => w.ray_hit.1 as f64,
            _ => 0.,
        })
    }

    fn prop(&self, obj: &str, prop: &str) -> Value {
        let w = self.w;
        let objs = w.expand(obj);
        let i = match self.cur {
            Some(c) if objs.contains(&w.insts[c].obj) => Some(c),
            _ => w.pick(obj, self.sol).first().copied(),
        };
        i.map(|i| w.get(i, prop)).unwrap_or(Value::Num(0.))
    }

    fn call(&self, name: &str, args: &[Expr]) -> Option<Value> {
        let w = self.w;
        let obj = || match args.first() {
            Some(Expr::Var(o)) => o.clone(),
            _ => String::new(),
        };
        Some(Value::Num(match name {
            "count" => w.expand(&obj()).iter().map(|o| w.all_of(o).count()).sum::<usize>() as f64,
            "picked" => w.pick(&obj(), self.sol).len() as f64,
            // axis("Pad1LX"): eje de un mando de -1 a 1 (0 si no hay mando).
            "axis" => {
                let k = match args.first() {
                    Some(Expr::Var(v)) if w.input.axes.contains_key(v) => v.clone(),
                    Some(a) => expr::eval(a, self).text(),
                    None => String::new(),
                };
                w.input.axes.get(&k).copied().unwrap_or(0.) as f64
            }
            // saved("clave", por_defecto): valor de la partida guardada
            "saved" => {
                let k = args.first().map(|a| expr::eval(a, self).text()).unwrap_or_default();
                return Some(w.storage.get(&k).cloned().unwrap_or_else(|| args.get(1).map(|a| expr::eval(a, self)).unwrap_or(Value::Num(0.))));
            }
            // tile(Mapa, x, y): baldosa en esa posición del mundo (-1 = vacía)
            "tile" => {
                let n = |k: usize| args.get(k).map(|a| expr::eval(a, self).num() as f32).unwrap_or(0.);
                let Some(&i) = w.pick(&obj(), self.sol).first() else { return Some(Value::Num(-1.)) };
                let me = &w.insts[i];
                let Some(Kind::Tilemap { tile, cols, .. }) = w.types.get(&me.obj).map(|t| &t.kind) else { return Some(Value::Num(-1.)) };
                let (c, r) = (((n(1) - me.x) / tile).floor(), ((n(2) - me.y) / tile).floor());
                if c < 0. || r < 0. || c as usize >= *cols {
                    -1.
                } else {
                    me.tiles.get(r as usize * cols + c as usize).copied().unwrap_or(-1) as f64
                }
            }
            "timer" => {
                let n = args.first().map(|a| expr::eval(a, self).text()).unwrap_or_default();
                w.timers.get(&n).map(|t| w.time - t).unwrap_or(0.)
            }
            _ => return None,
        }))
    }

    fn random(&self) -> f64 {
        self.w.random()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world(json: &str) -> World {
        World::new(serde_json::from_str(json).unwrap())
    }

    const BASE: &str = r#"{"name":"t","width":200,"height":200,"first_scene":"S","scenes":[{"name":"S",
        "objects":[{"name":"A","type":"box","w":10,"h":10,"color":[0,0,0]},
                   {"name":"B","type":"box","w":10,"h":10,"color":[0,0,0]}],
        "groups":{"Todos":["A","B"]},
        "instances":[{"obj":"A","x":0,"y":0},{"obj":"A","x":50,"y":50},{"obj":"B","x":5,"y":5}],
        "vars":{"s":0},
        "events":EVENTS}]}"#;

    fn with(events: &str) -> World {
        world(&BASE.replace("EVENTS", events))
    }

    #[test]
    fn seleccion_por_colision_y_crear() {
        let mut w = with(r#"[
            {"conds":[{"op":"collides","args":["A","B"]}],"acts":[{"op":"delete","args":["A"]},{"op":"add_var","args":["s","1"]}]},
            {"conds":[{"op":"start"}],"acts":[{"op":"create","args":["B","100","100",""]},{"op":"set","args":["B","x","7"]}]}
        ]"#);
        w.step(0.016, Input::default());
        assert_eq!(w.insts.iter().filter(|i| i.obj == "A").count(), 1); // solo el A que toca a B
        assert_eq!(w.vars["s"], Value::Num(1.));
        let bs: Vec<f32> = w.insts.iter().filter(|i| i.obj == "B").map(|i| i.x).collect();
        assert_eq!(bs, vec![5., 7.]); // set solo afectó al B recién creado
    }

    #[test]
    fn invertida_grupos_foreach_once() {
        let mut w = with(r#"[
            {"conds":[{"op":"collides","args":["A","B"],"not":true}],"acts":[{"op":"set","args":["A","tag","1"]}]},
            {"conds":[{"op":"compare_prop","args":["Todos","x","<","10"]}],"acts":[{"op":"set","args":["Todos","y","99"]}]},
            {"kind":"for_each","text":"A","acts":[{"op":"add_var","args":["s","A.x"]}]},
            {"conds":[{"op":"trigger_once"}],"acts":[{"op":"add_var","args":["n","1"]}]},
            {"conds":[{"op":"or","sub":[{"op":"compare","args":["1","=","2"]},{"op":"compare","args":["s",">=","50"]}]}],"acts":[{"op":"set_var","args":["o","\"si\""]}]}
        ]"#);
        w.step(0.016, Input::default());
        w.step(0.016, Input::default());
        let a: Vec<Value> = w.insts.iter().filter(|i| i.obj == "A").map(|i| w.get(i.id as usize - 1, "tag")).collect();
        assert_eq!(a, vec![Value::Num(0.), Value::Num(1.)]); // solo el A que no toca a B
        assert_eq!(w.insts.iter().filter(|i| i.y == 99.).count(), 2); // A(0) y B(5) por el grupo
        assert_eq!(w.vars["n"], Value::Num(1.)); // solo una vez
        assert_eq!(w.vars["o"], Value::Str("si".into()));
    }

    #[test]
    fn funciones_y_temporizador() {
        let json = BASE.replace("EVENTS", r#"[
            {"conds":[{"op":"call_cond","args":["mayor","s","3"]}],"acts":[{"op":"set_var","args":["ok","1"]}]},
            {"acts":[{"op":"call","args":["sumar","2"]}]},
            {"conds":[{"op":"timer","args":["t","1"]}],"acts":[{"op":"change_scene","args":["S"]}]}
        ]"#).replace(r#""scenes""#, r#""functions":[
            {"name":"sumar","params":["k"],"events":[{"acts":[{"op":"add_var","args":["s","k"]}]}]},
            {"name":"mayor","is_condition":true,"params":["a","b"],"events":[
                {"conds":[{"op":"compare","args":["a",">","b"]}],"acts":[{"op":"return","args":["1"]}]}]}
        ],"scenes""#);
        let mut w = world(&json);
        w.step(0.016, Input::default()); // s=2
        assert!(!w.vars.contains_key("ok"));
        w.step(0.016, Input::default()); // s=2 → no; luego s=4
        w.step(0.016, Input::default()); // 4 > 3 → ok
        assert_eq!(w.vars["ok"], Value::Num(1.));
        for _ in 0..30 {
            w.step(0.05, Input::default());
        }
        assert!(w.time < 1.); // el temporizador reinició la escena
    }

    #[test]
    fn plataformas_aterriza() {
        let mut w = world(r#"{"name":"t","width":200,"height":200,"first_scene":"S","scenes":[{"name":"S",
            "objects":[{"name":"P","type":"box","w":10,"h":10,"color":[0,0,0],"behaviors":[{"type":"platformer"}]},
                       {"name":"G","type":"box","w":200,"h":10,"color":[0,0,0],"behaviors":[{"type":"platform"}]}],
            "instances":[{"obj":"P","x":50,"y":0},{"obj":"G","x":0,"y":100}]}]}"#);
        for _ in 0..120 {
            w.step(1. / 60., Input::default());
        }
        assert_eq!(w.insts[0].y, 90.);
        assert!(w.insts[0].on_floor);
        let mut jump = Input::default();
        jump.down.insert("Space".into());
        w.step(1. / 60., jump);
        w.step(1. / 60., Input::default());
        assert!(w.insts[0].y < 90.);
    }

    /// Mundo con un personaje "P" (20x30) y plataformas dadas como JSON.
    fn mundo_plataformas(p_extra: &str, objetos: &str, instancias: &str, eventos: &str) -> World {
        world(&format!(r#"{{"name":"t","width":800,"height":600,"first_scene":"S","scenes":[{{"name":"S",
            "objects":[{{"name":"P","type":"box","w":20,"h":30,"color":[0,0,0],"behaviors":[{{"type":"platformer"{p_extra}}}]}},{objetos}],
            "instances":[{instancias}],"events":[{eventos}]}}]}}"#))
    }

    fn correr(w: &mut World, segundos: f32, teclas: &[&str]) {
        let mut inp = Input::default();
        inp.down = teclas.iter().map(|k| k.to_string()).collect();
        for _ in 0..(segundos * 60.) as usize {
            w.step(1. / 60., inp.clone());
        }
    }

    fn p(w: &World) -> &Inst {
        w.insts.iter().find(|i| i.obj == "P").unwrap()
    }

    const SUELO: &str = r#"{"name":"G","type":"box","w":800,"h":20,"color":[0,0,0],"behaviors":[{"type":"platform"}]}"#;

    #[test]
    fn plataformas_pendiente_y_muro() {
        // Rampa: caja rotada -20° (sube hacia la derecha) apoyada en el suelo.
        let mut w = mundo_plataformas("", &format!(r#"{SUELO},
            {{"name":"R","type":"box","w":200,"h":20,"color":[0,0,0],"behaviors":[{{"type":"platform"}}]}}"#),
            r#"{"obj":"P","x":100,"y":170},{"obj":"G","x":0,"y":200},{"obj":"R","x":200,"y":165,"angle":-20}"#, "");
        let mut lo = f32::MAX;
        for _ in 0..90 {
            correr(&mut w, 1. / 60., &["Right"]);
            if p(&w).on_floor {
                lo = lo.min(p(&w).y);
            }
        }
        assert!(lo < 130., "subió la rampa caminando: y mínima en el suelo = {lo}");

        // Muro: se detiene pegado a él.
        let mut w = mundo_plataformas("", &format!(r#"{SUELO},
            {{"name":"M","type":"box","w":20,"h":200,"color":[0,0,0],"behaviors":[{{"type":"platform"}}]}}"#),
            r#"{"obj":"P","x":100,"y":170},{"obj":"G","x":0,"y":200},{"obj":"M","x":300,"y":0}"#, "");
        correr(&mut w, 2., &["Right"]);
        let x = p(&w).x + p(&w).w;
        assert!((299.0..=300.01).contains(&x), "borde derecho {x}");
        assert!(p(&w).on_floor);
    }

    #[test]
    fn plataformas_escalera_y_jumpthru() {
        let mut w = mundo_plataformas("", &format!(r#"{SUELO},
            {{"name":"L","type":"box","w":20,"h":200,"color":[0,0,0],"behaviors":[{{"type":"platform","ladder":true}}]}}"#),
            r#"{"obj":"P","x":100,"y":170},{"obj":"G","x":0,"y":200},{"obj":"L","x":100,"y":0}"#, "");
        correr(&mut w, 0.1, &[]);
        correr(&mut w, 0.5, &["Up"]);
        let y = p(&w).y;
        assert!(y < 110., "subió la escalera: y = {y}");
        correr(&mut w, 0.5, &[]);
        assert_eq!(p(&w).y, y); // en la escalera no cae

        // Jumpthru: aterriza encima, flecha abajo lo atraviesa, y desde abajo se sube saltando.
        let mut w = mundo_plataformas("", &format!(r#"{SUELO},
            {{"name":"J","type":"box","w":200,"h":10,"color":[0,0,0],"behaviors":[{{"type":"platform","jumpthru":true}}]}}"#),
            r#"{"obj":"P","x":100,"y":50},{"obj":"G","x":0,"y":300},{"obj":"J","x":50,"y":200}"#, "");
        correr(&mut w, 1., &[]);
        assert_eq!((p(&w).y, p(&w).on_floor), (170., true));
        correr(&mut w, 1. / 60., &["Down"]);
        correr(&mut w, 1., &[]);
        assert_eq!((p(&w).y, p(&w).on_floor), (270., true));
        correr(&mut w, 0.3, &["Space"]);
        correr(&mut w, 1.5, &[]);
        assert_eq!((p(&w).y, p(&w).on_floor), (170., true)); // el salto (~140 px) alcanza los 100 px
    }

    #[test]
    fn plataformas_borde_y_movil() {
        let mut w = mundo_plataformas(r#","can_grab_platforms":true"#,
            r#"{"name":"B","type":"box","w":100,"h":100,"color":[0,0,0],"behaviors":[{"type":"platform","grabbable":true}]}"#,
            r#"{"obj":"P","x":176,"y":20},{"obj":"B","x":200,"y":100}"#, "");
        correr(&mut w, 1., &[]);
        assert_eq!(w.insts[0].pstate, GRABBING);
        assert_eq!(p(&w).y, 100.);
        correr(&mut w, 0.1, &["Space"]);
        assert!(p(&w).y < 100.); // salta desde el borde

        // Plataforma móvil: lleva al personaje.
        let mut w = mundo_plataformas("",
            r#"{"name":"M","type":"box","w":200,"h":20,"color":[0,0,0],"behaviors":[{"type":"platform"}]}"#,
            r#"{"obj":"P","x":100,"y":170},{"obj":"M","x":50,"y":200}"#,
            r#"{"acts":[{"op":"add","args":["M","x","60*dt"]}]}"#);
        correr(&mut w, 0.2, &[]);
        let x0 = p(&w).x;
        correr(&mut w, 1., &[]);
        let moved = p(&w).x - x0;
        assert!((55.0..65.0).contains(&moved), "avanzó {moved}");
        assert!(p(&w).on_floor);
    }

    #[test]
    fn mapa_de_baldosas() {
        // 10x4 baldosas de 32: suelo completo en la fila 3; en la fila 2, una baldosa de pasto (5) que no choca.
        let mut data = vec![-1; 40];
        data[30..40].fill(0);
        data[2 * 10 + 6] = 5;
        let data = serde_json::to_string(&data).unwrap();
        let mut w = mundo_plataformas("", &format!(r#"{{"name":"M","type":"tilemap","tileset":"t.png","tile":32,"cols":10,"rows":4,
            "data":{data},"pass":[5],"behaviors":[{{"type":"platform"}}]}}"#),
            r#"{"obj":"P","x":40,"y":0},{"obj":"M","x":0,"y":0}"#,
            r#"{"conds":[{"op":"key_down","args":["X"]}],"acts":[{"op":"set_tile","args":["M","floor(P.x / 32)","3","-1"]},{"op":"set_tile","args":["M","floor((P.x + P.w) / 32)","3","-1"]}]},
               {"acts":[{"op":"set_var","args":["bajo","tile(M, P.cx, P.y + P.h + 1)"]}]}"#);
        correr(&mut w, 1., &[]);
        assert_eq!((p(&w).y, p(&w).on_floor), (66., true)); // 96 - 30
        assert_eq!(w.vars["bajo"], Value::Num(0.));
        // Camina sobre las uniones entre baldosas sin trabarse; el pasto (decoración) no lo frena.
        correr(&mut w, 0.8, &["Right"]);
        assert!(p(&w).x > 200., "avanzó hasta x = {}", p(&w).x);
        assert_eq!((p(&w).y, p(&w).on_floor), (66., true));
        // Romper el suelo bajo sus pies: cae.
        correr(&mut w, 1. / 60., &["X"]);
        correr(&mut w, 0.5, &[]);
        assert!(p(&w).y > 100., "cayó: y = {}", p(&w).y);
        assert_eq!(w.shapes(1).len(), 2); // el suelo quedó en dos tiras

        // Nivel con un hueco de 3 baldosas: al caer pegado a la pared derecha del hueco no se queda
        // "parado" sobre el borde de la pared (un rectángulo que solo roza al personaje no es suelo).
        let mut data = vec![-1; 20 * 11];
        for r in 8..11 {
            for c in (0..9).chain(12..20) {
                data[r * 20 + c] = 0;
            }
        }
        let data = serde_json::to_string(&data).unwrap();
        let mut w = mundo_plataformas("", &format!(r#"{{"name":"M","type":"tilemap","tileset":"t.png","tile":32,"cols":20,"rows":11,
            "data":{data},"behaviors":[{{"type":"platform"}}]}}"#),
            r#"{"obj":"P","x":40,"y":200},{"obj":"M","x":0,"y":0}"#, "");
        correr(&mut w, 2.5, &["Right"]);
        assert!(p(&w).y > 400., "cayó por el hueco: y = {}", p(&w).y);
    }

    #[test]
    fn si_no_y_partida_guardada() {
        let mut w = with(r#"[
            {"conds":[{"op":"compare","args":["s",">","5"]}],"acts":[{"op":"set_var","args":["r","\"grande\""]}]},
            {"kind":"else","conds":[{"op":"compare","args":["s",">","2"]}],"acts":[{"op":"set_var","args":["r","\"medio\""]}]},
            {"kind":"else","acts":[{"op":"set_var","args":["r","\"chico\""]}]},
            {"conds":[{"op":"start"}],"acts":[{"op":"save_value","args":["record","saved(\"record\", 7) + 1"]}]}
        ]"#);
        w.step(0.016, Input::default());
        assert_eq!(w.vars["r"], Value::Str("chico".into()));
        assert_eq!(w.storage["record"], Value::Num(8.));
        assert!(w.cmds.iter().any(|c| matches!(c, Cmd::Persist(v) if v["record"] == Value::Num(8.))));
        w.vars.insert("s".into(), Value::Num(3.));
        w.step(0.016, Input::default());
        assert_eq!(w.vars["r"], Value::Str("medio".into()));
        w.vars.insert("s".into(), Value::Num(9.));
        w.step(0.016, Input::default());
        assert_eq!(w.vars["r"], Value::Str("grande".into()));
    }

    #[test]
    fn alarmas_esperar_y_grilla() {
        let mut w = with(r#"[
            {"conds":[{"op":"start"}],"acts":[{"op":"set_alarm","args":["A","bum","0.5"]},{"op":"set_var","args":["paso","1"]},
                {"op":"wait","args":["1"]},{"op":"set_var","args":["paso","2"]},{"op":"set","args":["A","y","50"]}],
             "sub":[{"acts":[{"op":"set_var","args":["sub","1"]}]}]},
            {"conds":[{"op":"alarm","args":["A","bum"]}],"acts":[{"op":"add_var","args":["alarmas","picked(A)"]}]}
        ]"#);
        let run = |w: &mut World, secs: f32| for _ in 0..(secs * 60.) as usize { w.step(1. / 60., Input::default()) };
        run(&mut w, 0.3);
        assert_eq!((w.vars["paso"].clone(), w.vars.get("sub")), (Value::Num(1.), None));
        run(&mut w, 0.4);
        assert_eq!(w.vars["alarmas"], Value::Num(2.)); // las 2 instancias de A, una sola vez cada una
        run(&mut w, 0.5);
        assert_eq!((w.vars["paso"].clone(), w.vars["sub"].clone()), (Value::Num(2.), Value::Num(1.)));
        assert_eq!(w.insts.iter().filter(|i| i.obj == "A" && i.y == 50.).count(), 2); // mismas instancias de antes
        run(&mut w, 1.);
        assert_eq!(w.vars["alarmas"], Value::Num(2.));

        // Grilla: 400 objetos en fila, cada uno toca solo a sus vecinos.
        let objs: String = (0..400).map(|k| format!(r#"{{"obj":"A","x":{},"y":0}}"#, k * 9)).collect::<Vec<_>>().join(",");
        let mut w = world(&BASE.replace("EVENTS", r#"[{"conds":[{"op":"collides","args":["A","B"]}],"acts":[{"op":"add_var","args":["s","1"]}]}]"#)
            .replace(r#"{"obj":"A","x":0,"y":0},{"obj":"A","x":50,"y":50},{"obj":"B","x":5,"y":5}"#, &format!(r#"{objs},{{"obj":"B","x":1800,"y":0}}"#)));
        let t = std::time::Instant::now();
        w.step(1. / 60., Input::default());
        assert_eq!(w.vars["s"], Value::Num(1.));
        let pairs = w.hits(&(0..400).collect::<Vec<_>>(), &(0..400).collect::<Vec<_>>(), 0.);
        assert_eq!(pairs.len(), 399 * 2); // cada vecino en ambos sentidos
        assert!(t.elapsed().as_millis() < 200, "lento: {:?}", t.elapsed());
    }

    #[test]
    fn tween_y_camara() {
        let mut w = with(r#"[
            {"conds":[{"op":"start"}],"acts":[{"op":"tween","args":["B","x","105","1","ease_out"]},
                {"op":"camera_follow","args":["B","0"]},{"op":"camera_bounds","args":["0","0","400","300"]}]}
        ]"#);
        let run = |w: &mut World, secs: f32| for _ in 0..(secs * 60.).round() as usize { w.step(1. / 60., Input::default()) };
        run(&mut w, 0.5);
        let x = w.insts[2].x;
        assert!(x > 55. + 5. && x < 105., "a mitad de camino con ease_out va adelantado: {x}");
        run(&mut w, 0.6);
        assert_eq!(w.insts[2].x, 105.);
        assert!(w.insts[2].tweens.is_empty());
        // La cámara sigue a B (centro 110, 10) pero no sale de los límites: medio ancho 100, medio alto 100.
        assert_eq!(w.camera, (110., 100.));
        assert_eq!((ease("bounce", 1.), ease("elastic", 0.), ease("back", 1.)), (1., 0., 1.));
    }

    #[test]
    fn scripts_rhai() {
        let mut w = with(r#"[
            {"acts":[{"op":"script","args":["let n = 0; for id in all(\"A\") { set_prop(id, \"x\", prop(id, \"x\") + 1.0); n += 1; } set_var(\"n\", n); save(\"visto\", true);"]}]},
            {"conds":[{"op":"script_if","args":["count(\"A\") == 2 && get_var(\"n\") == 2"]}],"acts":[{"op":"set_var","args":["ok","1"]}]},
            {"conds":[{"op":"start"}],"acts":[{"op":"script","args":["let id = create(\"B\", 10.0, 20.0); set_prop(id, \"angle\", point_angle(0.0, 0.0, 0.0, 1.0));"]}]},
            {"conds":[{"op":"start"}],"acts":[{"op":"script","args":["loop { }"]},{"op":"script","args":["esto no compila ("]}]}
        ]"#);
        w.step(0.016, Input::default());
        assert_eq!(w.vars["n"], Value::Num(2.));
        assert_eq!(w.vars["ok"], Value::Num(1.));
        assert_eq!((w.insts[0].x, w.insts[1].x), (1., 51.));
        assert_eq!(w.storage["visto"], Value::Bool(true));
        let nuevo = w.insts.iter().find(|i| i.obj == "B" && i.x == 10.).unwrap();
        assert_eq!(nuevo.angle, 90.);
        let log = w.log.borrow().join(" | ");
        assert!(log.contains("operations") || log.contains("operaciones"), "el ciclo infinito se corta: {log}");
        assert!(log.contains("sintaxis"), "{log}");
        assert!(crate::script::check("let a = 1 +").is_err() && crate::script::check("let a = 1;").is_ok());
    }

    #[test]
    fn propiedades_visuales() {
        let mut w = with(r##"[{"conds":[{"op":"start"}],"acts":[
            {"op":"set","args":["B","scale_x","2"]},{"op":"set","args":["B","tint","\"#ff0000\""]},{"op":"set","args":["B","visible","0"]}]}]"##);
        w.step(0.016, Input::default());
        let b = &w.insts[2];
        assert_eq!((b.w, b.x, b.tint, b.visible), (20., 0., [255, 0, 0], false)); // creció desde el centro (x 5 → 0)
        assert_eq!(w.get(2, "scale_x"), Value::Num(2.));
        assert_eq!(parse_color("verde"), Some([70, 200, 90]));
        assert_eq!(parse_color("#12ab"), None);
    }

    #[test]
    fn escena_3d() {
        let mut w = world(r#"{"name":"t","width":800,"height":600,"first_scene":"S","scenes":[{"name":"S","dim3":true,
            "objects":[{"name":"Jugador","type":"mesh","shape":"capsule","size":[1,2,1],"color":[200,80,80]},
                       {"name":"Caja","type":"mesh","size":[2,2,2]},
                       {"name":"Sol","type":"light","light":"sun"},{"name":"Cam","type":"camera3d"}],
            "instances":[{"obj":"Jugador","x":0,"y":1,"z":0},{"obj":"Caja","x":0,"y":1,"z":-4},{"obj":"Sol","x":0,"y":10,"z":0,"pitch":-60},
                         {"obj":"Cam","x":0,"y":3,"z":6}],
            "events":[{"conds":[{"op":"key_down","args":["Up"]}],"acts":[{"op":"move_forward","args":["Jugador","5 * dt"]}]},
                      {"conds":[{"op":"collides","args":["Jugador","Caja"]}],"acts":[{"op":"set_var","args":["choco","1"]}]},
                      {"acts":[{"op":"look_at3","args":["Cam","Jugador.x","Jugador.y","Jugador.z"]}]}]}]}"#);
        assert!(w.dim3);
        assert_eq!((w.insts[0].w, w.insts[0].h, w.insts[0].d), (1., 2., 1.));
        assert_eq!(w.insts[2].pitch, -60.);
        let mut up = Input::default();
        up.down.insert("Up".into());
        for _ in 0..30 {
            w.step(1. / 60., up.clone());
        }
        // Adelante es -Z: avanzó 2.5 m hacia la caja y todavía no la toca (borde en z = -3, jugador en -2.5 ± 0.5).
        assert!((w.insts[0].z + 2.5).abs() < 0.05 && w.insts[0].x.abs() < 1e-4, "z = {}", w.insts[0].z);
        assert!(!w.vars.contains_key("choco"));
        for _ in 0..12 {
            w.step(1. / 60., up.clone());
        }
        assert_eq!(w.vars["choco"], Value::Num(1.));
        // La cámara mira al jugador: hacia -Z (ángulo 0) y hacia abajo.
        let cam = &w.insts[3];
        assert!(cam.angle.abs() < 1. && cam.pitch < -5., "cámara {} {}", cam.angle, cam.pitch);
    }

    #[test]
    fn listas_y_estructuras() {
        let mut w = with(r#"[{"conds":[{"op":"start"}],"acts":[
            {"op":"list_add","args":["inv","\"espada\""]},{"op":"list_add","args":["inv","\"poción\""]},
            {"op":"list_remove","args":["inv","0"]},{"op":"list_set","args":["jug","\"vida\"","3"]},
            {"op":"set_var","args":["n","size(inv) + jug[\"vida\"]"]},{"op":"set_var","args":["t","inv[0]"]}]}]"#);
        w.step(0.016, Input::default());
        assert_eq!(w.vars["inv"], Value::List(vec![Value::Str("poción".into())]));
        assert_eq!(w.vars["n"], Value::Num(4.));
        assert_eq!(w.vars["t"], Value::Str("poción".into()));
        let j: Value = serde_json::from_str(r#"{"vida": 3}"#).unwrap();
        assert_eq!(w.vars["jug"], j);
    }

    #[test]
    fn ejemplos_corren() {
        for f in ["examples/plataformas/juego.json", "examples/recolector.json"] {
            let src = std::fs::read_to_string(format!("{}/{f}", env!("CARGO_MANIFEST_DIR"))).unwrap();
            let p: Project = serde_json::from_str(&src).unwrap_or_else(|e| panic!("{f}: {e}"));
            for s in &p.scenes {
                let mut w = World::new(p.clone());
                w.load_scene(&s.name);
                for _ in 0..120 {
                    w.step(1. / 60., Input::default());
                }
            }
        }
    }

    #[test]
    fn sat_rotado() {
        let a = Inst { w: 10., h: 10., ..Default::default() };
        let mut b = Inst { x: 12., w: 10., h: 10., ..Default::default() };
        assert!(sat(&a.hitbox(), &b.hitbox()).is_none());
        b.angle = 45.; // la esquina rotada alcanza a A
        assert!(sat(&a.hitbox(), &b.hitbox()).is_some());
    }

    /// Escena de 800x600 con los objetos, instancias y eventos dados (JSON sin corchetes externos).
    fn escena(extra: &str, objetos: &str, instancias: &str, eventos: &str) -> World {
        world(&format!(r#"{{"name":"t","width":800,"height":600,"first_scene":"S","scenes":[{{"name":"S"{extra},
            "objects":[{objetos}],"instances":[{instancias}],"events":[{eventos}]}}]}}"#))
    }

    fn por_obj<'a>(w: &'a World, o: &str) -> &'a Inst {
        w.insts.iter().find(|i| i.obj == o).unwrap()
    }

    const PISO_FISICO: &str = r#"{"name":"Piso","type":"box","w":800,"h":40,"color":[0,0,0],"behaviors":[{"type":"physics","body":"static"}]}"#;

    #[test]
    fn fisica_cae_y_reposa() {
        let mut w = escena("", &format!(r#"{PISO_FISICO},
            {{"name":"Caja","type":"box","w":40,"h":40,"color":[0,0,0],"behaviors":[{{"type":"physics"}}]}}"#),
            r#"{"obj":"Caja","x":380,"y":100},{"obj":"Piso","x":0,"y":500}"#,
            r#"{"conds":[{"op":"collides","args":["Caja","Piso"]}],"acts":[{"op":"set_var","args":["toca","1"]}]}"#);
        correr(&mut w, 0.3, &[]);
        assert!(por_obj(&w, "Caja").y > 130., "cae: y = {}", por_obj(&w, "Caja").y);
        assert!(w.get(0, "vy").num() > 100., "va cayendo");
        correr(&mut w, 3., &[]);
        let c = por_obj(&w, "Caja");
        assert!((c.y + c.h - 500.).abs() < 1.5, "reposa sobre el piso: abajo en {}", c.y + c.h);
        assert!(w.get(0, "vy").num().abs() < 5. && c.angle.abs() < 1., "quieta y derecha");
        assert_eq!(w.vars["toca"], Value::Num(1.));
        assert!(w.profile.iter().any(|(l, _)| l == "Física"));
        // Cambiar de escena rehace la física desde cero.
        w.load_scene("S");
        correr(&mut w, 0.1, &[]);
        assert!(por_obj(&w, "Caja").y < 130.);
    }

    #[test]
    fn fisica_rebote_impulso_y_teletransporte() {
        // Rebote: con restitución 0.8 vuelve a subir después de tocar el piso.
        let mut w = escena("", &format!(r#"{PISO_FISICO},
            {{"name":"Pelota","type":"box","w":30,"h":30,"color":[0,0,0],"behaviors":[{{"type":"physics","shape":"circle","restitution":0.8}}]}}"#),
            r#"{"obj":"Pelota","x":385,"y":200},{"obj":"Piso","x":0,"y":500}"#, "");
        let mut lowest = (0f32, 0usize);
        let mut subio = false;
        for k in 0..180 {
            correr(&mut w, 1. / 60., &[]);
            let y = por_obj(&w, "Pelota").y;
            if y > lowest.0 {
                lowest = (y, k);
            } else if lowest.0 > 440. && y < lowest.0 - 40. {
                subio = true;
            }
        }
        assert!(subio, "rebotó (más bajo {lowest:?})");

        // Sin gravedad: un impulso lo mueve; set x lo teletransporta; set_velocity y vx; un cinemático sigue a los eventos.
        let mut w = escena(r#","gravity":[0,0]"#, r#"{"name":"C","type":"box","w":50,"h":50,"color":[0,0,0],"behaviors":[{"type":"physics"}]},
            {"name":"K","type":"box","w":20,"h":20,"color":[0,0,0],"behaviors":[{"type":"physics","body":"kinematic"}]}"#,
            r#"{"obj":"C","x":100,"y":100},{"obj":"K","x":600,"y":100}"#,
            r#"{"conds":[{"op":"key_down","args":["A"]}],"acts":[{"op":"apply_impulse","args":["C","100","0"]}]},
               {"conds":[{"op":"key_down","args":["T"]}],"acts":[{"op":"set","args":["C","x","400"]}]},
               {"conds":[{"op":"key_down","args":["V"]}],"acts":[{"op":"set_velocity","args":["C","0","-50"]},{"op":"set_angular_velocity","args":["C","90"]}]},
               {"acts":[{"op":"add","args":["K","y","60 * dt"]}]}"#);
        correr(&mut w, 0.1, &[]);
        assert_eq!(por_obj(&w, "C").x, 100.);
        correr(&mut w, 1. / 60., &["A"]);
        correr(&mut w, 1., &[]);
        // Masa 1 kg (50x50 px, densidad 1): el impulso 100 da 100 px/s.
        assert!((w.get(0, "vx").num() - 100.).abs() < 1., "vx = {}", w.get(0, "vx").num());
        let x = por_obj(&w, "C").x;
        assert!((95.0..110.0).contains(&(x - 100.)), "avanzó {x}");
        correr(&mut w, 1. / 60., &["T"]);
        correr(&mut w, 1. / 60., &[]);
        assert!((por_obj(&w, "C").x - 400.).abs() < 5., "teletransportado: {}", por_obj(&w, "C").x);
        correr(&mut w, 1. / 60., &["V"]);
        correr(&mut w, 0.5, &[]);
        assert!((w.get(0, "vy").num() + 50.).abs() < 1. && (w.get(0, "angular_velocity").num() - 90.).abs() < 1.);
        assert!(por_obj(&w, "C").angle > 30., "gira: {}", por_obj(&w, "C").angle);
        assert!(por_obj(&w, "K").y > 180., "el cinemático sigue a los eventos: {}", por_obj(&w, "K").y);
    }

    #[test]
    fn fisica_juntas_y_borrado() {
        // Péndulo: una bisagra une la pesa a un clavo estático; la pesa cuelga y no se aleja del clavo.
        let mut w = escena("", r#"{"name":"Clavo","type":"box","w":10,"h":10,"color":[0,0,0],"behaviors":[{"type":"physics","body":"static"}]},
            {"name":"Pesa","type":"box","w":20,"h":20,"color":[0,0,0],"behaviors":[{"type":"physics"}]},
            {"name":"Globo","type":"box","w":20,"h":20,"color":[0,0,0],"behaviors":[{"type":"physics","gravity_scale":0}]}"#,
            r#"{"obj":"Clavo","x":395,"y":95},{"obj":"Pesa","x":490,"y":90},{"obj":"Globo","x":100,"y":100}"#,
            r#"{"conds":[{"op":"start"}],"acts":[{"op":"joint_revolute","args":["Pesa","Clavo","400","100"]},{"op":"joint_distance","args":["Globo","Clavo",""]}]},
               {"conds":[{"op":"key_down","args":["D"]}],"acts":[{"op":"delete","args":["Clavo"]}]}"#);
        correr(&mut w, 2., &[]);
        assert_eq!(w.fisica.juntas(), 2);
        let p = por_obj(&w, "Pesa");
        let d = (p.cx() - 400.).hypot(p.cy() - 100.);
        assert!((d - 100.).abs() < 3., "a 100 px del clavo: {d}");
        assert!(p.cy() > 120., "se balancea hacia abajo: {}", p.cy());
        correr(&mut w, 1. / 60., &["D"]);
        correr(&mut w, 1. / 60., &[]);
        assert_eq!(w.fisica.juntas(), 0, "las juntas se van con el cuerpo");
        assert!(!w.fisica.tiene(1));
    }

    #[test]
    fn raycast_elige_el_primero() {
        let mut w = escena("", r#"{"name":"Ojo","type":"box","w":10,"h":10,"color":[0,0,0]},
            {"name":"Muro","type":"box","w":20,"h":100,"color":[0,0,0]}"#,
            r#"{"obj":"Ojo","x":95,"y":95},{"obj":"Muro","x":300,"y":50},{"obj":"Muro","x":200,"y":50},{"obj":"Muro","x":400,"y":300}"#,
            r#"{"conds":[{"op":"raycast","args":["Ojo","0","500","Muro"]}],"acts":[{"op":"set","args":["Muro","tocado","1"]},{"op":"set_var","args":["rx","ray_x"]}]},
               {"conds":[{"op":"raycast","args":["Ojo","-90","500","Muro"],"not":true}],"acts":[{"op":"set_var","args":["arriba_libre","1"]}]},
               {"conds":[{"op":"raycast","args":["Ojo","0","50","Muro"]}],"acts":[{"op":"set_var","args":["corto","1"]}]}"#);
        w.step(1. / 60., Input::default());
        let tocados: Vec<f32> = w.insts.iter().filter(|i| i.vars.contains_key("tocado")).map(|i| i.x).collect();
        assert_eq!(tocados, vec![200.]); // el más cercano
        assert_eq!(w.vars["rx"], Value::Num(200.));
        assert_eq!(w.vars["arriba_libre"], Value::Num(1.));
        assert!(!w.vars.contains_key("corto"), "el rayo corto no llega");
        assert_eq!(ray_poly((0., 0.), (1., 0.), 100., &[(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)]), Some(0.)); // empieza adentro
    }

    #[test]
    fn pathfinder_rodea_la_pared() {
        let mut w = escena("", r#"{"name":"Bicho","type":"box","w":20,"h":20,"color":[0,0,0],"behaviors":[{"type":"pathfinder","speed":300,"cell":16}]},
            {"name":"Pared","type":"box","w":20,"h":600,"color":[0,0,0],"behaviors":[{"type":"obstacle"}]}"#,
            r#"{"obj":"Bicho","x":90,"y":190},{"obj":"Pared","x":290,"y":-200}"#,
            r#"{"conds":[{"op":"start"}],"acts":[{"op":"move_to","args":["Bicho","500","200"]}]},
               {"conds":[{"op":"path_found","args":["Bicho"]}],"acts":[{"op":"set_var","args":["hay","1"]},{"op":"set_var","args":["largo","max(largo, Bicho.path_length)"]}]},
               {"conds":[{"op":"path_done","args":["Bicho"]},{"op":"trigger_once"}],"acts":[{"op":"set_var","args":["llego","time"]}]}"#);
        let mut max_y = 0f32;
        let mut choco = false;
        for _ in 0..(4. * 60.) as usize {
            w.step(1. / 60., Input::default());
            let b = por_obj(&w, "Bicho").clone();
            max_y = max_y.max(b.cy());
            choco |= sat(&b.hitbox(), &por_obj(&w, "Pared").hitbox()).is_some();
        }
        assert_eq!(w.vars["hay"], Value::Num(1.));
        assert!(w.vars["largo"].num() > 550., "rodea: largo {}", w.vars["largo"].num());
        assert!(max_y > 400., "pasó por debajo de la pared: {max_y}");
        assert!(!choco, "no atraviesa la pared");
        let b = por_obj(&w, "Bicho");
        assert!((b.cx() - 500.).abs() < 0.01 && (b.cy() - 200.).abs() < 0.01, "llegó: {} {}", b.cx(), b.cy());
        assert!(w.vars.contains_key("llego"));
        assert_eq!((w.get(0, "speed").num(), w.get(0, "path_length").num()), (0., 0.));
    }

    #[test]
    fn particulas_rafaga_vida_y_autodestruir() {
        let mut w = escena("", r#"{"name":"Boom","type":"particles","burst":20,"rate":0,"life":[0.5,0.5],"speed":[100,100],"auto_destroy":true,
                "color_start":[255,0,0],"color_end":[0,0,255],"alpha_start":255,"alpha_end":0,"size_start":10,"size_end":2},
            {"name":"Humo","type":"particles","rate":10,"max":5,"life":[10,10],"gravity":[0,-100]}"#,
            r#"{"obj":"Boom","x":100,"y":100,"layer":""},{"obj":"Humo","x":400,"y":400}"#,
            r#"{"conds":[{"op":"key_down","args":["E"]}],"acts":[{"op":"set_emitting","args":["Humo","0"]},{"op":"emit_burst","args":["Humo","3"]}]}"#);
        w.step(1. / 60., Input::default());
        let boom: Vec<&Particle> = w.particles.iter().filter(|p| p.obj == "Boom").collect();
        assert_eq!(boom.len(), 20, "ráfaga al crearse");
        assert!(boom.iter().all(|p| (p.x, p.y) == (108., 108.) && p.layer.is_empty()), "salen del centro del emisor");
        let p0 = boom[0].clone();
        correr(&mut w, 0.25, &[]);
        let p = w.particles.iter().find(|p| p.obj == "Boom").unwrap();
        assert!((p.age - 0.25).abs() < 0.02 && (p.x - p0.x).hypot(p.y - p0.y) > 20., "envejecen y se mueven");
        let def = match &w.object_def("Boom").unwrap().kind { Kind::Particles(d) => (**d).clone(), _ => unreachable!() };
        let (rgb, alpha, size) = crate::particulas::particle_look(&def, &Particle { age: 0.25, life: 0.5, ..Default::default() });
        assert_eq!((rgb, alpha, size), ([128, 0, 128], 127.5, 6.));
        correr(&mut w, 0.4, &[]);
        assert!(w.particles.iter().all(|p| p.obj != "Boom"), "mueren al cumplir su vida");
        assert!(w.insts.iter().all(|i| i.obj != "Boom"), "el emisor se borró solo");
        // El humo suelta 10 por segundo pero no más de 5 a la vez; la gravedad negativa lo sube.
        assert_eq!(w.particles.len(), 5);
        assert!(w.particles.iter().all(|p| p.y < 408.));
        correr(&mut w, 1. / 60., &["E"]);
        assert_eq!(w.get(0, "emitting"), Value::Num(0.));
        assert_eq!(w.particles.len(), 5, "la ráfaga respeta el máximo");
    }

    #[test]
    fn origen_y_mascara() {
        // Sprite 40x20 con origen en (10, 10): x, y es ese punto y gira alrededor de él.
        let sprite = |extra: &str| format!(r#"{{"name":"S","type":"sprite","w":40,"h":20,"animations":[{{"name":"a","frames":["a.png"]}}]{extra}}}"#);
        let mut w = escena("", &format!(r#"{},{{"name":"B","type":"box","w":10,"h":10,"color":[0,0,0]}}"#, sprite(r#","origin":[10,10]"#)),
            r#"{"obj":"S","x":100,"y":100},{"obj":"B","x":125,"y":95}"#,
            r#"{"conds":[{"op":"collides","args":["S","B"]}],"acts":[{"op":"set_var","args":["toca","1"]}]}"#);
        let s = w.insts[0].clone();
        assert_eq!(s.origin, Some((0.25, 0.5)));
        assert_eq!((s.cx(), s.cy()), (110., 100.));
        assert_eq!(s.draw_rect(), (90., 90., 40., 20., 100., 100.));
        w.step(1. / 60., Input::default());
        assert_eq!(w.vars["toca"], Value::Num(1.)); // B (125..135) está dentro del sprite (90..130)
        // Girado 90°: el sprite ahora va de y 90 a 130 alrededor del origen; ya no toca a B.
        w.insts[0].angle = 90.;
        let r = w.insts[0].hitbox();
        let (x0, y0) = r.iter().fold((f32::MAX, f32::MAX), |a, p| (a.0.min(p.0), a.1.min(p.1)));
        assert!((x0 - 90.).abs() < 1e-3 && (y0 - 90.).abs() < 1e-3, "{r:?}");
        assert!((w.insts[0].cx() - 100.).abs() < 1e-3 && (w.insts[0].cy() - 110.).abs() < 1e-3);
        w.vars.clear();
        w.step(1. / 60., Input::default());
        assert!(!w.vars.contains_key("toca"));
        // Escala desde el origen: x, y no cambian.
        w.put(0, "scale_x", Value::Num(2.));
        assert_eq!((w.insts[0].x, w.insts[0].w), (100., 80.));
        w.put(0, "cx", Value::Num(0.));
        assert!(w.insts[0].cx().abs() < 1e-3);

        // Máscara triangular en la mitad izquierda: B en la esquina de abajo a la derecha no toca.
        let mut w = escena("", &format!(r#"{},{{"name":"B","type":"box","w":4,"h":4,"color":[0,0,0]}}"#, sprite(r#","mask":[[0,0],[20,0],[0,20]]"#)),
            r#"{"obj":"S","x":0,"y":0},{"obj":"B","x":30,"y":14},{"obj":"B","x":2,"y":2}"#,
            r#"{"conds":[{"op":"collides","args":["S","B"]}],"acts":[{"op":"set","args":["B","toca","1"]}]}"#);
        w.step(1. / 60., Input::default());
        assert_eq!((w.get(1, "toca"), w.get(2, "toca")), (Value::Num(0.), Value::Num(1.)));
        assert_eq!(w.polys(0)[0].len(), 3);
        // Sin origen ni máscara: igual que siempre.
        let w = escena("", &sprite(""), r#"{"obj":"S","x":10,"y":10,"angle":30}"#, "");
        let i = &w.insts[0];
        assert_eq!((i.origin, i.mask.is_empty(), i.cx(), i.draw_rect()), (None, true, 30., (10., 10., 40., 20., 30., 20.)));
        assert_eq!(i.mask_poly(), i.hitbox().to_vec());
    }

    #[test]
    fn perfilador_mandos_efectos_y_texto() {
        let mut w = with(r#"[{"conds":[{"op":"key_down","args":["Pad1A"]}],"acts":[{"op":"set_var","args":["eje","axis(\"Pad1LX\") + axis(\"Nada\")"]}]},
            {"acts":[{"op":"set_effect","args":["A","flash","0.5"]},{"op":"set_effect","args":["B","outline",""]}]}]"#);
        let mut inp = Input::default();
        inp.down.insert("Pad1A".into());
        inp.axes.insert("Pad1LX".into(), -0.5);
        w.step(1. / 60., inp);
        assert_eq!(w.vars["eje"], Value::Num(-0.5));
        let labels: Vec<&str> = w.profile.iter().map(|p| p.0.as_str()).collect();
        assert_eq!(labels, ["Evento 1: Tecla Pad1A presionada", "Evento 2: siempre", "Comportamientos"]);
        assert_eq!((w.insts[0].effect.as_str(), w.insts[0].effect_amount), ("flash", 0.5));
        assert_eq!((w.insts[2].effect.as_str(), w.insts[2].effect_amount), ("outline", 1.));
        assert_eq!(w.project.version, FORMAT_VERSION, "World::new migra el proyecto");
        // Texto en líneas: cada letra mide 10 px.
        let m = |s: &str| s.chars().count() as f32 * 10.;
        assert_eq!(wrap_lines("hola que tal\nchau", 20., 75., m), ["hola", "que tal", "chau"]);
        assert_eq!(wrap_lines("abcdefghij", 20., 40., m), ["abcd", "efgh", "ij"]);
        assert_eq!(wrap_lines("una sola", 20., 0., m), ["una sola"]);
        assert_eq!(crate::catalog::key_label("Pad1A"), "Mando: A");
        assert_eq!(crate::catalog::key_label("Pad2Up"), "Mando 2: cruceta arriba");
    }
}
