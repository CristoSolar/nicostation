//! Editor visual de NicoStation: escenas, objetos, capas, variables y hoja de eventos (egui).
//! Uso: `nicostation-editor [proyecto.json]` · `nicostation-editor --resave entrada.json salida.json`.
//!
//! Tiene dos modos: «fácil» (por defecto, pensado para niños: bloques de colores, recetas, cajitas)
//! y «experto» (expresiones y ventana de instrucciones completas).
//!
//! Opciones ocultas para probar la interfaz:
//! `--experto` arranca en modo experto · `--grande` con letra grande ·
//! `--escena <nombre>` elige la escena · `--abrir <qué>` abre una vista al iniciar:
//! `eventos`, `cajitas`, `bloques`, `asistente`, `recetas`, `receta` (Disparar, con sus preguntas),
//! `selector` (¿CUÁNDO pasa?), `selector-acciones`, `tecla` (bloque con teclado), `objeto` (bloque con objetos),
//! `ficha` (abre la ventanita de la primera ficha de objeto), `arreglar` (muestra el primer problema).

#[path = "../editor/bloques.rs"]
mod bloques;
#[path = "../editor/cajitas.rs"]
mod cajitas;
#[path = "../editor/eventos.rs"]
mod eventos;
#[path = "../editor/facil.rs"]
mod facil;
#[path = "../editor/recetario.rs"]
mod recetario;
#[path = "../editor/historial.rs"]
mod historial;
#[path = "../editor/iconos.rs"]
mod iconos;
#[path = "../editor/tema.rs"]
mod tema;

use eframe::egui::{
    self, Align, Align2, AtomExt, Button, Color32, DragValue, Frame, Key, Layout, Margin, Modifiers, PointerButton, Pos2, Rect,
    RichText, Sense, Stroke, TextEdit, Ui, Vec2,
};
use eventos::{InstrEdit, Res, Salida, Target};
use historial::Historial;
use iconos as ic;
use nicostation::catalog::P;
use nicostation::model::*;
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use tema::*;

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Escena,
    Eventos,
    Funciones,
    Variables,
}

#[derive(PartialEq, Clone, Copy)]
enum Nivel {
    Info,
    Ok,
    Warn,
    Error,
}

const BEHAVIORS: &[&str] = &[
    "platformer", "platform", "top_down", "bullet", "rotate", "destroy_outside", "bound_to_layout", "screen_wrap", "draggable",
];
const UNDO_HINT: &str = "Ctrl+Z para deshacer";
/// Ancho de la columna de etiquetas en las rejillas de propiedades.
const LBL_W: f32 = 96.;

/// Diálogo de exportación web.
struct Export {
    dir: String,
    res: Option<Result<usize, String>>,
}

struct App {
    p: Project,
    path: Option<PathBuf>,
    saved: String,
    /// JSON del proyecto en el último cuadro (para título, sucio y deshacer).
    now: String,
    hist: Historial,
    title: String,
    msg: (Nivel, String),
    tab: Tab,
    sc: usize,
    sel_obj: Option<String>,
    sel_inst: Option<usize>,
    hover_inst: Option<usize>,
    cursor_w: Option<Pos2>,
    placing: bool,
    cur_layer: String,
    pan: Vec2,
    zoom: f32,
    /// Ajusta la vista al área de juego hasta que el usuario mueva o haga zoom.
    fit: bool,
    snap: bool,
    grid: f32,
    drag_off: Option<Vec2>,
    tex: HashMap<String, Option<egui::TextureHandle>>,
    ed: Option<InstrEdit>,
    exp: Option<Export>,
    /// Escena con eventos pendiente de confirmar su borrado.
    confirm: Option<usize>,
    func: usize,
    fparams: Option<(usize, String)>,
    /// Campos de "nuevo nombre": globales, escena, instancia, objeto, grupo.
    nb: [String; 5],
    tabs_w: f32,
    kids: Vec<Child>,
    // ---- modo fácil ----
    facil: bool,
    grande: bool,
    /// Tamaños aplicados (facil, grande) para no repetir.
    aplicado: Option<(bool, bool)>,
    sel: Option<facil::Selector>,
    rec: Option<recetario::Recetario>,
    nueva_caj: Option<cajitas::Nueva>,
    asis: Option<bloques::Asistente>,
    /// Campo de ingrediente nuevo en Mis bloques.
    fnuevo: String,
    /// Resultado de `validate` y el JSON con que se calculó.
    probs: Vec<String>,
    probs_de: String,
    /// Evento a mostrar (ruta, hasta cuándo).
    jump: Option<(Vec<usize>, f64)>,
    /// Consejo de la última receta agregada.
    nota: Option<String>,
    files: Vec<String>,
    files_at: Option<std::time::Instant>,
    /// Ancho izquierdo y derecho ocupados en la barra superior (para no tapar las pestañas).
    bar_lr: (f32, f32),
}

fn new_scene(name: &str) -> Scene {
    Scene { name: name.into(), background: [30, 30, 40], layers: vec![Layer { name: String::new(), visible: true, fixed: false }], ..Default::default() }
}

fn new_project() -> Project {
    Project {
        name: "Mi juego".into(), width: 800., height: 600., first_scene: "Escena".into(), globals: Vars::new(),
        objects: vec![], scenes: vec![new_scene("Escena")], functions: vec![],
    }
}

fn load(path: &Path) -> Result<Project, String> {
    let s = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&s).map_err(|e| e.to_string())
}

/// Ruta de `p` relativa a la carpeta `base` (con `..` si hace falta).
pub fn rel(base: &Path, p: &Path) -> String {
    let (b, t): (Vec<_>, Vec<_>) = (base.components().collect(), p.components().collect());
    let n = b.iter().zip(&t).take_while(|(x, y)| x == y).count();
    if n == 0 {
        return p.to_string_lossy().into();
    }
    let mut r = PathBuf::new();
    for _ in n..b.len() {
        r.push("..");
    }
    t[n..].iter().for_each(|c| r.push(c));
    r.to_string_lossy().replace('\\', "/")
}

/// Combo de texto con opciones fijas; `empty` es la etiqueta del valor vacío.
pub fn combo<S: AsRef<str>>(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, v: &mut String, opts: &[S], empty: &str) -> bool {
    let show = |s: &str| if s.is_empty() { empty.to_string() } else { s.to_string() };
    let mut ch = false;
    egui::ComboBox::from_id_salt(id).selected_text(show(v)).show_ui(ui, |ui| {
        for o in opts {
            let o = o.as_ref();
            ch |= ui.selectable_label(v == o, show(o)).clicked() && { *v = o.into(); true };
        }
    });
    ch
}

/// Campo de nombre: devuelve el nombre nuevo si es válido y no está ocupado.
fn name_edit(ui: &mut Ui, name: &str, w: f32, taken: &dyn Fn(&str) -> bool) -> Option<String> {
    let mut s = name.to_string();
    (ui.add(TextEdit::singleline(&mut s).desired_width(w)).changed() && !s.trim().is_empty() && !taken(&s)).then_some(s)
}

fn parse_val(s: &str) -> Value {
    match s {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        _ => match s.trim().parse::<f64>() {
            Ok(n) if Value::Num(n).text() == s => Value::Num(n),
            _ => Value::Str(s.into()),
        },
    }
}

fn grid(id: &str) -> egui::Grid {
    egui::Grid::new(id).num_columns(2).spacing([12., 6.]).min_col_width(LBL_W)
}

fn lbl(ui: &mut Ui, t: &str) {
    ui.label(muted(t));
}

/// Título de sección del panel derecho: ícono, título y detalle atenuado.
fn header(ui: &mut Ui, icon: &str, t: &str, detail: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(icon).size(16.).color(ACCENT));
        ui.label(RichText::new(t).font(sb(16.)));
        ui.label(muted(detail));
    });
    ui.add_space(2.);
}

/// Grupo de botones tipo control segmentado.
fn seg<R>(ui: &mut Ui, f: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::new().fill(BG).stroke((1., BORDER)).corner_radius(R).inner_margin(2).show(ui, |ui| {
        ui.spacing_mut().item_spacing.x = 2.;
        ui.spacing_mut().button_padding = Vec2::new(8., 3.);
        f(ui)
    }).inner
}

fn seg_btn<'a>(ui: &mut Ui, sel: bool, atoms: impl egui::IntoAtoms<'a>) -> egui::Response {
    ui.add(Button::selectable(sel, atoms).frame_when_inactive(sel))
}

fn card<R>(ui: &mut Ui, f: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::new().fill(SURFACE).stroke((1., BORDER)).corner_radius(R).inner_margin(8).show(ui, |ui| {
        ui.set_width(ui.available_width());
        f(ui)
    }).inner
}

fn vars_ui(ui: &mut Ui, id: &str, vars: &mut Vars, nb: &mut String) {
    let mut del = None;
    if vars.is_empty() {
        ui.label(muted("Sin variables todavía."));
    }
    egui::Grid::new(id).num_columns(3).spacing([8., 6.]).min_col_width(LBL_W).show(ui, |ui| {
        for (k, v) in vars.iter_mut() {
            ui.label(k);
            let mut s = v.text();
            if ui.add(TextEdit::singleline(&mut s).desired_width(140.)).changed() {
                *v = parse_val(&s);
            }
            if ib(ui, ic::TRASH, "Eliminar variable").clicked() {
                del = Some(k.clone());
            }
            ui.end_row();
        }
    });
    if let Some(k) = del {
        vars.remove(&k);
    }
    ui.horizontal(|ui| {
        let r = ui.add(TextEdit::singleline(nb).hint_text("nueva variable").desired_width(140.));
        let enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        if (ui.button(it(ic::PLUS, "Agregar")).clicked() || enter) && !nb.trim().is_empty() && !vars.contains_key(nb.as_str()) {
            vars.insert(std::mem::take(nb), Value::Num(0.));
        }
    });
}

// ---- renombrado de referencias ----

fn ren_expr(s: &str, old: &str, new: &str) -> String {
    let pat = format!("{old}.");
    let (mut out, mut i) = (String::with_capacity(s.len()), 0);
    while i < s.len() {
        if s[i..].starts_with(&pat) && !s[..i].chars().last().is_some_and(|c| c.is_alphanumeric() || c == '_') {
            out.push_str(new);
            out.push('.');
            i += pat.len();
        } else {
            let c = s[i..].chars().next().unwrap();
            out.push(c);
            i += c.len_utf8();
        }
    }
    out
}

fn ren_ins(l: &mut [Instr], t: P, old: &str, new: &str) {
    for i in l {
        let ps = nicostation::catalog::meta(&i.op).map_or(&[][..], |m| m.params);
        for (k, a) in i.args.iter_mut().enumerate() {
            match ps.get(k).map(|p| p.1) {
                Some(x) if x == t => {
                    if a == old {
                        *a = new.into();
                    }
                }
                Some(P::Expr) | None if t == P::Object => *a = ren_expr(a, old, new),
                _ => {}
            }
        }
        ren_ins(&mut i.sub, t, old, new);
    }
}

fn ren_evs(evs: &mut [Event], t: P, old: &str, new: &str) {
    for e in evs {
        if t == P::Object && e.kind == EventKind::ForEach && e.text == old {
            e.text = new.into();
        }
        ren_ins(&mut e.conds, t, old, new);
        ren_ins(&mut e.acts, t, old, new);
        ren_evs(&mut e.sub, t, old, new);
    }
}

/// Renombra en eventos de una escena (o de todas, y funciones, si `scene` es None).
fn ren_all(p: &mut Project, scene: Option<usize>, t: P, old: &str, new: &str) {
    for (i, s) in p.scenes.iter_mut().enumerate() {
        if scene.is_none_or(|k| k == i) {
            ren_evs(&mut s.events, t, old, new);
            if t == P::Object {
                s.instances.iter_mut().filter(|n| n.obj == old).for_each(|n| n.obj = new.into());
                s.groups.values_mut().flatten().filter(|o| *o == old).for_each(|o| *o = new.into());
            }
        }
    }
    if scene.is_none() {
        p.functions.iter_mut().for_each(|f| ren_evs(&mut f.events, t, old, new));
    }
}

fn count_events(evs: &[Event]) -> usize {
    evs.iter().map(|e| 1 + count_events(&e.sub)).sum()
}

fn beh_label(t: &str) -> &str {
    match t {
        "platformer" => "Personaje de plataformas",
        "platform" => "Plataforma",
        "top_down" => "Movimiento vista superior",
        "bullet" => "Proyectil",
        "rotate" => "Rotación",
        "destroy_outside" => "Destruir fuera de pantalla",
        "bound_to_layout" => "Limitar a la escena",
        "screen_wrap" => "Envolver pantalla",
        "draggable" => "Arrastrable",
        _ => t,
    }
}

fn field_label(k: &str) -> &str {
    match k {
        "max_speed" => "Velocidad máx.",
        "acceleration" => "Aceleración",
        "deceleration" => "Desaceleración",
        "gravity" => "Gravedad",
        "max_falling_speed" => "Caída máx.",
        "jump_speed" => "Salto",
        "jump_sustain_time" => "Sostén salto (s)",
        "jumpthru" => "Atravesable desde abajo",
        "allow_diagonals" => "Diagonales",
        "rotate" => "Girar al moverse",
        "speed" => "Velocidad",
        _ => k,
    }
}

