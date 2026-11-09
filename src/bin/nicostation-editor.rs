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
//! `ficha` (abre la ventanita de la primera ficha de objeto), `arreglar` (muestra el primer problema),
//! `mapa` (elige el primer mapa de baldosas de la escena, o crea uno con las baldosas de ejemplo, y pinta un paisaje de prueba),
//! `mapa-nuevo` (como «+ Mapa»: pregunta qué baldosas usar), `sprite-nuevo` (como «+ Sprite»: dibujar o elegir imágenes),
//! `dibujo` (editor de dibujo con una animación de prueba de 3 cuadros), `cebolla` (lo mismo, en el cuadro 2 con piel de cebolla),
//! `baldosas` (editor de dibujo con la imagen de baldosas del mapa y su rejilla),
//! `dibujo-guardado` (dibuja la animación de prueba y la guarda en `img/` como si se apretara «Guardar dibujo»),
//! `bienvenida` (pantalla «¿Qué juego quieres hacer hoy?»), `nuevo-plantilla` (bienvenida con la ventanita de nombre y
//! carpeta de la primera plantilla abierta; Enter crea el juego), `misiones` (abre el panel de misiones),
//! `mision-activa` (empieza la primera misión del juego y marca «¡Listo!» en su primer paso si es de los que se marcan a mano),
//! `mision-cumplida` (lo mismo y aprieta «¡Listo!» en todos los pasos manuales: si los automáticos ya se cumplen, muestra el festejo),
//! `vista` («Jugar aquí» con el héroe caminando a la derecha 2,5 s), `vista-pausa` (lo mismo, en pausa y con el héroe elegido),
//! `depurador` (corriendo con el héroe elegido), `registro` / `rendimiento` (esas pestañas del depurador; rendimiento con «ver toda
//! la escena»), `vista-abajo` (la vista previa abajo, con la pestaña Eventos), `recarga` (cambia el fondo y agrega una moneda mientras
//! juega: se ve «Cambios aplicados»), `jugar` (sin ensayo, con el teclado en el juego: probar con `--tecla Space`),
//! `si-no` (agrega un «si no, cuando» y un «si no» debajo del primer evento),
//! `nodos` (Eventos en vista Nodos: la hoja de la escena como nodos, editable), `nodos-borrador` (lo mismo, con una acción
//! nueva sin completar metida en un camino: queda como borrador y la hoja no cambia), `nodos-eventos` (con la hoja al lado),
//! `nodos-convertido` (pasa la hoja a un grafo aparte y lo muestra), `nodos-paleta` (lo mismo, con el buscador de nodos
//! abierto), `nodos-error` (lo mismo, con dos nodos rotos a propósito para ver los errores marcados).
//! Estudio 2D: `particulas` (emisor nuevo con el editor de partículas), `fondo` (fondo que se repite y se mueve),
//! `origen` / `mascara` (editor de origen o de máscara del primer sprite), `ajustes` (Ajustes del juego), `recursos`
//! (explorador de archivos), `multi` (varias instancias elegidas, con manijas y alinear), `buscar` (buscar en eventos),
//! `lista` (cajitas con listas y estructuras abiertas), `perfilador` (vista previa en pausa con el perfilador; mejor con `--experto`).
//!
//! En la escena: Shift+clic o un rectángulo eligen varias; Ctrl+C / Ctrl+V copian y pegan (también en otra escena),
//! Ctrl+D duplica, Ctrl+A elige todo, Supr borra. En Eventos, Ctrl+F busca. Arrastrar archivos a la ventana los importa.
//!
//! Vista previa: F6 empieza / pausa, F7 avanza un cuadro. Con el juego enfocado (clic en él) el teclado va al juego.
//!
//! `--captura salida.png` dibuja el editor sin ventana (1360×820, wgpu fuera de pantalla) y guarda la imagen;
//! con `--tecla Enter` (u otra tecla de egui) la aprieta a mitad de la captura, y `--guion "clic 400 300; tecla Delete"`
//! simula mouse y teclado antes (ver `captura.rs`).
//!
//! Sin proyecto en la línea de comandos se muestra la bienvenida. `NICOSTATION_JUEGOS` cambia la carpeta donde se
//! proponen los juegos nuevos (por defecto `~/Juegos`); los juegos recientes van en `$XDG_CONFIG_HOME/nicostation/recientes.json`.
//! Las opciones que crean imágenes necesitan un proyecto con archivo (escriben en su carpeta `img/`).

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
#[path = "../editor/mapa.rs"]
mod mapa;
#[path = "../editor/dibujo.rs"]
mod dibujo;
#[path = "../editor/inicio.rs"]
mod inicio;
#[path = "../editor/misiones.rs"]
mod misiones;
#[path = "../editor/captura.rs"]
mod captura;
#[path = "../editor/vista.rs"]
mod vista;
#[path = "../editor/depurador.rs"]
mod depurador;
#[path = "../editor/nodos.rs"]
mod nodos;
#[path = "../editor/seleccion.rs"]
mod seleccion;
#[path = "../editor/recursos.rs"]
mod recursos;
#[path = "../editor/origen.rs"]
mod origen;
#[path = "../editor/lista.rs"]
mod lista;
#[path = "../editor/ajustes.rs"]
mod ajustes;
#[path = "../editor/particulas.rs"]
mod particulas;
#[path = "../editor/buscar.rs"]
mod buscar;

use eframe::egui::{
    self, Align, Align2, AtomExt, Button, Color32, DragValue, Frame, Key, Layout, Margin, Modifiers, PointerButton, Pos2, Rect,
    RichText, Sense, Stroke, TextEdit, Ui, Vec2,
};
use eventos::{InstrEdit, Res, Salida, Target};
use seleccion::Arrastre;
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
    Recursos,
}

#[derive(PartialEq, Clone, Copy)]
enum Nivel {
    Info,
    Ok,
    Warn,
    Error,
}

const BEHAVIORS: &[&str] = &[
    "platformer", "platform", "top_down", "bullet", "rotate", "physics", "pathfinder", "obstacle", "destroy_outside", "bound_to_layout", "screen_wrap",
    "draggable",
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
    // ---- mapas y dibujo ----
    pintor: mapa::Pintor,
    dib: Option<dibujo::Dibujo>,
    /// Objeto recién creado que espera una elección (dibujar, elegir imágenes, baldosas).
    nuevo: Option<Nuevo>,
    // ---- bienvenida y misiones ----
    /// Pantalla de bienvenida (tapa todo el editor mientras está).
    inicio: Option<inicio::Inicio>,
    mis: misiones::Misiones,
    /// JSON con que se revisaron las misiones por última vez.
    mis_json: String,
    /// Misiones de otros juegos abiertos en esta sesión (por ruta).
    mis_sesion: HashMap<PathBuf, misiones::Misiones>,
    /// Hora de egui del último cuadro.
    t: f64,
    // ---- vista previa ----
    /// Juego corriendo dentro del editor («Jugar aquí»).
    vista: Option<vista::Sesion>,
    vprefs: vista::Prefs,
    /// Partida guardada de la vista previa (sobrevive a detener y volver a jugar).
    vista_partida: Vars,
    /// `--abrir vista…` pendiente (se aplica en el primer cuadro).
    vista_abrir: Option<String>,
    // ---- nodos ----
    /// Vista «Nodos» de Eventos y Mis bloques (lienzo, selección, buscador).
    nv: nodos::Vista,
    // ---- estudio 2D ----
    /// Más instancias elegidas además de `sel_inst` (selección múltiple).
    sel_mas: Vec<usize>,
    /// Lo que se arrastra en el lienzo (mover, escalar, girar, rectángulo de selección).
    arrastre: Option<seleccion::Arrastre>,
    /// Instancias copiadas (Ctrl+C).
    clip: Option<seleccion::Portapapeles>,
    /// Pegadas seguidas (cada una un poco más corrida).
    pegadas: u32,
    explorador: recursos::Explorador,
    /// Diálogo de ajustes del juego abierto.
    ajustes: bool,
    /// Vista previa de cada emisor de partículas (por objeto).
    psim: HashMap<String, particulas::Sim>,
    origen_st: origen::Estado,
    buscador: buscar::Buscador,
    /// Fuentes del proyecto registradas en egui: ruta → lista para usar (se aplica al cuadro siguiente).
    fuentes: vista::Fuentes,
    /// Abrir la sección de origen y máscara en el próximo cuadro (`--abrir origen`).
    abrir_origen: bool,
    /// Segundos desde el cuadro anterior.
    dt: f32,
}

/// Pregunta que sigue a «+ Sprite» y «+ Mapa».
enum Nuevo {
    Sprite(String),
    Mapa(String),
}

/// Pedidos del panel de propiedades que necesitan al editor.
enum PropAct {
    /// Abrir el editor de dibujo en la animación `n`.
    Dibujar(usize),
    DibujarBaldosas,
    CrearBaldosas,
    /// Elegir (e importar si está afuera) un archivo para un campo del objeto.
    Importar(recursos::Tipo, Campo),
}

/// Campo del objeto que recibe un archivo elegido.
#[derive(Clone, Copy)]
enum Campo {
    Fondo,
    Particulas,
    Fuente,
}

/// Lo que necesita el panel de propiedades de un objeto.
struct PropCx<'a> {
    base: &'a Path,
    files: &'a [String],
    /// Imágenes ya cargadas de este objeto (y sus variantes «gris:» / «blanco:» para el efecto).
    tex: &'a HashMap<String, egui::TextureHandle>,
    usos: Option<&'a HashMap<String, usize>>,
    facil: bool,
    dt: f32,
    t: f32,
    sim: &'a mut particulas::Sim,
    origen: &'a mut origen::Estado,
    /// Tamaño de la pantalla del juego.
    pantalla: Vec2,
    /// Abrir la sección de origen y máscara (para `--abrir origen`).
    abrir_origen: bool,
}

/// Efectos visuales: (valor, nombre fácil, nombre experto, ícono, explicación).
const EFECTOS: [(&str, &str, &str, &str, &str); 5] = [
    ("", "Ninguno", "Ninguno", ic::X_CIRCLE, "Se ve normal."),
    ("flash", "Destello", "Destello", ic::SUN, "Se pone blanco, como cuando lo golpean."),
    ("outline", "Borde", "Silueta", ic::POLYGON, "Un borde blanco alrededor (para resaltarlo)."),
    ("grayscale", "Gris", "Escala de grises", ic::CIRCLE_HALF, "Sin colores, en gris (congelado, apagado)."),
    ("wave", "Ondas", "Ondas", ic::WAVES, "Se ondula como si estuviera bajo el agua."),
];

fn new_scene(name: &str) -> Scene {
    Scene { name: name.into(), background: [30, 30, 40], layers: vec![Layer { name: String::new(), visible: true, fixed: false, parallax: [1., 1.] }], ..Default::default() }
}

fn new_project() -> Project {
    Project {
        name: "Mi juego".into(), width: 800., height: 600., first_scene: "Escena".into(), globals: Vars::new(),
        objects: vec![], scenes: vec![new_scene("Escena")], functions: vec![], plantilla: String::new(), settings: Default::default(), version: FORMAT_VERSION }
}

/// Lee y migra un proyecto (como el reproductor).
fn load(path: &Path) -> Result<Project, String> {
    parse_project(&std::fs::read(path).map_err(|e| e.to_string())?)
}

/// Abre un proyecto para editarlo: migra formatos viejos y avisa si viene de una versión más nueva.
fn abrir_proyecto(app: &mut App, f: PathBuf) -> bool {
    match ajustes::cargar(&f) {
        Ok((p, aviso)) => {
            app.set_project(p, Some(f));
            if let Some(a) = aviso {
                app.say(Nivel::Warn, a);
            }
            true
        }
        Err(e) => {
            app.say(Nivel::Error, format!("No se pudo abrir {}: {e}", f.display()));
            false
        }
    }
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
    if s.trim_start().starts_with(['[', '{']) {
        if let Ok(v @ (Value::List(_) | Value::Map(_))) = serde_json::from_str::<Value>(s) {
            return v;
        }
    }
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
            if matches!(v, Value::List(_) | Value::Map(_)) {
                // Listas y estructuras: editor por filas (con JSON a mano si se quiere).
                ui.vertical(|ui| {
                    ui.set_max_width(320.);
                    lista::editor(ui, (id, k.as_str()), v, true);
                });
            } else {
                let mut s = v.text();
                if ui.add(TextEdit::singleline(&mut s).desired_width(140.)).on_hover_text("Número, texto, true/false, o [lista] / {estructura} en JSON").changed() {
                    *v = parse_val(&s);
                }
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
        ui.menu_button(it(ic::CARET_DOWN, "Lista o estructura"), |ui| {
            menu_spacing(ui);
            let n = if nb.trim().is_empty() { (1..).map(|k| format!("lista{k}")).find(|n| !vars.contains_key(n)).unwrap() } else { nb.trim().to_string() };
            if ui.button(it(ic::LIST, "Nueva lista")).clicked() && !vars.contains_key(&n) {
                vars.insert(n.clone(), Value::List(vec![]));
                nb.clear();
                ui.close();
            }
            if ui.button(it(ic::BRACES, "Nueva estructura")).clicked() && !vars.contains_key(&n) {
                vars.insert(n, Value::Map(Default::default()));
                nb.clear();
                ui.close();
            }
        });
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
            s.graphs.iter_mut().chain([&mut s.node_drafts]).for_each(|g| nodos::ren_grafo(g, t, old, new, ren_expr));
            if t == P::Object {
                s.instances.iter_mut().filter(|n| n.obj == old).for_each(|n| n.obj = new.into());
                s.groups.values_mut().flatten().filter(|o| *o == old).for_each(|o| *o = new.into());
            }
        }
    }
    if scene.is_none() {
        p.functions.iter_mut().for_each(|f| ren_evs(&mut f.events, t, old, new));
        p.functions.iter_mut().flat_map(|f| f.graph.iter_mut().chain([&mut f.node_drafts])).for_each(|g| nodos::ren_grafo(g, t, old, new, ren_expr));
    }
}

fn count_events(evs: &[Event]) -> usize {
    evs.iter().map(|e| 1 + count_events(&e.sub)).sum()
}

fn beh_label(t: &str, facil: bool) -> &str {
    match (t, facil) {
        ("platformer", _) => "Personaje de plataformas",
        ("platform", _) => "Plataforma",
        ("top_down", true) => "Caminar visto desde arriba",
        ("top_down", false) => "Movimiento vista superior",
        ("bullet", _) => "Proyectil",
        ("rotate", true) => "Girar solo",
        ("rotate", false) => "Rotación",
        ("physics", true) => "Física (cae, rebota y empuja)",
        ("physics", false) => "Física 2D",
        ("pathfinder", true) => "Buscar el camino",
        ("pathfinder", false) => "Buscar caminos (A*)",
        ("obstacle", true) => "Obstáculo (lo esquivan)",
        ("obstacle", false) => "Obstáculo para caminos",
        ("destroy_outside", _) => "Destruir fuera de pantalla",
        ("bound_to_layout", _) => "Limitar a la escena",
        ("screen_wrap", _) => "Envolver pantalla",
        ("draggable", _) => "Arrastrable",
        _ => t,
    }
}

fn beh_help(t: &str) -> &'static str {
    match t {
        "physics" => "Se mueve como en la vida real: la gravedad lo hace caer, choca, rebota y se apila con otros objetos con física.",
        "pathfinder" => "Va hasta un punto esquivando los objetos que tienen «Obstáculo». Se le dice adónde ir con una acción.",
        "obstacle" => "Los objetos que buscan caminos no pasan por encima de este.",
        "platformer" => "Corre y salta sobre las plataformas con las flechas.",
        "platform" => "Suelo o pared para los personajes de plataformas.",
        "top_down" => "Se mueve en 4 u 8 direcciones con las flechas.",
        "bullet" => "Avanza solo hacia donde mira.",
        _ => "",
    }
}

/// Etiqueta y explicación de cada campo de comportamiento.
fn field_label(k: &str, facil: bool) -> (&str, &'static str) {
    match (k, facil) {
        ("max_speed", _) => ("Velocidad máx.", ""),
        ("acceleration", _) => ("Aceleración", ""),
        ("deceleration", _) => ("Desaceleración", ""),
        ("gravity", _) => ("Gravedad", ""),
        ("max_falling_speed", _) => ("Caída máx.", ""),
        ("jump_speed", _) => ("Salto", ""),
        ("jump_sustain_time", _) => ("Sostén salto (s)", ""),
        ("jumpthru", _) => ("Atravesable desde abajo", ""),
        ("allow_diagonals", _) => ("Diagonales", ""),
        ("rotate", _) => ("Girar al moverse", "Mira hacia donde va"),
        ("speed", _) => ("Velocidad", ""),
        ("body", true) => ("Se mueve", ""),
        ("body", false) => ("Cuerpo", "dynamic: lo mueven las fuerzas · static: no se mueve · kinematic: lo mueven los eventos"),
        ("density", true) => ("Peso", "Más alto = más pesado: empuja más y cuesta moverlo"),
        ("density", false) => ("Densidad", "Masa por área"),
        ("friction", true) => ("Roce", "0 = resbala como hielo, 1 = no resbala"),
        ("friction", false) => ("Fricción", "0 a 1"),
        ("restitution", true) => ("Rebote", "0 = no rebota, 1 = rebota igual de alto"),
        ("restitution", false) => ("Restitución", "0 = sin rebote, 1 = elástico"),
        ("fixed_rotation", true) => ("No gira", "Nunca se da vuelta aunque choque"),
        ("fixed_rotation", false) => ("Rotación fija", "No gira por los choques"),
        ("gravity_scale", true) => ("Gravedad propia", "Cuánto le afecta la gravedad: 1 = normal, 0 = flota, 2 = cae el doble"),
        ("gravity_scale", false) => ("Escala de gravedad", ""),
        ("shape", true) => ("Forma", "La forma con la que choca"),
        ("shape", false) => ("Forma", ""),
        ("bullet", true) => ("Muy rápido", "Para cosas muy veloces que podrían atravesar paredes"),
        ("bullet", false) => ("Detección continua", "Para objetos muy rápidos (CCD)"),
        ("cell", true) => ("Paso", "Tamaño de la cuadrícula para buscar el camino"),
        ("cell", false) => ("Celda", "Tamaño de celda de la grilla A* en px"),
        ("diagonals", _) => ("Diagonales", "Puede caminar en diagonal"),
        ("grabbable", true) => ("Se agarra del borde", "Los personajes pueden colgarse de su borde"),
        ("grabbable", false) => ("Borde agarrable", "Los Platformer con «agarrar bordes» se cuelgan de aquí"),
        ("ladder", _) => ("Escalera", "Se sube y se baja con las flechas"),
        ("ladder_speed", _) => ("En escalera", "Velocidad al subir o bajar escaleras"),
        ("can_grab_platforms", true) => ("Se cuelga de bordes", ""),
        ("can_grab_platforms", false) => ("Agarrar bordes", "Se cuelga de las plataformas con «Borde agarrable»"),
        ("can_go_down_from_jumpthru", true) => ("Baja con flecha", "Flecha abajo baja de las plataformas que se atraviesan"),
        ("can_go_down_from_jumpthru", false) => ("Bajar de atravesables", "Flecha abajo baja de las plataformas atravesables"),
        ("slope_max_angle", true) => ("Rampa máxima", "Hasta qué ángulo puede subir"),
        ("slope_max_angle", false) => ("Pendiente máx. (°)", ""),
        _ => (k, ""),
    }
}

/// Opciones fijas de algunos campos de texto: (valor, fácil, experto).
fn field_opts(k: &str) -> &'static [(&'static str, &'static str, &'static str)] {
    match k {
        "body" => &[("dynamic", "Cae y rebota", "Dinámico"), ("static", "Quieto", "Estático"), ("kinematic", "Con eventos", "Cinemático")],
        "shape" => &[("box", "Caja", "Caja"), ("circle", "Círculo", "Círculo")],
        _ => &[],
    }
}

