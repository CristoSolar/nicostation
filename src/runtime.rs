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
}

#[derive(Debug, Clone, PartialEq)]
pub enum Cmd {
    PlaySound { file: String, volume: f32 },
    PlayMusic { file: String, volume: f32 },
    StopMusic,
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

/// Instancias elegidas por objeto real (no por grupo). Objeto ausente = todas sus instancias.
pub type Sol = HashMap<String, Vec<usize>>;

pub struct World {
    pub project: Rc<Project>,
    pub globals: Vars,
    pub scene: usize,
    pub insts: Vec<Inst>,
    pub vars: Vars,
    pub camera: (f32, f32),
    pub zoom: f32,
    pub time: f64,
    pub dt: f32,
    pub input: Input,
    pub cmds: Vec<Cmd>,
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
    pub fn new(project: Project) -> Self {
        let first = project.first_scene.clone();
        let mut w = World {
            globals: project.globals.clone(),
            project: Rc::new(project),
            scene: 0,
            insts: vec![],
            vars: Vars::new(),
            camera: (0., 0.),
            zoom: 1.,
            time: 0.,
            dt: 0.,
            input: Input::default(),
            cmds: vec![],
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

    fn warn(&self, m: String) {
        let mut log = self.log.borrow_mut();
        if !log.contains(&m) {
            eprintln!("{m}");
            log.push(m);
        }
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
        self.insts.clear();
        self.vars = sc.vars.clone();
        self.timers.clear();
        self.every.clear();
        self.once_now.clear();
        self.once_last.clear();
        self.camera = (p.width / 2., p.height / 2.);
        self.zoom = 1.;
        self.time = 0.;
        self.first_frame = true;
        self.dragging = None;
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
        };
        let tiles = match &t.kind {
            Kind::Tilemap { cols, rows, data, .. } => (0..cols * rows).map(|k| data.get(k).copied().unwrap_or(-1)).collect(),
            _ => vec![],
        };
        self.next_id += 1;
        self.insts.push(Inst {
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

    fn shapes_touch(&self, i: usize, j: usize) -> bool {
        let b = self.shapes(j);
        self.shapes(i).iter().any(|a| b.iter().any(|b| sat(a, b).is_some()))
    }

    // ---------- frame ----------

    /// Avanza un frame. Orden como GDevelop: animaciones y comportamientos → eventos → post-eventos.
    pub fn step(&mut self, dt: f32, input: Input) {
        self.dt = dt.min(0.05);
        self.time += self.dt as f64;
        self.input = input;
        self.animate();
        self.behaviors_pre();
        self.once_last = std::mem::take(&mut self.once_now);
        let p = self.project.clone();
        self.run_events(&p.scenes[self.scene].events, &Sol::new());
        self.behaviors_post();
        self.insts.retain(|i| !i.dead);
        self.first_frame = false;
        if let Some(s) = self.pending.take() {
            self.load_scene(&s);
        }
    }

    fn run_events(&mut self, evs: &[Event], sol: &Sol) {
        for e in evs {
            if e.disabled || self.pending.is_some() {
                continue;
            }
            match e.kind {
                EventKind::Comment => {}
                EventKind::Group => self.run_events(&e.sub, sol),
                EventKind::Standard => self.run_once(e, sol.clone()),
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

    fn run_once(&mut self, e: &Event, mut s: Sol) {
        if self.conds(&e.conds, &mut s) {
            self.acts(&e.acts, &mut s);
            self.run_events(&e.sub, &s);
        }
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
                let hit = |w: &World, i: usize, j: usize| {
                    i != j
                        && if dist > 0. {
                            (w.insts[i].cx() - w.insts[j].cx()).hypot(w.insts[i].cy() - w.insts[j].cy()) < dist
                        } else {
                            w.shapes_touch(i, j)
                        }
                };
                let mine = self.pick(a(0), sol);
                if c.not {
                    // Invertida: solo se filtra la primera lista (las que no tocan a ninguna).
                    return self.filter(a(0), false, sol, |w, i| !others.iter().any(|&j| hit(w, i, j)));
                }
                let (mut ka, mut kb) = (vec![], vec![]);
                for &i in &mine {
                    for &j in &others {
                        if hit(self, i, j) {
                            if !ka.contains(&i) { ka.push(i) }
                            if !kb.contains(&j) { kb.push(j) }
                        }
                    }
                }
                let ok = !ka.is_empty();
                self.set_picked(a(0), sol, ka);
                self.set_picked(a(1), sol, kb);
                ok
            }
            "cursor_on" => self.filter(a(0), c.not, sol, |w, i| {
                let (mx, my) = w.mouse_on(&w.insts[i].layer);
                w.shapes(i).iter().any(|s| point_in(s, mx, my))
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
            "is_moving" => self.filter(a(0), c.not, sol, |w, i| w.insts[i].vx != 0. || w.insts[i].vy != 0.),
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
                            for shape in self.shapes(j) {
                                if let Some((dx, dy)) = sat(&self.insts[i].hitbox(), &shape) {
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
            "reset_timer" => {
                self.timers.insert(a(0).into(), self.time);
            }
            "change_scene" => self.pending = Some(a(0).into()),
            "restart_scene" => self.pending = Some(self.scene_def().name.clone()),
            "camera" => self.camera = (num(self, a(0), sol, None), num(self, a(1), sol, None)),
            "camera_zoom" => self.zoom = num(self, a(0), sol, None).max(0.01),
            "play_sound" | "play_music" => {
                let volume = if a(1).is_empty() { 100. } else { num(self, a(1), sol, None) };
                let file = a(0).to_string();
                self.cmds.push(if c.op == "play_sound" { Cmd::PlaySound { file, volume } } else { Cmd::PlayMusic { file, volume } });
            }
            "stop_music" => self.cmds.push(Cmd::StopMusic),
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
            "anim" => Value::Str(self.anim_name(i)),
            "text" => Value::Str(me.text.clone()),
            _ => me.vars.get(p).cloned().unwrap_or(Value::Num(0.)),
        }
    }

    fn put(&mut self, i: usize, p: &str, v: Value) {
        let me = &mut self.insts[i];
        let f = v.num() as f32;
        match p {
            "x" => me.x = f,
            "y" => me.y = f,
            "cx" => me.x = f - me.w / 2.,
            "cy" => me.y = f - me.h / 2.,
            "w" => me.w = f,
            "h" => me.h = f,
            "angle" => me.angle = f,
            "opacity" => me.opacity = f.clamp(0., 255.),
            "z" => me.z = f,
            "text" => me.text = v.text(),
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

    fn random(&self) -> f64 {
        // xorshift64*
        let mut x = self.rng.get();
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng.set(x);
        (x.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64 / (1u64 << 53) as f64
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
                    Behavior::Platformer { .. } => self.platformer(i, &b, &plats),
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

    fn behaviors_post(&mut self) {
        let p = self.project.clone();
        let (vw, vh) = (p.width / self.zoom, p.height / self.zoom);
        for i in 0..self.insts.len() {
            for b in &self.types[&self.insts[i].obj].behaviors {
                let fixed = self.layer_fixed(&self.insts[i].layer);
                let me = &mut self.insts[i];
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
            }
        }
    }
}

impl Inst {
    pub fn cx(&self) -> f32 {
        self.x + self.w / 2.
    }
    pub fn cy(&self) -> f32 {
        self.y + self.h / 2.
    }
    /// Rectángulo rotado alrededor de su centro.
    pub fn hitbox(&self) -> [(f32, f32); 4] {
        let (c, s) = (self.angle.to_radians().cos(), self.angle.to_radians().sin());
        let (hw, hh) = (self.w / 2., self.h / 2.);
        [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)].map(|(x, y)| (self.cx() + x * c - y * s, self.cy() + x * s + y * c))
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

fn point_in(poly: &[(f32, f32)], x: f32, y: f32) -> bool {
    (0..poly.len()).all(|k| {
        let (p, q) = (poly[k], poly[(k + 1) % poly.len()]);
        (q.0 - p.0) * (y - p.1) - (q.1 - p.1) * (x - p.0) >= 0.
    })
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
        (Value::Str(a), _) => Value::Str(a.clone() + &v.text()),
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
}