/// Devuelve true si hay que quitar el comportamiento.
fn behavior_ui(ui: &mut Ui, b: &mut Behavior, i: usize) -> bool {
    let Ok(serde_json::Value::Object(mut m)) = serde_json::to_value(&*b) else { return false };
    let (mut ch, mut del) = (false, false);
    let t = m.get("type").and_then(|t| t.as_str()).unwrap_or("?").to_string();
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(beh_label(&t)).font(sb(14.)));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| del = ib(ui, ic::TRASH, "Quitar comportamiento").clicked());
        });
        let fields: Vec<_> = m.iter_mut().filter(|(k, _)| *k != "type").collect();
        if fields.is_empty() {
            return;
        }
        egui::Grid::new(("beh", i)).num_columns(2).spacing([12., 6.]).min_col_width(LBL_W).show(ui, |ui| {
            for (k, v) in fields {
                match v {
                    serde_json::Value::Bool(x) => {
                        lbl(ui, field_label(k));
                        ch |= ui.checkbox(x, "").changed();
                    }
                    serde_json::Value::Number(n) => {
                        lbl(ui, field_label(k));
                        let mut f = n.as_f64().unwrap_or(0.);
                        if ui.add(DragValue::new(&mut f).speed(1.)).changed() {
                            *v = json!(f);
                            ch = true;
                        }
                    }
                    _ => continue,
                }
                ui.end_row();
            }
        });
    });
    if ch {
        if let Ok(nb) = serde_json::from_value(serde_json::Value::Object(m)) {
            *b = nb;
        }
    }
    del
}

fn new_behavior(t: &str) -> Option<Behavior> {
    serde_json::from_value(json!({ "type": t })).or_else(|_| serde_json::from_value(json!({ "type": t, "speed": 100.0 }))).ok()
}

/// Propiedades según el tipo de objeto, animaciones, comportamientos y variables.
fn obj_props(ui: &mut Ui, o: &mut ObjectDef, base: &Path, nb: &mut String, kid: Option<&HashMap<String, usize>>) {
    grid("objprops").show(ui, |ui| match &mut o.kind {
        Kind::Box { w, h, color } => {
            lbl(ui, "Ancho");
            ui.add(DragValue::new(w).range(1..=4096));
            ui.end_row();
            lbl(ui, "Alto");
            ui.add(DragValue::new(h).range(1..=4096));
            ui.end_row();
            lbl(ui, "Color");
            ui.color_edit_button_srgb(color);
            ui.end_row();
        }
        Kind::Text { text, size, color } => {
            lbl(ui, "Texto");
            ui.add(TextEdit::singleline(text).desired_width(150.));
            ui.end_row();
            lbl(ui, "Tamaño");
            ui.add(DragValue::new(size).range(4..=400));
            ui.end_row();
            lbl(ui, "Color");
            ui.color_edit_button_srgb(color);
            ui.end_row();
        }
        Kind::Sprite { w, h, .. } => {
            lbl(ui, "Ancho");
            ui.add(DragValue::new(w).range(0..=4096)).on_hover_text("0 = tamaño de la imagen");
            ui.end_row();
            lbl(ui, "Alto");
            ui.add(DragValue::new(h).range(0..=4096)).on_hover_text("0 = tamaño de la imagen");
            ui.end_row();
        }
    });
    ui.add_space(4.);
    if let Kind::Sprite { animations, .. } = &mut o.kind {
        egui::CollapsingHeader::new(section("Animaciones", Some(animations.len()))).id_salt("anims").default_open(true).show(ui, |ui| {
            let mut del = None;
            for (i, a) in animations.iter_mut().enumerate() {
                ui.push_id(i, |ui| {
                    card(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.add(TextEdit::singleline(&mut a.name).desired_width(90.));
                            ui.add(DragValue::new(&mut a.fps).range(0.1..=120.).suffix(" fps"));
                            ui.checkbox(&mut a.looping, "Bucle");
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ib(ui, ic::TRASH, "Eliminar animación").clicked() {
                                    del = Some(i);
                                }
                            });
                        });
                        let mut fdel = None;
                        for (j, f) in a.frames.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(muted(ic::IMAGE));
                                ui.add(egui::Label::new(f.as_str()).truncate());
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if ib(ui, ic::X, "Quitar imagen").clicked() {
                                        fdel = Some(j);
                                    }
                                });
                            });
                        }
                        if let Some(j) = fdel {
                            a.frames.remove(j);
                        }
                        if ui.button(it(ic::PLUS, "Añadir imágenes…")).clicked() {
                            if let Some(fs) = rfd::FileDialog::new().add_filter("Imagen", &["png", "jpg", "jpeg"]).set_directory(base).pick_files() {
                                a.frames.extend(fs.iter().map(|f| rel(base, f)));
                            }
                        }
                    });
                });
            }
            if let Some(i) = del {
                animations.remove(i);
            }
            if ui.button(it(ic::PLUS, "Animación")).clicked() {
                animations.push(Animation { name: format!("anim{}", animations.len()), frames: vec![], fps: 10., looping: true });
            }
        });
    }
    egui::CollapsingHeader::new(section("Comportamientos", Some(o.behaviors.len()))).id_salt("behs").default_open(true).show(ui, |ui| {
        let mut del = None;
        for (i, b) in o.behaviors.iter_mut().enumerate() {
            if behavior_ui(ui, b, i) {
                del = Some(i);
            }
        }
        if let Some(i) = del {
            o.behaviors.remove(i);
        }
        egui::ComboBox::from_id_salt("addbeh").selected_text(format!("{}  Añadir comportamiento", ic::PLUS)).width(220.).show_ui(ui, |ui| {
            for t in BEHAVIORS {
                if ui.selectable_label(false, beh_label(t)).clicked() {
                    o.behaviors.extend(new_behavior(t));
                }
            }
        });
    });
    match kid {
        Some(usos) => {
            let name = o.name.clone();
            egui::CollapsingHeader::new(section("Cajitas del objeto", Some(o.vars.len()))).id_salt("ovars").show(ui, |ui| {
                cajitas::del_objeto(ui, &name, &mut o.vars, &|v| usos.get(v).copied().unwrap_or(0), nb)
            });
        }
        None => drop(egui::CollapsingHeader::new(section("Variables del objeto", Some(o.vars.len()))).id_salt("ovars").show(ui, |ui| vars_ui(ui, "objvars", &mut o.vars, nb))),
    }
}

fn kind_icon(k: &Kind) -> &'static str {
    match k {
        Kind::Sprite { .. } => ic::IMAGE,
        Kind::Text { .. } => ic::TEXT_T,
        Kind::Box { .. } => ic::CUBE,
    }
}

fn kind_name(k: &Kind) -> &'static str {
    match k {
        Kind::Sprite { .. } => "Sprite",
        Kind::Text { .. } => "Texto",
        Kind::Box { .. } => "Caja",
    }
}

fn shell_quote(s: &str) -> String {
    if s.chars().all(|c| c.is_alphanumeric() || "/._-~".contains(c)) { s.into() } else { format!("'{}'", s.replace('\'', "'\\''")) }
}

impl App {
    fn new() -> Self {
        let p = new_project();
        let now = serde_json::to_string_pretty(&p).unwrap_or_default();
        App {
            p, path: None, saved: now.clone(), hist: Historial::new(now.clone()), now, title: String::new(), msg: (Nivel::Info, "Listo.".into()),
            tab: Tab::Escena, sc: 0, sel_obj: None, sel_inst: None, hover_inst: None, cursor_w: None, placing: false, cur_layer: String::new(),
            pan: Vec2::ZERO, zoom: 1., fit: true, snap: true, grid: 32., drag_off: None, tex: HashMap::new(), ed: None, exp: None, confirm: None,
            func: 0, fparams: None, nb: Default::default(), tabs_w: 440., kids: vec![],
            facil: true, grande: false, aplicado: None, sel: None, rec: None, nueva_caj: None, asis: None, fnuevo: String::new(),
            probs: vec![], probs_de: String::new(), jump: None, nota: None, files: vec![], files_at: None, bar_lr: (0., 0.),
        }
    }

    fn json(&self) -> String {
        serde_json::to_string_pretty(&self.p).unwrap_or_default()
    }

    fn set_project(&mut self, p: Project, path: Option<PathBuf>) {
        let kids = std::mem::take(&mut self.kids);
        let (facil, grande) = (self.facil, self.grande);
        *self = App { p, path, kids, facil, grande, ..App::new() };
        self.now = self.json();
        self.saved = self.now.clone();
        self.hist = Historial::new(self.now.clone());
    }

    fn dirty(&self) -> bool {
        self.now != self.saved
    }

    fn say(&mut self, n: Nivel, m: impl Into<String>) {
        self.msg = (n, m.into());
    }

    /// Confirma el estado actual en el historial antes de una acción estructural.
    fn snap_hist(&mut self) {
        self.now = self.json();
        self.hist.commit(&self.now);
    }

    fn restore(&mut self, s: String, what: &str) {
        if let Ok(p) = serde_json::from_str(&s) {
            self.p = p;
        }
        if self.p.scenes.is_empty() {
            self.p.scenes.push(new_scene("Escena"));
        }
        self.sc = self.sc.min(self.p.scenes.len() - 1);
        self.sel_inst = self.sel_inst.filter(|i| *i < self.p.scenes[self.sc].instances.len());
        self.func = self.func.min(self.p.functions.len().saturating_sub(1));
        (self.ed, self.fparams, self.drag_off) = (None, None, None);
        self.sel = None;
        self.now = s;
        self.say(Nivel::Info, what);
    }

    fn undo(&mut self) {
        let now = self.json();
        match self.hist.undo(&now) {
            Some(s) => self.restore(s, "Deshecho"),
            None => self.say(Nivel::Info, "No hay nada que deshacer"),
        }
    }

    fn redo(&mut self) {
        let now = self.json();
        match self.hist.redo(&now) {
            Some(s) => self.restore(s, "Rehecho"),
            None => self.say(Nivel::Info, "No hay nada que rehacer"),
        }
    }

    fn base_dir(&self) -> PathBuf {
        let d = self.path.as_ref().and_then(|p| p.parent()).filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        d.canonicalize().unwrap_or_else(|_| d.to_path_buf())
    }

    fn discard_ok(&self) -> bool {
        !self.dirty()
            || rfd::MessageDialog::new().set_title("NicoStation").set_description("Hay cambios sin guardar. ¿Descartarlos?")
                .set_buttons(rfd::MessageButtons::YesNo).show() == rfd::MessageDialogResult::Yes
    }

    fn save(&mut self, ask: bool) -> bool {
        if ask || self.path.is_none() {
            let Some(p) = rfd::FileDialog::new().add_filter("Proyecto NicoStation", &["json"]).set_file_name("proyecto.json").save_file() else {
                return false;
            };
            self.path = Some(p);
            self.tex.clear();
        }
        let path = self.path.clone().unwrap();
        let s = self.json();
        match std::fs::write(&path, &s) {
            Ok(()) => {
                self.saved = s.clone();
                self.now = s;
                self.say(Nivel::Ok, format!("Guardado en {}", path.display()));
                true
            }
            Err(e) => {
                self.say(Nivel::Error, format!("Error al guardar: {e}"));
                false
            }
        }
    }

    fn open(&mut self) {
        if !self.discard_ok() {
            return;
        }
        if let Some(f) = rfd::FileDialog::new().add_filter("Proyecto NicoStation", &["json"]).pick_file() {
            match load(&f) {
                Ok(p) => self.set_project(p, Some(f)),
                Err(e) => self.say(Nivel::Error, format!("No se pudo abrir: {e}")),
            }
        }
    }

    fn run(&mut self) {
        if !self.save(false) {
            return;
        }
        let exe = std::env::current_exe().map(|e| e.with_file_name("nicostation"));
        let (n, m) = match exe {
            Ok(e) if e.exists() => match Command::new(&e).arg(self.path.as_ref().unwrap()).spawn() {
                Ok(c) => {
                    self.kids.push(c);
                    (Nivel::Ok, "Ejecutando el juego…".into())
                }
                Err(err) => (Nivel::Error, format!("No se pudo ejecutar {}: {err}", e.display())),
            },
            _ => (Nivel::Error, "No se encontró el binario «nicostation» junto al editor (compila con `cargo build --bin nicostation`).".into()),
        };
        self.say(n, m);
    }

    fn open_export(&mut self) {
        if (self.path.is_none() || self.dirty()) && !self.save(false) {
            self.say(Nivel::Warn, "Guarda el proyecto antes de exportar.");
            return;
        }
        self.exp = Some(Export { dir: self.base_dir().join("web").display().to_string(), res: None });
    }

    fn do_export(&mut self) {
        if self.dirty() && !self.save(false) {
            return;
        }
        let (Some(path), Some(ex)) = (self.path.clone(), self.exp.as_mut()) else { return };
        let res = nicostation::export::export_web(&path, Path::new(ex.dir.trim())).map(|f| f.len());
        let dir = ex.dir.trim().to_string();
        ex.res = Some(res.clone());
        match res {
            Ok(n) => self.say(Nivel::Ok, format!("Exportado a {dir} ({n} archivos). Ábrelo con un servidor web local.")),
            Err(e) => self.say(Nivel::Error, format!("No se pudo exportar: {}", e.lines().next().unwrap_or("").trim_end_matches(':'))),
        }
    }