/// Devuelve true si hay que quitar el comportamiento.
fn behavior_ui(ui: &mut Ui, b: &mut Behavior, i: usize, facil: bool) -> bool {
    let Ok(serde_json::Value::Object(mut m)) = serde_json::to_value(&*b) else { return false };
    let (mut ch, mut del) = (false, false);
    let t = m.get("type").and_then(|t| t.as_str()).unwrap_or("?").to_string();
    card(ui, |ui| {
        ui.horizontal(|ui| {
            let r = ui.label(RichText::new(beh_label(&t, facil)).font(sb(14.)));
            if !beh_help(&t).is_empty() {
                r.on_hover_text(beh_help(&t));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| del = ib(ui, ic::TRASH, "Quitar comportamiento").clicked());
        });
        if facil && !beh_help(&t).is_empty() {
            ui.label(RichText::new(beh_help(&t)).small().color(TEXT_MUTED));
        }
        let fields: Vec<_> = m.iter_mut().filter(|(k, _)| *k != "type").collect();
        if fields.is_empty() {
            return;
        }
        egui::Grid::new(("beh", i)).num_columns(2).spacing([12., 6.]).min_col_width(LBL_W).show(ui, |ui| {
            for (k, v) in fields {
                let (nombre, ayuda) = field_label(k, facil);
                let etiqueta = |ui: &mut Ui| {
                    let r = ui.label(muted(nombre));
                    if !ayuda.is_empty() {
                        r.on_hover_text(ayuda);
                    }
                };
                match v {
                    serde_json::Value::Bool(x) => {
                        etiqueta(ui);
                        ch |= ui.checkbox(x, "").changed();
                    }
                    serde_json::Value::Number(n) => {
                        etiqueta(ui);
                        let mut f = n.as_f64().unwrap_or(0.);
                        let r = match k.as_str() {
                            "friction" | "restitution" => ui.add(egui::Slider::new(&mut f, 0.0..=1.0).max_decimals(2)),
                            "density" | "gravity_scale" => ui.add(DragValue::new(&mut f).speed(0.05).range(0.0..=100.).max_decimals(2)),
                            _ => ui.add(DragValue::new(&mut f).speed(1.)),
                        };
                        if r.changed() {
                            *v = json!(f);
                            ch = true;
                        }
                    }
                    serde_json::Value::String(sv) if !field_opts(k).is_empty() => {
                        etiqueta(ui);
                        seg(ui, |ui| {
                            for (val, kid, exp) in field_opts(k) {
                                if seg_btn(ui, sv == val, if facil { *kid } else { *exp }).clicked() && sv != val {
                                    *sv = val.to_string();
                                    ch = true;
                                }
                            }
                        });
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

/// Propiedades de un mapa de baldosas.
fn tilemap_props(ui: &mut Ui, o: &mut ObjectDef, base: &Path, files: &[String], tex: Option<&egui::TextureHandle>, kid: bool) -> Option<PropAct> {
    let Kind::Tilemap { tileset, tile, cols, rows, data, pass } = &mut o.kind else { return None };
    let mut act = None;
    if kid {
        ui.label(RichText::new(mapa::TEXTO_NINOS).color(TEXT_SOFT));
        ui.add_space(4.);
    }
    grid("mapprops").show(ui, |ui| {
        lbl(ui, "Baldosas");
        let imgs: Vec<&String> = files.iter().filter(|f| [".png", ".jpg", ".jpeg"].iter().any(|x| f.to_lowercase().ends_with(x))).collect();
        let shown = if tileset.is_empty() { "(sin imagen)".to_string() } else { tileset.clone() };
        egui::ComboBox::from_id_salt("tileset").selected_text(shown).width(160.).show_ui(ui, |ui| {
            for f in imgs {
                if ui.selectable_label(tileset == f, f.as_str()).clicked() {
                    *tileset = f.clone();
                }
            }
            ui.separator();
            if ui.button(it(ic::FOLDER_OPEN, "Elegir archivo…")).clicked() {
                if let Some(f) = rfd::FileDialog::new().add_filter("Imagen", &["png", "jpg", "jpeg"]).set_directory(base).pick_file() {
                    *tileset = rel(base, &f);
                }
            }
            if ui.button(it(ic::SPARKLE, "Crear baldosas de ejemplo")).clicked() {
                act = Some(PropAct::CrearBaldosas);
            }
        });
        ui.end_row();
        lbl(ui, "Baldosa");
        ui.add(DragValue::new(tile).range(4..=256).suffix(" px")).on_hover_text("Tamaño de cada baldosa (cuadrada)");
        ui.end_row();
        let (mut c, mut r) = (*cols, *rows);
        lbl(ui, "Columnas");
        let ch_c = ui.add(DragValue::new(&mut c).range(1..=1000)).changed();
        ui.end_row();
        lbl(ui, "Filas");
        let ch_r = ui.add(DragValue::new(&mut r).range(1..=1000)).changed();
        ui.end_row();
        if ch_c || ch_r {
            *data = mapa::redimensionar(data, *cols, *rows, c, r);
            (*cols, *rows) = (c, r);
        }
    });
    let n = mapa::cantidad(tex, *tile);
    let info = match tex {
        Some(t) => format!("{n} baldosas en la imagen ({}×{} px) · {} de decoración", t.size()[0], t.size()[1], pass.len()),
        None if tileset.is_empty() => "Sin imagen: se ven cuadrados de colores.".into(),
        None => format!("No se encontró «{tileset}»: se ven cuadrados de colores."),
    };
    ui.label(muted(info));
    ui.horizontal_wrapped(|ui| {
        if ui.button(it(ic::PALETTE, "Dibujar baldosas")).on_hover_text("Abre el editor de dibujo con la imagen de baldosas").clicked() {
            act = Some(PropAct::DibujarBaldosas);
        }
        if tileset.is_empty() && ui.button(it(ic::SPARKLE, "Crear baldosas de ejemplo")).clicked() {
            act = Some(PropAct::CrearBaldosas);
        }
    });
    ui.label(muted("Selecciona el mapa en el lienzo para pintarlo."));
    act
}

/// Combo para elegir un archivo del proyecto de un tipo, con «(ninguno)» y un botón para buscar o importar.
/// Devuelve true si piden buscar afuera.
fn archivo_combo(ui: &mut Ui, id: &str, v: &mut String, files: &[String], tipo: recursos::Tipo, vacio: &str) -> bool {
    let lista: Vec<&String> = files.iter().filter(|f| recursos::Tipo::de(f) == Some(tipo)).collect();
    let shown = if v.is_empty() { vacio.to_string() } else { v.rsplit('/').next().unwrap_or(v).to_string() };
    let mut buscar = false;
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt(id).selected_text(shown).width(140.).show_ui(ui, |ui| {
            if ui.selectable_label(v.is_empty(), vacio).clicked() {
                v.clear();
            }
            for f in lista {
                if ui.selectable_label(v == f, f.as_str()).clicked() {
                    *v = f.clone();
                }
            }
        }).response.on_hover_text(v.as_str());
        buscar = ib(ui, ic::FOLDER_OPEN, "Elegir o importar un archivo").clicked();
    });
    buscar
}

/// Propiedades según el tipo de objeto, animaciones, comportamientos y variables.
fn obj_props(ui: &mut Ui, o: &mut ObjectDef, nb: &mut String, cx: &mut PropCx) -> Option<PropAct> {
    let facil = cx.facil;
    let ts = match &o.kind {
        Kind::Tilemap { tileset, .. } => cx.tex.get(tileset),
        _ => None,
    };
    let mut act = tilemap_props(ui, o, cx.base, cx.files, ts, cx.usos.is_some());
    grid("objprops").show(ui, |ui| match &mut o.kind {
        Kind::Particles(_) => {}
        Kind::Background { image, w, h, scroll } => {
            lbl(ui, "Imagen");
            if archivo_combo(ui, "fondoimg", image, cx.files, recursos::Tipo::Imagen, "(sin imagen)") {
                act = Some(PropAct::Importar(recursos::Tipo::Imagen, Campo::Fondo));
            }
            ui.end_row();
            lbl(ui, "Tamaño");
            ui.horizontal(|ui| {
                ui.add(DragValue::new(w).range(1..=100000).suffix(" px"));
                ui.label(muted("×"));
                ui.add(DragValue::new(h).range(1..=100000).suffix(" px"));
                if ib(ui, ic::FRAME_CORNERS, "Del tamaño de la pantalla del juego").clicked() {
                    (*w, *h) = (cx.pantalla.x, cx.pantalla.y);
                }
            });
            ui.end_row();
            lbl(ui, if facil { "Se mueve solo" } else { "Desplazamiento" });
            ui.horizontal(|ui| {
                ui.add(DragValue::new(&mut scroll[0]).speed(1.).range(-5000.0..=5000.).prefix("x ")).on_hover_text("Pixeles por segundo hacia la derecha (negativo: izquierda)");
                ui.add(DragValue::new(&mut scroll[1]).speed(1.).range(-5000.0..=5000.).prefix("y ")).on_hover_text("Pixeles por segundo hacia abajo (negativo: arriba)");
            });
            ui.end_row();
        }
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
        Kind::Text { text, size, color, font, align, wrap } => {
            lbl(ui, "Texto");
            ui.add(TextEdit::multiline(text).desired_width(170.).desired_rows(1)).on_hover_text("Puedes poner {expresiones} que se actualizan solas. Enter hace otro renglón.");
            ui.end_row();
            lbl(ui, "Tamaño");
            ui.add(DragValue::new(size).range(4..=400));
            ui.end_row();
            lbl(ui, "Color");
            ui.color_edit_button_srgb(color);
            ui.end_row();
            lbl(ui, if facil { "Letra" } else { "Fuente" });
            if archivo_combo(ui, "fuente", font, cx.files, recursos::Tipo::Fuente, "(la del juego)") {
                act = Some(PropAct::Importar(recursos::Tipo::Fuente, Campo::Fuente));
            }
            ui.end_row();
            lbl(ui, if facil { "Alinear" } else { "Alineación" });
            seg(ui, |ui| {
                for (v, icon, tip) in [("left", ic::TEXT_ALIGN_LEFT, "A la izquierda"), ("center", ic::TEXT_ALIGN_CENTER, "Al centro"), ("right", ic::TEXT_ALIGN_RIGHT, "A la derecha")] {
                    if seg_btn(ui, align == v, RichText::new(icon).size(16.)).on_hover_text(tip).clicked() {
                        *align = v.into();
                    }
                }
            });
            ui.end_row();
            lbl(ui, if facil { "Ancho del renglón" } else { "Ancho de línea" });
            ui.horizontal(|ui| {
                ui.add(DragValue::new(wrap).range(0..=8192).suffix(" px")).on_hover_text("Si el texto es más largo, sigue en el renglón de abajo. 0 = todo en un renglón");
                if *wrap == 0. {
                    ui.label(muted("un renglón"));
                }
            });
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
        Kind::Tilemap { .. } => {}
        Kind::Mesh { shape, model, size, color, metallic, roughness, texture, emission } => {
            lbl(ui, "Forma");
            egui::ComboBox::from_id_salt("forma3d").selected_text(format!("{shape:?}")).show_ui(ui, |ui| {
                for s in [Shape3::Cube, Shape3::Sphere, Shape3::Plane, Shape3::Capsule, Shape3::Cylinder, Shape3::Cone, Shape3::Model] {
                    ui.selectable_value(shape, s, format!("{s:?}"));
                }
            });
            ui.end_row();
            if *shape == Shape3::Model {
                lbl(ui, "Modelo glTF");
                ui.add(TextEdit::singleline(model).desired_width(150.).hint_text("modelos/casa.glb"));
                ui.end_row();
            }
            lbl(ui, "Tamaño (m)");
            ui.horizontal(|ui| {
                for v in size.iter_mut() {
                    ui.add(DragValue::new(v).speed(0.05).range(0.01..=1000.));
                }
            });
            ui.end_row();
            lbl(ui, "Color");
            ui.color_edit_button_srgb(color);
            ui.end_row();
            lbl(ui, "Metálico");
            ui.add(egui::Slider::new(metallic, 0.0..=1.0));
            ui.end_row();
            lbl(ui, "Rugosidad");
            ui.add(egui::Slider::new(roughness, 0.0..=1.0)).on_hover_text("0 = espejo, 1 = mate");
            ui.end_row();
            lbl(ui, "Textura");
            ui.add(TextEdit::singleline(texture).desired_width(150.).hint_text("opcional"));
            ui.end_row();
            lbl(ui, "Brillo propio");
            ui.color_edit_button_srgb(emission);
            ui.end_row();
        }
        Kind::Light { light, color, energy, range, spot_angle, shadows } => {
            lbl(ui, "Tipo");
            egui::ComboBox::from_id_salt("luz3d").selected_text(match light { LightKind::Sun => "Sol", LightKind::Point => "Puntual", LightKind::Spot => "Foco" }).show_ui(ui, |ui| {
                ui.selectable_value(light, LightKind::Sun, "Sol");
                ui.selectable_value(light, LightKind::Point, "Puntual");
                ui.selectable_value(light, LightKind::Spot, "Foco");
            });
            ui.end_row();
            lbl(ui, "Color");
            ui.color_edit_button_srgb(color);
            ui.end_row();
            lbl(ui, "Energía");
            ui.add(DragValue::new(energy).speed(0.05).range(0.0..=100.));
            ui.end_row();
            if *light != LightKind::Sun {
                lbl(ui, "Alcance (m)");
                ui.add(DragValue::new(range).speed(0.1).range(0.1..=1000.));
                ui.end_row();
            }
            if *light == LightKind::Spot {
                lbl(ui, "Apertura");
                ui.add(DragValue::new(spot_angle).range(1.0..=179.).suffix("°"));
                ui.end_row();
            }
            lbl(ui, "Sombras");
            ui.checkbox(shadows, "");
            ui.end_row();
        }
        Kind::Camera3d { fov, near, far } => {
            lbl(ui, "Campo de visión");
            ui.add(DragValue::new(fov).range(10.0..=170.).suffix("°"));
            ui.end_row();
            lbl(ui, "Cerca / lejos (m)");
            ui.horizontal(|ui| {
                ui.add(DragValue::new(near).speed(0.01).range(0.001..=10.));
                ui.add(DragValue::new(far).speed(10.).range(1.0..=100000.));
            });
            ui.end_row();
        }
    });
    ui.add_space(4.);
    // Fondo: vista previa del mosaico moviéndose.
    if let Kind::Background { image, scroll, .. } = &o.kind {
        if let Some(t) = cx.tex.get(image) {
            let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 90.), Sense::hover());
            let k = (60. / t.size_vec2().y).clamp(0.1, 4.);
            let off = Vec2::new(-scroll[0], -scroll[1]) * cx.t * k;
            vista::mosaico(ui.painter(), r, t, t.size_vec2() * k, off, Color32::WHITE);
            ui.painter().rect_stroke(r, 4, Stroke::new(1., BORDER), egui::StrokeKind::Inside);
            if *scroll != [0., 0.] {
                ui.ctx().request_repaint();
            }
        }
        ui.label(muted(if facil { "Se repite para llenar todo su tamaño. En una capa con «Lejanía» parece que está lejos." } else { "Mosaico que llena w × h; con paralaje en su capa da profundidad." }));
    }
    // Partículas: recetas, vista previa y todos los parámetros.
    if let Kind::Particles(d) = &mut o.kind {
        let tex = cx.tex.get(&d.image);
        let imgs: Vec<String> = cx.files.iter().filter(|f| recursos::Tipo::de(f) == Some(recursos::Tipo::Imagen)).cloned().collect();
        let (_, ped) = particulas::editor(ui, d, cx.sim, cx.dt, tex, &imgs, facil);
        if ped.is_some() {
            act = Some(PropAct::Importar(recursos::Tipo::Imagen, Campo::Particulas));
        }
    }
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
                        ui.horizontal_wrapped(|ui| {
                            let r = ui.add(Button::new((RichText::new(ic::PALETTE).color(ACCENT), RichText::new("Dibujar"))))
                                .on_hover_text("Dibuja o cambia las imágenes de esta animación con el editor de pixeles");
                            if r.clicked() {
                                act = Some(PropAct::Dibujar(i));
                            }
                            if ui.button(it(ic::PLUS, "Añadir imágenes…")).clicked() {
                                if let Some(fs) = rfd::FileDialog::new().add_filter("Imagen", &["png", "jpg", "jpeg"]).set_directory(cx.base).pick_files() {
                                    a.frames.extend(fs.iter().map(|f| recursos::importar(cx.base, f).unwrap_or_else(|_| rel(cx.base, f))));
                                }
                            }
                        });
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
    // Sprite: origen y máscara sobre la primera imagen.
    if let Kind::Sprite { animations, origin, mask, w, h } = &mut o.kind {
        let f = animations.first().and_then(|a| a.frames.first()).cloned();
        let tex = f.as_ref().and_then(|f| cx.tex.get(f));
        let img = tex.map_or(Vec2::new(if *w > 0. { *w } else { 32. }, if *h > 0. { *h } else { 32. }), |t| t.size_vec2());
        let mut hdr = egui::CollapsingHeader::new(section(if facil { "Centro y borde de choque" } else { "Origen y máscara" }, None)).id_salt("origenmask");
        if cx.abrir_origen {
            hdr = hdr.open(Some(true));
        }
        hdr.show(ui, |ui| {
            if tex.is_none() {
                ui.label(muted("Primero ponle una imagen (Dibujar o Añadir imágenes)."));
            }
            origen::editor(ui, cx.origen, tex, img, origin, mask, facil);
        });
    }
    // Efecto visual (objetos 2D).
    if !matches!(o.kind, Kind::Mesh { .. } | Kind::Light { .. } | Kind::Camera3d { .. } | Kind::Particles(_)) {
        let n = EFECTOS.iter().position(|e| e.0 == o.effect).unwrap_or(0);
        egui::CollapsingHeader::new(section(if facil { "Efecto" } else { "Efecto visual" }, None)).id_salt("efecto").default_open(!o.effect.is_empty()).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(4.);
                for (k, (v, kid, exp, icon, tip)) in EFECTOS.iter().enumerate() {
                    if herr_btn(ui, k == n, icon, if facil { kid } else { exp }, tip, Vec2::new(58., 50.)).clicked() {
                        o.effect = v.to_string();
                    }
                }
            });
            efecto_muestra(ui, o, cx);
            ui.label(muted(EFECTOS[EFECTOS.iter().position(|e| e.0 == o.effect).unwrap_or(0)].4));
            if !facil {
                ui.label(RichText::new("Valor inicial de la propiedad «effect» (se cambia con eventos; «effect_amount» de 0 a 1). En el editor se ve aproximado.").small().color(TEXT_MUTED));
            }
        });
    }
    egui::CollapsingHeader::new(section("Comportamientos", Some(o.behaviors.len()))).id_salt("behs").default_open(true).show(ui, |ui| {
        let mut del = None;
        for (i, b) in o.behaviors.iter_mut().enumerate() {
            if behavior_ui(ui, b, i, facil) {
                del = Some(i);
            }
        }
        if let Some(i) = del {
            o.behaviors.remove(i);
        }
        egui::ComboBox::from_id_salt("addbeh").selected_text(format!("{}  Añadir comportamiento", ic::PLUS)).width(220.).show_ui(ui, |ui| {
            for t in BEHAVIORS {
                let r = ui.selectable_label(false, beh_label(t, facil));
                let r = if beh_help(t).is_empty() { r } else { r.on_hover_text(beh_help(t)) };
                if r.clicked() {
                    o.behaviors.extend(new_behavior(t));
                }
            }
        });
    });
    match cx.usos {
        Some(usos) => {
            let name = o.name.clone();
            egui::CollapsingHeader::new(section("Cajitas del objeto", Some(o.vars.len()))).id_salt("ovars").show(ui, |ui| {
                cajitas::del_objeto(ui, &name, &mut o.vars, &|v| usos.get(v).copied().unwrap_or(0), nb)
            });
        }
        None => drop(egui::CollapsingHeader::new(section("Variables del objeto", Some(o.vars.len()))).id_salt("ovars").show(ui, |ui| vars_ui(ui, "objvars", &mut o.vars, nb))),
    }
    act
}

/// Muestra del efecto: el objeto dibujado con el efecto elegido (las ondas se mueven).
fn efecto_muestra(ui: &mut Ui, o: &ObjectDef, cx: &PropCx) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 84.), Sense::hover());
    let p = ui.painter().with_clip_rect(r);
    p.rect_filled(r, 6, Color32::from_rgb(40, 44, 60));
    let fx = o.effect.as_str();
    let caja = Rect::from_center_size(r.center(), Vec2::splat(56.));
    let q = [caja.left_top(), caja.right_top(), caja.right_bottom(), caja.left_bottom()];
    match &o.kind {
        Kind::Sprite { animations, .. } => match animations.first().and_then(|a| a.frames.first()).and_then(|f| cx.tex.get(f).map(|t| (f, t))) {
            Some((f, t)) => {
                let sz = t.size_vec2();
                let k = (64. / sz.x.max(sz.y)).min(8.);
                let rr = Rect::from_center_size(r.center(), sz * k);
                let q = [rr.left_top(), rr.right_top(), rr.right_bottom(), rr.left_bottom()];
                let tx = vista::Texs { n: t, gris: cx.tex.get(&format!("gris:{f}")), blanco: cx.tex.get(&format!("blanco:{f}")) };
                vista::sprite_fx(&p, q, false, &tx, Color32::WHITE, fx, 1., cx.t);
            }
            None => drop(p.text(r.center(), Align2::CENTER_CENTER, ic::IMAGE, egui::FontId::proportional(28.), TEXT_FAINT)),
        },
        Kind::Box { color, .. } | Kind::Text { color, .. } => {
            let [rr, g, b] = vista::color_fx(*color, fx, 1.);
            let c = Color32::from_rgb(rr, g, b);
            if matches!(o.kind, Kind::Text { .. }) {
                p.text(r.center(), Align2::CENTER_CENTER, "Aa", egui::FontId::proportional(40.), c);
            } else {
                p.add(egui::Shape::convex_polygon(q.to_vec(), c, Stroke::NONE));
                if fx == "outline" {
                    p.add(egui::Shape::closed_line(q.to_vec(), Stroke::new(2., Color32::WHITE)));
                }
            }
        }
        _ => drop(p.text(r.center(), Align2::CENTER_CENTER, ic::SPARKLE, egui::FontId::proportional(28.), TEXT_SOFT)),
    }
    if fx == "wave" {
        ui.ctx().request_repaint();
    }
}