    fn scene(&mut self) -> &mut Scene {
        &mut self.p.scenes[self.sc]
    }

    /// Objetos y grupos visibles desde la escena actual.
    fn obj_names(&self) -> Vec<String> {
        let s = &self.p.scenes[self.sc];
        self.p.objects_of(s).map(|o| o.name.clone()).chain(s.groups.keys().cloned()).collect()
    }

    fn name_taken(&self, n: &str) -> bool {
        self.p.objects.iter().chain(self.p.scenes.iter().flat_map(|s| &s.objects)).any(|o| o.name == n)
            || self.p.scenes[self.sc].groups.contains_key(n)
    }

    fn obj_mut(&mut self, name: &str) -> Option<&mut ObjectDef> {
        let sc = self.sc;
        self.p.scenes[sc].objects.iter_mut().chain(self.p.objects.iter_mut()).find(|o| o.name == name)
    }

    fn texture(&mut self, ctx: &egui::Context, path: &str) -> Option<egui::TextureHandle> {
        if !self.tex.contains_key(path) {
            let t = image::open(self.base_dir().join(path)).ok().map(|img| {
                let img = img.to_rgba8();
                let c = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
                ctx.load_texture(path, c, egui::TextureOptions::NEAREST)
            });
            self.tex.insert(path.into(), t);
        }
        self.tex[path].clone()
    }

    fn obj_size(&self, o: &ObjectDef) -> Vec2 {
        match &o.kind {
            Kind::Box { w, h, .. } => Vec2::new(*w, *h),
            Kind::Text { text, size, .. } => Vec2::new(text.chars().count().max(1) as f32 * size * 0.55, *size),
            Kind::Sprite { w, h, animations } if *w <= 0. || *h <= 0. => {
                let t = animations.first().and_then(|a| a.frames.first()).and_then(|f| self.tex.get(f).cloned().flatten());
                t.map_or(Vec2::splat(32.), |t| t.size_vec2())
            }
            Kind::Sprite { w, h, .. } => Vec2::new(*w, *h),
        }
    }

    fn events_mut(&mut self, t: Target) -> Option<&mut Vec<Event>> {
        match t {
            Target::Scene(i) => self.p.scenes.get_mut(i).map(|s| &mut s.events),
            Target::Func(i) => self.p.functions.get_mut(i).map(|f| &mut f.events),
        }
    }

    fn lists(&self) -> eventos::Lists {
        let s = &self.p.scenes[self.sc];
        eventos::Lists {
            objs: self.obj_names(),
            layers: s.layers.iter().map(|l| l.name.clone()).collect(),
            scenes: self.p.scenes.iter().map(|s| s.name.clone()).collect(),
            funcs: self.p.functions.iter().map(|f| (f.name.clone(), f.is_condition, f.params.clone())).collect(),
            anims: self.p.objects_of(s)
                .filter_map(|o| match &o.kind {
                    Kind::Sprite { animations, .. } => Some((o.name.clone(), animations.iter().map(|a| a.name.clone()).collect())),
                    _ => None,
                })
                .collect(),
            base: self.base_dir(),
            groups: s.groups.keys().cloned().collect(),
            thumbs: HashMap::new(),
            anim_tex: HashMap::new(),
            vars: s.vars.keys().cloned().collect(),
            globals: self.p.globals.keys().cloned().collect(),
            obj_vars: self.p.objects_of(s).map(|o| (o.name.clone(), o.vars.keys().cloned().collect())).collect(),
            files: self.files.clone(),
            locals: self.func_target().map(|i| self.p.functions[i].params.clone()).unwrap_or_default(),
        }
    }

    /// Listas con miniaturas y archivos (modo fácil).
    fn kid_lists(&mut self, ctx: &egui::Context) -> eventos::Lists {
        if self.files_at.is_none_or(|t| t.elapsed().as_secs() >= 5) {
            self.files = buscar_archivos(&self.base_dir());
            self.files_at = Some(std::time::Instant::now());
        }
        let mut l = self.lists();
        let s = &self.p.scenes[self.sc];
        let objs: Vec<ObjectDef> = self.p.objects_of(s).cloned().collect();
        if self.func_target().is_some() {
            // En Mis bloques se pueden usar los objetos de cualquier escena.
            for o in self.p.scenes.iter().flat_map(|s| &s.objects) {
                if !l.objs.contains(&o.name) {
                    l.objs.push(o.name.clone());
                }
            }
        }
        for o in &objs {
            let t = match &o.kind {
                Kind::Box { color, .. } => eventos::Thumb::Color(*color),
                Kind::Text { color, .. } => eventos::Thumb::Text(*color),
                Kind::Sprite { animations, .. } => {
                    for a in animations {
                        if let Some(t) = a.frames.first().and_then(|f| self.texture(ctx, f)) {
                            l.anim_tex.insert((o.name.clone(), a.name.clone()), t);
                        }
                    }
                    animations.first().and_then(|a| a.frames.first()).and_then(|f| self.texture(ctx, f)).map_or(eventos::Thumb::Empty, eventos::Thumb::Tex)
                }
            };
            l.thumbs.insert(o.name.clone(), t);
        }
        l
    }

    /// Mis bloques abierto en la pestaña actual.
    fn func_target(&self) -> Option<usize> {
        (self.tab == Tab::Funciones && self.func < self.p.functions.len()).then_some(self.func)
    }

    /// Hoja que se está mirando (para recetas).
    fn cur_target(&self) -> Target {
        self.func_target().map_or(Target::Scene(self.sc), Target::Func)
    }

    fn set_facil(&mut self, on: bool) {
        self.facil = on;
        self.ed = None;
        self.sel = None;
        self.say(Nivel::Info, if on { "Modo fácil: bloques de colores y recetas" } else { "Modo experto: expresiones y todas las opciones" });
    }

    fn add_instance(&mut self, obj: String, at: Pos2) {
        let at = if self.snap { (at.to_vec2() / self.grid).floor() * self.grid } else { at.to_vec2() };
        let layer = self.cur_layer.clone();
        let s = self.scene();
        s.instances.push(Instance { obj, x: at.x, y: at.y, layer, angle: 0., z: 0., w: None, h: None, vars: Vars::new() });
        self.sel_inst = Some(s.instances.len() - 1);
    }

    fn sel_valid(&self) -> Option<usize> {
        self.sel_inst.filter(|i| *i < self.p.scenes[self.sc].instances.len())
    }

    fn delete_instance(&mut self) {
        if let Some(i) = self.sel_valid() {
            self.snap_hist();
            let o = self.scene().instances.remove(i).obj;
            self.sel_inst = None;
            self.say(Nivel::Info, format!("Eliminada instancia de «{o}» · {UNDO_HINT}"));
        }
    }

    fn duplicate_instance(&mut self) {
        if let Some(i) = self.sel_valid() {
            self.snap_hist();
            let g = self.grid;
            let s = self.scene();
            let mut n = s.instances[i].clone();
            (n.x, n.y) = (n.x + g, n.y + g);
            s.instances.push(n);
            self.sel_inst = Some(s.instances.len() - 1);
            self.say(Nivel::Info, "Instancia duplicada");
        }
    }

    fn nudge(&mut self, d: Vec2) {
        if let Some(i) = self.sel_valid() {
            let ins = &mut self.scene().instances[i];
            (ins.x, ins.y) = (ins.x + d.x, ins.y + d.y);
        }
    }

    fn delete_scene(&mut self, i: usize) {
        if self.p.scenes.len() < 2 || i >= self.p.scenes.len() {
            return;
        }
        self.snap_hist();
        let old = self.p.scenes.remove(i).name;
        if self.p.first_scene == old {
            self.p.first_scene = self.p.scenes[0].name.clone();
        }
        self.sc = i.min(self.p.scenes.len() - 1);
        (self.sel_inst, self.sel_obj) = (None, None);
        self.cur_layer.clear();
        self.say(Nivel::Info, format!("Eliminada la escena «{old}» · {UNDO_HINT}"));
    }

    // ---- barra superior ----

    fn file_menu(&mut self, ui: &mut Ui) {
        menu_spacing(ui);
        let f = self.facil;
        if ui.radio(f, "Modo fácil").clicked() && !f {
            self.set_facil(true);
        }
        if ui.radio(!f, "Modo experto").clicked() && f {
            self.set_facil(false);
        }
        let mut g = self.grande;
        if ui.checkbox(&mut g, "Letra grande").changed() {
            self.grande = g;
        }
        ui.separator();
        if ui.button(it(ic::FILE_PLUS, "Nuevo")).clicked() && self.discard_ok() {
            self.set_project(new_project(), None);
        }
        if ui.button(it(ic::FOLDER_OPEN, "Abrir…")).clicked() {
            self.open();
        }
        ui.separator();
        if ui.add(Button::new(it(ic::FLOPPY, "Guardar")).shortcut_text("Ctrl+S")).clicked() {
            self.save(false);
        }
        if ui.add(Button::new(it(ic::FLOPPY_BACK, "Guardar como…")).shortcut_text("Ctrl+Shift+S")).clicked() {
            self.save(true);
        }
        ui.separator();
        if ui.button(it(ic::GLOBE, "Exportar web…")).clicked() {
            self.open_export();
        }
        ui.separator();
        if ui.button(it(ic::SIGN_OUT, "Salir")).clicked() && self.discard_ok() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn top_bar(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        // Si no entra todo: 1 = sin nombre del programa y botones más cortos, 2 = además pestañas sin íconos.
        let nivel = if full.width() >= 1240. { 0 } else if full.width() >= 900. { 1 } else { 2 };
        let compact = nivel > 0;
        let mut lr = self.bar_lr;
        ui.horizontal_centered(|ui| {
            let b = Button::new(RichText::new(ic::LIST).size(18.)).frame_when_inactive(false);
            let (r, _) = egui::containers::menu::MenuButton::from_button(b).ui(ui, |ui| self.file_menu(ui));
            r.on_hover_text("Archivo");
            ui.add_space(4.);
            ui.label(RichText::new(ic::GAMEPAD).size(18.).color(ACCENT));
            if !compact {
                ui.label(RichText::new("NicoStation").font(sb(15.)));
                let file = self.path.as_ref().and_then(|p| p.file_name()).map_or("sin título".into(), |f| f.to_string_lossy().to_string());
                ui.add(egui::Label::new(muted(format!("{}{}", self.p.name, if self.dirty() { " *" } else { "" }))).truncate()).on_hover_text(file);
                lr.0 = ui.min_rect().width();
            }
            let right = ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let hint = RichText::new("F5").small().color(ON_ACCENT.gamma_multiply(0.75));
                if primary(ui, (on_accent(ic::PLAY), on_accent("Probar").font(sb(14.)), hint)).on_hover_text("Guardar y ejecutar el juego").clicked() {
                    self.run();
                }
                if nivel == 2 {
                    // Exportar queda en el menú Archivo.
                } else if self.facil || compact {
                    if ib(ui, ic::GLOBE, "Exportar web: una versión para el navegador").clicked() {
                        self.open_export();
                    }
                } else if ui.button(it(ic::GLOBE, "Exportar web")).on_hover_text("Generar una versión para navegador").clicked() {
                    self.open_export();
                }
                if compact {
                    if ib(ui, ic::FLOPPY, "Guardar (Ctrl+S)").clicked() {
                        self.save(false);
                    }
                } else if ui.button(it(ic::FLOPPY, "Guardar")).on_hover_text("Ctrl+S").clicked() {
                    self.save(false);
                }
                ui.add_space(4.);
                if ib_on(ui, self.hist.can_redo(&self.now), ic::REDO, "Rehacer (Ctrl+Shift+Z)").clicked() {
                    self.redo();
                }
                if ib_on(ui, self.hist.can_undo(&self.now), ic::UNDO, "Deshacer (Ctrl+Z)").clicked() {
                    self.undo();
                }
                // Muy angosto: el modo y la letra quedan en el menú (≡).
                if nivel == 2 {
                    return ui.min_rect().width();
                }
                ui.add_space(6.);
                let g = self.grande;
                let r = ui.add(Button::selectable(g, RichText::new(ic::TEXT_AA).size(18.)).frame_when_inactive(g).min_size(Vec2::splat(30.)))
                    .on_hover_text(if g { "Letra grande: activada" } else { "Letra grande" });
                if r.clicked() {
                    self.grande = !g;
                }
                let f = self.facil;
                let mut set = None;
                if compact {
                    let t = if f { "Fácil" } else { "Experto" };
                    let b = Button::new((RichText::new(t).font(sb(14.)), RichText::new(ic::CARET_DOWN).color(TEXT_MUTED)));
                    let (r, _) = egui::containers::menu::MenuButton::from_button(b).ui(ui, |ui| {
                        menu_spacing(ui);
                        if ui.radio(f, "Modo fácil: bloques de colores y recetas").clicked() {
                            set = Some(true);
                        }
                        if ui.radio(!f, "Modo experto: todas las opciones").clicked() {
                            set = Some(false);
                        }
                    });
                    r.on_hover_text("Cambiar de modo");
                } else {
                    seg(ui, |ui| {
                        if seg_btn(ui, !f, "Experto").on_hover_text("Todas las opciones, con expresiones").clicked() {
                            set = Some(false);
                        }
                        if seg_btn(ui, f, "Fácil").on_hover_text("Bloques de colores, recetas y cajitas").clicked() {
                            set = Some(true);
                        }
                    });
                }
                if let Some(v) = set.filter(|v| *v != f) {
                    self.set_facil(v);
                }
                ui.min_rect().width()
            });
            lr.1 = right.inner;
        });
        if (lr.0 - self.bar_lr.0).abs() > 0.5 || (lr.1 - self.bar_lr.1).abs() > 0.5 {
            self.bar_lr = lr;
            ui.ctx().request_repaint();
        }
        // Pestañas centradas sin tapar los costados (el ancho se mide en el cuadro anterior).
        let left_end = full.left() + if compact { 70. } else { self.bar_lr.0 };
        let (lo, hi) = (left_end + self.tabs_w / 2. + 12., full.right() - self.bar_lr.1 - self.tabs_w / 2. - 12.);
        let cx = if lo <= hi { full.center().x.clamp(lo, hi) } else { lo };
        let rect = Rect::from_center_size(Pos2::new(cx, full.center().y), Vec2::new(self.tabs_w, 34.));
        let r = ui.scope_builder(egui::UiBuilder::new().max_rect(rect).layout(Layout::left_to_right(Align::Center)), |ui| self.tabs(ui, nivel < 2));
        let w = r.response.rect.width();
        if (w - self.tabs_w).abs() > 0.5 {
            self.tabs_w = w;
            ui.ctx().request_repaint();
        }
    }

    fn tabs(&mut self, ui: &mut Ui, icons: bool) {
        Frame::new().fill(BG).stroke((1., BORDER)).corner_radius(R + 2).inner_margin(3).show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.;
            let tabs = if self.facil {
                [(Tab::Escena, ic::SQUARES, "Escena"), (Tab::Eventos, ic::LIST, "Eventos"), (Tab::Funciones, ic::PUZZLE, "Mis bloques"), (Tab::Variables, ic::ARCHIVE_BOX, "Cajitas")]
            } else {
                [(Tab::Escena, ic::SQUARES, "Escena"), (Tab::Eventos, ic::LIST, "Eventos"), (Tab::Funciones, ic::FUNCTION, "Funciones"), (Tab::Variables, ic::BRACES, "Variables")]
            };
            for (k, (t, icon, name)) in tabs.into_iter().enumerate() {
                let sel = self.tab == t;
                let bg = ui.painter().add(egui::Shape::Noop);
                let atoms = (
                    RichText::new(if icons { icon } else { "" }).color(if sel { ACCENT } else { TEXT_MUTED }),
                    RichText::new(name).font(sb(14.)).color(if sel { TEXT } else { TEXT_MUTED }),
                );
                let r = ui.add(Button::new(atoms).fill(Color32::TRANSPARENT).stroke(Stroke::NONE).min_size(Vec2::new(0., 28.)))
                    .on_hover_text(format!("Ctrl+{}", k + 1));
                let fill = if sel { SURFACE_HOVER } else if r.hovered() { SURFACE } else { Color32::TRANSPARENT };
                ui.painter().set(bg, egui::epaint::RectShape::filled(r.rect, R, fill));
                if sel {
                    let u = Rect::from_min_max(Pos2::new(r.rect.left() + 10., r.rect.bottom() - 2.), Pos2::new(r.rect.right() - 10., r.rect.bottom()));
                    ui.painter().rect_filled(u, 1, ACCENT);
                }
                if r.has_focus() {
                    ui.painter().rect_stroke(r.rect, R, Stroke::new(1., ACCENT), egui::StrokeKind::Inside);
                }
                if r.clicked() {
                    self.tab = t;
                }
            }
        });
    }

    // ---- panel izquierdo ----

    fn left(&mut self, ui: &mut Ui) {
        let sc = self.sc;
        ui.label(RichText::new("Escena").font(sb(12.)).color(TEXT_MUTED));
        let (mut add, mut del) = (false, false);
        ui.horizontal(|ui| ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            del = ib_on(ui, self.p.scenes.len() > 1, ic::TRASH, "Eliminar escena").clicked();
            add = ib(ui, ic::PLUS, "Nueva escena").clicked();
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                let w = ui.available_width();
                egui::ComboBox::from_id_salt("escena").selected_text(self.p.scenes[sc].name.clone()).width(w).show_ui(ui, |ui| {
                    for (i, s) in self.p.scenes.iter().enumerate() {
                        ui.selectable_value(&mut self.sc, i, &s.name);
                    }
                });
            });
        }));
        if add {
            let n = (1..).map(|k| format!("Escena{k}")).find(|n| self.p.scene(n).is_none()).unwrap();
            self.p.scenes.push(new_scene(&n));
            self.sc = self.p.scenes.len() - 1;
        }
        if del {
            if self.p.scenes[sc].events.is_empty() {
                self.delete_scene(sc);
            } else {
                self.confirm = Some(sc);
            }
        }
        if self.sc != sc {
            (self.sel_inst, self.sel_obj) = (None, None);
            self.cur_layer.clear();
            return;
        }
        ui.add_space(2.);
        grid("scene").show(ui, |ui| {
            lbl(ui, "Nombre");
            let names: Vec<String> = self.p.scenes.iter().map(|s| s.name.clone()).collect();
            if let Some(n) = name_edit(ui, &self.p.scenes[sc].name, 150., &|n| names.iter().any(|s| s == n)) {
                let old = std::mem::replace(&mut self.p.scenes[sc].name, n.clone());
                if self.p.first_scene == old {
                    self.p.first_scene = n.clone();
                }
                ren_all(&mut self.p, None, P::Scene, &old, &n);
            }
            ui.end_row();
            lbl(ui, "Fondo");
            ui.color_edit_button_srgb(&mut self.p.scenes[sc].background);
            ui.end_row();
        });
        ui.add_space(4.);
        ui.separator();
        let s = &self.p.scenes[sc];
        let (no, ng, nl) = (s.objects.len() + self.p.objects.len(), s.groups.len(), s.layers.len());
        egui::CollapsingHeader::new(section("Objetos", Some(no))).id_salt("objs").default_open(true).show(ui, |ui| self.objects_ui(ui));
        egui::CollapsingHeader::new(section("Grupos", Some(ng))).id_salt("grps").show(ui, |ui| self.groups_ui(ui));
        egui::CollapsingHeader::new(section("Capas", Some(nl))).id_salt("caps").default_open(true).show(ui, |ui| self.layers_ui(ui));
        egui::CollapsingHeader::new(section("Proyecto", None)).id_salt("proj").show(ui, |ui| {
            grid("proj").show(ui, |ui| {
                lbl(ui, "Nombre");
                ui.add(TextEdit::singleline(&mut self.p.name).desired_width(150.));
                ui.end_row();
                lbl(ui, "Ancho");
                ui.add(DragValue::new(&mut self.p.width).range(16..=8192).suffix(" px"));
                ui.end_row();
                lbl(ui, "Alto");
                ui.add(DragValue::new(&mut self.p.height).range(16..=8192).suffix(" px"));
                ui.end_row();
                lbl(ui, "Escena inicial");
                let names: Vec<String> = self.p.scenes.iter().map(|s| s.name.clone()).collect();
                combo(ui, "first", &mut self.p.first_scene, &names, "(ninguna)");
                ui.end_row();
            });
        });
    }

    fn objects_ui(&mut self, ui: &mut Ui) {
        let s = &self.p.scenes[self.sc];
        let list: Vec<(String, &str, bool)> = s.objects.iter().map(|o| (o.name.clone(), kind_icon(&o.kind), false))
            .chain(self.p.objects.iter().map(|o| (o.name.clone(), kind_icon(&o.kind), true)))
            .collect();
        if list.is_empty() {
            ui.label(muted("Esta escena no tiene objetos. Agrega uno:"));
        }
        ui.spacing_mut().item_spacing.y = 2.;
        for (name, icon, global) in list {
            let sel = self.sel_obj.as_deref() == Some(&name);
            let atoms = (
                RichText::new(icon).color(if sel { ACCENT } else { TEXT_MUTED }),
                RichText::new(&name).atom_grow(true).atom_align(Align2::LEFT_CENTER),
                RichText::new(if global { "global" } else { "" }).small().color(TEXT_MUTED),
            );
            let r = ui.add(Button::selectable(sel, atoms).frame_when_inactive(sel).sense(Sense::click_and_drag()).min_size(Vec2::new(ui.available_width(), 26.)))
                .on_hover_text("Arrastra al lienzo para crear una instancia");
            if r.clicked() || r.drag_started() {
                self.sel_obj = Some(name.clone());
            }
            if r.dragged() {
                r.dnd_set_drag_payload(name);
            }
        }
        ui.spacing_mut().item_spacing.y = 6.;
        ui.add_space(4.);
        ui.horizontal_wrapped(|ui| {
            for kind in [
                Kind::Sprite { w: 0., h: 0., animations: vec![Animation { name: "quieto".into(), frames: vec![], fps: 10., looping: true }] },
                Kind::Text { text: "Texto".into(), size: 24., color: [255; 3] },
                Kind::Box { w: 32., h: 32., color: [200, 80, 80] },
            ] {
                let base = kind_name(&kind);
                if ui.add(Button::new(it(kind_icon(&kind), base))).on_hover_text(format!("Nuevo objeto {}", base.to_lowercase())).clicked() {
                    let n = (1..).map(|k| if k == 1 { base.to_string() } else { format!("{base}{k}") }).find(|n| !self.name_taken(n)).unwrap();
                    self.scene().objects.push(ObjectDef { name: n.clone(), kind, behaviors: vec![], vars: Vars::new() });
                    self.sel_obj = Some(n);
                }
            }
        });
    }

    fn groups_ui(&mut self, ui: &mut Ui) {
        let sc = self.sc;
        let objs: Vec<String> = self.p.objects_of(&self.p.scenes[sc]).map(|o| o.name.clone()).collect();
        let taken: Vec<String> = self.obj_names();
        let (mut ren, mut del) = (None, None);
        if self.p.scenes[sc].groups.is_empty() {
            ui.label(muted("Sin grupos. Un grupo permite usar varios objetos como uno solo en los eventos."));
        }
        for (g, members) in self.p.scenes[sc].groups.iter_mut() {
            ui.push_id(g, |ui| {
                card(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(muted(ic::STACK));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ib(ui, ic::TRASH, "Eliminar grupo").clicked() {
                                del = Some(g.clone());
                            }
                            if let Some(n) = name_edit(ui, g, f32::INFINITY, &|n| taken.iter().any(|t| t == n)) {
                                ren = Some((g.clone(), n));
                            }
                        });
                    });
                    let mut rm = None;
                    for (i, m) in members.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.add_space(22.);
                            ui.label(m);
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ib(ui, ic::X, "Quitar del grupo").clicked() {
                                    rm = Some(i);
                                }
                            });
                        });
                    }
                    if let Some(i) = rm {
                        members.remove(i);
                    }
                    egui::ComboBox::from_id_salt("add").selected_text(format!("{}  Añadir objeto", ic::PLUS)).show_ui(ui, |ui| {
                        let free: Vec<&String> = objs.iter().filter(|o| !members.contains(o)).collect();
                        for o in free {
                            if ui.selectable_label(false, o).clicked() {
                                members.push(o.clone());
                            }
                        }
                    });
                });
            });
        }
        let groups = &mut self.p.scenes[sc].groups;
        if let Some((old, n)) = ren {
            let m = groups.remove(&old).unwrap_or_default();
            groups.insert(n.clone(), m);
            ren_all(&mut self.p, Some(sc), P::Object, &old, &n);
        } else if let Some(g) = del {
            groups.remove(&g);
            self.msg = (Nivel::Info, format!("Eliminado el grupo «{g}» · {UNDO_HINT}"));
        }
        ui.horizontal(|ui| ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let add = ib(ui, ic::PLUS, "Crear grupo").clicked();
            let r = ui.add(TextEdit::singleline(&mut self.nb[4]).hint_text("nuevo grupo").desired_width(f32::INFINITY));
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
            if (add || enter) && !self.nb[4].trim().is_empty() && !taken.contains(&self.nb[4]) {
                let n = std::mem::take(&mut self.nb[4]);
                self.p.scenes[sc].groups.insert(n, vec![]);
            }
      }));
    }

    fn layers_ui(&mut self, ui: &mut Ui) {
        let sc = self.sc;
        let names: Vec<String> = self.p.scenes[sc].layers.iter().map(|l| l.name.clone()).collect();
        let (mut act, mut ren) = (None, None);
        let n = self.p.scenes[sc].layers.len();
        ui.spacing_mut().item_spacing.x = 2.;
        for (i, l) in self.p.scenes[sc].layers.iter_mut().enumerate() {
            ui.push_id(i, |ui| {
                ui.horizontal(|ui| {
                    let cur = self.cur_layer == l.name;
                    let pin = Button::new(RichText::new(ic::PIN).size(16.).color(if cur { ACCENT } else { TEXT_FAINT })).frame_when_inactive(false);
                    if ui.add(pin).on_hover_text("Capa activa: aquí se colocan las instancias nuevas").clicked() {
                        self.cur_layer = l.name.clone();
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().button_padding = Vec2::new(5., 5.);
                        let b = Button::new(RichText::new(ic::DOTS).size(16.)).frame_when_inactive(false).min_size(Vec2::splat(26.));
                        let (r, _) = egui::containers::menu::MenuButton::from_button(b).ui(ui, |ui| {
                            menu_spacing(ui);
                            if ui.add_enabled(i > 0, Button::new(it(ic::CARET_UP, "Subir (se dibuja antes)"))).clicked() {
                                act = Some((i, -1));
                            }
                            if ui.add_enabled(i + 1 < n, Button::new(it(ic::CARET_DOWN, "Bajar (se dibuja después)"))).clicked() {
                                act = Some((i, 1));
                            }
                            if !l.name.is_empty() {
                                ui.separator();
                                if ui.button(danger_it("Eliminar capa")).clicked() {
                                    act = Some((i, 0));
                                }
                            }
                        });
                        r.on_hover_text("Más opciones de la capa");
                        let hud = Button::selectable(l.fixed, RichText::new("HUD").small()).frame_when_inactive(l.fixed);
                        if ui.add(hud).on_hover_text("Fija en pantalla: no sigue a la cámara").clicked() {
                            l.fixed = !l.fixed;
                        }
                        let eye = if l.visible { ic::EYE } else { ic::EYE_SLASH };
                        if ib(ui, eye, if l.visible { "Ocultar en el editor y el juego" } else { "Mostrar" }).clicked() {
                            l.visible = !l.visible;
                        }
                        ui.add_space(6.);
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            if l.name.is_empty() {
                                ui.label(muted("Base"));
                            } else if let Some(n) = name_edit(ui, &l.name, f32::INFINITY, &|n| names.iter().any(|s| s == n)) {
                                ren = Some((l.name.clone(), n));
                            }
                        });
                    });
                });
            });
        }
        ui.spacing_mut().item_spacing.x = 8.;
        if let Some((old, n)) = ren {
            let s = &mut self.p.scenes[sc];
            s.layers.iter_mut().filter(|l| l.name == old).for_each(|l| l.name = n.clone());
            s.instances.iter_mut().filter(|i| i.layer == old).for_each(|i| i.layer = n.clone());
            ren_all(&mut self.p, Some(sc), P::Layer, &old, &n);
            if self.cur_layer == old {
                self.cur_layer = n;
            }
        }
        let s = &mut self.p.scenes[sc];
        match act {
            Some((i, 0)) => {
                let old = s.layers.remove(i).name;
                s.instances.iter_mut().filter(|n| n.layer == old).for_each(|n| n.layer.clear());
                self.cur_layer.clear();
                self.msg = (Nivel::Info, format!("Eliminada la capa «{old}» · {UNDO_HINT}"));
            }
            Some((i, -1)) if i > 0 => s.layers.swap(i, i - 1),
            Some((i, 1)) if i + 1 < s.layers.len() => s.layers.swap(i, i + 1),
            _ => {}
        }
        if ui.button(it(ic::PLUS, "Capa")).clicked() {
            let n = (1..).map(|k| format!("Capa{k}")).find(|n| !names.contains(n)).unwrap();
            s.layers.push(Layer { name: n, visible: true, fixed: false });
        }
    }

    // ---- panel derecho ----

    fn right(&mut self, ui: &mut Ui) {
        let sc = self.sc;
        if let Some(i) = self.sel_valid() {
            let s = &self.p.scenes[sc];
            let layers: Vec<String> = s.layers.iter().map(|l| l.name.clone()).collect();
            let osz = self.p.objects_of(s).find(|o| o.name == s.instances[i].obj).map_or(Vec2::splat(32.), |o| self.obj_size(o));
            header(ui, ic::CROSSHAIR, "Instancia", &format!("#{i}"));
            let ins = &mut self.p.scenes[sc].instances[i];
            grid("inst").show(ui, |ui| {
                lbl(ui, "Objeto");
                ui.label(&ins.obj);
                ui.end_row();
                for (n, v) in [("X", &mut ins.x), ("Y", &mut ins.y), ("Ángulo", &mut ins.angle), ("Z", &mut ins.z)] {
                    lbl(ui, n);
                    ui.add(DragValue::new(v).speed(1.));
                    ui.end_row();
                }
                lbl(ui, "Capa");
                combo(ui, "capa", &mut ins.layer, &layers, "Base");
                ui.end_row();
                for (n, v, d) in [("Ancho propio", &mut ins.w, osz.x), ("Alto propio", &mut ins.h, osz.y)] {
                    lbl(ui, n);
                    ui.horizontal(|ui| {
                        let mut on = v.is_some();
                        if ui.checkbox(&mut on, "").on_hover_text("Usar un tamaño distinto al del objeto").changed() {
                            *v = on.then_some(d);
                        }
                        if let Some(x) = v {
                            ui.add(DragValue::new(x).range(1..=8192));
                        }
                    });
                    ui.end_row();
                }
            });
            let nv = ins.vars.len();
            egui::CollapsingHeader::new(section(if self.facil { "Cajitas de esta copia" } else { "Variables de instancia" }, Some(nv))).id_salt("ivars")
                .show(ui, |ui| vars_ui(ui, "instvars", &mut ins.vars, &mut self.nb[2]));
            ui.horizontal(|ui| {
                if ui.button(it(ic::COPY, "Duplicar")).on_hover_text("Ctrl+D").clicked() {
                    self.duplicate_instance();
                }
                if ui.button(danger_it("Eliminar")).on_hover_text("Supr").clicked() {
                    self.delete_instance();
                }
            });
            ui.add_space(6.);
            ui.separator();
            ui.add_space(4.);
        }
        let base = self.base_dir();
        let sel = self.sel_obj.clone().filter(|n| self.p.objects_of(&self.p.scenes[sc]).any(|o| &o.name == n));
        if let Some(name) = sel {
            let global = self.p.objects.iter().any(|o| o.name == name);
            let kind = self.obj_mut(&name).map_or("", |o| kind_name(&o.kind));
            header(ui, ic::CUBE, "Objeto", &format!("{kind}{}", if global { " · global" } else { "" }));
            grid("objname").show(ui, |ui| {
                lbl(ui, "Nombre");
                if let Some(n) = name_edit(ui, &name, 150., &|n| self.name_taken(n)) {
                    self.obj_mut(&name).unwrap().name = n.clone();
                    ren_all(&mut self.p, if global { None } else { Some(sc) }, P::Object, &name, &n);
                    self.sel_obj = Some(n);
                }
                ui.end_row();
            });
            let name = self.sel_obj.clone().unwrap_or(name);
            let mut nb = std::mem::take(&mut self.nb[3]);
            let usos: Option<HashMap<String, usize>> = self.facil.then(|| {
                let vars: Vec<String> = self.p.objects_of(&self.p.scenes[sc]).find(|o| o.name == name).map(|o| o.vars.keys().cloned().collect()).unwrap_or_default();
                vars.into_iter().map(|v| {
                    let f = |i: &Instr| cajitas::usa_obj_var(i, &name, &v);
                    let n = self.p.scenes.iter().map(|s| cajitas::contar(&s.events, &f)).sum::<usize>()
                        + self.p.functions.iter().map(|x| cajitas::contar(&x.events, &f)).sum::<usize>();
                    (v, n)
                }).collect()
            });
            if let Some(o) = self.obj_mut(&name) {
                obj_props(ui, o, &base, &mut nb, usos.as_ref());
            }
            self.nb[3] = nb;
            ui.add_space(6.);
            if ui.button(danger_it("Eliminar objeto")).on_hover_text("Elimina también sus instancias").clicked() {
                self.snap_hist();
                self.p.scenes[sc].objects.retain(|o| o.name != name);
                self.p.objects.retain(|o| o.name != name);
                for s in &mut self.p.scenes {
                    s.instances.retain(|i| i.obj != name);
                    s.groups.values_mut().for_each(|g| g.retain(|o| *o != name));
                }
                (self.sel_obj, self.sel_inst) = (None, None);
                self.say(Nivel::Info, format!("Eliminado «{name}» y sus instancias · {UNDO_HINT}"));
            }
        } else if self.sel_inst.is_none() {
            ui.add_space(40.);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(ic::CURSOR).size(28.).color(TEXT_FAINT));
                ui.label(muted("Selecciona un objeto de la lista o una instancia en el lienzo."));
            });
        }
    }

    // ---- lienzo ----

    fn canvas(&mut self, ui: &mut Ui) {
        Frame::new().inner_margin(Margin::symmetric(8, 6)).show(ui, |ui| {
            ui.horizontal(|ui| {
                seg(ui, |ui| {
                    if seg_btn(ui, !self.placing, it(ic::CURSOR, "Seleccionar")).on_hover_text("Seleccionar y mover instancias").clicked() {
                        self.placing = false;
                    }
                    if seg_btn(ui, self.placing, it(ic::PLUS, "Colocar")).on_hover_text("Clic en el lienzo crea una instancia del objeto seleccionado").clicked() {
                        self.placing = true;
                    }
                });
                seg(ui, |ui| {
                    if seg_btn(ui, self.snap, it(ic::MAGNET, "Rejilla")).on_hover_text("Imantar a la rejilla").clicked() {
                        self.snap = !self.snap;
                    }
                    for g in [8., 16., 32.] {
                        if seg_btn(ui, self.grid == g, format!("{g}")).on_hover_text("Tamaño de la rejilla").clicked() {
                            self.grid = g;
                        }
                    }
                });
                if ui.button(it(ic::FIT, "Ajustar vista")).clicked() {
                    self.fit = true;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add(egui::Label::new(RichText::new("Rueda: zoom. Botón derecho: mover la vista").small().color(TEXT_MUTED)).truncate());
                });
            });
        });
        let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let r = resp.rect;
        let (pw, ph) = (self.p.width, self.p.height);
        if self.fit {
            self.zoom = (r.width() / pw).min(r.height() / ph) * 0.9;
            self.pan = (r.size() - Vec2::new(pw, ph) * self.zoom) / 2.;
        }
        if resp.hovered() {
            let dy = ui.input(|i| i.smooth_scroll_delta.y);
            if let (true, Some(m)) = (dy != 0., resp.hover_pos()) {
                let w = (m - r.min - self.pan) / self.zoom;
                self.zoom = (self.zoom * (dy * 0.003).exp()).clamp(0.05, 20.);
                self.pan = m - r.min - w * self.zoom;
                self.fit = false;
            }
        }
        if resp.dragged_by(PointerButton::Middle) || resp.dragged_by(PointerButton::Secondary) {
            self.pan += resp.drag_delta();
            self.fit = false;
        }
        let (org, z) = (r.min + self.pan, self.zoom);
        let to_s = |p: Vec2| org + p * z;
        let to_w = |p: Pos2| ((p - org) / z).to_pos2();
        self.cursor_w = resp.hover_pos().map(to_w);

        // Precarga de imágenes y lista de dibujo ordenada por capa y z.
        let s = &self.p.scenes[self.sc];
        let frames: Vec<String> = self.p.objects_of(s)
            .filter_map(|o| match &o.kind {
                Kind::Sprite { animations, .. } => animations.first().and_then(|a| a.frames.first()).cloned(),
                _ => None,
            })
            .collect();
        let ctx = ui.ctx().clone();
        frames.iter().for_each(|f| drop(self.texture(&ctx, f)));
        let s = &self.p.scenes[self.sc];
        let lidx = |n: &str| s.layers.iter().position(|l| l.name == n);
        let mut order: Vec<usize> = (0..s.instances.len()).filter(|&i| lidx(&s.instances[i].layer).is_none_or(|k| s.layers[k].visible)).collect();
        order.sort_by(|&a, &b| {
            let (a, b) = (&s.instances[a], &s.instances[b]);
            (lidx(&a.layer).unwrap_or(0), a.z).partial_cmp(&(lidx(&b.layer).unwrap_or(0), b.z)).unwrap()
        });
        let painter = painter.with_clip_rect(r);
        painter.rect_filled(r, 0., BG);
        let game = Rect::from_min_max(to_s(Vec2::ZERO), to_s(Vec2::new(pw, ph)));
        let [br, bg, bb] = s.background;
        painter.rect_filled(game, 0., Color32::from_rgb(br, bg, bb));
        if self.snap && self.grid * z >= 6. {
            let st = Stroke::new(1., Color32::from_white_alpha(10));
            let mut x = 0.;
            while x <= pw {
                painter.line_segment([to_s(Vec2::new(x, 0.)), to_s(Vec2::new(x, ph))], st);
                x += self.grid;
            }
            let mut y = 0.;
            while y <= ph {
                painter.line_segment([to_s(Vec2::new(0., y)), to_s(Vec2::new(pw, y))], st);
                y += self.grid;
            }
        }
        painter.rect_stroke(game, 0., Stroke::new(1., BORDER_STRONG), egui::StrokeKind::Outside);
        let mut items: Vec<(usize, Rect)> = vec![];
        for &i in &order {
            let ins = &s.instances[i];
            let pos = Vec2::new(ins.x, ins.y);
            let Some(o) = self.p.objects_of(s).find(|o| o.name == ins.obj) else {
                let wr = Rect::from_min_size(pos.to_pos2(), Vec2::splat(32.));
                painter.rect_stroke(Rect::from_min_max(to_s(pos), to_s(pos + wr.size())), 0., (2., DANGER), egui::StrokeKind::Inside);
                items.push((i, wr));
                continue;
            };
            let sz = self.obj_size(o);
            let sz = Vec2::new(ins.w.unwrap_or(sz.x), ins.h.unwrap_or(sz.y));
            let sr = Rect::from_min_max(to_s(pos), to_s(pos + sz));
            match &o.kind {
                Kind::Box { color: [r, g, b], .. } => drop(painter.rect_filled(sr, 0., Color32::from_rgb(*r, *g, *b))),
                Kind::Text { text, size, color: [r, g, b] } => {
                    painter.text(sr.min, Align2::LEFT_TOP, text, egui::FontId::proportional(size * z), Color32::from_rgb(*r, *g, *b));
                }
                Kind::Sprite { animations, .. } => {
                    match animations.first().and_then(|a| a.frames.first()).and_then(|f| self.tex.get(f).cloned().flatten()) {
                        Some(t) => drop(painter.image(t.id(), sr, Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)), Color32::WHITE)),
                        None => {
                            // Sin imagen: marcador neutro con ícono.
                            painter.rect_filled(sr, 0., SURFACE_HOVER);
                            painter.rect_stroke(sr, 0., Stroke::new(1., BORDER_STRONG), egui::StrokeKind::Inside);
                            painter.text(sr.center(), Align2::CENTER_CENTER, ic::IMAGE, egui::FontId::proportional((sr.height() * 0.5).clamp(8., 32.)), TEXT_FAINT);
                        }
                    }
                }
            }
            items.push((i, Rect::from_min_size(pos.to_pos2(), sz)));
        }
        if s.instances.is_empty() {
            let t = if self.p.objects_of(s).next().is_some() { "Arrastra un objeto desde la lista para colocarlo" } else { "Crea un objeto en el panel izquierdo para empezar" };
            painter.text(game.center(), Align2::CENTER_CENTER, t, egui::FontId::proportional(14.), TEXT_MUTED);
        }
        let hit = |p: Pos2| items.iter().rev().find(|(_, wr)| wr.contains(p)).map(|x| x.0);
        let srect = |wr: &Rect| Rect::from_min_max(to_s(wr.min.to_vec2()), to_s(wr.max.to_vec2()));
        self.hover_inst = self.cursor_w.and_then(hit);
        if let Some((_, wr)) = items.iter().find(|x| Some(x.0) == self.hover_inst && self.hover_inst != self.sel_inst) {
            painter.rect_stroke(srect(wr), 0., Stroke::new(1., BORDER_STRONG), egui::StrokeKind::Outside);
        }
        if let Some((_, wr)) = items.iter().find(|x| Some(x.0) == self.sel_inst) {
            let sr = srect(wr).expand(1.);
            painter.rect_stroke(sr, 0., Stroke::new(1.5, ACCENT), egui::StrokeKind::Outside);
            for c in [sr.left_top(), sr.right_top(), sr.left_bottom(), sr.right_bottom()] {
                let h = Rect::from_center_size(c, Vec2::splat(6.));
                painter.rect_filled(h, 1, ACCENT);
                painter.rect_stroke(h, 1, Stroke::new(1., BG), egui::StrokeKind::Outside);
            }
        }

        // Interacción
        let sc = self.sc;
        if resp.drag_started_by(PointerButton::Primary) {
            if let Some(p) = ui.input(|i| i.pointer.press_origin()).map(to_w) {
                self.sel_inst = hit(p);
                if let Some(i) = self.sel_inst {
                    let ins = &self.p.scenes[sc].instances[i];
                    self.drag_off = Some(Vec2::new(ins.x, ins.y) - p.to_vec2());
                    self.sel_obj = Some(ins.obj.clone());
                }
            }
        }
        if let (true, Some(i), Some(off), Some(p)) = (resp.dragged_by(PointerButton::Primary), self.sel_inst, self.drag_off, resp.interact_pointer_pos()) {
            let mut q = to_w(p).to_vec2() + off;
            if self.snap {
                q = (q / self.grid).round() * self.grid;
            }
            let ins = &mut self.p.scenes[sc].instances[i];
            (ins.x, ins.y) = (q.x, q.y);
        }
        if resp.drag_stopped() {
            self.drag_off = None;
        }
        if resp.hovered() && self.drag_off.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if resp.hovered() && self.hover_inst.is_some() && !self.placing {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }
        if resp.clicked() {
            if let Some(p) = resp.interact_pointer_pos().map(to_w) {
                match self.sel_obj.clone() {
                    Some(o) if self.placing => self.add_instance(o, p),
                    _ => {
                        self.sel_inst = hit(p);
                        if let Some(i) = self.sel_inst {
                            self.sel_obj = Some(self.p.scenes[sc].instances[i].obj.clone());
                        }
                    }
                }
            }
        }
        if let (Some(o), Some(p)) = (resp.dnd_release_payload::<String>(), resp.hover_pos()) {
            self.add_instance((*o).clone(), to_w(p));
        }
    }

    // ---- hojas de eventos ----

    fn sheet(&mut self, ui: &mut Ui, t: Target) {
        if self.facil {
            return self.sheet_kid(ui, t);
        }
        let objs = self.obj_names();
        let Some(evs) = self.events_mut(t) else { return };
        let mut out = Salida::default();
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            if evs.is_empty() {
                ui.add_space(48.);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(ic::LIST).size(32.).color(TEXT_FAINT));
                    ui.label(RichText::new("Todavía no hay eventos").font(sb(16.)));
                    ui.label(muted("Cada evento tiene condiciones y acciones: cuando se cumplen las condiciones, se ejecutan las acciones."));
                });
                ui.add_space(12.);
            } else {
                eventos::cabecera(ui);
                eventos::sheet(ui, evs, &[], &objs, &mut out);
            }
            ui.add_space(6.);
            let add = |ui: &mut Ui, evs: &mut Vec<Event>| {
                if ui.button(it(ic::PLUS, "Evento")).clicked() {
                    evs.push(Event::default());
                }
                if ui.button(it(ic::NOTE, "Comentario")).clicked() {
                    evs.push(Event { kind: EventKind::Comment, ..Default::default() });
                }
                if ui.button(it(ic::FOLDERS, "Grupo")).clicked() {
                    evs.push(Event { kind: EventKind::Group, text: "Grupo".into(), ..Default::default() });
                }
            };
            if evs.is_empty() {
                ui.vertical_centered(|ui| ui.horizontal(|ui| {
                    ui.add_space((ui.available_width() - 330.).max(0.) / 2.);
                    add(ui, evs)
                }));
            } else {
                ui.horizontal(|ui| {
                    ui.add_space(eventos::GUTTER + 8.);
                    add(ui, evs)
                });
            }
            ui.add_space(24.);
        });
        if let Some(o) = out.open {
            let ins = if o.new { Instr::default() } else { eventos::get(evs, &o).unwrap_or_default() };
            self.ed = Some(InstrEdit { at: o, target: t, ins, search: String::new(), focus: true });
        }
        if let Some(m) = out.msg {
            self.say(Nivel::Info, m);
        }
    }

    /// Hoja del modo fácil: tarjetas CUANDO / ENTONCES, aviso de problemas y recetas.
    fn sheet_kid(&mut self, ui: &mut Ui, t: Target) {
        let ctx = ui.ctx().clone();
        let l = self.kid_lists(&ctx);
        let owner = match t {
            Target::Scene(i) => format!("Escena «{}»", self.p.scenes[i].name),
            Target::Func(i) => format!("Función «{}»", self.p.functions[i].name),
        };
        let (probs, otros) = facil::problemas(&self.probs, &owner);
        let now_t = ctx.input(|i| i.time);
        if self.jump.as_ref().is_some_and(|j| j.1 < now_t) {
            self.jump = None;
        }
        let jump = self.jump.as_ref().map(|j| j.0.clone());
        let mut out = Salida::default();
        let mut ver = false;
        let mut nota_cerrar = false;
        let func = matches!(t, Target::Func(_));
        let nota = self.nota.clone();
        let Some(evs) = self.events_mut(t) else { return };
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            ui.set_max_width(ui.available_width().min(1280.));
            ver = facil::aviso(ui, probs.len(), if func { &[] } else { &otros });
            if let Some(n) = &nota {
                Frame::new().fill(blend(CAT_MOV, SURFACE, 0.16)).stroke((1., blend(CAT_MOV, SURFACE, 0.5))).corner_radius(10).inner_margin(Margin::symmetric(12, 8)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(ic::LIGHTBULB).size(20.).color(CAT_MOV));
                        let w = ui.available_width() - 40.;
                        ui.add_sized([w, 0.], egui::Label::new(RichText::new(n).size(15.).color(TEXT)).wrap());
                        nota_cerrar = ib(ui, ic::X, "Cerrar consejo").clicked();
                    });
                });
            }
            if !probs.is_empty() || nota.is_some() || (!func && !otros.is_empty()) {
                ui.add_space(6.);
            }
            if evs.is_empty() {
                let (a, b) = facil::vacio(ui, func);
                if a {
                    evs.push(Event::default());
                }
                out.recetas |= b;
                return;
            }
            let h = facil::Hoja { l: &l, probs: &probs, jump: jump.as_deref() };
            facil::hoja(ui, evs, &[], &h, &mut out);
            ui.add_space(8.);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().button_padding = Vec2::new(14., 8.);
                if primary(ui, (on_accent(ic::PLUS).size(17.), on_accent("Nuevo evento").font(sb(15.)))).on_hover_text("Un evento dice CUANDO pasa algo, ENTONCES qué hacer").clicked() {
                    evs.push(Event::default());
                }
                if ui.add(Button::new((RichText::new(ic::COOKING_POT).size(17.).color(CAT_FUNC), RichText::new("Recetas").font(sb(15.))))).on_hover_text("Trozos de juego listos para usar").clicked() {
                    out.recetas = true;
                }
                if ui.add(Button::new((RichText::new(ic::NOTE).size(17.).color(WARN), RichText::new("Nota").font(sb(15.))))).on_hover_text("Una nota para explicar algo; el juego no la usa").clicked() {
                    evs.push(Event { kind: EventKind::Comment, ..Default::default() });
                }
                if ui.add(Button::new((RichText::new(ic::FOLDERS).size(17.).color(TEXT_SOFT), RichText::new("Grupo").font(sb(15.))))).on_hover_text("Una carpeta para ordenar eventos").clicked() {
                    evs.push(Event { kind: EventKind::Group, text: "Grupo".into(), ..Default::default() });
                }
            });
            ui.add_space(40.);
        });
        if nota_cerrar {
            self.nota = None;
        }
        if !probs.is_empty() && facil::MOSTRAR_PRIMERO.swap(false, std::sync::atomic::Ordering::Relaxed) {
            ver = true;
        }
        if ver {
            if let Some(p) = probs.first() {
                self.jump = Some((p.ev.clone(), now_t + 2.5));
            }
        }
        if let Some((n, g)) = out.new_var {
            self.crear_cajita(&n, g);
        }
        if out.recetas {
            self.rec = Some(recetario::Recetario::default());
        }
        if let Some(o) = out.open {
            let ins = if o.new { None } else { self.events_mut(t).and_then(|e| eventos::get(e, &o)) };
            self.sel = Some(facil::Selector::new(o, t, ins));
        }
        if let Some(m) = out.msg {
            self.say(Nivel::Info, m);
        }
    }

    fn crear_cajita(&mut self, n: &str, global: bool) {
        let sc = self.sc;
        if self.p.globals.contains_key(n) || self.p.scenes[sc].vars.contains_key(n) {
            return;
        }
        let vars = if global { &mut self.p.globals } else { &mut self.p.scenes[sc].vars };
        vars.insert(n.into(), Value::Num(0.));
        self.say(Nivel::Ok, format!("Nueva cajita «{n}» (empieza en 0)"));
    }

    /// Mis bloques (funciones) en modo fácil.
    fn funcs_kid(&mut self, ui: &mut Ui) {
        let panel = Frame::new().fill(PANEL).inner_margin(14);
        let mut act = None;
        egui::Panel::left("misbloques").default_size(260.).size_range(220.0..=420.).frame(panel).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| act = bloques::lista(ui, &self.p.functions, self.func));
        });
        let names: Vec<String> = self.p.functions.iter().map(|f| f.name.clone()).collect();
        Frame::new().inner_margin(14).show(ui, |ui| {
            let fi = self.func;
            if fi < self.p.functions.len() {
                let taken = |n: &str| names.iter().any(|s| s == n);
                if let Some(a) = bloques::cabecera(ui, fi, &mut self.p.functions[fi], &mut self.fnuevo, &taken) {
                    act = Some(a);
                }
                ui.add_space(10.);
                self.sheet(ui, Target::Func(fi));
            } else if bloques::vacio(ui) {
                act = Some(bloques::Act::Nuevo);
            }
        });
        match act {
            Some(bloques::Act::Sel(i)) => self.func = i,
            Some(bloques::Act::Nuevo) => self.asis = Some(bloques::Asistente::default()),
            Some(bloques::Act::Borrar(i)) if i < self.p.functions.len() => {
                self.snap_hist();
                let f = self.p.functions.remove(i);
                self.func = i.min(self.p.functions.len().saturating_sub(1));
                self.say(Nivel::Info, format!("Bloque «{}» borrado · {UNDO_HINT}", f.name));
            }
            Some(bloques::Act::Renombrar(i, n)) if i < self.p.functions.len() => {
                self.snap_hist();
                let old = std::mem::replace(&mut self.p.functions[i].name, n.clone());
                ren_all(&mut self.p, None, P::Func, &old, &n);
                self.say(Nivel::Ok, format!("Ahora el bloque se llama «{n}»"));
            }
            _ => {}
        }
    }

    fn variables_kid(&mut self, ui: &mut Ui) {
        let sc = self.sc;
        match cajitas::pestana(ui, &mut self.p, sc, &mut self.nueva_caj) {
            Some(cajitas::Accion::Borrar { global, name }) => {
                self.snap_hist();
                if global {
                    self.p.globals.remove(&name);
                } else {
                    self.p.scenes[sc].vars.remove(&name);
                }
                self.say(Nivel::Info, format!("Cajita «{name}» borrada · {UNDO_HINT}"));
            }
            Some(cajitas::Accion::Renombrar { global, old, new }) => {
                self.snap_hist();
                let vars = if global { &mut self.p.globals } else { &mut self.p.scenes[sc].vars };
                if let Some(v) = vars.remove(&old) {
                    vars.insert(new.clone(), v);
                }
                for (k, s) in self.p.scenes.iter_mut().enumerate() {
                    if global || k == sc {
                        cajitas::ren_var(&mut s.events, &old, &new);
                    }
                }
                self.p.functions.iter_mut().for_each(|f| cajitas::ren_var(&mut f.events, &old, &new));
                self.say(Nivel::Ok, format!("La cajita «{old}» ahora se llama «{new}» (también en los bloques)"));
            }
            None => {}
        }
    }

    fn funcs(&mut self, ui: &mut Ui) {
        if self.facil {
            return self.funcs_kid(ui);
        }
        let panel = Frame::new().fill(PANEL).inner_margin(12);
        egui::Panel::left("funcs").default_size(260.).frame(panel).show(ui, |ui| {
            ui.label(section("Funciones", Some(self.p.functions.len())));
            ui.add_space(2.);
            ui.spacing_mut().item_spacing.y = 2.;
            for (i, f) in self.p.functions.iter().enumerate() {
                let sel = self.func == i;
                let atoms = (
                    RichText::new(ic::FUNCTION).color(if sel { ACCENT } else { TEXT_MUTED }),
                    RichText::new(&f.name).atom_grow(true).atom_align(Align2::LEFT_CENTER),
                    RichText::new(if f.is_condition { "condición" } else { "acción" }).small().color(TEXT_MUTED),
                );
                if ui.add(Button::selectable(sel, atoms).frame_when_inactive(sel).min_size(Vec2::new(ui.available_width(), 26.))).clicked() {
                    self.func = i;
                }
            }
            ui.spacing_mut().item_spacing.y = 6.;
            ui.add_space(4.);
            if ui.button(it(ic::PLUS, "Nueva función")).clicked() {
                let n = (1..).map(|k| format!("funcion{k}")).find(|n| !self.p.functions.iter().any(|f| &f.name == n)).unwrap();
                self.p.functions.push(Function { name: n, is_condition: false, params: vec![], events: vec![] });
                self.func = self.p.functions.len() - 1;
            }
            let fi = self.func;
            if fi >= self.p.functions.len() {
                return;
            }
            ui.add_space(4.);
            ui.separator();
            ui.add_space(4.);
            let names: Vec<String> = self.p.functions.iter().map(|f| f.name.clone()).collect();
            grid("func").show(ui, |ui| {
                lbl(ui, "Nombre");
                if let Some(n) = name_edit(ui, &names[fi], 130., &|n| names.iter().any(|s| s == n)) {
                    let old = std::mem::replace(&mut self.p.functions[fi].name, n.clone());
                    ren_all(&mut self.p, None, P::Func, &old, &n);
                }
                ui.end_row();
                lbl(ui, "Tipo");
                let f = &mut self.p.functions[fi];
                seg(ui, |ui| {
                    if seg_btn(ui, !f.is_condition, "Acción").clicked() {
                        f.is_condition = false;
                    }
                    if seg_btn(ui, f.is_condition, "Condición").on_hover_text("Devuelve verdadero o falso con la acción «Devolver»").clicked() {
                        f.is_condition = true;
                    }
                });
                ui.end_row();
                lbl(ui, "Parámetros");
                if self.fparams.as_ref().is_none_or(|f| f.0 != fi) {
                    self.fparams = Some((fi, self.p.functions[fi].params.join(", ")));
                }
                let buf = &mut self.fparams.as_mut().unwrap().1;
                if ui.add(TextEdit::singleline(buf).hint_text("a, b, c").desired_width(130.)).on_hover_text("Separados por coma; llegan como variables locales").changed() {
                    self.p.functions[fi].params = buf.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                }
                ui.end_row();
            });
            ui.add_space(6.);
            if ui.button(danger_it("Eliminar función")).clicked() {
                self.snap_hist();
                let f = self.p.functions.remove(fi);
                self.func = fi.min(self.p.functions.len().saturating_sub(1));
                self.say(Nivel::Info, format!("Eliminada la función «{}» · {UNDO_HINT}", f.name));
            }
        });
        Frame::new().inner_margin(12).show(ui, |ui| {
            if self.func < self.p.functions.len() {
                self.sheet(ui, Target::Func(self.func));
            } else {
                ui.add_space(48.);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(ic::FUNCTION).size(32.).color(TEXT_FAINT));
                    ui.label(RichText::new("Sin funciones").font(sb(16.)));
                    ui.label(muted("Una función agrupa eventos para reutilizarlos como acción o condición propia."));
                });
            }
        });
    }

    fn variables(&mut self, ui: &mut Ui) {
        if self.facil {
            return self.variables_kid(ui);
        }
        let sc = self.sc;
        let name = self.p.scenes[sc].name.clone();
        let (a, b) = self.nb.split_at_mut(1);
        ui.columns(2, |c| {
            let panel = |ui: &mut Ui, icon: &str, t: &str, sub: &str, vars: &mut Vars, nb: &mut String, id: &str| {
                Frame::new().fill(PANEL).stroke((1., BORDER)).corner_radius(R).inner_margin(12).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    header(ui, icon, t, "");
                    ui.label(muted(sub));
                    ui.add_space(6.);
                    vars_ui(ui, id, vars, nb);
                });
            };
            panel(&mut c[0], ic::GLOBE, "Variables globales", "Disponibles en todas las escenas.", &mut self.p.globals, &mut a[0], "globvars");
            panel(&mut c[1], ic::BRACES, &format!("Variables de «{name}»"), "Solo existen en esta escena.", &mut self.p.scenes[sc].vars, &mut b[0], "scenevars");
        });
    }

    // ---- diálogos ----

    /// Selector de bloques, recetario y asistente de Mis bloques.
    fn modales_kid(&mut self, ctx: &egui::Context) {
        if let Some(mut s) = self.sel.take() {
            let l = self.kid_lists(ctx);
            let pregunta = matches!(s.target, Target::Func(i) if self.p.functions.get(i).is_some_and(|f| f.is_condition));
            let mut nv = None;
            let r = facil::selector(ctx, &mut s, &l, pregunta, &mut nv);
            if let Some((n, g)) = nv {
                self.crear_cajita(&n, g);
            }
            match (r, s.ins.clone()) {
                (None, _) => self.sel = Some(s),
                (Some(facil::SelRes::Cancel), _) | (Some(_), None) => {}
                (Some(r), Some(ins)) => {
                    self.snap_hist();
                    for (n, g) in facil::cajitas_faltantes(&ins, &l) {
                        self.crear_cajita(&n, g);
                    }
                    let del = r == facil::SelRes::Del;
                    if let Some(evs) = self.events_mut(s.target) {
                        facil::aplicar(evs, &s.at, &ins, del);
                    }
                    if del {
                        self.say(Nivel::Info, format!("Bloque borrado · {UNDO_HINT}"));
                    } else if s.at.new {
                        self.say(Nivel::Ok, format!("Bloque agregado: {}", catalog_sentence(&ins)));
                    }
                }
            }
        }
        if let Some(mut r) = self.rec.take() {
            let l = self.kid_lists(ctx);
            match recetario::recetario(ctx, &mut r, &l) {
                None => self.rec = Some(r),
                Some(recetario::RecRes::Cerrar) => {}
                Some(recetario::RecRes::Agregar(i, evs)) => {
                    self.snap_hist();
                    let rc = &nicostation::recetas::RECETAS[i];
                    let t = self.cur_target();
                    let n = evs.len();
                    if let Some(list) = self.events_mut(t) {
                        list.extend(evs);
                    }
                    let sc = self.sc;
                    for (k, v) in rc.variables_iniciales() {
                        if !self.p.globals.contains_key(&k) {
                            self.p.scenes[sc].vars.entry(k).or_insert(v);
                        }
                    }
                    let ev = if n == 1 { "1 evento nuevo".to_string() } else { format!("{n} eventos nuevos") };
                    self.nota = Some(format!("Receta «{}» lista: agregué {ev} al final de la hoja. {}", rc.nombre, rc.nota).trim().to_string());
                    self.say(Nivel::Ok, format!("Receta «{}» agregada ({ev})", rc.nombre));
                    if !matches!(self.tab, Tab::Eventos | Tab::Funciones) {
                        self.tab = Tab::Eventos;
                    }
                }
            }
        }
        if let Some(mut a) = self.asis.take() {
            let names: Vec<String> = self.p.functions.iter().map(|f| f.name.clone()).collect();
            match bloques::asistente(ctx, &mut a, &|n| names.iter().any(|x| x == n)) {
                None => self.asis = Some(a),
                Some(None) => {}
                Some(Some(f)) => {
                    self.snap_hist();
                    let n = f.name.clone();
                    self.p.functions.push(f);
                    self.func = self.p.functions.len() - 1;
                    self.tab = Tab::Funciones;
                    self.say(Nivel::Ok, format!("Nuevo bloque «{n}». Ahora agrégale eventos."));
                }
            }
        }
    }

    fn confirm_ui(&mut self, ctx: &egui::Context) {
        let Some(i) = self.confirm.filter(|i| *i < self.p.scenes.len()) else { return self.confirm = None };
        let s = &self.p.scenes[i];
        let (name, n) = (s.name.clone(), count_events(&s.events));
        let mut res = None;
        let r = egui::Modal::new(egui::Id::unique("confirmar")).frame(Frame::window(&ctx.global_style()).inner_margin(16)).show(ctx, |ui| {
            ui.set_width(380.);
            ui.horizontal(|ui| {
                ui.label(RichText::new(ic::WARNING).size(18.).color(WARN));
                ui.heading(format!("¿Eliminar la escena «{name}»?"));
            });
            ui.label(muted(format!("Tiene {n} evento{}. Puedes recuperarla con Ctrl+Z.", if n == 1 { "" } else { "s" })));
            ui.add_space(8.);
            ui.horizontal(|ui| {
                if ui.button(danger_it("Eliminar escena")).clicked() {
                    res = Some(true);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Cancelar").clicked() {
                        res = Some(false);
                    }
                });
            });
        });
        if res.is_none() && r.should_close() {
            res = Some(false);
        }
        if let Some(ok) = res {
            self.confirm = None;
            if ok {
                self.delete_scene(i);
            }
        }
    }

    fn export_ui(&mut self, ctx: &egui::Context) {
        let Some(ex) = self.exp.as_mut() else { return };
        let (mut close, mut go) = (false, false);
        let base = self.path.as_ref().and_then(|p| p.parent()).map(Path::to_path_buf).unwrap_or_default();
        let r = egui::Modal::new(egui::Id::unique("exportar")).frame(Frame::window(&ctx.global_style()).inner_margin(16)).show(ctx, |ui| {
            ui.set_width(540.);
            ui.horizontal(|ui| {
                ui.label(RichText::new(ic::GLOBE).size(18.).color(ACCENT));
                ui.heading("Exportar web");
            });
            ui.add_space(4.);
            match &ex.res {
                None => {
                    ui.label(muted("Copia el reproductor, la página y los archivos del juego a una carpeta."));
                    ui.add_space(6.);
                    lbl(ui, "Carpeta de destino");
                    ui.horizontal(|ui| ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ib(ui, ic::FOLDER_OPEN, "Elegir carpeta").clicked() {
                            if let Some(d) = rfd::FileDialog::new().set_directory(&base).pick_folder() {
                                ex.dir = d.display().to_string();
                            }
                        }
                        ui.add(TextEdit::singleline(&mut ex.dir).desired_width(f32::INFINITY));
                    }));
                    ui.add_space(10.);
                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if primary(ui, (on_accent(ic::EXPORT), on_accent("Exportar").font(sb(14.)))).clicked() {
                                go = true;
                            }
                            if ui.button("Cancelar").clicked() {
                                close = true;
                            }
                        });
                    });
                }
                Some(Ok(n)) => {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(ic::CHECK).color(OK));
                        ui.label(format!("Exportado a {} ({n} archivos).", ex.dir.trim()));
                    });
                    ui.label(muted("Ábrelo con un servidor web local (los navegadores no cargan WebAssembly desde file://):"));
                    let cmd = format!("python3 -m http.server -d {} 8000", shell_quote(ex.dir.trim()));
                    ui.horizontal(|ui| ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ib(ui, ic::COPY, "Copiar comando").clicked() {
                            ui.ctx().copy_text(cmd.clone());
                        }
                        ui.add(TextEdit::singleline(&mut cmd.as_str()).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY));
                    }));
                    ui.label(muted("Luego abre http://localhost:8000 en el navegador."));
                    ui.add_space(10.);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Cerrar").clicked() {
                            close = true;
                        }
                    });
                }
                Some(Err(e)) => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(ic::WARNING).color(WARN));
                        ui.label(RichText::new("No se pudo exportar").font(sb(14.)));
                    });
                    let mut t = e.as_str();
                    ui.add(TextEdit::multiline(&mut t).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY).desired_rows(3));
                    ui.add_space(10.);
                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("Cerrar").clicked() {
                                close = true;
                            }
                            if ui.button("Volver").clicked() {
                                ex.res = None;
                            }
                        });
                    });
                }
            }
        });
        if close || r.should_close() {
            self.exp = None;
        } else if go {
            self.do_export();
        }
    }

    fn status(&mut self, ui: &mut Ui) {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if self.tab == Tab::Escena {
                ui.spacing_mut().item_spacing.x = 16.;
                ui.label(RichText::new(format!("{:.0} %", self.zoom * 100.)).monospace().color(TEXT_MUTED)).on_hover_text("Zoom");
                let c = self.cursor_w.map_or("x -    y -".into(), |p| format!("x {:<5.0}y {:<5.0}", p.x, p.y));
                ui.label(RichText::new(c).monospace().color(TEXT_MUTED)).on_hover_text("Posición del cursor en la escena");
                let n = self.sel_valid().map_or("Sin selección".into(), |i| format!("1 seleccionada ({})", self.p.scenes[self.sc].instances[i].obj));
                ui.label(muted(n));
            }
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 6.;
                let (n, m) = &self.msg;
                let (icon, col) = match n {
                    Nivel::Ok => (Some(ic::CHECK), OK),
                    Nivel::Warn => (Some(ic::WARNING), WARN),
                    Nivel::Error => (Some(ic::X), DANGER),
                    Nivel::Info => (None, TEXT_MUTED),
                };
                if let Some(i) = icon {
                    ui.label(RichText::new(i).color(col));
                }
                ui.add(egui::Label::new(RichText::new(m).color(if *n == Nivel::Info { TEXT_MUTED } else { TEXT })).truncate());
            });
        });
    }
}