fn kind_icon(k: &Kind) -> &'static str {
    match k {
        Kind::Background { .. } => ic::IMAGE,
        Kind::Particles(_) => ic::SPARKLE,
        Kind::Sprite { .. } => ic::IMAGE,
        Kind::Text { .. } => ic::TEXT_T,
        Kind::Box { .. } => ic::CUBE,
        Kind::Tilemap { .. } => ic::MAP,
        Kind::Mesh { .. } => ic::CUBE,
        Kind::Light { .. } => ic::LIGHTBULB,
        Kind::Camera3d { .. } => ic::CAMERA,
    }
}

fn kind_name(k: &Kind) -> &'static str {
    match k {
        Kind::Background { .. } => "Fondo",
        Kind::Particles(_) => "Partículas",
        Kind::Sprite { .. } => "Sprite",
        Kind::Text { .. } => "Texto",
        Kind::Box { .. } => "Caja",
        Kind::Tilemap { .. } => "Mapa",
        Kind::Mesh { .. } => "Objeto 3D",
        Kind::Light { .. } => "Luz 3D",
        Kind::Camera3d { .. } => "Cámara 3D",
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
            pintor: mapa::Pintor::default(), dib: None, nuevo: None,
            inicio: None, mis: Default::default(), mis_json: String::new(), mis_sesion: HashMap::new(), t: 0.,
            vista: None, vprefs: vista::Prefs::default(), vista_partida: Vars::new(), vista_abrir: None, nv: nodos::Vista::default(),
            sel_mas: vec![], arrastre: None, clip: None, pegadas: 0, explorador: Default::default(), ajustes: false, psim: HashMap::new(),
            origen_st: Default::default(), buscador: Default::default(), fuentes: HashMap::new(), dt: 0., abrir_origen: false,
        }
    }

    fn json(&self) -> String {
        serde_json::to_string_pretty(&self.p).unwrap_or_default()
    }

    fn set_project(&mut self, p: Project, path: Option<PathBuf>) {
        let kids = std::mem::take(&mut self.kids);
        let (facil, grande, t) = (self.facil, self.grande, self.t);
        let vprefs = std::mem::take(&mut self.vprefs);
        let clip = self.clip.take();
        // El avance de las misiones de este juego queda guardado por si se vuelve a abrir en la sesión.
        let mut mis_sesion = std::mem::take(&mut self.mis_sesion);
        if let Some(old) = self.path.as_ref() {
            mis_sesion.insert(old.clone(), std::mem::take(&mut self.mis));
        }
        let mis = path.as_ref().and_then(|n| mis_sesion.remove(n)).unwrap_or_else(|| misiones::Misiones::cargar(path.as_deref()));
        if let Some(n) = path.as_ref() {
            inicio::recordar(n);
        }
        *self = App { p, path, kids, facil, grande, t, mis, mis_sesion, vprefs, clip, ..App::new() };
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
        let n = self.p.scenes[self.sc].instances.len();
        self.sel_inst = self.sel_inst.filter(|i| *i < n);
        self.sel_mas.retain(|i| *i < n);
        self.arrastre = None;
        self.func = self.func.min(self.p.functions.len().saturating_sub(1));
        (self.ed, self.fparams, self.drag_off) = (None, None, None);
        self.sel = None;
        self.nv.reset();
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
        let otra_ruta = ask || self.path.is_none();
        if otra_ruta {
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
                if otra_ruta {
                    self.mis.mover(&path);
                    inicio::recordar(&path);
                }
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
        if self.discard_ok() {
            self.open_file();
        }
    }

    fn open_file(&mut self) {
        if let Some(f) = rfd::FileDialog::new().add_filter("Proyecto NicoStation", &["json"]).pick_file() {
            abrir_proyecto(self, f);
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
                    self.mis.probado(self.t);
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

    /// Versión gris (1) o silueta blanca (2) de una imagen, para mostrar los efectos.
    fn tex_var(&mut self, ctx: &egui::Context, path: &str, var: u8) -> Option<egui::TextureHandle> {
        let key = format!("{}:{path}", if var == 1 { "gris" } else { "blanco" });
        if !self.tex.contains_key(&key) {
            let t = image::open(self.base_dir().join(path)).ok().map(|img| ctx.load_texture(&key, vista::variante(&img.to_rgba8(), var), egui::TextureOptions::NEAREST));
            self.tex.insert(key.clone(), t);
        }
        self.tex[&key].clone()
    }

    /// Registra en egui una fuente TTF/OTF del proyecto (queda lista en el cuadro siguiente).
    fn fuente(&mut self, ctx: &egui::Context, font: &str) {
        if font.is_empty() || self.fuentes.contains_key(font) {
            return;
        }
        let estado = match std::fs::read(self.base_dir().join(font)) {
            Ok(b) => {
                let fam = egui::FontFamily::Name(font.into());
                ctx.add_font(egui::epaint::text::FontInsert::new(font, egui::FontData::from_owned(b), vec![egui::epaint::text::InsertFontFamily {
                    family: fam,
                    priority: egui::epaint::text::FontPriority::Highest,
                }]));
                1
            }
            Err(_) => 0,
        };
        self.fuentes.insert(font.to_string(), estado);
    }

    fn obj_size(&self, o: &ObjectDef) -> Vec2 {
        match &o.kind {
            Kind::Background { w, h, .. } => Vec2::new(*w, *h),
            Kind::Particles(_) => Vec2::splat(24.),
            // En el lienzo 2D, un objeto 3D se ve desde arriba (32 px por metro) hasta el visor 3D (F7).
            Kind::Mesh { size, .. } => Vec2::new(size[0] * 32., size[2] * 32.),
            Kind::Light { .. } | Kind::Camera3d { .. } => Vec2::splat(24.),
            Kind::Box { w, h, .. } => Vec2::new(*w, *h),
            Kind::Text { wrap, size, .. } if *wrap > 0. => Vec2::new(*wrap, *size),
            Kind::Text { text, size, .. } => Vec2::new(text.lines().map(|l| l.chars().count()).max().unwrap_or(1).max(1) as f32 * size * 0.55, *size * text.lines().count().max(1) as f32),
            Kind::Sprite { w, h, animations, .. } if *w <= 0. || *h <= 0. => {
                let t = animations.first().and_then(|a| a.frames.first()).and_then(|f| self.tex.get(f).cloned().flatten());
                t.map_or(Vec2::splat(32.), |t| t.size_vec2())
            }
            Kind::Sprite { w, h, .. } => Vec2::new(*w, *h),
            Kind::Tilemap { tile, cols, rows, .. } => Vec2::new(*cols as f32 * tile, *rows as f32 * tile),
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
                Kind::Background { .. } | Kind::Particles(_) => eventos::Thumb::Color([120, 140, 170]),
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
                Kind::Tilemap { tileset, .. } => self.texture(ctx, tileset).map_or(eventos::Thumb::Color([89, 192, 89]), eventos::Thumb::Tex),
                Kind::Mesh { color, .. } => eventos::Thumb::Color(*color),
                Kind::Light { color, .. } => eventos::Thumb::Color(*color),
                Kind::Camera3d { .. } => eventos::Thumb::Color([120, 120, 130]),
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
        s.instances.push(Instance { obj, x: at.x, y: at.y, layer, ..Default::default() });
        self.sel_inst = Some(s.instances.len() - 1);
    }

    fn sel_valid(&self) -> Option<usize> {
        self.sel_inst.filter(|i| *i < self.p.scenes[self.sc].instances.len())
    }

    /// Todas las instancias elegidas (la principal y las demás), ordenadas y sin repetir.
    fn seleccion(&self) -> Vec<usize> {
        let n = self.p.scenes[self.sc].instances.len();
        let mut v: Vec<usize> = self.sel_inst.into_iter().chain(self.sel_mas.iter().copied()).filter(|i| *i < n).collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    /// Elige una sola instancia (o ninguna).
    fn elegir(&mut self, i: Option<usize>) {
        self.sel_inst = i;
        self.sel_mas.clear();
        if let Some(i) = i {
            self.sel_obj = Some(self.p.scenes[self.sc].instances[i].obj.clone());
        }
    }

    /// Shift+clic: agrega o saca una instancia de la selección.
    fn alternar(&mut self, i: usize) {
        let mut v = self.seleccion();
        match v.iter().position(|x| *x == i) {
            Some(k) => drop(v.remove(k)),
            None => v.push(i),
        }
        self.poner_seleccion(v);
    }

    fn poner_seleccion(&mut self, v: Vec<usize>) {
        self.sel_inst = v.last().copied();
        self.sel_mas = v;
        if let Some(i) = self.sel_inst {
            self.sel_obj = Some(self.p.scenes[self.sc].instances[i].obj.clone());
        }
    }

    /// Dónde está una instancia (posición, tamaño, giro y origen normalizado).
    fn geo(&self, i: usize) -> Option<seleccion::Geo> {
        let s = &self.p.scenes[self.sc];
        let ins = s.instances.get(i)?;
        let (sz, origin) = match self.p.objects_of(s).find(|o| o.name == ins.obj) {
            Some(o) => {
                let base = self.obj_size(o);
                let origin = match &o.kind {
                    Kind::Sprite { origin: Some(org), animations, .. } => {
                        let img = animations.first().and_then(|a| a.frames.first()).and_then(|f| self.tex.get(f).cloned().flatten()).map_or(base, |t| t.size_vec2());
                        Some(Vec2::new(org[0] / img.x.max(1.), org[1] / img.y.max(1.)))
                    }
                    Kind::Particles(_) | Kind::Light { .. } | Kind::Camera3d { .. } => Some(Vec2::splat(0.5)),
                    _ => None,
                };
                (base, origin)
            }
            None => (Vec2::splat(32.), None),
        };
        let size = Vec2::new(ins.w.unwrap_or(sz.x), ins.h.unwrap_or(sz.y));
        Some(seleccion::Geo { pos: Vec2::new(ins.x, ins.y), size, angle: ins.angle, origin })
    }

    fn delete_instance(&mut self) {
        let v = self.seleccion();
        if v.is_empty() {
            return;
        }
        self.snap_hist();
        let s = self.scene();
        let mut nombres = vec![];
        for &i in v.iter().rev() {
            nombres.push(s.instances.remove(i).obj);
        }
        self.elegir(None);
        let m = if nombres.len() == 1 { format!("Eliminada instancia de «{}»", nombres[0]) } else { format!("Eliminadas {} instancias", nombres.len()) };
        self.say(Nivel::Info, format!("{m} · {UNDO_HINT}"));
    }

    fn duplicate_instance(&mut self) {
        let v = self.seleccion();
        if v.is_empty() {
            return;
        }
        self.snap_hist();
        let g = self.grid;
        let s = self.scene();
        let n0 = s.instances.len();
        for &i in &v {
            let mut n = s.instances[i].clone();
            (n.x, n.y) = (n.x + g, n.y + g);
            s.instances.push(n);
        }
        self.poner_seleccion((n0..n0 + v.len()).collect());
        self.say(Nivel::Info, if v.len() == 1 { "Instancia duplicada".to_string() } else { format!("{} instancias duplicadas", v.len()) });
    }

    fn nudge(&mut self, d: Vec2) {
        for i in self.seleccion() {
            let ins = &mut self.scene().instances[i];
            (ins.x, ins.y) = (ins.x + d.x, ins.y + d.y);
        }
    }

    /// Ctrl+C: copia las instancias elegidas (y los objetos de la escena que usan, para pegarlas en otra escena).
    fn copiar(&mut self, ctx: &egui::Context) {
        let v = self.seleccion();
        if v.is_empty() {
            return;
        }
        let s = &self.p.scenes[self.sc];
        let instancias: Vec<Instance> = v.iter().map(|&i| s.instances[i].clone()).collect();
        let mut objetos: Vec<ObjectDef> = vec![];
        for ins in &instancias {
            if let Some(o) = s.objects.iter().find(|o| o.name == ins.obj) {
                if !objetos.iter().any(|x| x.name == o.name) {
                    objetos.push(o.clone());
                }
            }
        }
        let c = seleccion::Portapapeles { instancias, objetos };
        ctx.copy_text(c.texto());
        let n = c.instancias.len();
        self.clip = Some(c);
        self.pegadas = 0;
        self.say(Nivel::Ok, format!("{n} instancia{} copiada{} · Ctrl+V para pegar (también en otra escena)", if n == 1 { "" } else { "s" }, if n == 1 { "" } else { "s" }));
    }

    /// Ctrl+V: pega lo copiado. Con el mouse sobre el lienzo, donde está el mouse; si no, un poco corrido.
    fn pegar(&mut self, texto: Option<String>) {
        if let Some(c) = texto.as_deref().and_then(seleccion::Portapapeles::de_texto) {
            self.clip = Some(c);
        }
        let Some(c) = self.clip.clone() else { return };
        if c.instancias.is_empty() {
            return;
        }
        self.snap_hist();
        let sc = self.sc;
        // Objetos que faltan en esta escena.
        for o in &c.objetos {
            let visible = self.p.objects_of(&self.p.scenes[sc]).any(|x| x.name == o.name);
            if !visible {
                self.p.scenes[sc].objects.push(o.clone());
            }
        }
        self.pegadas += 1;
        let esquina = c.esquina();
        let destino = match self.cursor_w {
            Some(p) if self.tab == Tab::Escena => {
                let p = p.to_vec2();
                if self.snap { (p / self.grid).round() * self.grid } else { p }
            }
            _ => esquina + Vec2::splat(self.grid * self.pegadas as f32),
        };
        let layer_ok: Vec<String> = self.p.scenes[sc].layers.iter().map(|l| l.name.clone()).collect();
        let s = &mut self.p.scenes[sc];
        let n0 = s.instances.len();
        for ins in &c.instancias {
            let mut n = ins.clone();
            n.x = ins.x - esquina.x + destino.x;
            n.y = ins.y - esquina.y + destino.y;
            if !layer_ok.contains(&n.layer) {
                n.layer.clear();
            }
            s.instances.push(n);
        }
        self.poner_seleccion((n0..n0 + c.instancias.len()).collect());
        self.say(Nivel::Ok, format!("Pegado: {} instancia{} · {UNDO_HINT}", c.instancias.len(), if c.instancias.len() == 1 { "" } else { "s" }));
    }

    /// Alinea o distribuye las instancias elegidas (un paso de deshacer).
    fn alinear(&mut self, modo: Option<seleccion::Alinear>, distribuir_h: Option<bool>) {
        let v = self.seleccion();
        if v.len() < 2 {
            return;
        }
        let cajas: Vec<Rect> = v.iter().filter_map(|&i| self.geo(i)).map(|g| g.aabb()).collect();
        let d = match (modo, distribuir_h) {
            (Some(m), _) => seleccion::alinear(&cajas, m),
            (None, Some(h)) => seleccion::distribuir(&cajas, h),
            _ => return,
        };
        self.snap_hist();
        for (&i, d) in v.iter().zip(d) {
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
        self.sel_mas.clear();
        self.cur_layer.clear();
        self.say(Nivel::Info, format!("Eliminada la escena «{old}» · {UNDO_HINT}"));
    }

    // ---- recursos ----

    /// Copia archivos de afuera a la carpeta del juego (img/, snd/, fuentes/, modelos/). Devuelve las rutas nuevas.
    fn importar_archivos(&mut self, fs: Vec<PathBuf>) -> Vec<String> {
        if fs.is_empty() || !self.carpeta_ok() {
            return vec![];
        }
        let base = self.base_dir();
        let (mut ok, mut err) = (vec![], vec![]);
        for f in fs {
            match recursos::importar(&base, &f) {
                Ok(r) => ok.push(r),
                Err(e) => err.push(e),
            }
        }
        self.files_at = None;
        match (ok.len(), err.first()) {
            (_, Some(e)) => self.say(Nivel::Warn, e.clone()),
            (1, None) => self.say(Nivel::Ok, format!("Importado: {}", ok[0])),
            (n, None) => self.say(Nivel::Ok, format!("Importados {n} archivos")),
        }
        if let Some(r) = ok.last() {
            self.explorador.sel = Some(r.clone());
        }
        ok
    }

    /// Elige un archivo de un tipo (si está afuera del juego, lo copia adentro).
    fn elegir_importar(&mut self, tipo: recursos::Tipo) -> Option<String> {
        if !self.carpeta_ok() {
            return None;
        }
        let base = self.base_dir();
        let dir = base.join(tipo.carpeta());
        let f = rfd::FileDialog::new().add_filter(tipo.nombre(false), tipo.extensiones()).set_directory(if dir.exists() { dir } else { base }).pick_file()?;
        self.importar_archivos(vec![f]).pop()
    }

    fn importar_dialogo(&mut self) {
        let exts: Vec<&str> = recursos::TIPOS.iter().flat_map(|t| t.extensiones().iter().copied()).collect();
        if let Some(fs) = rfd::FileDialog::new().add_filter("Imágenes, sonidos, fuentes y modelos", &exts).pick_files() {
            self.importar_archivos(fs);
        }
    }

    /// Renombra o mueve un archivo del juego y actualiza todo lo que lo usa (y guarda, para que no queden rotos).
    fn renombrar_recurso(&mut self, viejo: &str, nuevo: &str) {
        let base = self.base_dir();
        if let Err(e) = recursos::mover(&base, viejo, nuevo) {
            return self.say(Nivel::Error, format!("No se pudo renombrar: {e}"));
        }
        self.snap_hist();
        let n = recursos::renombrar_en_proyecto(&mut self.p, viejo, nuevo);
        self.tex.retain(|k, _| !k.ends_with(viejo));
        self.fuentes.remove(viejo);
        self.files_at = None;
        self.explorador.sel = Some(nuevo.to_string());
        self.explorador.nombre = None;
        let guardado = self.path.is_some() && self.save(false);
        let refs = if n == 1 { "1 lugar".to_string() } else { format!("{n} lugares") };
        self.say(Nivel::Ok, format!("«{viejo}» ahora es «{nuevo}»; actualicé {refs}{}", if guardado { " y guardé el juego" } else { "" }));
    }

    fn borrar_recurso(&mut self, f: &str) {
        let base = self.base_dir();
        match std::fs::remove_file(base.join(f)) {
            Ok(()) => {
                self.tex.retain(|k, _| !k.ends_with(f));
                self.files_at = None;
                self.explorador.sel = None;
                let usos = recursos::usos(&self.p, f).len();
                if usos > 0 {
                    self.say(Nivel::Warn, format!("Borrado «{f}»: {usos} lugar{} del juego lo usaba{}", if usos == 1 { "" } else { "es" }, if usos == 1 { "" } else { "n" }));
                } else {
                    self.say(Nivel::Info, format!("Borrado «{f}»"));
                }
            }
            Err(e) => self.say(Nivel::Error, format!("No se pudo borrar: {e}")),
        }
    }

    /// Pestaña Recursos.
    fn recursos_tab(&mut self, ui: &mut Ui) {
        let base = self.base_dir();
        if self.files_at.is_none_or(|t| t.elapsed().as_secs() >= 3) {
            self.files = buscar_archivos(&base);
            self.files_at = Some(std::time::Instant::now());
        }
        let files = self.files.clone();
        let ctx = ui.ctx().clone();
        let mut thumbs = HashMap::new();
        for f in recursos::a_precargar(&self.explorador, &files) {
            if let Some(t) = self.texture(&ctx, &f) {
                thumbs.insert(f, t);
            }
        }
        let refs = recursos::referencias(&self.p);
        let guardado = self.path.is_some();
        match recursos::ui(ui, &mut self.explorador, &base, &files, &thumbs, &refs, self.facil, guardado) {
            Some(recursos::Accion::Importar) => self.importar_dialogo(),
            Some(recursos::Accion::Renombrar(a, b)) => self.renombrar_recurso(&a, &b),
            Some(recursos::Accion::Borrar(f)) => self.borrar_recurso(&f),
            None => {}
        }
    }

    // ---- buscar en eventos ----

    fn barra_buscar(&mut self, ui: &mut Ui) {
        if !self.buscador.abierto {
            return;
        }
        let now = self.now.clone();
        self.buscador.actualizar(&self.p, self.sc, &now, self.facil);
        let r = buscar::barra(ui, &mut self.buscador, self.facil);
        ui.add_space(6.);
        match r {
            Some(buscar::Accion::Ir(h)) => self.ir_a(&h),
            Some(buscar::Accion::Reemplazar(todos)) if !self.facil => {
                let b = &self.buscador;
                let hs: Vec<buscar::Hallazgo> = if todos { b.resultados.clone() } else { b.resultados.get(b.actual).cloned().into_iter().collect() };
                let (q, rr) = (b.texto.clone(), b.reemplazo.clone());
                self.snap_hist();
                let n = buscar::reemplazar(&mut self.p, &hs, &q, &rr);
                self.say(if n > 0 { Nivel::Ok } else { Nivel::Info }, format!("{n} reemplazo{} · {UNDO_HINT}", if n == 1 { "" } else { "s" }));
            }
            _ => {}
        }
    }

    /// Lleva a un resultado de la búsqueda: su hoja, su evento resaltado.
    fn ir_a(&mut self, h: &buscar::Hallazgo) {
        match h.target {
            Target::Scene(i) => {
                if self.sc != i {
                    self.sc = i;
                    self.elegir(None);
                }
                self.tab = Tab::Eventos;
                self.nv.en_escena = false;
            }
            Target::Func(i) => {
                self.func = i;
                self.tab = Tab::Funciones;
                self.nv.en_func = false;
            }
        }
        self.jump = Some((h.ev.clone(), self.t + 3.));
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
        if ui.button(it(ic::FILE_PLUS, "Nuevo…")).clicked() && self.discard_ok() {
            self.inicio = Some(inicio::Inicio::new(true));
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
        if ui.button(it(ic::GEAR, "Ajustes del juego…")).clicked() {
            self.ajustes = true;
        }
        if ui.button(it(ic::FILE_IMAGE, "Importar archivos…")).on_hover_text("Imágenes, sonidos, fuentes o modelos. También puedes arrastrarlos a la ventana.").clicked() {
            self.importar_dialogo();
        }
        ui.separator();
        if ui.button(it(ic::SIGN_OUT, "Salir")).clicked() && self.discard_ok() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn top_bar(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        // Si no entra todo: 1 = sin nombre del programa y botones más cortos, 2 = además pestañas sin íconos.
        let nivel = if full.width() >= 1400. { 0 } else if full.width() >= 900. { 1 } else { 2 };
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
                if primary(ui, (on_accent(ic::PLAY), on_accent("Probar").font(sb(14.)), hint)).on_hover_text("Guardar y ejecutar el juego en su propia ventana, con sonido").clicked() {
                    self.run();
                }
                // Con el juego corriendo, sus controles están en la barra de la vista previa.
                if self.vista.is_none() {
                    let tip = "Jugar dentro del editor, con depurador (F6). Si cambias algo, se actualiza solo.";
                    let r = if nivel == 2 {
                        ib(ui, ic::MONITOR_PLAY, tip)
                    } else {
                        ui.add(Button::new((RichText::new(ic::MONITOR_PLAY).color(ACCENT), RichText::new("Jugar aquí").font(sb(14.))))).on_hover_text(tip)
                    };
                    if r.clicked() {
                        let ctx = ui.ctx().clone();
                        self.vista_empezar(&ctx);
                    }
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
                ui.add_space(2.);
                // Con la etiqueta «Misiones» solo si sobra lugar (si no, ícono con insignia).
                if self.mis.boton(ui, &self.p, full.width() < 1480.).clicked() {
                    self.mis.abierto = !self.mis.abierto;
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
                [(Tab::Escena, ic::SQUARES, "Escena"), (Tab::Eventos, ic::LIST, "Eventos"), (Tab::Funciones, ic::PUZZLE, "Mis bloques"), (Tab::Variables, ic::ARCHIVE_BOX, "Cajitas"), (Tab::Recursos, ic::FOLDER_OPEN, "Archivos")]
            } else {
                [(Tab::Escena, ic::SQUARES, "Escena"), (Tab::Eventos, ic::LIST, "Eventos"), (Tab::Funciones, ic::FUNCTION, "Funciones"), (Tab::Variables, ic::BRACES, "Variables"), (Tab::Recursos, ic::FOLDER_OPEN, "Recursos")]
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
                    // Con el juego en todo el centro, ir a una pestaña lo deja abajo para poder editar mientras corre.
                    self.vprefs.grande &= self.vista.is_none();
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
            self.sel_mas.clear();
        self.sel_mas.clear();
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
            // Gravedad de la física: solo si algún objeto de la escena tiene «Física».
            let fisica = self.p.objects_of(&self.p.scenes[sc]).any(|o| o.behaviors.iter().any(|b| matches!(b, Behavior::Physics { .. })));
            if fisica {
                let g = &mut self.p.scenes[sc].gravity;
                if self.facil {
                    lbl(ui, "Fuerza de caída");
                    ui.add(egui::Slider::new(&mut g[1], -2000.0..=3000.).max_decimals(0)).on_hover_text("Qué tan fuerte caen las cosas con física (980 = como en la Tierra; negativo = caen hacia arriba)");
                } else {
                    lbl(ui, "Gravedad");
                    ui.horizontal(|ui| {
                        ui.add(DragValue::new(&mut g[0]).speed(5.).prefix("x "));
                        ui.add(DragValue::new(&mut g[1]).speed(5.).prefix("y "));
                    }).response.on_hover_text("Física 2D, en px/s² (por defecto 0, 980)");
                }
                ui.end_row();
            }
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
            if ui.button(it(ic::GEAR, "Ajustes del juego…")).on_hover_text("Escalado, pixel art, FPS, ícono y pantalla de carga").clicked() {
                self.ajustes = true;
            }
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
                Kind::Sprite { w: 0., h: 0., animations: vec![Animation { name: "quieto".into(), frames: vec![], fps: 10., looping: true }], mask: vec![], origin: None },
                Kind::Text { text: "Texto".into(), size: 24., color: [255; 3], align: "left".into(), font: String::new(), wrap: 0. },
                Kind::Box { w: 32., h: 32., color: [200, 80, 80] },
            ] {
                let base = kind_name(&kind);
                if ui.add(Button::new(it(kind_icon(&kind), base))).on_hover_text(format!("Nuevo objeto {}", base.to_lowercase())).clicked() {
                    let n = self.nombre_libre(base);
                    let sprite = matches!(kind, Kind::Sprite { .. });
                    self.scene().objects.push(ObjectDef { name: n.clone(), kind, behaviors: vec![], vars: Vars::new(), effect: String::new() });
                    self.sel_obj = Some(n.clone());
                    if sprite {
                        self.nuevo = Some(Nuevo::Sprite(n));
                    }
                }
            }
            if ui.add(Button::new(it(ic::MAP, "Mapa"))).on_hover_text(mapa::TEXTO_NINOS).clicked() {
                self.nuevo_mapa();
            }
            if ui.add(Button::new(it(ic::MOUNTAINS, "Fondo"))).on_hover_text("Una imagen que se repite: cielo, pasto, agua").clicked() {
                self.nuevo_fondo();
            }
            if ui.add(Button::new(it(ic::SPARKLE, "Partículas"))).on_hover_text("Fuego, humo, chispas, lluvia, nieve, explosiones, magia").clicked() {
                self.nuevas_particulas(0);
            }
        });
    }

    /// «+ Fondo»: imagen que se repite del tamaño de la pantalla, detrás de todo.
    fn nuevo_fondo(&mut self) {
        self.snap_hist();
        let n = self.nombre_libre("Fondo");
        let image = self.files.iter().find(|f| recursos::Tipo::de(f) == Some(recursos::Tipo::Imagen) && f.to_lowercase().contains("fondo")).cloned().unwrap_or_default();
        let kind = Kind::Background { image, w: self.p.width, h: self.p.height, scroll: [0., 0.] };
        let layer = self.p.scenes[self.sc].layers.iter().find(|l| l.name.to_lowercase().contains("fondo")).map(|l| l.name.clone()).unwrap_or_default();
        let s = self.scene();
        s.objects.push(ObjectDef { name: n.clone(), kind, behaviors: vec![], vars: Vars::new(), effect: String::new() });
        s.instances.push(Instance { obj: n.clone(), layer, z: -100., ..Default::default() });
        let i = s.instances.len() - 1;
        self.elegir(Some(i));
        self.sel_obj = Some(n);
        self.say(Nivel::Ok, "Fondo nuevo: elige su imagen en las propiedades (a la derecha)");
    }

    /// «+ Partículas»: emisor con la receta `k` (0 = fuego), en el medio de la pantalla.
    fn nuevas_particulas(&mut self, k: usize) {
        self.snap_hist();
        let (nombre, _, def) = particulas::presets().swap_remove(k.min(6));
        let n = self.nombre_libre(&format!("Particulas{}", if k == 0 { String::new() } else { nombre.to_string() }).replace(['ó', ' '], ""));
        let (x, y) = (self.p.width / 2., self.p.height / 2.);
        let layer = self.cur_layer.clone();
        let s = self.scene();
        s.objects.push(ObjectDef { name: n.clone(), kind: Kind::Particles(Box::new(def)), behaviors: vec![], vars: Vars::new(), effect: String::new() });
        s.instances.push(Instance { obj: n.clone(), x, y, layer, ..Default::default() });
        let i = s.instances.len() - 1;
        self.elegir(Some(i));
        self.say(Nivel::Ok, format!("Partículas nuevas ({nombre}): cámbialas con las recetas y los ajustes de la derecha"));
    }

    fn nombre_libre(&self, base: &str) -> String {
        (1..).map(|k| if k == 1 { base.to_string() } else { format!("{base}{k}") }).find(|n| !self.name_taken(n)).unwrap()
    }

    /// «+ Mapa»: objeto mapa de baldosas del tamaño del juego (el doble de ancho), con una instancia en (0, 0) que es plataforma.
    fn nuevo_mapa(&mut self) {
        self.snap_hist();
        let n = self.nombre_libre("Mapa");
        let tile = 32.;
        let cols = (self.p.width / tile).ceil() as usize * 2;
        let rows = (self.p.height / tile).ceil() as usize;
        let kind = Kind::Tilemap { tileset: String::new(), tile, cols, rows, data: vec![], pass: vec![] };
        let platform = Behavior::Platform { jumpthru: false, ladder: false, grabbable: false };
        let s = self.scene();
        s.objects.push(ObjectDef { name: n.clone(), kind, behaviors: vec![platform], vars: Vars::new(), effect: String::new() });
        s.instances.push(Instance { obj: n.clone(), ..Default::default() });
        self.sel_inst = Some(s.instances.len() - 1);
        self.sel_obj = Some(n.clone());
        self.pintor = mapa::Pintor::default();
        self.nuevo = Some(Nuevo::Mapa(n));
        self.say(Nivel::Ok, "Mapa nuevo: elige sus baldosas y píntalo en el lienzo");
    }

    /// Crea (si hace falta) las baldosas de ejemplo y se las pone al mapa `obj`.
    fn usar_baldosas_ejemplo(&mut self, obj: &str) {
        if !self.carpeta_ok() {
            return;
        }
        match mapa::crear_ejemplo(&self.base_dir()) {
            Ok(f) => {
                self.tex.remove(&f);
                self.files_at = None;
                if let Some(Kind::Tilemap { tileset, tile, pass, .. }) = self.obj_mut(obj).map(|o| &mut o.kind) {
                    *tileset = f.clone();
                    *tile = 32.;
                    if pass.is_empty() {
                        *pass = mapa::EJEMPLO_PASS.to_vec();
                    }
                }
                self.say(Nivel::Ok, format!("Baldosas de ejemplo listas en {f}: pasto, tierra, piedra, ladrillo, madera, agua, nube y hojas"));
            }
            Err(e) => self.say(Nivel::Error, format!("No se pudieron crear las baldosas: {e}")),
        }
    }

    /// Las imágenes nuevas se guardan junto al proyecto: hace falta que esté guardado.
    fn carpeta_ok(&mut self) -> bool {
        if self.path.is_some() {
            return true;
        }
        self.say(Nivel::Warn, "Primero guarda el proyecto (Ctrl+S): las imágenes se guardan en su carpeta.");
        false
    }

    /// Abre el editor de dibujo para una animación o para la imagen de baldosas de un mapa.
    fn abrir_dibujo(&mut self, destino: dibujo::Destino) {
        if !self.carpeta_ok() {
            return;
        }
        let base = self.base_dir();
        let (obj, rutas, fps, titulo, rejilla, nuevo_wh) = match &destino {
            dibujo::Destino::Anim { obj, anim } => {
                let Some(Kind::Sprite { animations, w, h, .. }) = self.obj_mut(obj).map(|o| o.kind.clone()) else { return };
                let Some(a) = animations.get(*anim) else { return };
                let wh = if w > 0. && h > 0. && w <= 256. && h <= 256. { (w as usize, h as usize) } else { (32, 32) };
                (obj.clone(), a.frames.clone(), a.fps, format!("{obj} · animación «{}»", a.name), None, wh)
            }
            dibujo::Destino::Baldosas { obj } => {
                let Some(Kind::Tilemap { tileset, tile, .. }) = self.obj_mut(obj).map(|o| o.kind.clone()) else { return };
                let t = (tile as usize).clamp(4, 256);
                let rutas = if tileset.is_empty() { vec![] } else { vec![tileset] };
                (obj.clone(), rutas, 10., format!("{obj} · baldosas"), Some(t), ((t * 4).min(256), (t * 2).min(256)))
            }
        };
        let (w, h, cuadros) = if rutas.is_empty() {
            (nuevo_wh.0, nuevo_wh.1, vec![])
        } else {
            match dibujo::cargar(&base, &rutas) {
                Ok(x) => x,
                Err(e) => return self.say(Nivel::Error, e),
            }
        };
        let mut d = dibujo::Dibujo::new(destino, titulo, w, h, cuadros, fps);
        d.rejilla = rejilla;
        self.dib = Some(d);
        self.say(Nivel::Info, format!("Dibujando «{obj}»"));
    }

    /// Guarda el dibujo en PNG y actualiza el objeto (un solo paso de deshacer).
    fn guardar_dibujo(&mut self, d: &mut dibujo::Dibujo) -> bool {
        let base = self.base_dir();
        let prefijo = match &d.destino {
            dibujo::Destino::Anim { obj, anim } => {
                let an = match self.obj_mut(obj).map(|o| &o.kind) {
                    Some(Kind::Sprite { animations, .. }) => animations.get(*anim).map(|a| a.name.clone()).unwrap_or_default(),
                    _ => String::new(),
                };
                format!("{}_{}", dibujo::limpio(obj), dibujo::limpio(&an))
            }
            dibujo::Destino::Baldosas { obj } => format!("{}_baldosas", dibujo::limpio(obj)),
        };
        let rutas = match d.guardar(&base, &prefijo) {
            Ok(r) => r,
            Err(e) => {
                d.aviso = Some(e);
                return false;
            }
        };
        for r in &rutas {
            self.tex.remove(r);
        }
        self.files_at = None;
        self.snap_hist();
        let fps = d.fps;
        match d.destino.clone() {
            dibujo::Destino::Anim { obj, anim } => {
                if let Some(Kind::Sprite { animations, .. }) = self.obj_mut(&obj).map(|o| &mut o.kind) {
                    if let Some(a) = animations.get_mut(anim) {
                        a.frames = rutas.clone();
                        a.fps = fps;
                    }
                }
            }
            dibujo::Destino::Baldosas { obj } => {
                if let Some(Kind::Tilemap { tileset, .. }) = self.obj_mut(&obj).map(|o| &mut o.kind) {
                    *tileset = rutas[0].clone();
                }
            }
        }
        let n = rutas.len();
        self.say(Nivel::Ok, format!("Dibujo guardado ({n} imagen{}) · {UNDO_HINT}", if n == 1 { "" } else { "es" }));
        true
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
                        let facil = self.facil;
                        let (r, _) = egui::containers::menu::MenuButton::from_button(b).ui(ui, |ui| {
                            menu_spacing(ui);
                            ui.label(RichText::new(if facil { "Lejanía" } else { "Paralaje" }).font(sb(13.)).color(TEXT_MUTED));
                            if facil {
                                // Lejanía 0 = sigue a la cámara; 1 = quieta (como el cielo).
                                let mut lejos = 1. - l.parallax[0];
                                let t = if lejos < 0.05 { "cerca" } else if lejos > 0.95 { "quieta" } else { "lejos" };
                                if ui.add(egui::Slider::new(&mut lejos, 0.0..=1.0).show_value(false).text(t))
                                    .on_hover_text("Las cosas lejanas se mueven más despacio cuando la cámara avanza: así parece que tienen profundidad").changed() {
                                    l.parallax = [1. - lejos, 1. - lejos];
                                }
                            } else {
                                ui.horizontal(|ui| {
                                    ui.add(DragValue::new(&mut l.parallax[0]).speed(0.01).range(-2.0..=4.).prefix("x ")).on_hover_text("1 = sigue a la cámara, 0 = fija, 0.5 = a la mitad");
                                    ui.add(DragValue::new(&mut l.parallax[1]).speed(0.01).range(-2.0..=4.).prefix("y "));
                                });
                            }
                            ui.separator();
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
                        r.on_hover_text("Más opciones de la capa (lejanía, orden)");
                        if l.parallax != [1., 1.] {
                            let t = if self.facil { format!("{} {:.0}%", ic::MOUNTAINS, (1. - l.parallax[0]) * 100.) } else { format!("{:.2}", l.parallax[0]) };
                            ui.label(RichText::new(t).small().color(TEXT_MUTED)).on_hover_text(if self.facil { "Lejanía" } else { "Paralaje x" });
                        }
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
            s.layers.push(Layer { name: n, visible: true, fixed: false, parallax: [1., 1.] });
        }
    }

    // ---- panel derecho ----

    /// Panel de varias instancias elegidas: alinear, distribuir, capa y z para todas.
    fn right_varias(&mut self, ui: &mut Ui, v: &[usize]) {
        let sc = self.sc;
        let f = self.facil;
        header(ui, ic::SELECTION, if f { "Varias cosas elegidas" } else { "Selección" }, &format!("{} instancias", v.len()));
        ui.label(muted(if f { "Arrástralas para moverlas juntas. Las manijas las agrandan o las giran a todas." } else { "Arrastrar mueve todas; manijas: escalar (Shift = proporción) y girar (Shift = de a 15°)." }));
        ui.add_space(4.);
        ui.label(RichText::new(if f { "Ordenar" } else { "Alinear" }).font(sb(13.)).color(TEXT_MUTED));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(4.);
            for (m, icon, kid, exp) in [
                (seleccion::Alinear::Izquierda, ic::ALIGN_LEFT, "Izquierda", "Izquierda"),
                (seleccion::Alinear::CentroH, ic::ALIGN_CENTER_H, "Centro", "Centro H"),
                (seleccion::Alinear::Derecha, ic::ALIGN_RIGHT, "Derecha", "Derecha"),
                (seleccion::Alinear::Arriba, ic::ALIGN_TOP, "Arriba", "Arriba"),
                (seleccion::Alinear::CentroV, ic::ALIGN_CENTER_V, "Medio", "Centro V"),
                (seleccion::Alinear::Abajo, ic::ALIGN_BOTTOM, "Abajo", "Abajo"),
            ] {
                if herr_btn(ui, false, icon, if f { kid } else { exp }, "Alinear las elegidas", Vec2::new(62., 50.)).clicked() {
                    self.alinear(Some(m), None);
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(4.);
            let ok = v.len() >= 3;
            ui.add_enabled_ui(ok, |ui| {
                if herr_btn(ui, false, ic::DISTRIBUTE_H, if f { "Parejo ancho" } else { "Distribuir H" }, "Mismo espacio entre ellas, a lo ancho (3 o más)", Vec2::new(94., 50.)).clicked() {
                    self.alinear(None, Some(true));
                }
                if herr_btn(ui, false, ic::DISTRIBUTE_V, if f { "Parejo alto" } else { "Distribuir V" }, "Mismo espacio entre ellas, a lo alto (3 o más)", Vec2::new(94., 50.)).clicked() {
                    self.alinear(None, Some(false));
                }
            });
        });
        ui.add_space(6.);
        let layers: Vec<String> = self.p.scenes[sc].layers.iter().map(|l| l.name.clone()).collect();
        let s = &self.p.scenes[sc];
        let mut capa = s.instances[v[0]].layer.clone();
        let misma = v.iter().all(|&i| s.instances[i].layer == capa);
        let mut z = s.instances[v[0]].z;
        grid("varias").show(ui, |ui| {
            lbl(ui, "Capa");
            if !misma {
                capa = "(varias)".into();
            }
            let mut opts = layers.clone();
            if !misma {
                opts.insert(0, "(varias)".into());
            }
            if combo(ui, "capa_varias", &mut capa, &opts, "Base") && capa != "(varias)" {
                for &i in v {
                    self.p.scenes[sc].instances[i].layer = capa.clone();
                }
            }
            ui.end_row();
            lbl(ui, "Z (todas)");
            if ui.add(DragValue::new(&mut z).speed(0.1)).changed() {
                for &i in v {
                    self.p.scenes[sc].instances[i].z = z;
                }
            }
            ui.end_row();
        });
        ui.horizontal_wrapped(|ui| {
            if ui.button(it(ic::COPY, "Copiar")).on_hover_text("Ctrl+C").clicked() {
                let ctx = ui.ctx().clone();
                self.copiar(&ctx);
            }
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

    fn right(&mut self, ui: &mut Ui) {
        let sc = self.sc;
        let varias = self.seleccion();
        if varias.len() > 1 {
            self.right_varias(ui, &varias);
        } else if let Some(i) = self.sel_valid() {
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
            if self.files_at.is_none_or(|t| t.elapsed().as_secs() >= 5) {
                self.files = buscar_archivos(&base);
                self.files_at = Some(std::time::Instant::now());
            }
            let files = self.files.clone();
            // Imágenes y fuentes de este objeto (y las variantes para mostrar el efecto).
            let (mut imgs, mut font, mut fx) = (vec![], String::new(), false);
            if let Some(o) = self.obj_mut(&name) {
                fx = !o.effect.is_empty();
                match &o.kind {
                    Kind::Tilemap { tileset, .. } => imgs.push(tileset.clone()),
                    Kind::Sprite { animations, .. } => imgs.extend(animations.first().and_then(|a| a.frames.first()).cloned()),
                    Kind::Background { image, .. } => imgs.push(image.clone()),
                    Kind::Particles(d) => imgs.push(d.image.clone()),
                    Kind::Text { font: f, .. } => font = f.clone(),
                    _ => {}
                }
            }
            let ctx = ui.ctx().clone();
            self.fuente(&ctx, &font);
            let mut tex: HashMap<String, egui::TextureHandle> = HashMap::new();
            for f in imgs.iter().filter(|f| !f.is_empty()) {
                if let Some(t) = self.texture(&ctx, f) {
                    tex.insert(f.clone(), t);
                }
                if fx {
                    for (k, var) in [("gris", 1), ("blanco", 2)] {
                        if let Some(t) = self.tex_var(&ctx, f, var) {
                            tex.insert(format!("{k}:{f}"), t);
                        }
                    }
                }
            }
            let mut sim = self.psim.remove(&name).unwrap_or_default();
            let mut ost = std::mem::take(&mut self.origen_st);
            let abrir_origen = std::mem::take(&mut self.abrir_origen);
            let mut act = None;
            let (facil, dt, t, pantalla) = (self.facil, self.dt, self.t as f32, Vec2::new(self.p.width, self.p.height));
            if let Some(o) = self.obj_mut(&name) {
                let mut cx = PropCx {
                    base: &base, files: &files, tex: &tex, usos: usos.as_ref(), facil, dt, t, sim: &mut sim, origen: &mut ost, pantalla, abrir_origen,
                };
                act = obj_props(ui, o, &mut nb, &mut cx);
            }
            self.origen_st = ost;
            self.psim.insert(name.clone(), sim);
            self.nb[3] = nb;
            match act {
                Some(PropAct::Dibujar(a)) => self.abrir_dibujo(dibujo::Destino::Anim { obj: name.clone(), anim: a }),
                Some(PropAct::DibujarBaldosas) => self.abrir_dibujo(dibujo::Destino::Baldosas { obj: name.clone() }),
                Some(PropAct::CrearBaldosas) => self.usar_baldosas_ejemplo(&name),
                Some(PropAct::Importar(tipo, campo)) => {
                    if let Some(r) = self.elegir_importar(tipo) {
                        match (self.obj_mut(&name).map(|o| &mut o.kind), campo) {
                            (Some(Kind::Background { image, .. }), Campo::Fondo) => *image = r,
                            (Some(Kind::Particles(d)), Campo::Particulas) => d.image = r,
                            (Some(Kind::Text { font, .. }), Campo::Fuente) => *font = r,
                            _ => {}
                        }
                    }
                }
                None => {}
            }
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
                self.sel_mas.clear();
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

    /// Instancia seleccionada que es un mapa de baldosas, con el nombre del objeto.
    fn mapa_sel(&self) -> Option<(usize, String)> {
        let i = self.sel_valid().filter(|_| self.seleccion().len() <= 1)?;
        let s = &self.p.scenes[self.sc];
        let o = self.p.objects_of(s).find(|o| o.name == s.instances[i].obj)?;
        matches!(o.kind, Kind::Tilemap { .. }).then(|| (i, o.name.clone()))
    }

    /// ¿Se está pintando un mapa en el lienzo?
    fn pintando(&self) -> Option<(usize, String)> {
        self.mapa_sel().filter(|_| self.pintor.on && self.tab == Tab::Escena)
    }

    /// Panel con herramientas y paleta de baldosas del mapa seleccionado.
    fn panel_mapa(&mut self, ui: &mut Ui, name: &str) {
        let ctx = ui.ctx().clone();
        let Some(Kind::Tilemap { tileset, tile, .. }) = self.obj_mut(name).map(|o| o.kind.clone()) else { return };
        let tex = self.texture(&ctx, &tileset);
        let mut pass = match self.obj_mut(name).map(|o| &mut o.kind) {
            Some(Kind::Tilemap { pass, .. }) => std::mem::take(pass),
            _ => vec![],
        };
        let mut listo = false;
        let frame = Frame::new().fill(PANEL).inner_margin(12);
        egui::Panel::left("paleta_mapa").exact_size(262.).resizable(false).frame(frame).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                header(ui, ic::MAP, "Pintar mapa", name);
                let on = self.pintor.on;
                ui.horizontal(|ui| {
                    seg(ui, |ui| {
                        if seg_btn(ui, on, it(ic::PAINT_BRUSH, "Pintar")).on_hover_text("Pintar baldosas en el lienzo").clicked() {
                            self.pintor.on = true;
                        }
                        if seg_btn(ui, !on, it(ic::CURSOR, "Mover")).on_hover_text("Seleccionar y mover objetos").clicked() {
                            self.pintor.on = false;
                        }
                    })
                });
                if self.facil {
                    ui.add_space(2.);
                    ui.label(RichText::new(mapa::TEXTO_NINOS).color(TEXT_SOFT));
                }
                ui.add_space(6.);
                ui.label(RichText::new("Herramientas").font(sb(13.)).color(TEXT_MUTED));
                mapa::herramientas(ui, &mut self.pintor);
                ui.add_space(6.);
                ui.label(RichText::new("Baldosas").font(sb(13.)).color(TEXT_MUTED));
                if tex.is_none() {
                    ui.label(muted(if tileset.is_empty() { "Sin imagen de baldosas: elige una en las propiedades." } else { "No se encontró la imagen de baldosas." }));
                }
                mapa::paleta(ui, tex.as_ref(), tile, &mut pass, &mut self.pintor.baldosa);
                ui.add_space(8.);
                if ui.add(Button::new(it(ic::CHECK, "Listo")).min_size(Vec2::new(ui.available_width(), 32.))).on_hover_text("Terminar de pintar (Esc)").clicked() {
                    listo = true;
                }
                ui.add_space(4.);
                ui.label(RichText::new("Clic derecho: borrar · Rueda: zoom · Botón del medio: mover la vista").small().color(TEXT_MUTED));
            });
        });
        if let Some(Kind::Tilemap { pass: p, .. }) = self.obj_mut(name).map(|o| &mut o.kind) {
            *p = pass;
        }
        if listo {
            self.elegir(None);
        }
    }

    /// Pintado del mapa: rejilla, vista previa bajo el cursor y trazos (un paso de deshacer por trazo).
    fn pintar_mapa(&mut self, ui: &Ui, resp: &egui::Response, painter: &egui::Painter, ii: usize, name: &str, org: Pos2, z: f32) {
        let ins = &self.p.scenes[self.sc].instances[ii];
        let (pos, iw, ih) = (Vec2::new(ins.x, ins.y), ins.w, ins.h);
        let Some(Kind::Tilemap { tileset, tile, cols, rows, .. }) = self.obj_mut(name).map(|o| o.kind.clone()) else { return };
        let (cols, rows) = (cols.max(1), rows.max(1));
        let sz = Vec2::new(iw.unwrap_or(cols as f32 * tile), ih.unwrap_or(rows as f32 * tile));
        let (o_s, cs) = (org + pos * z, Vec2::new(sz.x / cols as f32, sz.y / rows as f32) * z);
        let tex = self.tex.get(&tileset).cloned().flatten();
        mapa::rejilla(painter, o_s, cs, cols, rows);
        let cell = |p: Pos2| (((p.x - o_s.x) / cs.x).floor() as i64, ((p.y - o_s.y) / cs.y).floor() as i64);
        let dentro = |c: (i64, i64)| c.0 >= 0 && c.1 >= 0 && (c.0 as usize) < cols && (c.1 as usize) < rows;
        let crect = |c: (i64, i64)| Rect::from_min_size(o_s + Vec2::new(c.0 as f32 * cs.x, c.1 as f32 * cs.y), cs);
        let hover = resp.hover_pos().map(cell).filter(|c| dentro(*c));
        let pt = &self.pintor;
        match (pt.trazo, hover) {
            (Some(t), _) if pt.herr == mapa::Herr::Rect => {
                let a = (t.inicio.0.min(t.ultima.0).max(0), t.inicio.1.min(t.ultima.1).max(0));
                let b = (t.inicio.0.max(t.ultima.0).min(cols as i64 - 1), t.inicio.1.max(t.ultima.1).min(rows as i64 - 1));
                let r = crect(a).union(crect(b));
                let c = if t.borrar { DANGER } else { ACCENT };
                if !t.borrar {
                    for y in a.1..=b.1 {
                        for x in a.0..=b.0 {
                            mapa::baldosa(painter, crect((x, y)), pt.baldosa, tile, tex.as_ref(), Color32::from_white_alpha(150));
                        }
                    }
                }
                painter.rect_filled(r, 0., c.gamma_multiply(0.15));
                painter.rect_stroke(r, 0., Stroke::new(2., c), egui::StrokeKind::Inside);
            }
            (_, Some(c)) => {
                let r = crect(c);
                match pt.herr {
                    mapa::Herr::Borrador => {
                        painter.rect_filled(r, 0., DANGER.gamma_multiply(0.25));
                        painter.rect_stroke(r, 0., Stroke::new(2., DANGER), egui::StrokeKind::Inside);
                    }
                    mapa::Herr::Gotero => drop(painter.rect_stroke(r, 0., Stroke::new(2., Color32::WHITE), egui::StrokeKind::Inside)),
                    _ => {
                        mapa::baldosa(painter, r, pt.baldosa, tile, tex.as_ref(), Color32::from_white_alpha(170));
                        painter.rect_stroke(r, 0., Stroke::new(2., Color32::WHITE), egui::StrokeKind::Inside);
                    }
                }
                ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
            }
            _ => {}
        }
        // Trazos: desde que se aprieta el botón sobre el lienzo hasta que se suelta.
        let down = resp.is_pointer_button_down_on();
        let (prim, sec, p) = ui.input(|i| (i.pointer.primary_down(), i.pointer.secondary_down(), i.pointer.latest_pos()));
        let (herr, baldosa, trazo) = (self.pintor.herr, self.pintor.baldosa, self.pintor.trazo);
        let Some(Kind::Tilemap { data, .. }) = self.obj_mut(name).map(|o| &mut o.kind) else { return };
        match (down && (prim || sec), trazo, p.map(cell)) {
            (true, None, Some(c)) if dentro(c) => {
                let borrar = (sec && !prim) || herr == mapa::Herr::Borrador;
                let v = if borrar { -1 } else { baldosa };
                match herr {
                    mapa::Herr::Pincel | mapa::Herr::Borrador => drop(mapa::poner(data, cols, rows, c.0, c.1, v)),
                    mapa::Herr::Rellenar => drop(mapa::rellenar(data, cols, rows, c.0, c.1, v)),
                    mapa::Herr::Gotero if !borrar => {
                        let t = data.get(c.1 as usize * cols + c.0 as usize).copied().unwrap_or(-1);
                        if t >= 0 {
                            self.pintor.baldosa = t;
                            self.pintor.herr = mapa::Herr::Pincel;
                        }
                    }
                    _ => {}
                }
                self.pintor.trazo = Some(mapa::Trazo { inicio: c, ultima: c, borrar });
            }
            (true, Some(t), Some(c)) => {
                let v = if t.borrar { -1 } else { baldosa };
                if matches!(herr, mapa::Herr::Pincel | mapa::Herr::Borrador) && c != t.ultima {
                    mapa::poner_linea(data, cols, rows, t.ultima, c, v);
                }
                self.pintor.trazo = Some(mapa::Trazo { ultima: c, ..t });
            }
            (false, Some(t), _) => {
                if herr == mapa::Herr::Rect {
                    mapa::llenar_rect(data, cols, rows, t.inicio, t.ultima, if t.borrar { -1 } else { baldosa });
                }
                self.pintor.trazo = None;
            }
            _ => {}
        }
    }

    fn canvas(&mut self, ui: &mut Ui) {
        if let Some((_, name)) = self.mapa_sel() {
            self.panel_mapa(ui, &name);
        }
        let pintando = self.pintando();
        if pintando.is_none() {
            self.pintor.trazo = None;
        }
        Frame::new().inner_margin(Margin::symmetric(8, 6)).show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some((_, n)) = &pintando {
                    ui.label(RichText::new(ic::PAINT_BRUSH).size(16.).color(ACCENT));
                    ui.label(RichText::new(format!("Pintando «{n}» con {}", self.pintor.herr.nombre().to_lowercase())).font(sb(14.)));
                    ui.add_space(8.);
                } else {
                    seg(ui, |ui| {
                        if seg_btn(ui, !self.placing, it(ic::CURSOR, "Seleccionar")).on_hover_text("Seleccionar y mover instancias").clicked() {
                            self.placing = false;
                        }
                        if seg_btn(ui, self.placing, it(ic::PLUS, "Colocar")).on_hover_text("Clic en el lienzo crea una instancia del objeto seleccionado").clicked() {
                            self.placing = true;
                        }
                    });
                }
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
                let n_sel = self.seleccion().len();
                if n_sel >= 2 && pintando.is_none() {
                    let f = self.facil;
                    seg(ui, |ui| {
                        for (m, icon, kid, exp) in [
                            (seleccion::Alinear::Izquierda, ic::ALIGN_LEFT, "Juntar a la izquierda", "Alinear a la izquierda"),
                            (seleccion::Alinear::CentroH, ic::ALIGN_CENTER_H, "Centrar de costado", "Centrar horizontalmente"),
                            (seleccion::Alinear::Derecha, ic::ALIGN_RIGHT, "Juntar a la derecha", "Alinear a la derecha"),
                            (seleccion::Alinear::Arriba, ic::ALIGN_TOP, "Juntar arriba", "Alinear arriba"),
                            (seleccion::Alinear::CentroV, ic::ALIGN_CENTER_V, "Centrar de alto", "Centrar verticalmente"),
                            (seleccion::Alinear::Abajo, ic::ALIGN_BOTTOM, "Juntar abajo", "Alinear abajo"),
                        ] {
                            if seg_btn(ui, false, RichText::new(icon).size(16.)).on_hover_text(if f { kid } else { exp }).clicked() {
                                self.alinear(Some(m), None);
                            }
                        }
                    });
                    seg(ui, |ui| {
                        let ok = n_sel >= 3;
                        for (h, icon, kid, exp) in [(true, ic::DISTRIBUTE_H, "Separar parejo a lo ancho", "Distribuir horizontalmente"), (false, ic::DISTRIBUTE_V, "Separar parejo a lo alto", "Distribuir verticalmente")] {
                            let tip = if ok { if f { kid } else { exp } } else { "Elige 3 o más" };
                            if ui.add_enabled(ok, Button::selectable(false, RichText::new(icon).size(16.)).frame_when_inactive(false)).on_hover_text(tip).on_disabled_hover_text(tip).clicked() {
                                self.alinear(None, Some(h));
                            }
                        }
                    });
                }
                let hint = if pintando.is_some() { "Rueda: zoom. Botón del medio: mover la vista" } else { "Rueda: zoom · Botón derecho: mover la vista · Shift+clic o rectángulo: elegir varias" };
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add(egui::Label::new(RichText::new(hint).small().color(TEXT_MUTED)).truncate());
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
        if resp.dragged_by(PointerButton::Middle) || (resp.dragged_by(PointerButton::Secondary) && pintando.is_none()) {
            self.pan += resp.drag_delta();
            self.fit = false;
        }
        let (org, z) = (r.min + self.pan, self.zoom);
        let to_s = |p: Vec2| org + p * z;
        let to_w = |p: Pos2| ((p - org) / z).to_pos2();
        self.cursor_w = resp.hover_pos().map(to_w);

        // Precarga de imágenes y fuentes, y lista de dibujo ordenada por capa y z.
        let s = &self.p.scenes[self.sc];
        let (mut frames, mut fonts, mut fx_imgs) = (vec![], vec![], vec![]);
        for o in self.p.objects_of(s) {
            match &o.kind {
                Kind::Sprite { animations, .. } => {
                    let f = animations.first().and_then(|a| a.frames.first()).cloned();
                    if !o.effect.is_empty() {
                        fx_imgs.extend(f.clone());
                    }
                    frames.extend(f);
                }
                Kind::Tilemap { tileset, .. } if !tileset.is_empty() => frames.push(tileset.clone()),
                Kind::Background { image, .. } if !image.is_empty() => frames.push(image.clone()),
                Kind::Particles(d) if !d.image.is_empty() => frames.push(d.image.clone()),
                Kind::Text { font, .. } if !font.is_empty() => fonts.push(font.clone()),
                _ => {}
            }
        }
        let ctx = ui.ctx().clone();
        frames.iter().for_each(|f| drop(self.texture(&ctx, f)));
        fonts.iter().for_each(|f| self.fuente(&ctx, f));
        let variantes: HashMap<String, (Option<egui::TextureHandle>, Option<egui::TextureHandle>)> =
            fx_imgs.iter().map(|f| (f.clone(), (self.tex_var(&ctx, f, 1), self.tex_var(&ctx, f, 2)))).collect();
        let n_ins = self.p.scenes[self.sc].instances.len();
        let geos: Vec<Option<seleccion::Geo>> = (0..n_ins).map(|i| self.geo(i)).collect();
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
        let mut items: Vec<(usize, seleccion::Geo, bool)> = vec![];
        let t_anim = self.t as f32;
        for &i in &order {
            let ins = &s.instances[i];
            let Some(g) = geos[i] else { continue };
            let qs = g.quad().map(to_s);
            let sr = Rect::from_min_max(to_s(g.aabb().min.to_vec2()), to_s(g.aabb().max.to_vec2()));
            let Some(o) = self.p.objects_of(s).find(|o| o.name == ins.obj) else {
                painter.add(egui::Shape::closed_line(qs.to_vec(), Stroke::new(2., DANGER)));
                items.push((i, g, false));
                continue;
            };
            let fx = o.effect.as_str();
            match &o.kind {
                Kind::Background { image, .. } => match self.tex.get(image).cloned().flatten() {
                    Some(t) => {
                        vista::mosaico(&painter, sr, &t, t.size_vec2() * z, Vec2::ZERO, Color32::WHITE);
                        painter.rect_stroke(sr, 0., Stroke::new(1., Color32::from_white_alpha(30)), egui::StrokeKind::Inside);
                    }
                    None => {
                        painter.rect_filled(sr, 0., SURFACE_HOVER);
                        painter.text(sr.center(), Align2::CENTER_CENTER, format!("{}  Fondo sin imagen", ic::MOUNTAINS), egui::FontId::proportional(14.), TEXT_MUTED);
                    }
                },
                Kind::Particles(d) => {
                    let c = to_s(g.pos);
                    let tex = self.tex.get(&d.image).cloned().flatten();
                    match self.psim.get(&o.name).filter(|_| self.sel_obj.as_deref() == Some(o.name.as_str())) {
                        Some(sim) => sim.dibujar(&painter, c, z, d, tex.as_ref()),
                        None => particulas::icono_emisor(&painter, c, z),
                    }
                }
                // Vista desde arriba de los objetos 3D (el visor 3D llega en la etapa F7).
                Kind::Mesh { color: [r, g, b], .. } => {
                    painter.rect_filled(sr, 3., Color32::from_rgb(*r, *g, *b));
                    painter.text(sr.center(), Align2::CENTER_CENTER, ic::CUBE, egui::FontId::proportional((sr.height() * 0.5).clamp(8., 28.)), Color32::from_black_alpha(140));
                }
                Kind::Light { color: [r, g, b], .. } => drop(painter.text(sr.center(), Align2::CENTER_CENTER, ic::LIGHTBULB, egui::FontId::proportional(22. * z), Color32::from_rgb(*r, *g, *b))),
                Kind::Camera3d { .. } => drop(painter.text(sr.center(), Align2::CENTER_CENTER, ic::CAMERA, egui::FontId::proportional(22. * z), TEXT_SOFT)),
                Kind::Box { color, .. } => {
                    let [r, g, b] = vista::color_fx(*color, fx, 1.);
                    painter.add(egui::Shape::convex_polygon(qs.to_vec(), Color32::from_rgb(r, g, b), Stroke::NONE));
                    if fx == "outline" {
                        painter.add(egui::Shape::closed_line(qs.to_vec(), Stroke::new(2., Color32::WHITE)));
                    }
                }
                Kind::Text { text, size, color, font, align, wrap } => {
                    let [r, gg, b] = vista::color_fx(*color, fx, 1.);
                    let c = Color32::from_rgb(r, gg, b);
                    let (gal, dx) = vista::texto(&painter, text.clone(), size * z, c, vista::familia(&self.fuentes, font), align, wrap * z);
                    let at = qs[0] + seleccion::rot(Vec2::new(dx, 0.), g.angle);
                    let ts = egui::epaint::TextShape::new(at, gal, c);
                    painter.add(if g.angle != 0. { ts.with_angle(g.angle.to_radians()) } else { ts });
                }
                Kind::Sprite { animations, .. } => {
                    let f = animations.first().and_then(|a| a.frames.first());
                    match f.and_then(|f| self.tex.get(f).cloned().flatten()) {
                        Some(t) => {
                            let (gris, blanco) = f.and_then(|f| variantes.get(f).cloned()).unwrap_or((None, None));
                            let tx = vista::Texs { n: &t, gris: gris.as_ref(), blanco: blanco.as_ref() };
                            vista::sprite_fx(&painter, qs, false, &tx, Color32::WHITE, fx, 1., t_anim);
                        }
                        None => {
                            // Sin imagen: marcador neutro con ícono.
                            painter.add(egui::Shape::convex_polygon(qs.to_vec(), SURFACE_HOVER, Stroke::new(1., BORDER_STRONG)));
                            painter.text(sr.center(), Align2::CENTER_CENTER, ic::IMAGE, egui::FontId::proportional((sr.height() * 0.5).clamp(8., 32.)), TEXT_FAINT);
                        }
                    }
                }
                Kind::Tilemap { tileset, tile, cols, rows, data, .. } => {
                    let tex = self.tex.get(tileset).cloned().flatten();
                    let (c, r) = ((*cols).max(1), (*rows).max(1));
                    let mr = Rect::from_min_size(to_s(g.pos), g.size * z);
                    mapa::dibujar(&painter, mr.min, Vec2::new(mr.width() / c as f32, mr.height() / r as f32), *cols, *rows, data, *tile, tex.as_ref());
                    if data.iter().all(|v| *v < 0) {
                        painter.rect_stroke(mr, 0., Stroke::new(1., BORDER_STRONG), egui::StrokeKind::Inside);
                        painter.text(mr.center(), Align2::CENTER_CENTER, format!("{}  Mapa vacío: selecciónalo y píntalo", ic::MAP), egui::FontId::proportional(14.), TEXT_MUTED);
                    }
                }
            }
            // Mapas y fondos suelen cubrir toda la escena: se marcan para no taparle el clic a lo demás.
            items.push((i, g, matches!(o.kind, Kind::Tilemap { .. } | Kind::Background { .. })));
        }
        if s.instances.is_empty() {
            let t = if self.p.objects_of(s).next().is_some() { "Arrastra un objeto desde la lista para colocarlo" } else { "Crea un objeto en el panel izquierdo para empezar" };
            painter.text(game.center(), Align2::CENTER_CENTER, t, egui::FontId::proportional(14.), TEXT_MUTED);
        }
        // Los mapas solo se eligen con un clic si no hay otra cosa debajo.
        let hit = |p: Pos2| {
            let v = p.to_vec2();
            items.iter().rev().find(|(_, g, m)| !m && g.contiene(v)).or_else(|| items.iter().rev().find(|(_, g, _)| g.contiene(v))).map(|x| x.0)
        };
        let quad_s = |g: &seleccion::Geo| g.quad().map(to_s).to_vec();
        self.hover_inst = self.cursor_w.and_then(hit);
        let sel = self.seleccion();
        if let Some((_, g, _)) = items.iter().find(|x| Some(x.0) == self.hover_inst && !sel.contains(&x.0)) {
            painter.add(egui::Shape::closed_line(quad_s(g), Stroke::new(1., BORDER_STRONG)));
        }
        for (_, g, _) in items.iter().filter(|x| sel.contains(&x.0)) {
            painter.add(egui::Shape::closed_line(quad_s(g), Stroke::new(1.5, ACCENT)));
        }
        // Manijas: esquinas para escalar y un círculo arriba para girar.
        let sel_geos: Vec<(usize, seleccion::Geo)> = sel.iter().filter_map(|&i| geos.get(i).copied().flatten().map(|g| (i, g))).collect();
        let caja_sel = seleccion::union(&sel_geos.iter().map(|(_, g)| g.aabb()).collect::<Vec<_>>());
        let manijas: Option<([Pos2; 4], Pos2, Pos2)> = match (sel_geos.as_slice(), caja_sel) {
            _ if pintando.is_some() => None,
            ([(_, g)], _) => {
                let q = g.quad().map(to_s);
                let arriba = ((q[0] - q[3]).normalized()) * 24.;
                let mid = q[0] + (q[1] - q[0]) / 2.;
                Some((q, mid, mid + arriba))
            }
            (_, Some(c)) if sel_geos.len() > 1 => {
                let c = seleccion::a_pantalla(c, org, z).expand(4.);
                let q = [c.left_top(), c.right_top(), c.right_bottom(), c.left_bottom()];
                painter.rect_stroke(c, 2., Stroke::new(1., ACCENT.gamma_multiply(0.6)), egui::StrokeKind::Middle);
                Some((q, c.center_top(), c.center_top() - Vec2::new(0., 24.)))
            }
            _ => None,
        };
        if let Some((q, mid, knob)) = manijas {
            painter.line_segment([mid, knob], Stroke::new(1.5, ACCENT));
            painter.circle_filled(knob, 6., ACCENT);
            painter.circle_stroke(knob, 6., Stroke::new(1.5, BG));
            for c in q {
                let h = Rect::from_center_size(c, Vec2::splat(8.));
                painter.rect_filled(h, 1, ACCENT);
                painter.rect_stroke(h, 1, Stroke::new(1., BG), egui::StrokeKind::Outside);
            }
        }
        // ¿Qué manija hay bajo el puntero? (0..4 esquinas, 4 = girar)
        let manija_en = |p: Pos2| -> Option<usize> {
            let (q, _, knob) = manijas?;
            if (p - knob).length() <= 9. {
                return Some(4);
            }
            q.iter().position(|c| (p - *c).length() <= 8.)
        };
        if let Some(Arrastre::Caja { desde, hasta, .. }) = &self.arrastre {
            let rr = Rect::from_two_pos(to_s(*desde), to_s(*hasta));
            painter.rect_filled(rr, 0., ACCENT.gamma_multiply(0.08));
            painter.rect_stroke(rr, 0., Stroke::new(1., ACCENT), egui::StrokeKind::Middle);
        }

        // Interacción
        if let Some((ii, name)) = pintando {
            self.pintar_mapa(ui, &resp, &painter, ii, &name, org, z);
            if let (Some(o), Some(p)) = (resp.dnd_release_payload::<String>(), resp.hover_pos()) {
                self.add_instance((*o).clone(), to_w(p));
            }
            return;
        }
        let sc = self.sc;
        let shift = ui.input(|i| i.modifiers.shift);
        if resp.drag_started_by(PointerButton::Primary) {
            if let Some(ps) = ui.input(|i| i.pointer.press_origin()) {
                let pw_ = to_w(ps).to_vec2();
                self.arrastre = match manija_en(ps) {
                    Some(4) => {
                        let centro = match sel_geos.as_slice() {
                            [(_, g)] => g.pivot(),
                            _ => caja_sel.map_or(pw_, |c| c.center().to_vec2()),
                        };
                        let d = pw_ - centro;
                        Some(Arrastre::Girar { centro, a0: d.y.atan2(d.x).to_degrees(), ini: sel_geos.clone() })
                    }
                    Some(k) => caja_sel.map(|c| Arrastre::Escalar { esq: k, ini: sel_geos.clone(), caja: c }),
                    // Arrastrar sobre un mapa o fondo que no está elegido dibuja el rectángulo de selección.
                    None => match hit(to_w(ps)).filter(|i| sel.contains(i) || !items.iter().any(|x| x.0 == *i && x.2)) {
                        Some(i) => {
                            if shift && !sel.contains(&i) {
                                self.alternar(i);
                            } else if !sel.contains(&i) {
                                self.elegir(Some(i));
                            } else {
                                self.sel_inst = Some(i);
                                self.sel_obj = Some(self.p.scenes[sc].instances[i].obj.clone());
                            }
                            let v = self.seleccion();
                            let ini = v.iter().map(|&k| (k, Vec2::new(self.p.scenes[sc].instances[k].x, self.p.scenes[sc].instances[k].y))).collect();
                            Some(Arrastre::Mover { ini, desde: pw_ })
                        }
                        None => Some(Arrastre::Caja { desde: pw_, hasta: pw_, sumar: shift }),
                    },
                };
            }
        }
        if let (true, Some(p)) = (resp.dragged_by(PointerButton::Primary), resp.interact_pointer_pos()) {
            let pw_ = to_w(p).to_vec2();
            let (snap, grid) = (self.snap, self.grid);
            let insts = &mut self.p.scenes[sc].instances;
            match &mut self.arrastre {
                Some(Arrastre::Mover { ini, desde }) => {
                    let mut d = pw_ - *desde;
                    if let (true, Some((_, base))) = (snap, ini.first()) {
                        d = ((*base + d) / grid).round() * grid - *base;
                    }
                    for (k, p0) in ini.iter() {
                        if let Some(n) = insts.get_mut(*k) {
                            (n.x, n.y) = (p0.x + d.x, p0.y + d.y);
                        }
                    }
                }
                Some(Arrastre::Escalar { esq, ini, caja }) => {
                    let res: Vec<(usize, Vec2, Vec2)> = if ini.len() == 1 {
                        let (pos, size) = seleccion::escalar_una(&ini[0].1, *esq, pw_, shift);
                        vec![(ini[0].0, pos, size)]
                    } else {
                        seleccion::escalar_varias(ini, *caja, *esq, pw_, shift)
                    };
                    for (k, pos, size) in res {
                        if let Some(n) = insts.get_mut(k) {
                            (n.x, n.y) = (pos.x.round(), pos.y.round());
                            (n.w, n.h) = (Some(size.x.round().max(1.)), Some(size.y.round().max(1.)));
                        }
                    }
                }
                Some(Arrastre::Girar { centro, a0, ini }) => {
                    for (k, pos, ang) in seleccion::girar(ini, *centro, *a0, pw_, shift) {
                        if let Some(n) = insts.get_mut(k) {
                            (n.x, n.y, n.angle) = (pos.x, pos.y, ang);
                        }
                    }
                }
                Some(Arrastre::Caja { hasta, .. }) => *hasta = pw_,
                None => {}
            }
        }
        if resp.drag_stopped() {
            if let Some(Arrastre::Caja { desde, hasta, sumar }) = self.arrastre.take() {
                let rr = Rect::from_two_pos(desde.to_pos2(), hasta.to_pos2());
                // Los mapas (que suelen cubrir todo) solo entran si quedan enteros dentro del rectángulo.
                let dentro = items.iter().filter(|(_, g, m)| if *m { rr.contains_rect(g.aabb()) } else { rr.intersects(g.aabb()) }).map(|x| x.0);
                let mut v = if sumar { self.seleccion() } else { vec![] };
                v.extend(dentro);
                v.sort_unstable();
                v.dedup();
                self.poner_seleccion(v);
            }
            self.arrastre = None;
        }
        if resp.hovered() {
            let manija = resp.hover_pos().and_then(manija_en);
            let esq = match &self.arrastre {
                Some(Arrastre::Escalar { esq, .. }) => Some(*esq),
                None => manija.filter(|k| *k < 4),
                _ => None,
            };
            let icon = match &self.arrastre {
                Some(Arrastre::Mover { .. }) => Some(egui::CursorIcon::Grabbing),
                Some(Arrastre::Girar { .. }) => Some(egui::CursorIcon::Alias),
                Some(Arrastre::Caja { .. }) => Some(egui::CursorIcon::Crosshair),
                _ if esq.is_some() => Some(if matches!(esq, Some(0 | 2)) { egui::CursorIcon::ResizeNwSe } else { egui::CursorIcon::ResizeNeSw }),
                None if manija == Some(4) => Some(egui::CursorIcon::Alias),
                None if self.hover_inst.is_some() && !self.placing => Some(egui::CursorIcon::Grab),
                _ => None,
            };
            if let Some(c) = icon {
                ui.ctx().set_cursor_icon(c);
            }
        }
        if resp.clicked() {
            if let Some(p) = resp.interact_pointer_pos().map(to_w) {
                match self.sel_obj.clone() {
                    Some(o) if self.placing => self.add_instance(o, p),
                    _ => match hit(p) {
                        Some(i) if shift => self.alternar(i),
                        h => self.elegir(h),
                    },
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
        if self.jump.as_ref().is_some_and(|j| j.1 < self.t) {
            self.jump = None;
        }
        eventos::set_salto(self.jump.as_ref().map(|j| j.0.clone()));
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
            eventos::set_salto(None);
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
                self.eventos_o_nodos(ui, Target::Func(fi));
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
                        s.graphs.iter_mut().chain([&mut s.node_drafts]).for_each(|g| nodos::ren_var_grafo(g, &old, &new));
                    }
                }
                self.p.functions.iter_mut().for_each(|f| cajitas::ren_var(&mut f.events, &old, &new));
                self.p.functions.iter_mut().flat_map(|f| f.graph.iter_mut().chain([&mut f.node_drafts])).for_each(|g| nodos::ren_var_grafo(g, &old, &new));
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
                self.p.functions.push(Function { name: n, is_condition: false, params: vec![], events: vec![], ..Default::default() });
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
                self.eventos_o_nodos(ui, Target::Func(self.func));
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

    fn ajustes_ui(&mut self, ctx: &egui::Context) {
        if !self.ajustes {
            return;
        }
        if self.files_at.is_none_or(|t| t.elapsed().as_secs() >= 5) {
            self.files = buscar_archivos(&self.base_dir());
            self.files_at = Some(std::time::Instant::now());
        }
        let imgs: Vec<String> = self.files.iter().filter(|f| recursos::Tipo::de(f) == Some(recursos::Tipo::Imagen)).cloned().collect();
        let mut tex = HashMap::new();
        for f in [self.p.settings.icon.clone(), self.p.settings.loading_image.clone()] {
            if let Some(t) = self.texture(ctx, &f) {
                tex.insert(f, t);
            }
        }
        match ajustes::dialogo(ctx, &mut self.p, &imgs, &tex, self.facil) {
            Some(ajustes::Pedido::Cerrar) => self.ajustes = false,
            Some(ajustes::Pedido::ElegirIcono) => {
                if let Some(r) = self.elegir_importar(recursos::Tipo::Imagen) {
                    self.p.settings.icon = r;
                }
            }
            Some(ajustes::Pedido::ElegirCarga) => {
                if let Some(r) = self.elegir_importar(recursos::Tipo::Imagen) {
                    self.p.settings.loading_image = r;
                }
            }
            None => {}
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
            if self.tab == Tab::Escena && self.inicio.is_none() {
                ui.spacing_mut().item_spacing.x = 16.;
                ui.label(RichText::new(format!("{:.0} %", self.zoom * 100.)).monospace().color(TEXT_MUTED)).on_hover_text("Zoom");
                let c = self.cursor_w.map_or("x -    y -".into(), |p| format!("x {:<5.0}y {:<5.0}", p.x, p.y));
                ui.label(RichText::new(c).monospace().color(TEXT_MUTED)).on_hover_text("Posición del cursor en la escena");
                let v = self.seleccion();
                let n = match v.len() {
                    0 => "Sin selección".to_string(),
                    1 => format!("1 seleccionada ({})", self.p.scenes[self.sc].instances[v[0]].obj),
                    n => format!("{n} seleccionadas"),
                };
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
    /// Pregunta después de «+ Sprite» (dibujar o elegir imágenes) y «+ Mapa» (qué baldosas usar).
    fn nuevo_ui(&mut self, ctx: &egui::Context) {
        let Some(nv) = self.nuevo.as_ref() else { return };
        let (mapa_, name) = match nv {
            Nuevo::Sprite(n) => (false, n.clone()),
            Nuevo::Mapa(n) => (true, n.clone()),
        };
        if self.obj_mut(&name).is_none() {
            self.nuevo = None;
            return;
        }
        if self.files_at.is_none_or(|t| t.elapsed().as_secs() >= 5) {
            self.files = buscar_archivos(&self.base_dir());
            self.files_at = Some(std::time::Instant::now());
        }
        let imgs: Vec<String> = self.files.iter().filter(|f| [".png", ".jpg", ".jpeg"].iter().any(|x| f.to_lowercase().ends_with(x))).cloned().collect();
        let thumbs: Vec<(String, Option<egui::TextureHandle>)> = if mapa_ { imgs.iter().take(12).map(|f| (f.clone(), self.texture(ctx, f))).collect() } else { vec![] };
        let guardado = self.path.is_some();
        #[derive(PartialEq)]
        enum A {
            Dibujar,
            Elegir,
            Ejemplo,
            Usar(String),
            Guardar,
            Cerrar,
        }
        let mut act = None;
        let frame = Frame::window(&ctx.global_style()).inner_margin(20).corner_radius(12);
        let r = egui::Modal::new(egui::Id::unique("nuevo_objeto")).frame(frame).show(ctx, |ui| {
            ui.set_width(520.);
            ui.horizontal(|ui| {
                ui.label(RichText::new(if mapa_ { ic::MAP } else { ic::IMAGE }).size(22.).color(ACCENT));
                ui.label(RichText::new(if mapa_ { format!("Baldosas para «{name}»") } else { format!("Tu sprite «{name}»") }).font(sb(20.)));
            });
            ui.label(RichText::new(if mapa_ { mapa::TEXTO_NINOS } else { "¿Cómo quieres que se vea? Puedes dibujarlo pixel por pixel o usar imágenes que ya tengas." }).color(TEXT_SOFT));
            ui.add_space(8.);
            if !guardado {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(ic::WARNING).color(WARN));
                    ui.label("Para crear imágenes, primero guarda el proyecto.");
                    if ui.button(it(ic::FLOPPY, "Guardar proyecto…")).clicked() {
                        act = Some(A::Guardar);
                    }
                });
                ui.add_space(6.);
            }
            ui.spacing_mut().button_padding = Vec2::new(14., 9.);
            ui.horizontal_wrapped(|ui| {
                if mapa_ {
                    ui.add_enabled_ui(guardado, |ui| {
                        if primary(ui, (on_accent(ic::SPARKLE).size(17.), on_accent("Crear baldosas de ejemplo").font(sb(15.)))).on_hover_text("Pasto, tierra, piedra, ladrillo, madera, agua, nube y hojas").clicked() {
                            act = Some(A::Ejemplo);
                        }
                        if ui.button((RichText::new(ic::PALETTE).size(17.).color(ACCENT), RichText::new("Dibujar baldosas").font(sb(15.)))).clicked() {
                            act = Some(A::Dibujar);
                        }
                    });
                } else {
                    ui.add_enabled_ui(guardado, |ui| {
                        if primary(ui, (on_accent(ic::PALETTE).size(17.), on_accent("Dibujar uno nuevo").font(sb(15.)))).clicked() {
                            act = Some(A::Dibujar);
                        }
                    });
                }
                if ui.button((RichText::new(ic::FOLDER_OPEN).size(17.).color(TEXT_MUTED), RichText::new(if mapa_ { "Elegir archivo…" } else { "Elegir imágenes…" }).font(sb(15.)))).clicked() {
                    act = Some(A::Elegir);
                }
            });
            if mapa_ && !thumbs.is_empty() {
                ui.add_space(8.);
                ui.label(RichText::new("O usa una imagen del proyecto").font(sb(14.)).color(TEXT_MUTED));
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::splat(6.);
                    for (f, t) in &thumbs {
                        let (rect, r) = ui.allocate_exact_size(Vec2::new(76., 76.), Sense::click());
                        let p = ui.painter();
                        p.rect_filled(rect, 8, if r.hovered() { SURFACE_HOVER } else { SURFACE });
                        p.rect_stroke(rect, 8, Stroke::new(1., if r.hovered() { BORDER_STRONG } else { BORDER }), egui::StrokeKind::Inside);
                        let pic = rect.shrink(8.);
                        match t {
                            Some(t) => {
                                let s = t.size_vec2();
                                let k = (pic.width() / s.x).min(pic.height() / s.y);
                                p.image(t.id(), Rect::from_center_size(pic.center(), s * k), Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)), Color32::WHITE);
                            }
                            None => drop(p.text(pic.center(), Align2::CENTER_CENTER, ic::IMAGE, egui::FontId::proportional(24.), TEXT_FAINT)),
                        }
                        if r.on_hover_text(f.as_str()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            act = Some(A::Usar(f.clone()));
                        }
                    }
                });
            }
            ui.add_space(10.);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Después").on_hover_text("Puedes hacerlo luego desde las propiedades del objeto").clicked() {
                    act = Some(A::Cerrar);
                }
            });
        });
        if act.is_none() && r.should_close() {
            act = Some(A::Cerrar);
        }
        let Some(a) = act else { return };
        if a != A::Guardar {
            self.nuevo = None;
        }
        let base = self.base_dir();
        match a {
            A::Guardar => drop(self.save(false)),
            A::Cerrar => {}
            A::Ejemplo => self.usar_baldosas_ejemplo(&name),
            A::Dibujar if mapa_ => self.abrir_dibujo(dibujo::Destino::Baldosas { obj: name }),
            A::Dibujar => self.abrir_dibujo(dibujo::Destino::Anim { obj: name, anim: 0 }),
            A::Usar(f) => {
                if let Some(Kind::Tilemap { tileset, .. }) = self.obj_mut(&name).map(|o| &mut o.kind) {
                    *tileset = f;
                }
            }
            A::Elegir => {
                let d = rfd::FileDialog::new().add_filter("Imagen", &["png", "jpg", "jpeg"]).set_directory(&base);
                match self.obj_mut(&name).map(|o| &mut o.kind) {
                    Some(Kind::Tilemap { tileset, .. }) => {
                        if let Some(f) = d.pick_file() {
                            *tileset = rel(&base, &f);
                        }
                    }
                    Some(Kind::Sprite { animations, .. }) => {
                        if let (Some(fs), Some(a)) = (d.pick_files(), animations.first_mut()) {
                            a.frames.extend(fs.iter().map(|f| rel(&base, f)));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// `--abrir mapa`: elige (o crea) un mapa y pinta un paisaje de prueba con las funciones de pintado.
    fn abrir_mapa(&mut self) {
        self.tab = Tab::Escena;
        let s = &self.p.scenes[self.sc];
        let found = s.instances.iter().position(|i| self.p.objects_of(s).any(|o| o.name == i.obj && matches!(o.kind, Kind::Tilemap { .. })));
        let i = match found {
            Some(i) => i,
            None => {
                self.nuevo_mapa();
                self.nuevo = None;
                let n = self.sel_obj.clone().unwrap_or_default();
                self.usar_baldosas_ejemplo(&n);
                self.sel_inst.unwrap_or(0)
            }
        };
        self.sel_inst = Some(i);
        let name = self.p.scenes[self.sc].instances[i].obj.clone();
        self.sel_obj = Some(name.clone());
        if let Some(Kind::Tilemap { cols, rows, data, .. }) = self.obj_mut(&name).map(|o| &mut o.kind) {
            let (c, r) = (*cols, *rows);
            let (ci, ri) = (c as i64, r as i64);
            // Suelo: pasto arriba y tierra abajo, con un pozo de agua.
            mapa::llenar_rect(data, c, r, (0, ri - 3), (ci - 1, ri - 3), 0);
            mapa::llenar_rect(data, c, r, (0, ri - 2), (ci - 1, ri - 1), 1);
            mapa::llenar_rect(data, c, r, (10, ri - 3), (12, ri - 1), -1);
            mapa::llenar_rect(data, c, r, (10, ri - 2), (12, ri - 1), 5);
            // Pared de ladrillos, plataformas de madera y piedra, una nube y un arbusto.
            mapa::llenar_rect(data, c, r, (17, ri - 6), (18, ri - 4), 3);
            mapa::poner_linea(data, c, r, (4, ri - 7), (7, ri - 7), 4);
            mapa::poner_linea(data, c, r, (13, ri - 10), (16, ri - 10), 2);
            mapa::poner_linea(data, c, r, (3, 2), (5, 2), 6);
            mapa::llenar_rect(data, c, r, (21, ri - 5), (22, ri - 4), 7);
        }
        self.pintor = mapa::Pintor { herr: mapa::Herr::Rect, baldosa: 3, ..Default::default() };
        self.say(Nivel::Ok, "Mapa pintado de prueba (--abrir mapa)");
    }

    /// `--abrir dibujo` / `cebolla` / `baldosas`: editor de dibujo con un dibujo de prueba.
    fn abrir_dibujo_prueba(&mut self, que: &str) {
        if que == "baldosas" {
            self.abrir_mapa();
            let n = self.sel_obj.clone().unwrap_or_default();
            return self.abrir_dibujo(dibujo::Destino::Baldosas { obj: n });
        }
        let n = self.nombre_libre("Dibujo");
        let kind = Kind::Sprite { w: 0., h: 0., animations: vec![Animation { name: "salta".into(), frames: vec![], fps: 6., looping: true }], mask: vec![], origin: None };
        self.scene().objects.push(ObjectDef { name: n.clone(), kind, behaviors: vec![], vars: Vars::new(), effect: String::new() });
        self.sel_obj = Some(n.clone());
        self.abrir_dibujo(dibujo::Destino::Anim { obj: n, anim: 0 });
        if let Some(d) = self.dib.as_mut() {
            dibujo::demo(d);
            d.cebolla = que == "cebolla";
            d.cur = if que == "cebolla" { 1 } else { 0 };
            d.play = que != "cebolla";
        }
        if que == "dibujo-guardado" {
            if let Some(mut d) = self.dib.take() {
                self.guardar_dibujo(&mut d);
            }
        }
    }

    /// `--abrir si-no`: debajo del primer evento con condiciones, un «si no, cuando» y un «si no».
    fn abrir_si_no(&mut self) {
        self.tab = Tab::Eventos;
        let evs = &mut self.p.scenes[self.sc].events;
        let i = evs.iter().position(|e| e.kind == EventKind::Standard && !e.conds.is_empty()).unwrap_or_else(|| {
            evs.push(Event { conds: vec![Instr::new("key_down", &["Right"])], ..Default::default() });
            evs.len() - 1
        });
        let acts = evs[i].acts.clone();
        evs.insert(i + 1, Event { kind: EventKind::Else, conds: vec![Instr::new("key_down", &["Up"])], ..Default::default() });
        evs.insert(i + 2, Event { kind: EventKind::Else, acts, ..Default::default() });
    }

    /// `--abrir` de las herramientas de estudio 2D (partículas, origen y máscara, ajustes, recursos, selección múltiple, buscar, listas).
    fn abrir_estudio(&mut self, que: &str) {
        self.tab = Tab::Escena;
        let base = self.base_dir();
        self.files = buscar_archivos(&base);
        self.files_at = Some(std::time::Instant::now());
        match que {
            "particulas" => self.nuevas_particulas(0),
            "fondo" => {
                self.nuevo_fondo();
                let img = self.files.iter().find(|f| recursos::Tipo::de(f) == Some(recursos::Tipo::Imagen)).cloned().unwrap_or_default();
                if let Some(Kind::Background { image, scroll, .. }) = self.sel_obj.clone().and_then(|n| self.obj_mut(&n)).map(|o| &mut o.kind) {
                    if image.is_empty() {
                        *image = img;
                    }
                    *scroll = [-30., 0.];
                }
            }
            "origen" | "mascara" => {
                let s = &self.p.scenes[self.sc];
                let found = self.p.objects_of(s).find_map(|o| match &o.kind {
                    Kind::Sprite { animations, .. } => animations.first().and_then(|a| a.frames.first()).map(|f| (o.name.clone(), f.clone())),
                    _ => None,
                });
                let Some((n, f)) = found else { return };
                let (w, h) = image::image_dimensions(base.join(&f)).map(|(w, h)| (w as f32, h as f32)).unwrap_or((32., 32.));
                if let Some(Kind::Sprite { origin, mask, .. }) = self.obj_mut(&n).map(|o| &mut o.kind) {
                    *origin = Some([(w / 2.).round(), h]);
                    if que == "mascara" {
                        *mask = vec![[w * 0.25, h * 0.1], [w * 0.75, h * 0.1], [w * 0.95, h * 0.55], [w * 0.7, h], [w * 0.3, h], [w * 0.05, h * 0.55]].iter().map(|p| [p[0].round(), p[1].round()]).collect();
                    }
                }
                let i = self.p.scenes[self.sc].instances.iter().position(|i| i.obj == n);
                self.elegir(i);
                self.sel_obj = Some(n);
                self.abrir_origen = true;
                self.origen_st.modo = if que == "mascara" { origen::Modo::Mascara } else { origen::Modo::Origen };
            }
            "ajustes" => self.ajustes = true,
            "recursos" => {
                self.tab = Tab::Recursos;
                self.explorador.sel = self.files.iter().find(|f| recursos::Tipo::de(f) == Some(recursos::Tipo::Imagen)).cloned();
            }
            "multi" => {
                let s = &self.p.scenes[self.sc];
                let v: Vec<usize> = (0..s.instances.len())
                    .filter(|&i| s.layers.iter().find(|l| l.name == s.instances[i].layer).is_none_or(|l| l.visible))
                    .filter(|&i| self.p.objects_of(s).find(|o| o.name == s.instances[i].obj).is_some_and(|o| matches!(o.kind, Kind::Sprite { .. })))
                    .take(4)
                    .collect();
                self.poner_seleccion(v);
            }
            "buscar" => {
                self.tab = Tab::Eventos;
                let q = self.p.objects_of(&self.p.scenes[self.sc]).map(|o| o.name.clone()).find(|n| !buscar::buscar(&self.p, self.sc, n, false, false).is_empty()).unwrap_or_default();
                self.buscador.abrir();
                self.buscador.texto = q.clone();
                self.buscador.reemplazo = format!("{q}2");
                let json = self.json();
                self.buscador.actualizar(&self.p, self.sc, &json, self.facil);
                if let Some(h) = self.buscador.resultados.get(1).cloned() {
                    self.buscador.ir(1);
                    self.ir_a(&h);
                    self.jump = Some((h.ev, 1e9));
                }
            }
            "lista" => {
                self.tab = Tab::Variables;
                let vars = &mut self.p.scenes[self.sc].vars;
                let inv = serde_json::from_str(r#"["espada", "escudo", 3, true, [1, 2]]"#).unwrap();
                vars.entry("inventario".into()).or_insert(inv);
                let jug = serde_json::from_str(r#"{"nombre": "Nico", "vidas": 3, "llaves": ["roja", "azul"]}"#).unwrap();
                vars.entry("jugador".into()).or_insert(jug);
                cajitas::ABRIR_LISTAS.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            _ => {}
        }
    }

    /// Vista inicial pedida con `--abrir` (para probar la interfaz).
    fn abrir(&mut self, que: &str) {
        match que {
            "bienvenida" | "nuevo-plantilla" => {
                let ini = self.inicio.get_or_insert_with(|| inicio::Inicio::new(false));
                if que == "nuevo-plantilla" {
                    ini.elegir(0);
                }
                return;
            }
            "misiones" | "mision-activa" | "mision-cumplida" => {
                self.tab = Tab::Escena;
                self.mis.abierto = true;
                if que != "misiones" {
                    self.mis.empezar_primera(&self.p);
                    self.mis.listo(0.);
                }
                if que == "mision-cumplida" {
                    for _ in 0..8 {
                        self.mis.listo(0.);
                    }
                }
                return;
            }
            _ => {}
        }
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
            "mapa" => self.abrir_mapa(),
            "dibujo" | "cebolla" | "baldosas" | "dibujo-guardado" => {
                self.tab = Tab::Escena;
                self.abrir_dibujo_prueba(que);
            }
            "sprite-nuevo" => {
                self.tab = Tab::Escena;
                let n = self.nombre_libre("Sprite");
                let kind = Kind::Sprite { w: 0., h: 0., animations: vec![Animation { name: "quieto".into(), frames: vec![], fps: 10., looping: true }], mask: vec![], origin: None };
                self.scene().objects.push(ObjectDef { name: n.clone(), kind, behaviors: vec![], vars: Vars::new(), effect: String::new() });
                self.sel_obj = Some(n.clone());
                self.nuevo = Some(Nuevo::Sprite(n));
            }
            "mapa-nuevo" => {
                self.tab = Tab::Escena;
                self.nuevo_mapa();
            }
            "vista" | "vista-pausa" | "vista-abajo" | "depurador" | "registro" | "rendimiento" | "recarga" | "jugar" | "perfilador" => {
                self.tab = Tab::Escena;
                self.vista_abrir = Some(que.into());
            }
            "si-no" => self.abrir_si_no(),
            "particulas" | "fondo" | "origen" | "mascara" | "ajustes" | "recursos" | "multi" | "buscar" | "lista" => self.abrir_estudio(que),
            q if q.starts_with("nodos") => self.abrir_nodos(q),
            _ => {}
        }
    }
}

impl App {
    /// Pantalla de bienvenida a toda la ventana (sin proyecto, o con Archivo > Nuevo).
    fn bienvenida(&mut self, ui: &mut Ui) {
        egui::Panel::bottom("estado").exact_size(28.).frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(12, 0))).show(ui, |ui| self.status(ui));
        let Some(mut ini) = self.inicio.take() else { return };
        let mut a = None;
        egui::CentralPanel::default_margins().frame(Frame::new().fill(BG).inner_margin(16)).show(ui, |ui| a = ini.ui(ui));
        self.inicio = Some(ini);
        match a {
            None => {}
            Some(inicio::Accion::Volver) => self.inicio = None,
            Some(inicio::Accion::Vacio) => {
                self.set_project(new_project(), None);
                self.say(Nivel::Ok, "Juego nuevo y vacío. ¡A inventar!");
            }
            Some(inicio::Accion::Abrir) => self.open_file(),
            Some(inicio::Accion::Reciente(f)) => drop(abrir_proyecto(self, f)),
            Some(inicio::Accion::Creado(f)) => match ajustes::cargar(&f) {
                Ok((p, _)) => {
                    let name = p.name.clone();
                    self.set_project(p, Some(f.clone()));
                    if !self.facil {
                        self.set_facil(true);
                    }
                    self.tab = Tab::Escena;
                    self.mis.abierto = true;
                    self.say(Nivel::Ok, format!("¡Listo! Tu juego «{name}» está en {}", f.parent().unwrap_or(&f).display()));
                }
                Err(e) => self.say(Nivel::Error, format!("No se pudo abrir el juego nuevo: {e}")),
            },
        }
        let title = "NicoStation".to_string();
        if title != self.title {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }
}

/// Etiqueta flotante sobre la vista previa (fondo oscuro, ícono y texto).
fn chip(p: &egui::Painter, at: Pos2, align: Align2, icon: &str, t: &str, c: Color32) -> Rect {
    let g = p.layout_no_wrap(format!("{icon}  {t}"), egui::FontId::proportional(14.), c);
    let r = align.anchor_size(at, g.size() + Vec2::new(18., 10.));
    p.rect_filled(r, 8, Color32::from_black_alpha(200));
    p.rect_stroke(r, 8, Stroke::new(1., c.gamma_multiply(0.5)), egui::StrokeKind::Inside);
    p.galley(r.min + Vec2::new(9., 5.), g, c);
    r
}

/// Vista previa dentro del editor: armado del juego, barra de controles, dibujo y depurador.
impl App {
    /// Mundo del juego con el proyecto actual (imágenes cargadas y tamaño de los sprites resuelto).
    fn vista_mundo(&mut self, ctx: &egui::Context) -> (nicostation::runtime::World, HashMap<String, egui::TextureHandle>, String) {
        let json = self.json();
        let mut p = self.p.clone();
        let mut tex = HashMap::new();
        // Con efectos (en un objeto o cambiados por eventos), también la versión gris y la silueta de cada imagen.
        let efectos = json.contains("\"effect\"") || json.contains("set_effect");
        for f in vista::imagenes(&p) {
            if let Some(t) = self.texture(ctx, &f) {
                tex.insert(f.clone(), t);
            }
            if efectos {
                for (k, var) in [("gris", 1), ("blanco", 2)] {
                    if let Some(t) = self.tex_var(ctx, &f, var) {
                        tex.insert(format!("{k}:{f}"), t);
                    }
                }
            }
        }
        for o in p.objects.iter().chain(p.scenes.iter().flat_map(|s| s.objects.iter())) {
            if let Kind::Text { font, .. } = &o.kind {
                self.fuente(ctx, font);
            }
        }
        vista::preparar(&mut p, |f| tex.get(f).map(|t| t.size()));
        (nicostation::runtime::World::new(p), tex, json)
    }

    fn vista_escena_inicial(&self) -> String {
        if self.vprefs.desde_aqui { self.p.scenes[self.sc].name.clone() } else { self.p.first_scene.clone() }
    }

    fn vista_empezar(&mut self, ctx: &egui::Context) {
        let (mut w, tex, json) = self.vista_mundo(ctx);
        w.storage = self.vista_partida.clone();
        let esc = self.vista_escena_inicial();
        if w.scene_def().name != esc {
            w.load_scene(&esc);
        }
        let mut s = vista::Sesion::nueva(w, tex, json);
        let n = s.world.scene_def().name.clone();
        s.anotar(vista::Tipo::Escena, format!("Empezó el juego en la escena «{n}»"));
        self.vista = Some(s);
        self.mis.probado(self.t);
        self.say(Nivel::Ok, if self.facil { "¡A jugar! Haz clic en el juego para usar el teclado." } else { "Vista previa en marcha (F6 pausa, F7 avanza un cuadro)" });
    }

    /// Recarga en caliente: mismo juego con los cambios, en la misma escena, conservando globales y partida.
    fn vista_recargar(&mut self, ctx: &egui::Context) {
        if self.vista.is_none() {
            return;
        }
        let (w, tex, json) = self.vista_mundo(ctx);
        let t = self.t;
        if let Some(s) = self.vista.as_mut() {
            s.cambiar_mundo(w, tex, json);
            s.aviso = Some(("Cambios aplicados".into(), t + 2.));
        }
    }

    fn vista_detener(&mut self) {
        if let Some(s) = self.vista.take() {
            self.vista_partida = s.world.storage.clone();
            self.say(Nivel::Info, if self.facil { "Terminaste de jugar." } else { "Vista previa detenida" });
        }
    }

    /// F6: empezar, o pausar y seguir.
    fn vista_f6(&mut self, ctx: &egui::Context) {
        match self.vista.as_mut() {
            None => self.vista_empezar(ctx),
            Some(s) if s.fin => self.vista_empezar(ctx),
            Some(s) => s.pausa = !s.pausa,
        }
    }

    /// F7: un cuadro y queda en pausa.
    fn vista_f7(&mut self, ctx: &egui::Context) {
        if self.vista.is_none() {
            self.vista_empezar(ctx);
        }
        if let Some(s) = self.vista.as_mut() {
            s.pausa = true;
            if !s.fin {
                s.paso();
            }
        }
    }

    fn vista_barra(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let f = self.facil;
        let Some(s) = self.vista.as_ref() else { return };
        let (pausa, fin, ver_todo) = (s.pausa, s.fin, s.ver_todo);
        let escena = s.world.scene_def().name.clone();
        let pendiente = s.json != self.now;
        #[derive(PartialEq)]
        enum A {
            Play,
            Paso,
            Reiniciar,
            Detener,
            Desde(bool),
            Aplicar,
            VerTodo,
        }
        let mut act = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.;
            let (icon, t) = match (pausa || fin, f) {
                (true, true) => (ic::PLAY, if fin { "Jugar otra vez" } else { "Seguir" }),
                (true, false) => (ic::PLAY, if fin { "Empezar de nuevo" } else { "Seguir" }),
                (false, _) => (ic::PAUSE, "Pausa"),
            };
            let hint = RichText::new("F6").small().color(ON_ACCENT.gamma_multiply(0.75));
            if primary(ui, (on_accent(icon), on_accent(t).font(sb(14.)), hint)).on_hover_text(if pausa { "Seguir jugando (F6)" } else { "Congelar el juego para mirarlo (F6)" }).clicked() {
                act = Some(A::Play);
            }
            let paso = if f { "Paso a paso" } else { "Avanzar 1 cuadro" };
            let tip = if f { "Avanza el juego un poquito y lo deja en pausa (F7)" } else { "Avanza un cuadro (1/60 s) y pausa (F7)" };
            if ui.add_enabled(!fin, Button::new((RichText::new(ic::FOOTPRINTS).color(TEXT_MUTED), RichText::new(paso), RichText::new("F7").small().color(TEXT_FAINT)))).on_hover_text(tip).clicked() {
                act = Some(A::Paso);
            }
            if ui.button(it(ic::RESTART, if f { "Otra vez" } else { "Reiniciar escena" })).on_hover_text(format!("Empezar de nuevo la escena «{escena}» (las cajitas globales se conservan)")).clicked() {
                act = Some(A::Reiniciar);
            }
            if ui.button(it(ic::STOP, if f { "Terminar" } else { "Detener" })).on_hover_text("Cerrar la vista previa").clicked() {
                act = Some(A::Detener);
            }
            ui.add_space(4.);
            seg(ui, |ui| {
                for v in vista::VELOCIDADES {
                    let t = if v == 0.25 { "0.25x".to_string() } else if v == 0.5 { "0.5x".into() } else { format!("{v:.0}x") };
                    let tip = if v < 1. { "Cámara lenta" } else if v > 1. { "Rápido" } else { "Velocidad normal" };
                    if seg_btn(ui, self.vprefs.vel == v, t).on_hover_text(tip).clicked() {
                        self.vprefs.vel = v;
                    }
                }
            });
            let aqui = self.vprefs.desde_aqui;
            let editando = self.p.scenes[self.sc].name.clone();
            let mostrar = if aqui { format!("Esta escena ({editando})") } else { format!("Escena inicial ({})", self.p.first_scene) };
            ui.label(muted("Empezar en"));
            egui::ComboBox::from_id_salt("vista_desde").selected_text(mostrar).show_ui(ui, |ui| {
                if ui.selectable_label(!aqui, format!("Escena inicial ({})", self.p.first_scene)).clicked() && aqui {
                    act = Some(A::Desde(false));
                }
                if ui.selectable_label(aqui, format!("Empezar en esta escena ({editando})")).clicked() && !aqui {
                    act = Some(A::Desde(true));
                }
            }).response.on_hover_text("Dónde empieza el juego al apretar Jugar aquí (cambiarlo empieza de nuevo)");
            let r = seg(ui, |ui| seg_btn(ui, self.vprefs.recargar, it(ic::ARROWS_CLOCKWISE, "Recargar al cambiar")));
            if r.on_hover_text("Cuando cambias algo del juego, la vista previa se actualiza sola: la escena empieza de nuevo y se conservan las cajitas globales y la partida guardada").clicked() {
                self.vprefs.recargar = !self.vprefs.recargar;
            }
            if pendiente && !self.vprefs.recargar && primary(ui, (on_accent(ic::ARROWS_CLOCKWISE), on_accent("Aplicar cambios"))).on_hover_text("Hay cambios que la vista previa todavía no usa").clicked() {
                act = Some(A::Aplicar);
            }
            if ui.add(Button::selectable(ver_todo, RichText::new(ic::FRAME_CORNERS).size(16.)).min_size(Vec2::splat(28.)))
                .on_hover_text("Ver toda la escena (el rectángulo azul es lo que ve la cámara)").clicked() {
                act = Some(A::VerTodo);
            }
            ui.label(RichText::new(ic::SPEAKER_SLASH).size(16.).color(TEXT_MUTED)).on_hover_text("La vista previa no reproduce sonido; usa Probar (F5)");
            let g = self.vprefs.grande;
            let (icon, tip) = if g { (ic::CORNERS_IN, "Achicar: dejar el juego abajo y seguir editando") } else { (ic::CORNERS_OUT, "Agrandar: el juego ocupa todo el centro") };
            if ib(ui, icon, tip).clicked() {
                self.vprefs.grande = !g;
            }
        });
        match act {
            Some(A::Play) => self.vista_f6(&ctx),
            Some(A::Paso) => self.vista_f7(&ctx),
            Some(A::Reiniciar) => {
                if let Some(s) = self.vista.as_mut() {
                    s.reiniciar_escena();
                }
            }
            Some(A::Detener) => self.vista_detener(),
            Some(A::Desde(v)) => {
                self.vprefs.desde_aqui = v;
                self.vista_empezar(&ctx);
            }
            Some(A::Aplicar) => self.vista_recargar(&ctx),
            Some(A::VerTodo) => {
                if let Some(s) = self.vista.as_mut() {
                    s.ver_todo = !s.ver_todo;
                }
            }
            None => {}
        }
    }

    /// Barra, depurador al costado y el juego.
    fn vista_ui(&mut self, ui: &mut Ui) {
        egui::Panel::top("vista_barra").frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(10, 6))).show(ui, |ui| self.vista_barra(ui));
        let facil = self.facil;
        if let Some(s) = self.vista.as_mut() {
            egui::Panel::right("depurador").resizable(true).default_size(if facil { 320. } else { 340. }).size_range(260.0..=560.)
                .frame(Frame::new().fill(PANEL).inner_margin(10)).show(ui, |ui| depurador::panel(ui, s, facil));
        }
        self.vista_juego(ui);
    }

    fn vista_juego(&mut self, ui: &mut Ui) {
        let facil = self.facil;
        let t = self.t;
        let Some(s) = self.vista.as_mut() else { return };
        let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        painter.rect_filled(resp.rect, 0., Color32::BLACK);
        let (m, quads) = vista::dibujar(&painter, resp.rect.shrink(6.), s, &self.fuentes);
        s.marco = Some(m);
        vista::medir_textos(ui.ctx(), &mut s.world, &self.fuentes);
        // Foco de teclado: clic en el juego. Las flechas, Tab y Esc van al juego y no mueven el foco del editor.
        if std::mem::take(&mut s.pedir_foco) || resp.clicked() || resp.drag_started() {
            resp.request_focus();
        } else if resp.clicked_elsewhere() {
            ui.memory_mut(|mm| mm.surrender_focus(resp.id));
        }
        ui.memory_mut(|mm| mm.set_focus_lock_filter(resp.id, egui::EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: true }));
        s.foco = resp.has_focus();
        // Elegir una instancia: clic en pausa, o Alt+clic mientras corre. Los mapas solo si no hay otra cosa debajo.
        let alt = ui.input(|i| i.modifiers.alt);
        if resp.clicked() && (s.pausa || s.fin || alt) {
            if let Some(p) = resp.interact_pointer_pos() {
                let w = &s.world;
                let es_mapa = |i: usize| matches!(w.object_def(&w.insts[i].obj).map(|d| &d.kind), Some(Kind::Tilemap { .. }));
                let bajo: Vec<usize> = quads.iter().rev().filter(|(_, q)| vista::dentro(q, p)).map(|x| x.0).collect();
                let i = bajo.iter().find(|&&i| !es_mapa(i)).or(bajo.first()).copied();
                s.sel = i.map(|i| w.insts[i].id);
            }
        }
        let pa = painter.with_clip_rect(resp.rect);
        if let Some(i) = s.elegida() {
            if let Some((_, q)) = quads.iter().find(|(k, _)| *k == i) {
                pa.add(egui::Shape::closed_line(q.to_vec(), Stroke::new(4., Color32::from_black_alpha(170))));
                pa.add(egui::Shape::closed_line(q.to_vec(), Stroke::new(2., ACCENT)));
                let top = q.iter().map(|p| p.y).fold(f32::MAX, f32::min);
                let cx = q.iter().map(|p| p.x).sum::<f32>() / 4.;
                let at = Pos2::new(cx, (top - 4.).max(resp.rect.top() + 26.));
                chip(&pa, at, Align2::CENTER_BOTTOM, ic::CROSSHAIR, &s.world.insts[i].obj, ACCENT);
            }
        }
        if s.foco {
            pa.rect_stroke(m.rect.expand(2.), 2, Stroke::new(2., ACCENT), egui::StrokeKind::Outside);
        }
        // Arriba a la derecha: los marcadores (HUD) suelen ir a la izquierda.
        let arriba = Pos2::new(m.rect.right() - 8., m.rect.top() + 8.);
        if s.fin {
            let t = if facil { "El juego terminó. Aprieta «Jugar otra vez»." } else { "El juego pidió terminar (acción Salir)" };
            chip(&pa, m.rect.center(), Align2::CENTER_CENTER, ic::DOOR_OPEN, t, TEXT);
        } else if s.pausa {
            let t = if facil { "En pausa: toca algo del juego para ver qué es" } else { "En pausa: clic para elegir una instancia" };
            chip(&pa, arriba, Align2::RIGHT_TOP, ic::PAUSE, t, WARN);
        } else if !s.foco {
            let t = if facil { "Haz clic en el juego para jugar con el teclado" } else { "Clic en el juego para darle el teclado · Alt+clic elige una instancia" };
            chip(&pa, Pos2::new(m.rect.center().x, m.rect.bottom() - 10.), Align2::CENTER_BOTTOM, ic::KEYBOARD, t, TEXT);
        }
        if let Some((msg, hasta)) = s.aviso.clone() {
            if hasta > t {
                chip(&pa, Pos2::new(m.rect.center().x, m.rect.top() + 10.), Align2::CENTER_TOP, ic::CHECK, &msg, OK);
                ui.ctx().request_repaint();
            } else {
                s.aviso = None;
            }
        }
        if !s.pausa && !s.fin {
            ui.ctx().request_repaint();
        }
    }

    /// `--abrir vista…`: juego de prueba en marcha (se aplica en el primer cuadro, cuando ya hay imágenes).
    fn vista_prueba(&mut self, ctx: &egui::Context, que: &str) {
        if que == "vista-abajo" {
            self.vprefs.grande = false;
            self.tab = Tab::Eventos;
        }
        self.vista_empezar(ctx);
        let Some(s) = self.vista.as_mut() else { return };
        if que == "jugar" {
            // Sin ensayo: el juego recibe el teclado de verdad (para probar con `--tecla`).
            s.pestana = vista::Pestana::Registro;
            return;
        }
        // El héroe camina a la derecha un par de segundos (junta monedas, suenan cosas).
        for k in 0..150 {
            s.entrada.down.insert("Right".into());
            if k % 50 == 20 {
                s.entrada.tecla(Key::Space, true);
            } else if k % 50 == 30 {
                s.entrada.tecla(Key::Space, false);
            }
            s.paso();
        }
        s.entrada.soltar();
        s.pedir_foco = false;
        let w = &s.world;
        let heroe = w.insts.iter().find(|i| w.object_def(&i.obj).is_some_and(|d| d.behaviors.iter().any(|b| matches!(b, Behavior::Platformer { .. } | Behavior::TopDown { .. }))))
            .or_else(|| w.insts.iter().find(|i| w.object_def(&i.obj).is_some_and(|d| matches!(d.kind, Kind::Sprite { .. }))))
            .map(|i| i.id);
        match que {
            "vista-pausa" => {
                s.pausa = true;
                s.sel = heroe;
            }
            "depurador" => s.sel = heroe,
            "registro" => s.pestana = vista::Pestana::Registro,
            "rendimiento" => {
                s.pestana = vista::Pestana::Rendimiento;
                s.ver_todo = true;
            }
            "perfilador" => {
                s.pestana = vista::Pestana::Perfil;
                s.pausa = true;
            }
            "recarga" => {
                s.pestana = vista::Pestana::Registro;
                // Un cambio en el proyecto (fondo rojizo y una moneda más): la recarga en caliente lo aplica al terminar el cuadro.
                let sc = self.sc;
                self.p.scenes[sc].background = [120, 60, 90];
                if let Some(m) = self.p.scenes[sc].instances.iter().find(|i| i.obj == "Moneda").cloned() {
                    self.p.scenes[sc].instances.push(Instance { x: m.x + 64., ..m });
                }
            }
            _ => {}
        }
    }
}

fn catalog_sentence(i: &Instr) -> String {
    nicostation::catalog::sentence(i)
}

/// Recursos dentro de la carpeta del proyecto: imágenes, sonidos, fuentes y modelos.
fn buscar_archivos(base: &Path) -> Vec<String> {
    recursos::listar(base)
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
        let t_nuevo = ctx.input(|i| i.time);
        self.dt = ((t_nuevo - self.t) as f32).clamp(0., 0.1);
        self.t = t_nuevo;
        // Las fuentes del proyecto pedidas en el cuadro anterior ya están cargadas en egui.
        self.fuentes.values_mut().filter(|v| **v == 1).for_each(|v| *v = 2);
        if self.inicio.is_some() {
            return self.bienvenida(ui);
        }
        // Archivos arrastrados a la ventana: se importan a la carpeta del juego.
        let soltados: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
        if !soltados.is_empty() {
            let r = self.importar_archivos(soltados);
            if !r.is_empty() && self.tab != Tab::Escena {
                self.tab = Tab::Recursos;
            }
        }

        if let Some(q) = self.vista_abrir.take() {
            self.vista_prueba(&ctx, &q);
        }
        // Vista previa: el teclado del juego se lee antes de los atajos (si el juego tiene el foco, sus teclas no llegan al editor).
        if let Some(s) = self.vista.as_mut() {
            s.leer(&ctx);
        }

        // Atajos
        let typing = ctx.egui_wants_keyboard_input();
        let modal = self.ed.is_some() || self.exp.is_some() || self.confirm.is_some() || self.sel.is_some() || self.rec.is_some() || self.asis.is_some()
            || self.dib.is_some() || self.nuevo.is_some() || self.ajustes;
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
            // Copiar y pegar instancias: egui convierte Ctrl+C / Ctrl+V en Copy / Paste.
            let mut copy = scene_keys && i.consume_key(cmd, Key::C);
            let mut paste = (scene_keys && i.consume_key(cmd, Key::V)).then(String::new);
            if scene_keys {
                i.events.retain(|e| match e {
                    egui::Event::Copy => {
                        copy = true;
                        false
                    }
                    egui::Event::Paste(t) => {
                        paste = Some(t.clone());
                        false
                    }
                    _ => true,
                });
            }
            let todo = scene_keys && i.consume_key(cmd, Key::A);
            let find = !modal && matches!(self.tab, Tab::Eventos | Tab::Funciones) && i.consume_key(cmd, Key::F);
            for (key, t) in [(Key::Num1, Tab::Escena), (Key::Num2, Tab::Eventos), (Key::Num3, Tab::Funciones), (Key::Num4, Tab::Variables), (Key::Num5, Tab::Recursos)] {
                if !modal && i.consume_key(cmd, key) {
                    self.tab = t;
                    self.vprefs.grande &= self.vista.is_none();
                }
            }
            let f6 = !modal && i.consume_key(Modifiers::NONE, Key::F6);
            let f7 = !modal && i.consume_key(Modifiers::NONE, Key::F7);
            // Herramientas de pintado del mapa: B, E, G, R, I.
            if scene_keys && self.pintando().is_some() {
                if let Some(h) = mapa::Herr::TODAS.into_iter().find(|h| i.consume_key(Modifiers::NONE, h.tecla())) {
                    self.pintor.herr = h;
                }
            }
            let mut nudge = Vec2::ZERO;
            if scene_keys && self.sel_inst.is_some() {
                for (key, d) in [(Key::ArrowLeft, Vec2::new(-1., 0.)), (Key::ArrowRight, Vec2::new(1., 0.)), (Key::ArrowUp, Vec2::new(0., -1.)), (Key::ArrowDown, Vec2::new(0., 1.))] {
                    nudge += d * self.grid * i.count_and_consume_key(Modifiers::SHIFT, key) as f32;
                    nudge += d * i.count_and_consume_key(Modifiers::NONE, key) as f32;
                }
            }
            (save_as, save, i.consume_key(Modifiers::NONE, Key::F5) && !modal, undo, redo, dup, del, esc, nudge, f6, f7, (copy, paste, todo, find))
        });
        let (save_as, save, run, undo, redo, dup, del, esc, nudge, f6, f7, (copy, paste, todo, find)) = k;
        if copy {
            self.copiar(&ctx);
        }
        if let Some(t) = paste {
            self.pegar((!t.is_empty()).then_some(t));
        }
        if todo {
            let n = self.p.scenes[self.sc].instances.len();
            self.poner_seleccion((0..n).collect());
        }
        if find {
            self.buscador.abrir();
        }
        if f6 {
            self.vista_f6(&ctx);
        }
        if f7 {
            self.vista_f7(&ctx);
        }
        if let Some(s) = self.vista.as_mut() {
            s.avanzar(self.t, self.vprefs.vel);
        }
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
            self.elegir(None);
            self.arrastre = None;
        }
        if nudge != Vec2::ZERO {
            self.nudge(nudge);
        }

        let side = Frame::new().fill(PANEL).inner_margin(12);
        egui::Panel::top("barra").exact_size(44.).frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(8, 0))).show(ui, |ui| self.top_bar(ui));
        egui::Panel::bottom("estado").exact_size(28.).frame(Frame::new().fill(PANEL).inner_margin(Margin::symmetric(12, 0))).show(ui, |ui| self.status(ui));
        // Vista previa: abajo (para seguir editando) o en todo el centro.
        let grande = self.vista.is_some() && self.vprefs.grande;
        if self.vista.is_some() && !grande {
            egui::Panel::bottom("vista_dock").resizable(true).default_size(400.).size_range(220.0..=1200.).frame(Frame::new().fill(BG))
                .show(ui, |ui| self.vista_ui(ui));
        }
        if self.mis.abierto {
            egui::Panel::right("misiones").exact_size(misiones::ANCHO).frame(side).show(ui, |ui| self.mis.ui(ui, &self.p));
        }
        // En Mis bloques (modo fácil) el panel de la escena no se usa: más lugar para el bloque.
        if !grande && !(self.facil && self.tab == Tab::Funciones) && self.tab != Tab::Recursos {
            egui::Panel::left("izq").default_size(290.).size_range(240.0..=480.).frame(side)
                .show(ui, |ui| egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| self.left(ui)));
        }
        if !grande && self.tab == Tab::Escena {
            egui::Panel::right("der").default_size(300.).size_range(240.0..=480.).frame(side)
                .show(ui, |ui| egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| self.right(ui)));
        }
        let margin = match self.tab {
            _ if grande => Margin::ZERO,
            Tab::Escena | Tab::Funciones | Tab::Recursos => Margin::ZERO,
            _ => Margin::same(12),
        };
        egui::CentralPanel::default_margins().frame(Frame::new().fill(BG).inner_margin(margin)).show(ui, |ui| match self.tab {
            _ if grande => self.vista_ui(ui),
            Tab::Escena => self.canvas(ui),
            Tab::Eventos => {
                self.barra_buscar(ui);
                self.eventos_o_nodos(ui, Target::Scene(self.sc));
            }
            Tab::Recursos => self.recursos_tab(ui),
            Tab::Funciones => {
                if self.buscador.abierto {
                    Frame::new().inner_margin(Margin { left: 12, right: 12, top: 8, bottom: 0 }).show(ui, |ui| self.barra_buscar(ui));
                }
                self.funcs(ui);
            }
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
        self.nuevo_ui(&ctx);
        if let Some(mut d) = self.dib.take() {
            match dibujo::ventana(&ctx, &mut d, self.facil) {
                None => self.dib = Some(d),
                Some(dibujo::DibRes::Cancelar) => self.say(Nivel::Info, "Dibujo cerrado sin guardar"),
                Some(dibujo::DibRes::Guardar) => {
                    if !self.guardar_dibujo(&mut d) {
                        self.dib = Some(d);
                    }
                }
            }
        }
        self.confirm_ui(&ctx);
        self.export_ui(&ctx);
        self.ajustes_ui(&ctx);
        // Archivos encima de la ventana: aviso de que se pueden soltar.
        if ctx.input(|i| !i.raw.hovered_files.is_empty()) {
            let r = ctx.content_rect();
            let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::unique("soltar")));
            p.rect_filled(r, 0, Color32::from_black_alpha(160));
            p.rect_stroke(r.shrink(16.), 12, Stroke::new(3., ACCENT), egui::StrokeKind::Inside);
            p.text(r.center(), Align2::CENTER_CENTER, format!("{}  Suelta los archivos para agregarlos al juego", ic::FILE_IMAGE), egui::FontId::proportional(24.), TEXT);
        }

        // Historial: un paso por interacción terminada (sin botón presionado ni texto con foco).
        let now = self.json();
        if !ctx.input(|i| i.pointer.any_down()) && !ctx.egui_wants_keyboard_input() {
            self.hist.commit(&now);
        }
        // Recarga en caliente de la vista previa, cuando termina la interacción (no a mitad de un arrastre o mientras se escribe).
        let quieto = !ctx.input(|i| i.pointer.any_down()) && (!ctx.egui_wants_keyboard_input() || self.vista.as_ref().is_some_and(|s| s.foco));
        if self.vprefs.recargar && quieto && self.vista.as_ref().is_some_and(|s| s.json != now) {
            self.vista_recargar(&ctx);
        }
        // Revisión del proyecto (solo cuando cambió).
        if self.facil && now != self.probs_de {
            self.probs = nicostation::validate::validate(&self.p, Some(&self.base_dir()));
            self.probs_de = now.clone();
        }
        // Misiones: los pasos automáticos se revisan solo cuando cambió el proyecto (también al deshacer).
        if now != self.mis_json {
            self.mis.actualizar(&self.p, self.t);
            self.mis_json = now.clone();
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
    let (mut abrir, mut escena, mut file, mut captura, mut tecla, mut guion) = (None, None, None, None, None, String::new());
    let mut it_ = args.iter();
    while let Some(a) = it_.next() {
        match a.as_str() {
            "--experto" => app.facil = false,
            "--grande" => app.grande = true,
            "--abrir" => abrir = it_.next().cloned(),
            "--escena" => escena = it_.next().cloned(),
            "--captura" => captura = it_.next().cloned(),
            "--tecla" => tecla = it_.next().and_then(|k| Key::from_name(k)),
            "--guion" => guion = it_.next().cloned().unwrap_or_default(),
            _ => file = Some(a.clone()),
        }
    }
    if let Some(a) = file.as_ref() {
        let path = PathBuf::from(a);
        match path.exists() {
            true => drop(abrir_proyecto(&mut app, path)),
            false => {
                app.path = Some(path);
                app.say(Nivel::Info, format!("Proyecto nuevo: {a}"));
            }
        }
    }
    if let Some(e) = escena {
        app.sc = app.p.scenes.iter().position(|s| s.name == e).unwrap_or(0);
    }
    if file.is_none() && abrir.as_deref().is_none_or(|a| a == "bienvenida" || a == "nuevo-plantilla") {
        app.inicio = Some(inicio::Inicio::new(false));
    }
    if let Some(a) = abrir {
        app.abrir(&a);
    }
    if let Some(out) = captura {
        let ctx = egui::Context::default();
        tema::aplicar(&ctx);
        let pasos = captura::guion(&guion).unwrap_or_else(|e| {
            eprintln!("--guion: {e}");
            std::process::exit(2);
        });
        if let Err(e) = captura::capturar(&mut app, &ctx, tecla, pasos, Path::new(&out)) {
            eprintln!("No se pudo capturar: {e}");
            std::process::exit(1);
        }
        return Ok(());
    }
    let opts = eframe::NativeOptions {
        viewport: {
            // app_id «nicostation»: Wayland asocia la ventana con su entrada del lanzador (y su ícono).
            let vb = egui::ViewportBuilder::default().with_inner_size([1360., 820.]).with_min_inner_size([960., 600.]).with_title("NicoStation").with_app_id("nicostation");
            match image::load_from_memory(include_bytes!("../../assets/icono.png")) {
                Ok(img) => {
                    let img = img.to_rgba8();
                    let (width, height) = img.dimensions();
                    vb.with_icon(egui::IconData { rgba: img.into_raw(), width, height })
                }
                Err(_) => vb,
            }
        },
        ..Default::default()
    };
    eframe::run_native("NicoStation", opts, Box::new(|cc| {
        tema::aplicar(&cc.egui_ctx);
        Ok(Box::new(app))
    }))
}