impl App {
    /// Vista inicial pedida con `--abrir` (para probar la interfaz).
    fn abrir(&mut self, que: &str) {
        let t = Target::Scene(self.sc);
        let at = |cond| eventos::Open { ev: vec![0], cond, ip: vec![], new: true };
        let bloque = |app: &App, op: &str| {
            let l = app.lists();
            nicostation::catalog::meta(op).map(|m| facil::nuevo(m, &l))
        };
        self.tab = Tab::Eventos;
        match que {
            "cajitas" => self.tab = Tab::Variables,
            "bloques" => self.tab = Tab::Funciones,
            "asistente" => {
                self.tab = Tab::Funciones;
                self.asis = Some(bloques::Asistente { name: "saltar alto".into(), params: vec!["altura".into()], ..Default::default() });
            }
            "recetas" => self.rec = Some(recetario::Recetario::default()),
            "receta" => {
                let mut r = recetario::Recetario::default();
                r.elegir(6);
                r.vals[0] = self.obj_names().first().cloned().unwrap_or_default();
                self.rec = Some(r);
            }
            "selector" => self.sel = Some(facil::Selector::new(at(true), t, None)),
            "selector-acciones" => self.sel = Some(facil::Selector::new(at(false), t, None)),
            "tecla" => self.sel = Some(facil::Selector::new(at(true), t, bloque(self, "key_pressed"))),
            "objeto" => self.sel = Some(facil::Selector::new(at(true), t, bloque(self, "collides"))),
            "arreglar" => facil::MOSTRAR_PRIMERO.store(true, std::sync::atomic::Ordering::Relaxed),
            "ficha" => facil::ABRIR_FICHA.store(8, std::sync::atomic::Ordering::Relaxed),
            _ => {}
        }
    }
}

fn catalog_sentence(i: &Instr) -> String {
    nicostation::catalog::sentence(i)
}

/// Imágenes y sonidos dentro de la carpeta del proyecto (hasta 3 niveles).
fn buscar_archivos(base: &Path) -> Vec<String> {
    fn walk(base: &Path, dir: &Path, depth: usize, out: &mut Vec<String>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || out.len() >= 500 {
                continue;
            }
            if p.is_dir() {
                if depth < 3 && !matches!(name.as_str(), "target" | "web" | "node_modules") {
                    walk(base, &p, depth + 1, out);
                }
            } else if p.extension().and_then(|x| x.to_str()).is_some_and(|x| ["png", "jpg", "jpeg", "wav", "ogg", "mp3"].contains(&x.to_lowercase().as_str())) {
                out.push(rel(base, &p));
            }
        }
    }
    let mut out = vec![];
    walk(base, base, 1, &mut out);
    out.sort();
    out
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _f: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.kids.retain_mut(|c| matches!(c.try_wait(), Ok(None)));
        if self.p.scenes.is_empty() {
            self.p.scenes.push(new_scene("Escena"));
        }
        self.sc = self.sc.min(self.p.scenes.len() - 1);

        // Los primeros cuadros se repintan solos: las ventanas miden su tamaño en el primero.
        if ctx.cumulative_frame_nr() < 6 {
            ctx.request_repaint();
        }
        if self.aplicado != Some((self.facil, self.grande)) {
            tema::tamanos(&ctx, self.facil);
            ctx.set_zoom_factor(if self.grande { 1.25 } else { 1. });
            self.aplicado = Some((self.facil, self.grande));
        }

        // Atajos
        let typing = ctx.egui_wants_keyboard_input();
        let modal = self.ed.is_some() || self.exp.is_some() || self.confirm.is_some() || self.sel.is_some() || self.rec.is_some() || self.asis.is_some();
        let free = !typing && !modal;
        let scene_keys = free && self.tab == Tab::Escena;
        let (cmd, cs) = (Modifiers::COMMAND, Modifiers::COMMAND | Modifiers::SHIFT);
        let k = ctx.input_mut(|i| {
            let save_as = i.consume_key(cs, Key::S);
            let save = i.consume_key(cmd, Key::S);
            let redo = free && (i.consume_key(cs, Key::Z) || i.consume_key(cmd, Key::Y));
            let undo = free && i.consume_key(cmd, Key::Z);
            let dup = scene_keys && i.consume_key(cmd, Key::D);
            let del = scene_keys && i.consume_key(Modifiers::NONE, Key::Delete);
            let esc = scene_keys && i.consume_key(Modifiers::NONE, Key::Escape);
            for (key, t) in [(Key::Num1, Tab::Escena), (Key::Num2, Tab::Eventos), (Key::Num3, Tab::Funciones), (Key::Num4, Tab::Variables)] {
                if !modal && i.consume_key(cmd, key) {
                    self.tab = t;
                }
            }
            let mut nudge = Vec2::ZERO;
            if scene_keys && self.sel_inst.is_some() {
                for (key, d) in [(Key::ArrowLeft, Vec2::new(-1., 0.)), (Key::ArrowRight, Vec2::new(1., 0.)), (Key::ArrowUp, Vec2::new(0., -1.)), (Key::ArrowDown, Vec2::new(0., 1.))] {
                    nudge += d * self.grid * i.count_and_consume_key(Modifiers::SHIFT, key) as f32;
                    nudge += d * i.count_and_consume_key(Modifiers::NONE, key) as f32;
                }
            }
            (save_as, save, i.consume_key(Modifiers::NONE, Key::F5) && !modal, undo, redo, dup, del, esc, nudge)
        });
        let (save_as, save, run, undo, redo, dup, del, esc, nudge) = k;
        if save_as {
            self.save(true);
        } else if save {
            self.save(false);
        }
        if run {
            self.run();
        }
        if undo {
            self.undo();
        }
        if redo {
            self.redo();
        }
        if dup {
            self.duplicate_instance();
        }
        if del {
            self.delete_instance();
        }
        if esc {
            self.sel_inst = None;
        }
        if nudge != Vec2::ZERO {
            self.nudge(nudge);
        }

        let side = Frame::new().fill(PANEL).inner_margin(12);
        egui::Panel::top("barra").exact_size(44.).frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(8, 0))).show(ui, |ui| self.top_bar(ui));
        egui::Panel::bottom("estado").exact_size(28.).frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(12, 0))).show(ui, |ui| self.status(ui));
        // En Mis bloques (modo fácil) el panel de la escena no se usa: más lugar para el bloque.
        if !(self.facil && self.tab == Tab::Funciones) {
            egui::Panel::left("izq").default_size(290.).size_range(240.0..=480.).frame(side)
                .show(ui, |ui| egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| self.left(ui)));
        }
        if self.tab == Tab::Escena {
            egui::Panel::right("der").default_size(300.).size_range(240.0..=480.).frame(side)
                .show(ui, |ui| egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| self.right(ui)));
        }
        let margin = match self.tab {
            Tab::Escena | Tab::Funciones => Margin::ZERO,
            _ => Margin::same(12),
        };
        egui::CentralPanel::default_margins().frame(Frame::new().fill(BG).inner_margin(margin)).show(ui, |ui| match self.tab {
            Tab::Escena => self.canvas(ui),
            Tab::Eventos => self.sheet(ui, Target::Scene(self.sc)),
            Tab::Funciones => self.funcs(ui),
            Tab::Variables => self.variables(ui),
        });

        if let Some(mut ed) = self.ed.take() {
            let l = self.lists();
            match eventos::window(&ctx, &mut ed, &l) {
                None => self.ed = Some(ed),
                Some(Res::Cancel) => {}
                Some(r) => {
                    self.snap_hist();
                    if let Some(evs) = self.events_mut(ed.target) {
                        eventos::apply(evs, &ed, r == Res::Del);
                    }
                    if r == Res::Del {
                        self.say(Nivel::Info, format!("Eliminada la instrucción · {UNDO_HINT}"));
                    }
                }
            }
        }
        self.modales_kid(&ctx);
        self.confirm_ui(&ctx);
        self.export_ui(&ctx);

        // Historial: un paso por interacción terminada (sin botón presionado ni texto con foco).
        let now = self.json();
        if !ctx.input(|i| i.pointer.any_down()) && !ctx.egui_wants_keyboard_input() {
            self.hist.commit(&now);
        }
        // Revisión del proyecto (solo cuando cambió).
        if self.facil && now != self.probs_de {
            self.probs = nicostation::validate::validate(&self.p, Some(&self.base_dir()));
            self.probs_de = now.clone();
        }
        self.now = now;

        let file = self.path.as_ref().and_then(|p| p.file_name()).map_or("sin título".into(), |f| f.to_string_lossy().to_string());
        let title = format!("{}{file} - {} · NicoStation", if self.dirty() { "● " } else { "" }, self.p.name);
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }
}

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let [flag, src, dst] = args.as_slice() {
        if flag == "--resave" {
            let p = load(Path::new(src)).unwrap_or_else(|e| panic!("{src}: {e}"));
            std::fs::write(dst, serde_json::to_string_pretty(&p).unwrap()).expect("no se pudo escribir");
            return Ok(());
        }
    }
    let mut app = App::new();
    let (mut abrir, mut escena, mut file) = (None, None, None);
    let mut it_ = args.iter();
    while let Some(a) = it_.next() {
        match a.as_str() {
            "--experto" => app.facil = false,
            "--grande" => app.grande = true,
            "--abrir" => abrir = it_.next().cloned(),
            "--escena" => escena = it_.next().cloned(),
            _ => file = Some(a.clone()),
        }
    }
    if let Some(a) = file.as_ref() {
        let path = PathBuf::from(a);
        match path.exists().then(|| load(&path)) {
            Some(Ok(p)) => app.set_project(p, Some(path)),
            Some(Err(e)) => app.say(Nivel::Error, format!("No se pudo abrir {a}: {e}")),
            None => {
                app.path = Some(path);
                app.say(Nivel::Info, format!("Proyecto nuevo: {a}"));
            }
        }
    }
    if let Some(e) = escena {
        app.sc = app.p.scenes.iter().position(|s| s.name == e).unwrap_or(0);
    }
    if let Some(a) = abrir {
        app.abrir(&a);
    }
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1360., 820.]).with_min_inner_size([960., 600.]).with_title("NicoStation"),
        ..Default::default()
    };
    eframe::run_native("NicoStation", opts, Box::new(|cc| {
        tema::aplicar(&cc.egui_ctx);
        Ok(Box::new(app))
    }))
}
