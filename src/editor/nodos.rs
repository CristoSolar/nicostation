//! Vista «Nodos» de la hoja de eventos: un lienzo infinito con nodos y cables, como los Blueprints.
//! Lo que se dibuja es un `nicostation::nodos::Graph` del proyecto; el motor lo compila a eventos.
//!
//! Lienzo: rueda = zoom, arrastrar con el botón derecho o el del medio = mover la vista, arrastrar con el
//! izquierdo en el fondo = elegir varios. Clic derecho, Espacio o Tab = agregar un nodo. Arrastrar desde un pin
//! hasta otro = conectar; soltar un cable en el vacío abre el buscador con los nodos que encajan.
//! Supr borra, Ctrl+C / Ctrl+V copian y pegan, Ctrl+D duplica, Ctrl+A elige todo, F encuadra.

use super::eventos::Lists;
use super::facil::{self, ArgCx, Ficha, Pill};
use super::iconos as ic;
use super::tema::*;
use eframe::egui::{
    self, emath::TSTransform, epaint::CubicBezierShape, Align, Align2, Button, Color32, CornerRadius, FontId, Frame, Id, Key, Layout,
    Margin, Modifiers, Order, PointerButton, Pos2, Rect, RichText, Sense, Shape, Stroke, TextEdit, Ui, UiBuilder, Vec2,
};
use nicostation::catalog::{self, CATALOG, P};
use nicostation::model::*;
use nicostation::nodos::{self as ng, ArgSpec, Flow, Graph, Node, NodeId, NodeKind, PinT, Row, Wire};
use std::collections::{BTreeSet, HashMap};

/// Un pin de un nodo.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Pin {
    ExecIn(NodeId),
    ExecOut(NodeId, u8),
    DataOut(NodeId),
    DataIn(NodeId, u8),
}

enum Drag {
    Nodes { ids: Vec<NodeId>, last: Pos2 },
    Box { from: Pos2, to: Pos2, add: bool },
    Wire { from: Pin, to: Pos2 },
    Resize { id: NodeId, last: Pos2 },
}

#[derive(Clone, Copy, PartialEq)]
pub enum PalModo {
    /// Nodo nuevo; si viene de un cable, se conecta a ese pin.
    Nuevo(Option<Pin>),
    /// Sub-condición para un «O» / «Y».
    Sub(NodeId),
}

pub struct Paleta {
    /// Dónde se abre (pantalla) y dónde va el nodo (lienzo).
    pub screen: Pos2,
    pub at: Pos2,
    pub modo: PalModo,
    pub search: String,
    pub tab: usize,
    pub mas: bool,
    focus: bool,
}

/// Ventanita para cambiar un parámetro (o una sub-condición) de un nodo.
#[derive(Clone)]
pub struct Editar {
    pub node: NodeId,
    pub k: usize,
    pub sub: Option<usize>,
    pub screen: Pos2,
    abierta: u64,
}

/// Errores y eventos compilados de un grafo (los calcula el editor cuando cambia el proyecto).
#[derive(Clone, Default)]
pub struct Cache {
    pub key: u64,
    pub probs: Vec<(Option<NodeId>, String)>,
    pub eventos: Vec<Event>,
}

#[derive(Default)]
pub struct Vista {
    /// «Nodos» elegido en Eventos y en Mis bloques.
    pub en_escena: bool,
    pub en_func: bool,
    /// Grafo elegido de la escena.
    pub grafo: usize,
    cams: HashMap<String, Rect>,
    pub sel: BTreeSet<NodeId>,
    drag: Option<Drag>,
    pub paleta: Option<Paleta>,
    pub editar: Option<Editar>,
    menu: Option<(NodeId, Pos2)>,
    clip: Option<Graph>,
    pub ver_eventos: bool,
    pub ver_hoja: bool,
    pub encuadrar: bool,
    /// Hoja vista como nodos (solo para mirar): JSON de los eventos y el grafo.
    pub prev: Option<(u64, Graph)>,
    pub cache: Cache,
    key: String,
    /// Errores a mostrar: se alterna con un clic en la barra de estado.
    ir_error: usize,
}

pub struct Cx<'a> {
    pub l: &'a Lists,
    pub facil: bool,
    pub probs: &'a [(Option<NodeId>, String)],
    pub n_eventos: usize,
    /// Clave del grafo (para recordar la cámara de cada uno).
    pub key: &'a str,
    pub solo_ver: bool,
}

#[derive(Default)]
pub struct Salida {
    pub msg: Option<String>,
    pub new_var: Option<(String, bool)>,
}

// ---------- colores y medidas ----------

const FLOW_C: Color32 = Color32::from_rgb(0xa8, 0xa8, 0xb8);
const EXEC_C: Color32 = Color32::from_rgb(0xf2, 0xf2, 0xf5);
const LBL_W: f32 = 96.;

pub fn pin_color(t: PinT) -> Color32 {
    match t {
        PinT::Num => CAT_SCENE,
        PinT::Text => CAT_AUDIO,
        PinT::Bool => CAT_FUNC,
        PinT::Obj => CAT_OBJ,
        PinT::Any => CAT_MOV,
    }
}

fn op_of(n: &Node) -> &str {
    match &n.kind {
        NodeKind::Event { op } | NodeKind::Cond { op } | NodeKind::Act { op } => op,
        _ => "",
    }
}

fn node_color(n: &Node) -> Color32 {
    match &n.kind {
        NodeKind::Event { op } if op.is_empty() => CAT_TIME,
        NodeKind::Event { op } | NodeKind::Cond { op } | NodeKind::Act { op } => facil::cat_op(op).color,
        NodeKind::Seq { .. } | NodeKind::Repeat | NodeKind::ForEach | NodeKind::While => FLOW_C,
        NodeKind::Comment { .. } => TEXT_FAINT,
        NodeKind::Data { .. } => ng::data_out(n).map_or(CAT_VAR, pin_color),
    }
}

fn data_icon(f: &str) -> &'static str {
    match f {
        "num" => ic::HASH,
        "text" | "join" | "len" | "upper" | "lower" | "str" | "tonum" => ic::TEXT_T,
        "bool" => ic::TOGGLE_RIGHT,
        "obj" | "prop" | "count" => ic::SHAPES,
        "var" | "global" => ic::ARCHIVE_BOX,
        "dt" | "time" => ic::TIMER,
        "mousex" | "mousey" => ic::MOUSE,
        "expr" => ic::BRACES,
        "random" | "randint" => ic::DICE,
        "eq" | "ne" | "lt" | "le" | "gt" | "ge" | "and" | "or" | "not" | "if" => ic::EQUALS,
        _ => ic::MATH,
    }
}

fn node_icon(n: &Node) -> &'static str {
    match &n.kind {
        NodeKind::Event { op } if op.is_empty() => ic::CLOCK_CLOCKWISE,
        NodeKind::Event { op } | NodeKind::Act { op } => facil::op_icon(op),
        NodeKind::Cond { .. } => ic::GIT_BRANCH,
        NodeKind::Seq { .. } => ic::LIST_BULLETS,
        NodeKind::Repeat => ic::REPEAT,
        NodeKind::ForEach => ic::STACK,
        NodeKind::While => ic::ARROWS_CLOCKWISE,
        NodeKind::Comment { .. } => ic::NOTE,
        NodeKind::Data { f } => data_icon(f),
    }
}

fn node_title(n: &Node, facil: bool) -> String {
    match &n.kind {
        NodeKind::Event { op } if op.is_empty() && facil => "Siempre (cada cuadro)".into(),
        NodeKind::Event { op } if facil => format!("Cuando: {}", catalog::meta(op).map_or(op.as_str(), |m| m.label)),
        _ => ng::title(n),
    }
}

fn node_help(n: &Node) -> String {
    match &n.kind {
        NodeKind::Event { op } if op.is_empty() => "Pasa todo el tiempo, muchas veces por segundo. Úsalo con nodos «Si» para preguntar cosas.".into(),
        NodeKind::Event { op } | NodeKind::Cond { op } | NodeKind::Act { op } => catalog::kid(op).help.into(),
        NodeKind::Seq { .. } => "Hace lo de la salida 1, después lo de la 2, y así.".into(),
        NodeKind::Repeat => "Hace lo de «cada vez» varias veces seguidas; después sigue por «al terminar».".into(),
        NodeKind::ForEach => "Hace lo de «cada uno» una vez por cada copia del objeto, con esa copia elegida.".into(),
        NodeKind::While => "Repite lo de «mientras» hasta que sus «Si» dejen de cumplirse.".into(),
        NodeKind::Comment { .. } => "Una nota para explicar algo. El juego no la usa.".into(),
        NodeKind::Data { f } => ng::data_def(f).map_or(String::new(), |d| d.help.into()),
    }
}

// ---------- geometría ----------

struct Geo {
    rect: Rect,
    ein: Option<Pos2>,
    outs: Vec<Pos2>,
    dout: Option<(Pos2, PinT)>,
    args: Vec<Option<(Pos2, PinT)>>,
    rows: Vec<Row>,
}

fn row_rect(r: Rect, i: usize) -> Rect {
    let y = r.top() + ng::HEAD_H + ng::PAD / 2. + ng::ROW_H * i as f32;
    Rect::from_min_max(Pos2::new(r.left(), y), Pos2::new(r.right(), y + ng::ROW_H))
}

fn geo(n: &Node) -> Geo {
    let (w, h) = ng::size(n);
    let rect = Rect::from_min_size(Pos2::new(n.x, n.y), Vec2::new(w, h));
    let hy = rect.top() + ng::HEAD_H / 2.;
    let rows = ng::rows(n);
    let specs = ng::arg_specs(n);
    let nouts = ng::exec_outs(n).len();
    let mut outs = vec![Pos2::new(rect.right(), hy); nouts];
    let mut args = vec![None; specs.len()];
    for (i, r) in rows.iter().enumerate() {
        let y = row_rect(rect, i).center().y;
        match r {
            Row::Out(k) => outs[*k as usize] = Pos2::new(rect.right(), y),
            Row::Arg(k) => args[*k] = specs[*k].pin.map(|t| (Pos2::new(rect.left(), y), t)),
            _ => {}
        }
    }
    Geo { rect, ein: ng::has_exec_in(n).then(|| Pos2::new(rect.left(), hy)), outs, dout: ng::data_out(n).map(|t| (Pos2::new(rect.right(), hy), t)), args, rows }
}

fn pin_pos(geos: &HashMap<NodeId, Geo>, p: Pin) -> Option<Pos2> {
    match p {
        Pin::ExecIn(id) => geos.get(&id)?.ein,
        Pin::ExecOut(id, k) => geos.get(&id)?.outs.get(k as usize).copied(),
        Pin::DataOut(id) => geos.get(&id)?.dout.map(|d| d.0),
        Pin::DataIn(id, k) => geos.get(&id)?.args.get(k as usize).copied().flatten().map(|d| d.0),
    }
}

fn is_out(p: Pin) -> bool {
    matches!(p, Pin::ExecOut(..) | Pin::DataOut(_))
}

fn pin_node(p: Pin) -> NodeId {
    match p {
        Pin::ExecIn(id) | Pin::ExecOut(id, _) | Pin::DataOut(id) | Pin::DataIn(id, _) => id,
    }
}

fn data_type(geos: &HashMap<NodeId, Geo>, p: Pin) -> Option<PinT> {
    match p {
        Pin::DataOut(id) => geos.get(&id)?.dout.map(|d| d.1),
        Pin::DataIn(id, k) => geos.get(&id)?.args.get(k as usize).copied().flatten().map(|d| d.1),
        _ => None,
    }
}

/// ¿Se pueden unir estos dos pines? Devuelve (salida, entrada).
fn pair(geos: &HashMap<NodeId, Geo>, a: Pin, b: Pin) -> Option<(Pin, Pin)> {
    let (o, i) = if is_out(a) { (a, b) } else { (b, a) };
    if !is_out(o) || is_out(i) || pin_node(o) == pin_node(i) {
        return None;
    }
    let ok = match (o, i) {
        (Pin::ExecOut(..), Pin::ExecIn(_)) => true,
        (Pin::DataOut(_), Pin::DataIn(..)) => matches!((data_type(geos, o), data_type(geos, i)), (Some(s), Some(d)) if ng::compatible(s, d)),
        _ => false,
    };
    ok.then_some((o, i))
}

fn all_pins(id: NodeId, g: &Geo) -> Vec<(Pin, Pos2)> {
    let mut v = vec![];
    if let Some(p) = g.ein {
        v.push((Pin::ExecIn(id), p));
    }
    v.extend(g.outs.iter().enumerate().map(|(k, p)| (Pin::ExecOut(id, k as u8), *p)));
    if let Some((p, _)) = g.dout {
        v.push((Pin::DataOut(id), p));
    }
    v.extend(g.args.iter().enumerate().filter_map(|(k, a)| a.map(|(p, _)| (Pin::DataIn(id, k as u8), p))));
    v
}

/// Une dos pines (reemplaza lo que ya llegaba a esa entrada o salía de esa salida de flecha).
pub fn connect(g: &mut Graph, o: Pin, i: Pin) {
    match (o, i) {
        (Pin::ExecOut(f, k), Pin::ExecIn(t)) => {
            g.flow.retain(|x| !(x.from == f && x.out == k));
            g.flow.push(Flow { from: f, out: k, to: t });
        }
        (Pin::DataOut(f), Pin::DataIn(t, k)) => {
            g.wires.retain(|w| !(w.to == t && w.arg == k));
            g.wires.push(Wire { from: f, to: t, arg: k });
        }
        _ => {}
    }
}

fn disconnect(g: &mut Graph, p: Pin) {
    match p {
        Pin::ExecIn(id) => g.flow.retain(|f| f.to != id),
        Pin::ExecOut(id, k) => g.flow.retain(|f| !(f.from == id && f.out == k)),
        Pin::DataOut(id) => g.wires.retain(|w| w.from != id),
        Pin::DataIn(id, k) => g.wires.retain(|w| !(w.to == id && w.arg == k)),
    }
}

fn next_id(g: &Graph) -> NodeId {
    g.nodes.iter().map(|n| n.id).max().unwrap_or(0) + 1
}

pub fn bounds(g: &Graph, only: Option<&BTreeSet<NodeId>>) -> Option<Rect> {
    g.nodes.iter().filter(|n| only.is_none_or(|s| s.contains(&n.id))).map(|n| geo(n).rect).reduce(|a, b| a.union(b))
}

/// Vista que encuadra el grafo (o lo elegido): todo si entra; si no, desde arriba a la izquierda sin achicar de más.
fn frame(g: &Graph, only: Option<&BTreeSet<NodeId>>, size: Vec2) -> Rect {
    let size = if size.x < 50. || size.y < 50. { Vec2::new(1000., 700.) } else { size };
    let Some(b) = bounds(g, only) else { return Rect::from_min_size(Pos2::new(-60., -60.), size) };
    let b = b.expand(48.);
    let fit = (size.x / b.width()).min(size.y / b.height());
    let z = fit.clamp(0.85, 1.0);
    if fit >= 0.85 {
        Rect::from_center_size(b.center(), size / z)
    } else {
        // Si no entra, mostrar el principio (los primeros eventos) a un tamaño legible.
        let w = size / z;
        let x0 = if b.width() <= w.x { b.center().x - w.x / 2. } else { b.left() };
        Rect::from_min_size(Pos2::new(x0, b.top()), w)
    }
}

// ---------- dibujo ----------

fn bezier(p: &egui::Painter, a: Pos2, b: Pos2, stroke: Stroke) {
    let dx = ((b.x - a.x).abs() * 0.5).max(40.);
    p.add(Shape::CubicBezier(CubicBezierShape::from_points_stroke([a, a + Vec2::new(dx, 0.), b - Vec2::new(dx, 0.), b], false, Color32::TRANSPARENT, stroke)));
}

fn exec_shape(c: Pos2, s: f32) -> Vec<Pos2> {
    vec![Pos2::new(c.x - s, c.y - s), Pos2::new(c.x + s * 0.2, c.y - s), Pos2::new(c.x + s, c.y), Pos2::new(c.x + s * 0.2, c.y + s), Pos2::new(c.x - s, c.y + s)]
}

fn draw_pin(p: &egui::Painter, c: Pos2, data: Option<PinT>, on: bool, hl: bool) {
    match data {
        None => {
            let pts = exec_shape(c, 6.);
            p.add(Shape::convex_polygon(pts, if on { EXEC_C } else { SURFACE }, Stroke::new(1.6, EXEC_C)));
        }
        Some(t) => {
            let col = pin_color(t);
            p.circle(c, 5.5, if on { col } else { SURFACE }, Stroke::new(1.8, col));
        }
    }
    if hl {
        p.circle_stroke(c, 10., Stroke::new(2., ACCENT));
    }
}

fn grid(ui: &Ui, zoom: f32) {
    let r = ui.clip_rect();
    let step = if zoom < 0.45 { 96. } else { 32. };
    let p = ui.painter();
    let w = 1. / zoom.max(0.05);
    let minor = blend(BORDER, BG, 0.45);
    let major = blend(BORDER, BG, 0.9);
    let (x0, y0) = ((r.left() / step).floor() as i64, (r.top() / step).floor() as i64);
    let (x1, y1) = ((r.right() / step).ceil() as i64, (r.bottom() / step).ceil() as i64);
    if x1 - x0 > 400 || y1 - y0 > 400 {
        return;
    }
    for i in x0..=x1 {
        let x = i as f32 * step;
        p.line_segment([Pos2::new(x, r.top()), Pos2::new(x, r.bottom())], Stroke::new(w, if i % 5 == 0 { major } else { minor }));
    }
    for j in y0..=y1 {
        let y = j as f32 * step;
        p.line_segment([Pos2::new(r.left(), y), Pos2::new(r.right(), y)], Stroke::new(w, if j % 5 == 0 { major } else { minor }));
    }
}

fn trunc(s: &str, n: usize) -> String {
    let s = s.replace('\n', " ");
    if s.chars().count() > n { format!("{}...", s.chars().take(n.saturating_sub(1)).collect::<String>()) } else { s }
}

/// Ficha compacta (como las del modo fácil, más chica para que entre en el nodo).
fn chip(ui: &mut Ui, cell: Rect, text: &str, st: &Pill, id: Id, enabled: bool) -> egui::Response {
    let font = FontId::proportional(13.);
    let g = ui.painter().layout_no_wrap(text.to_string(), font.clone(), st.fg);
    let ig = st.icon.map(|i| ui.painter().layout_no_wrap(i.to_string(), FontId::proportional(13.), st.fg));
    let w = (g.size().x + 16. + ig.as_ref().map_or(0., |g| g.size().x + 4.)).min(cell.width());
    let rect = Rect::from_min_size(Pos2::new(cell.left(), cell.center().y - 11.), Vec2::new(w, 22.));
    let r = ui.interact(rect, id, if enabled { Sense::click() } else { Sense::hover() });
    let p = ui.painter().with_clip_rect(cell.intersect(ui.clip_rect()));
    let hov = enabled && r.hovered();
    p.rect_filled(rect, 11, if hov { blend(Color32::WHITE, st.fill, 0.25) } else { st.fill });
    if let Some(s) = st.stroke {
        p.rect_stroke(rect, 11, Stroke::new(1.5, s), egui::StrokeKind::Inside);
    }
    if hov {
        p.rect_stroke(rect.expand(1.), 12, Stroke::new(1.5, ACCENT), egui::StrokeKind::Outside);
    }
    let mut x = rect.left() + 8.;
    if let Some(ig) = ig {
        let sz = ig.size();
        p.galley(Pos2::new(x, rect.center().y - sz.y / 2.), ig, st.fg);
        x += sz.x + 4.;
    }
    let sz = g.size();
    p.galley(Pos2::new(x, rect.center().y - sz.y / 2.), g, st.fg);
    if enabled { r.on_hover_cursor(egui::CursorIcon::PointingHand) } else { r }
}

/// ¿Este parámetro se escribe directo en el nodo (campo de texto) en vez de con una ficha?
fn campo(n: &Node, s: &ArgSpec, facil: bool) -> bool {
    if let NodeKind::Data { f } = &n.kind {
        if matches!(f.as_str(), "num" | "text" | "expr") {
            return true;
        }
    }
    !facil && matches!(s.p, P::Expr | P::Text | P::Var | P::Prop | P::Anim)
}

enum A {
    Click(NodeId, bool),
    DragStart(NodeId),
    Resize(NodeId),
    PinDrag(Pin),
    Menu(NodeId),
    Edit(NodeId, usize, Option<usize>, Rect),
    Toggle(NodeId),
    SeqAdd(NodeId),
    SeqDel(NodeId),
    AddSub(NodeId, Rect),
    DelSub(NodeId, usize),
    Disconnect(Pin),
    Pan(Vec2),
}

// ---------- lienzo ----------

pub fn lienzo(ui: &mut Ui, g: &mut Graph, st: &mut Vista, cx: &Cx) -> Salida {
    let mut out = Salida::default();
    let ctx = ui.ctx().clone();
    let outer = ui.available_rect_before_wrap();
    if st.key != cx.key {
        // Al pasar de un grafo a otro se olvida lo elegido (la primera vez se respeta, para `--abrir`).
        if !st.key.is_empty() {
            (st.drag, st.editar, st.paleta, st.menu) = (None, None, None, None);
            st.sel.clear();
        }
        st.key = cx.key.to_string();
    }
    let mut sr = match st.cams.get(cx.key) {
        Some(r) if !st.encuadrar => *r,
        _ => {
            let only = (!st.sel.is_empty() && st.sel.iter().all(|id| g.nodes.iter().any(|n| n.id == *id))).then(|| st.sel.clone());
            st.encuadrar = false;
            frame(g, only.as_ref(), outer.size())
        }
    };
    let geos: HashMap<NodeId, Geo> = g.nodes.iter().map(|n| (n.id, geo(n))).collect();
    let mut probs: HashMap<NodeId, Vec<&str>> = HashMap::new();
    for (id, m) in cx.probs {
        if let Some(id) = id {
            probs.entry(*id).or_default().push(m);
        }
    }
    let reach = ng::reachable(g);
    let wired: HashMap<(NodeId, u8), NodeId> = g.wires.iter().map(|w| ((w.to, w.arg), w.from)).collect();
    let out_on: BTreeSet<(NodeId, u8)> = g.flow.iter().map(|f| (f.from, f.out)).collect();
    let in_on: BTreeSet<NodeId> = g.flow.iter().map(|f| f.to).collect();
    let dout_on: BTreeSet<NodeId> = g.wires.iter().map(|w| w.from).collect();
    let titles: HashMap<NodeId, String> = g.nodes.iter().map(|n| (n.id, node_title(n, cx.facil))).collect();
    let dragging_pin = match &st.drag {
        Some(Drag::Wire { from, .. }) => Some(*from),
        _ => None,
    };
    let edit_on = !cx.solo_ver;
    let mut acts: Vec<A> = vec![];
    let mut to_global = TSTransform::IDENTITY;
    let mut pointer: Option<Pos2> = None;
    let scene = egui::Scene::new()
        .zoom_range(0.15..=2.0)
        .scroll_zooms(true)
        .drag_pan_buttons(egui::DragPanButtons::MIDDLE | egui::DragPanButtons::SECONDARY)
        .max_inner_size(Vec2::splat(200_000.));
    let resp = scene.show(ui, &mut sr, |ui| {
        to_global = ui.ctx().layer_transform_to_global(ui.layer_id()).unwrap_or_default();
        let from_g = to_global.inverse();
        pointer = ui.input(|i| i.pointer.latest_pos()).map(|p| from_g * p);
        grid(ui, to_global.scaling);
        // Recuadros de comentario, detrás de todo.
        for n in g.nodes.iter_mut().filter(|n| matches!(n.kind, NodeKind::Comment { .. })) {
            comentario(ui, n, &geos[&n.id], st.sel.contains(&n.id), edit_on, &mut acts);
        }
        // Cables.
        let p = ui.painter().clone();
        for f in &g.flow {
            if let (Some(a), Some(b)) = (pin_pos(&geos, Pin::ExecOut(f.from, f.out)), pin_pos(&geos, Pin::ExecIn(f.to))) {
                let dim = !reach.contains(&f.from);
                bezier(&p, a, b, Stroke::new(3., if dim { EXEC_C.gamma_multiply(0.45) } else { EXEC_C }));
            }
        }
        for w in &g.wires {
            if let (Some(a), Some(b)) = (pin_pos(&geos, Pin::DataOut(w.from)), pin_pos(&geos, Pin::DataIn(w.to, w.arg))) {
                let c = data_type(&geos, Pin::DataOut(w.from)).map_or(TEXT_MUTED, pin_color);
                bezier(&p, a, b, Stroke::new(2.4, c));
            }
        }
        // Nodos.
        for n in g.nodes.iter_mut().filter(|n| !matches!(n.kind, NodeKind::Comment { .. })) {
            let gm = &geos[&n.id];
            let nc = NodoCx {
                sel: st.sel.contains(&n.id),
                errs: probs.get(&n.id).map(|v| v.as_slice()).unwrap_or(&[]),
                lejos: !reach.contains(&n.id),
                wired: &wired,
                titles: &titles,
                cx,
                edit_on,
            };
            nodo(ui, n, gm, &nc, &mut acts);
        }
        // Pines (encima de los nodos para que se puedan agarrar).
        for (id, gm) in &geos {
            for (pin, pos) in all_pins(*id, gm) {
                let on = match pin {
                    Pin::ExecIn(i) => in_on.contains(&i),
                    Pin::ExecOut(i, k) => out_on.contains(&(i, k)),
                    Pin::DataOut(i) => dout_on.contains(&i),
                    Pin::DataIn(i, k) => wired.contains_key(&(i, k)),
                };
                let hl = dragging_pin.is_some_and(|d| d != pin && pair(&geos, d, pin).is_some());
                draw_pin(ui.painter(), pos, data_type(&geos, pin), on, hl);
                if edit_on {
                    let r = ui.interact(Rect::from_center_size(pos, Vec2::splat(18.)), Id::unique(("pin", cx.key, *id, format!("{pin:?}"))), Sense::click_and_drag());
                    let r = r.on_hover_cursor(egui::CursorIcon::Crosshair);
                    if r.drag_started_by(PointerButton::Primary) {
                        acts.push(A::PinDrag(pin));
                    }
                    if r.secondary_clicked() || (r.clicked() && ui.input(|i| i.modifiers.alt)) {
                        acts.push(A::Disconnect(pin));
                    }
                }
            }
        }
        // Cable que se está arrastrando.
        if let Some(Drag::Wire { from, to }) = &st.drag {
            if let Some(a) = pin_pos(&geos, *from) {
                let c = data_type(&geos, *from).map_or(EXEC_C, pin_color);
                let (a, b) = if is_out(*from) { (a, *to) } else { (*to, a) };
                bezier(ui.painter(), a, b, Stroke::new(2.5, c.gamma_multiply(0.85)));
            }
        }
        if let Some(Drag::Box { from, to, .. }) = &st.drag {
            let r = Rect::from_two_pos(*from, *to);
            ui.painter().rect(r, 2, ACCENT.gamma_multiply(0.08), Stroke::new(1. / to_global.scaling, ACCENT), egui::StrokeKind::Inside);
        }
    });
    let bg = resp.response;
    let shift = ctx.input(|i| i.modifiers.shift || i.modifiers.command);
    let to_screen = |p: Pos2| to_global * p;

    // ---- acciones de los nodos ----
    for a in acts {
        match a {
            A::Click(id, add) => {
                if add || shift {
                    if !st.sel.remove(&id) {
                        st.sel.insert(id);
                    }
                } else {
                    st.sel = BTreeSet::from([id]);
                }
                st.menu = None;
            }
            A::DragStart(id) => {
                if !st.sel.contains(&id) {
                    st.sel = BTreeSet::from([id]);
                }
                let mut ids: Vec<NodeId> = st.sel.iter().copied().collect();
                // Mover un comentario lleva los nodos que tiene adentro.
                for c in g.nodes.iter().filter(|n| st.sel.contains(&n.id) && matches!(n.kind, NodeKind::Comment { .. })) {
                    let cr = geos[&c.id].rect;
                    ids.extend(g.nodes.iter().filter(|n| n.id != c.id && cr.contains_rect(geos[&n.id].rect) && !ids.contains(&n.id)).map(|n| n.id).collect::<Vec<_>>());
                }
                if let Some(p) = pointer {
                    st.drag = Some(Drag::Nodes { ids, last: p });
                }
            }
            A::Resize(id) => {
                if let Some(p) = pointer {
                    st.drag = Some(Drag::Resize { id, last: p });
                }
            }
            A::PinDrag(pin) => {
                if let Some(p) = pointer {
                    st.drag = Some(Drag::Wire { from: pin, to: p });
                }
            }
            A::Menu(id) => {
                if !st.sel.contains(&id) {
                    st.sel = BTreeSet::from([id]);
                }
                if let Some(p) = ctx.input(|i| i.pointer.latest_pos()) {
                    st.menu = Some((id, p));
                }
            }
            A::Edit(node, k, sub, r) => {
                st.editar = Some(Editar { node, k, sub, screen: to_screen(r.left_bottom()) + Vec2::new(0., 4.), abierta: ctx.cumulative_frame_nr() });
                st.paleta = None;
            }
            A::Toggle(id) => {
                if let Some(n) = g.nodes.iter_mut().find(|n| n.id == id) {
                    let on = n.args.first().is_some_and(|v| matches!(v.as_str(), "1" | "true"));
                    n.args = vec![if on { "0" } else { "1" }.into()];
                }
            }
            A::SeqAdd(id) | A::SeqDel(id) => {
                let add = matches!(a, A::SeqAdd(_));
                if let Some(NodeKind::Seq { n }) = g.nodes.iter_mut().find(|n| n.id == id).map(|n| &mut n.kind) {
                    if add && *n < 16 {
                        *n += 1;
                    } else if !add && *n > 1 {
                        *n -= 1;
                        let k = *n;
                        g.flow.retain(|f| !(f.from == id && f.out >= k));
                    }
                }
            }
            A::AddSub(id, r) => {
                let at = r.left_bottom();
                st.paleta = Some(Paleta { screen: to_screen(at), at, modo: PalModo::Sub(id), search: String::new(), tab: 1, mas: false, focus: true });
            }
            A::DelSub(id, i) => {
                if let Some(n) = g.nodes.iter_mut().find(|n| n.id == id) {
                    if i < n.sub.len() {
                        n.sub.remove(i);
                    }
                }
            }
            A::Disconnect(p) => {
                disconnect(g, p);
                out.msg = Some("Cable quitado · Ctrl+Z para deshacer".into());
            }
            A::Pan(d) => sr = sr.translate(-d),
        }
    }

    // ---- arrastres ----
    let primary_down = ctx.input(|i| i.pointer.primary_down());
    if let (Some(p), Some(d)) = (pointer, st.drag.as_mut()) {
        match d {
            Drag::Nodes { ids, last } => {
                let delta = p - *last;
                if delta != Vec2::ZERO {
                    for n in g.nodes.iter_mut().filter(|n| ids.contains(&n.id)) {
                        n.x += delta.x;
                        n.y += delta.y;
                    }
                    *last = p;
                }
            }
            Drag::Resize { id, last } => {
                let delta = p - *last;
                if let Some(NodeKind::Comment { w, h }) = g.nodes.iter_mut().find(|n| n.id == *id).map(|n| &mut n.kind) {
                    *w = (*w + delta.x).max(120.);
                    *h = (*h + delta.y).max(60.);
                }
                *last = p;
            }
            Drag::Wire { to, .. } => *to = p,
            Drag::Box { to, .. } => *to = p,
        }
    }
    if !primary_down {
        match st.drag.take() {
            Some(Drag::Nodes { ids, .. }) => {
                // Al soltar, se acomodan a una grilla de 8.
                for n in g.nodes.iter_mut().filter(|n| ids.contains(&n.id)) {
                    n.x = (n.x / 8.).round() * 8.;
                    n.y = (n.y / 8.).round() * 8.;
                }
            }
            Some(Drag::Box { from, to, add }) => {
                let r = Rect::from_two_pos(from, to);
                let hit: BTreeSet<NodeId> = geos.iter().filter(|(_, gm)| gm.rect.intersects(r)).map(|(id, _)| *id).collect();
                if add {
                    st.sel.extend(hit);
                } else {
                    st.sel = hit;
                }
            }
            Some(Drag::Wire { from, to }) => soltar_cable(g, st, &geos, from, to, to_screen(to), cx),
            _ => {}
        }
    }
    // ---- fondo ----
    if bg.drag_started_by(PointerButton::Primary) {
        if let Some(p) = pointer {
            st.drag = Some(Drag::Box { from: p, to: p, add: shift });
        }
        st.menu = None;
    }
    if bg.clicked() {
        st.sel.clear();
        (st.menu, st.paleta) = (None, None);
    }
    let abrir_paleta = |st: &mut Vista, p: Pos2, g: &Graph| {
        let tab = if g.nodes.iter().any(|n| matches!(n.kind, NodeKind::Event { .. })) { 2 } else { 0 };
        st.paleta = Some(Paleta { screen: to_screen(p), at: p, modo: PalModo::Nuevo(None), search: String::new(), tab, mas: false, focus: true });
        st.menu = None;
    };
    if edit_on && (bg.secondary_clicked() || bg.double_clicked()) {
        if let Some(p) = pointer {
            abrir_paleta(st, p, g);
        }
    }

    // ---- teclado ----
    let over = ui.rect_contains_pointer(outer);
    let keys = over && !ctx.egui_wants_keyboard_input() && st.paleta.is_none() && st.editar.is_none();
    if keys {
        let (cmd, none) = (Modifiers::COMMAND, Modifiers::NONE);
        let k = ctx.input_mut(|i| {
            (
                i.consume_key(none, Key::Delete) || i.consume_key(none, Key::Backspace),
                i.consume_key(cmd, Key::C),
                i.consume_key(cmd, Key::V),
                i.consume_key(cmd, Key::D),
                i.consume_key(cmd, Key::A),
                i.consume_key(none, Key::F),
                i.consume_key(none, Key::Space) || i.consume_key(none, Key::Tab),
                i.consume_key(none, Key::Escape),
            )
        });
        let (del, copy, paste, dup, all, f, pal, esc) = k;
        if del && edit_on && !st.sel.is_empty() {
            let n = borrar(g, &st.sel);
            st.sel.clear();
            out.msg = Some(format!("{n} nodo{} borrado{} · Ctrl+Z para deshacer", if n == 1 { "" } else { "s" }, if n == 1 { "" } else { "s" }));
        }
        if copy || dup {
            st.clip = copiar(g, &st.sel);
        }
        if (paste || dup) && edit_on {
            if let Some(c) = st.clip.clone() {
                let at = if dup { bounds(&c, None).map(|b| b.min + Vec2::splat(40.)) } else { pointer };
                st.sel = pegar(g, &c, at);
                let (n, pl) = (st.sel.len(), if st.sel.len() == 1 { "" } else { "s" });
                out.msg = Some(format!("{n} nodo{pl} {}{pl}", if dup { "duplicado" } else { "pegado" }));
            }
        }
        if all {
            st.sel = g.nodes.iter().map(|n| n.id).collect();
        }
        if f {
            let only = (!st.sel.is_empty()).then(|| st.sel.clone());
            sr = frame(g, only.as_ref(), outer.size());
        }
        if pal && edit_on {
            if let Some(p) = pointer {
                abrir_paleta(st, p, g);
            }
        }
        if esc {
            st.sel.clear();
            st.menu = None;
        }
    }
    st.cams.insert(cx.key.to_string(), sr);

    // ---- encima del lienzo ----
    estado(&ctx, outer, g, st, cx, &geos, &mut sr);
    st.cams.insert(cx.key.to_string(), sr);
    if g.nodes.is_empty() && !cx.solo_ver {
        egui::Area::new(Id::unique(("nodos_vacio", cx.key))).order(Order::Middle).fixed_pos(outer.center() - Vec2::new(210., 70.)).show(&ctx, |ui| {
            Frame::new().fill(PANEL).stroke((1., BORDER)).corner_radius(12).inner_margin(18).show(ui, |ui| {
                ui.set_width(384.);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(ic::GRAPH).size(30.).color(ACCENT));
                    ui.label(RichText::new("Un grafo vacío").font(sb(17.)));
                    ui.label(RichText::new("Haz clic derecho (o aprieta Espacio) para agregar tu primer nodo. Empieza por un «Cuando», y une sus flechas con lo que quieres que pase.").color(TEXT_SOFT));
                    ui.add_space(6.);
                    if primary(ui, (on_accent(ic::PLUS), on_accent("Agregar nodo").font(sb(14.)))).clicked() {
                        let at = to_global.inverse() * outer.center();
                        st.paleta = Some(Paleta { screen: outer.center() - Vec2::new(220., 120.), at, modo: PalModo::Nuevo(None), search: String::new(), tab: 0, mas: false, focus: true });
                    }
                });
            });
        });
    }
    if let Some(m) = menu(&ctx, g, st, edit_on) {
        out.msg = Some(m);
    }
    editar(&ctx, g, st, cx, &mut out);
    paleta(&ctx, g, st, cx, &geos);
    out
}

struct NodoCx<'a> {
    sel: bool,
    errs: &'a [&'a str],
    lejos: bool,
    wired: &'a HashMap<(NodeId, u8), NodeId>,
    titles: &'a HashMap<NodeId, String>,
    cx: &'a Cx<'a>,
    edit_on: bool,
}

fn nodo(ui: &mut Ui, n: &mut Node, gm: &Geo, nc: &NodoCx, acts: &mut Vec<A>) {
    let r = gm.rect;
    let cx = nc.cx;
    let col = node_color(n);
    let resp = ui.interact(r, Id::unique(("nodo", cx.key, n.id)), Sense::click_and_drag());
    if resp.clicked() {
        acts.push(A::Click(n.id, false));
    }
    if resp.drag_started_by(PointerButton::Primary) {
        acts.push(A::DragStart(n.id));
    }
    if resp.dragged_by(PointerButton::Secondary) || resp.dragged_by(PointerButton::Middle) {
        acts.push(A::Pan(resp.drag_delta()));
    }
    if resp.secondary_clicked() && nc.edit_on {
        acts.push(A::Menu(n.id));
    }
    let help = node_help(n);
    if !help.is_empty() {
        resp.on_hover_text(&help);
    }
    let alpha = if nc.lejos || n.off { 0.55 } else { 1. };
    let p = ui.painter().clone();
    p.rect_filled(r.translate(Vec2::new(0., 3.)), 10, Color32::from_black_alpha(70));
    p.rect_filled(r, 10, SURFACE.gamma_multiply(alpha).to_opaque());
    let head = Rect::from_min_size(r.min, Vec2::new(r.width(), ng::HEAD_H));
    let event = matches!(n.kind, NodeKind::Event { .. });
    let hfill = blend(col, SURFACE, if event { 0.62 } else if matches!(n.kind, NodeKind::Data { .. }) { 0.3 } else { 0.4 });
    let rounded = if gm.rows.is_empty() { CornerRadius::same(10) } else { CornerRadius { nw: 10, ne: 10, sw: 0, se: 0 } };
    p.rect_filled(head, rounded, hfill.gamma_multiply(alpha).to_opaque());
    let (bw, bc) = if !nc.errs.is_empty() {
        (2., DANGER)
    } else if nc.sel {
        (2., ACCENT)
    } else {
        (1., if event { blend(col, SURFACE, 0.8) } else { BORDER_STRONG })
    };
    p.rect_stroke(r, 10, Stroke::new(bw, bc), egui::StrokeKind::Outside);
    // Cabecera: ícono y título.
    let left = r.left() + if gm.ein.is_some() { 18. } else { 12. };
    let right_pad = if !gm.outs.is_empty() || gm.dout.is_some() { 20. } else { 10. };
    let tc = if event { Color32::WHITE } else { TEXT };
    p.text(Pos2::new(left, head.center().y), Align2::LEFT_CENTER, node_icon(n), FontId::proportional(16.), if event { Color32::WHITE } else { col });
    let mut t = nc.titles.get(&n.id).cloned().unwrap_or_default();
    if n.not {
        t = match t.split_once(": ") {
            Some((a, b)) => format!("{a} NO: {b}"),
            None => format!("NO: {t}"),
        };
    }
    let out0 = ng::exec_outs(n).first().cloned().unwrap_or_default();
    let out0_w = if out0.is_empty() { 0. } else { p.layout_no_wrap(out0.clone(), FontId::proportional(12.), TEXT_SOFT).size().x + 6. };
    let maxw = r.right() - right_pad - out0_w - (left + 24.);
    let mut g = p.layout_no_wrap(t.clone(), sb(14.), tc);
    let mut cut = t.chars().count();
    while g.size().x > maxw && cut > 4 {
        cut -= 2;
        g = p.layout_no_wrap(trunc(&t, cut), sb(14.), tc);
    }
    p.galley(Pos2::new(left + 24., head.center().y - g.size().y / 2.), g, tc);
    if !out0.is_empty() {
        p.text(Pos2::new(r.right() - right_pad, head.center().y), Align2::RIGHT_CENTER, &out0, FontId::proportional(12.), TEXT_SOFT);
    }
    if n.off {
        p.text(Pos2::new(r.right() - right_pad - out0_w, r.top() - 4.), Align2::RIGHT_BOTTOM, "desactivado", FontId::proportional(12.), TEXT_MUTED);
    }
    // Filas.
    let specs = ng::arg_specs(n);
    let outs = ng::exec_outs(n);
    let op = op_of(n).to_string();
    for (i, row) in gm.rows.iter().enumerate() {
        let rr = row_rect(r, i);
        match row {
            Row::Out(k) => {
                let l = outs.get(*k as usize).cloned().unwrap_or_default();
                p.text(Pos2::new(r.right() - 16., rr.center().y), Align2::RIGHT_CENTER, l, FontId::proportional(13.), TEXT_SOFT);
            }
            Row::SeqButtons => {
                if nc.edit_on {
                    let b = Rect::from_min_size(Pos2::new(r.left() + 12., rr.top() + 3.), Vec2::new(26., 22.));
                    if ui.put(b, Button::new(RichText::new(ic::PLUS).size(13.))).on_hover_text("Otra salida").clicked() {
                        acts.push(A::SeqAdd(n.id));
                    }
                    if ui.put(b.translate(Vec2::new(30., 0.)), Button::new(RichText::new(ic::MINUS).size(13.))).on_hover_text("Una salida menos").clicked() {
                        acts.push(A::SeqDel(n.id));
                    }
                }
            }
            Row::Arg(k) => {
                let s = &specs[*k];
                let x0 = r.left() + 14.;
                let lg = p.layout_no_wrap(trunc(&s.name, 16), FontId::proportional(12.5), TEXT_MUTED);
                let lw = lg.size().x.min(LBL_W);
                p.galley(Pos2::new(x0, rr.center().y - lg.size().y / 2.), lg, TEXT_MUTED);
                let cell = Rect::from_min_max(Pos2::new(x0 + lw + 8., rr.top() + 2.), Pos2::new(r.right() - 10., rr.bottom() - 2.));
                if let Some(src) = nc.wired.get(&(n.id, *k as u8)) {
                    let name = nc.titles.get(src).cloned().unwrap_or_default();
                    p.text(Pos2::new(cell.left(), cell.center().y), Align2::LEFT_CENTER, format!("{} {}", ic::PLUGS_CONNECTED, trunc(&name, 18)), FontId::proportional(12.5), TEXT_SOFT);
                    continue;
                }
                valor(ui, n, *k, s, cell, &op, nc, acts);
            }
            Row::Sub(j) => {
                let txt = n.sub.get(*j).map(catalog::sentence).unwrap_or_default();
                let cell = Rect::from_min_max(Pos2::new(r.left() + 14., rr.top() + 3.), Pos2::new(r.right() - 36., rr.bottom() - 3.));
                let st = facil::pill_style(Ficha::Hueco, col);
                let rsp = chip(ui, cell, &trunc(&txt, 26), &st, Id::unique(("sub", cx.key, n.id, *j)), nc.edit_on).on_hover_text(&txt);
                if rsp.clicked() {
                    acts.push(A::Edit(n.id, 0, Some(*j), rsp.rect));
                }
                if nc.edit_on {
                    let b = Rect::from_min_size(Pos2::new(r.right() - 32., rr.top() + 3.), Vec2::new(22., 22.));
                    if ui.put(b, Button::new(RichText::new(ic::X).size(12.)).frame_when_inactive(false)).on_hover_text("Quitar").clicked() {
                        acts.push(A::DelSub(n.id, *j));
                    }
                }
            }
            Row::AddSub => {
                if nc.edit_on {
                    let b = Rect::from_min_size(Pos2::new(r.left() + 12., rr.top() + 3.), Vec2::new(140., 22.));
                    let rsp = ui.put(b, Button::new((RichText::new(ic::PLUS).size(12.).color(col), RichText::new("otra condición").size(12.5))));
                    if rsp.clicked() {
                        acts.push(A::AddSub(n.id, rsp.rect));
                    }
                }
            }
        }
    }
    // Errores debajo del nodo.
    if !nc.errs.is_empty() {
        let txt = nc.errs.iter().take(3).map(|e| format!("{} {e}", ic::WARNING)).collect::<Vec<_>>().join("\n");
        let g = p.layout(txt, FontId::proportional(12.5), facil::ROJO_TXT, r.width() + 40.);
        let br = Rect::from_min_size(Pos2::new(r.left(), r.bottom() + 6.), g.size() + Vec2::new(16., 10.));
        p.rect(br, 6, blend(DANGER, BG, 0.2), Stroke::new(1., blend(DANGER, BG, 0.6)), egui::StrokeKind::Inside);
        p.galley(br.min + Vec2::new(8., 5.), g, facil::ROJO_TXT);
    }
}

#[allow(clippy::too_many_arguments)]
fn valor(ui: &mut Ui, n: &mut Node, k: usize, s: &ArgSpec, cell: Rect, op: &str, nc: &NodoCx, acts: &mut Vec<A>) {
    let cx = nc.cx;
    let def = match &n.kind {
        NodeKind::Data { f } => ng::data_def(f).and_then(|d| d.ins.get(k)).map_or("", |i| i.def),
        _ => "",
    };
    let v = n.args.get(k).cloned().unwrap_or_else(|| def.to_string());
    if matches!(&n.kind, NodeKind::Data { f } if f == "bool") {
        let on = matches!(v.as_str(), "1" | "true");
        let st = facil::pill_style(Ficha::Normal, CHIP);
        if chip(ui, cell, if on { "sí" } else { "no" }, &st, Id::unique(("arg", cx.key, n.id, k)), nc.edit_on).on_hover_text("Clic para cambiar").clicked() {
            acts.push(A::Toggle(n.id));
        }
        return;
    }
    if campo(n, s, cx.facil) {
        let mut t = v.clone();
        let mono = matches!(s.p, P::Expr);
        let mut cu = ui.new_child(UiBuilder::new().max_rect(cell).layout(Layout::left_to_right(Align::Center)));
        if !nc.edit_on {
            cu.disable();
        }
        let te = TextEdit::singleline(&mut t)
            .font(if mono { FontId::monospace(12.5) } else { FontId::proportional(13.) })
            .margin(Margin::symmetric(5, 2))
            .desired_width(cell.width());
        let r = cu.add_sized(Vec2::new(cell.width(), 22.), te);
        if s.p == P::Expr {
            if let Err(e) = nicostation::expr::parse(&t) {
                ui.painter().rect_stroke(r.rect, 4, Stroke::new(1.5, DANGER), egui::StrokeKind::Outside);
                r.on_hover_text(facil::expr_amable(&e));
            }
        }
        if t != v {
            if n.args.len() <= k {
                let defs: Vec<String> = match &n.kind {
                    NodeKind::Data { f } => ng::data_def(f).map_or(vec![], |d| d.ins.iter().map(|i| i.def.to_string()).collect()),
                    _ => vec![],
                };
                while n.args.len() <= k {
                    n.args.push(defs.get(n.args.len()).cloned().unwrap_or_default());
                }
            }
            n.args[k] = t;
        }
        return;
    }
    let (txt, f, tip) = facil::ficha(op, k, s.p, &v, cx.l);
    let col = node_color(n);
    let st = facil::pill_style(f, col);
    let r = chip(ui, cell, &trunc(&txt, 20), &st, Id::unique(("arg", cx.key, n.id, k)), nc.edit_on);
    let r = r.on_hover_text(tip.unwrap_or_else(|| "Clic para cambiar".into()));
    if r.clicked() {
        acts.push(A::Edit(n.id, k, None, r.rect));
    }
}

fn comentario(ui: &mut Ui, n: &mut Node, gm: &Geo, sel: bool, edit_on: bool, acts: &mut Vec<A>) {
    let r = gm.rect;
    let p = ui.painter().clone();
    p.rect(r, 10, Color32::from_rgba_unmultiplied(0x7a, 0x7a, 0x90, 22), Stroke::new(if sel { 2. } else { 1. }, if sel { ACCENT } else { BORDER_STRONG }), egui::StrokeKind::Inside);
    let head = Rect::from_min_size(r.min, Vec2::new(r.width(), 34.));
    p.rect_filled(head, CornerRadius { nw: 10, ne: 10, sw: 0, se: 0 }, Color32::from_rgba_unmultiplied(0x7a, 0x7a, 0x90, 40));
    let resp = ui.interact(head, Id::unique(("coment", n.id)), Sense::click_and_drag());
    if resp.clicked() {
        acts.push(A::Click(n.id, false));
    }
    if resp.drag_started_by(PointerButton::Primary) {
        acts.push(A::DragStart(n.id));
    }
    if resp.secondary_clicked() && edit_on {
        acts.push(A::Menu(n.id));
    }
    p.text(Pos2::new(r.left() + 12., head.center().y), Align2::LEFT_CENTER, ic::NOTE, FontId::proportional(16.), WARN);
    let text = n.args.first().cloned().unwrap_or_default();
    let tr = Rect::from_min_max(Pos2::new(r.left() + 36., r.top() + 8.), Pos2::new(r.right() - 40., r.bottom() - 8.));
    let g = p.layout(text, sb(15.), TEXT_SOFT, tr.width());
    p.with_clip_rect(tr.intersect(ui.clip_rect())).galley(tr.min, g, TEXT_SOFT);
    if edit_on {
        let b = Rect::from_min_size(Pos2::new(r.right() - 34., r.top() + 6.), Vec2::new(24., 22.));
        if ui.put(b, Button::new(RichText::new(ic::PENCIL).size(13.)).frame_when_inactive(false)).on_hover_text("Cambiar el texto").clicked() {
            acts.push(A::Edit(n.id, 0, None, Rect::from_min_size(b.left_bottom(), Vec2::ZERO)));
        }
        let h = Rect::from_min_size(r.right_bottom() - Vec2::splat(14.), Vec2::splat(14.));
        p.line_segment([h.right_top() + Vec2::new(-2., 4.), h.left_bottom() + Vec2::new(4., -2.)], Stroke::new(1.5, TEXT_FAINT));
        let hr = ui.interact(h, Id::unique(("coment_tam", n.id)), Sense::drag()).on_hover_cursor(egui::CursorIcon::ResizeNwSe);
        if hr.drag_started() {
            acts.push(A::Resize(n.id));
        }
    }
}

/// Al soltar un cable: sobre un pin conecta; sobre un nodo, al primer pin que encaje; en el vacío abre el buscador.
fn soltar_cable(g: &mut Graph, st: &mut Vista, geos: &HashMap<NodeId, Geo>, from: Pin, to: Pos2, screen: Pos2, cx: &Cx) {
    let mut best: Option<(f32, Pin)> = None;
    for (id, gm) in geos {
        for (pin, pos) in all_pins(*id, gm) {
            let d = pos.distance(to);
            if d < 16. && pair(geos, from, pin).is_some() && best.is_none_or(|b| d < b.0) {
                best = Some((d, pin));
            }
        }
    }
    if best.is_none() {
        if let Some((id, gm)) = geos.iter().find(|(_, gm)| gm.rect.contains(to)) {
            best = all_pins(*id, gm).into_iter().find(|(p, _)| pair(geos, from, *p).is_some()).map(|(p, _)| (0., p));
        }
    }
    match best {
        Some((_, pin)) => {
            if let Some((o, i)) = pair(geos, from, pin) {
                connect(g, o, i);
            }
        }
        None if !geos.values().any(|gm| gm.rect.contains(to)) && !cx.solo_ver => {
            let tab = match from {
                Pin::ExecOut(..) => 2,
                Pin::ExecIn(_) => 0,
                Pin::DataOut(_) => 2,
                Pin::DataIn(..) => 4,
            };
            st.paleta = Some(Paleta { screen, at: to, modo: PalModo::Nuevo(Some(from)), search: String::new(), tab, mas: false, focus: true });
        }
        None => {}
    }
}

/// Borra nodos y sus cables. Devuelve cuántos.
pub fn borrar(g: &mut Graph, ids: &BTreeSet<NodeId>) -> usize {
    let before = g.nodes.len();
    g.nodes.retain(|n| !ids.contains(&n.id));
    g.flow.retain(|f| !ids.contains(&f.from) && !ids.contains(&f.to));
    g.wires.retain(|w| !ids.contains(&w.from) && !ids.contains(&w.to));
    before - g.nodes.len()
}

fn copiar(g: &Graph, ids: &BTreeSet<NodeId>) -> Option<Graph> {
    if ids.is_empty() {
        return None;
    }
    Some(Graph {
        name: String::new(),
        nodes: g.nodes.iter().filter(|n| ids.contains(&n.id)).cloned().collect(),
        flow: g.flow.iter().filter(|f| ids.contains(&f.from) && ids.contains(&f.to)).copied().collect(),
        wires: g.wires.iter().filter(|w| ids.contains(&w.from) && ids.contains(&w.to)).copied().collect(),
    })
}

/// Pega una copia con números nuevos; `at` = dónde va la esquina de arriba a la izquierda.
fn pegar(g: &mut Graph, c: &Graph, at: Option<Pos2>) -> BTreeSet<NodeId> {
    let base = next_id(g);
    let map: HashMap<NodeId, NodeId> = c.nodes.iter().enumerate().map(|(k, n)| (n.id, base + k as NodeId)).collect();
    let b = bounds(c, None).map_or(Pos2::ZERO, |b| b.min);
    let off = at.map_or(Vec2::splat(40.), |a| a - b);
    let mut out = BTreeSet::new();
    for n in &c.nodes {
        let mut m = n.clone();
        m.id = map[&n.id];
        m.x = ((n.x + off.x) / 8.).round() * 8.;
        m.y = ((n.y + off.y) / 8.).round() * 8.;
        out.insert(m.id);
        g.nodes.push(m);
    }
    g.flow.extend(c.flow.iter().map(|f| Flow { from: map[&f.from], out: f.out, to: map[&f.to] }));
    g.wires.extend(c.wires.iter().map(|w| Wire { from: map[&w.from], to: map[&w.to], arg: w.arg }));
    out
}

pub fn alinear(g: &mut Graph, ids: &BTreeSet<NodeId>, vertical: bool) {
    let sel: Vec<&Node> = g.nodes.iter().filter(|n| ids.contains(&n.id)).collect();
    let Some(m) = sel.iter().map(|n| if vertical { n.y } else { n.x }).reduce(f32::min) else { return };
    for n in g.nodes.iter_mut().filter(|n| ids.contains(&n.id)) {
        if vertical {
            n.y = m;
        } else {
            n.x = m;
        }
    }
}

/// Barra de estado del lienzo: si compila (y cuántos eventos), errores sin nodo y ayuda.
fn estado(ctx: &egui::Context, outer: Rect, g: &Graph, st: &mut Vista, cx: &Cx, geos: &HashMap<NodeId, Geo>, sr: &mut Rect) {
    let n_err = cx.probs.len();
    egui::Area::new(Id::unique(("nodos_estado", cx.key))).order(Order::Middle).fixed_pos(outer.left_bottom() + Vec2::new(12., -46.)).show(ctx, |ui| {
        Frame::new().fill(PANEL).stroke((1., BORDER)).corner_radius(8).inner_margin(Margin::symmetric(10, 6)).show(ui, |ui| {
            ui.horizontal(|ui| {
                if cx.solo_ver {
                    ui.label(RichText::new(ic::EYE).color(TEXT_MUTED));
                    ui.label(RichText::new("La hoja vista como nodos (solo para mirar)").color(TEXT_SOFT));
                    return;
                }
                if n_err == 0 {
                    ui.label(RichText::new(ic::CHECK_CIRCLE).color(OK));
                    let t = format!("Compila: {} evento{}", cx.n_eventos, if cx.n_eventos == 1 { "" } else { "s" });
                    ui.label(RichText::new(t).color(TEXT));
                } else {
                    ui.label(RichText::new(ic::WARNING_CIRCLE).color(DANGER));
                    let t = format!("{n_err} problema{} por arreglar", if n_err == 1 { "" } else { "s" });
                    let r = ui.add(egui::Label::new(RichText::new(t).color(facil::ROJO_TXT).underline()).sense(Sense::click()));
                    if r.on_hover_text("Clic para ir al nodo con el problema").clicked() {
                        let ids: Vec<NodeId> = cx.probs.iter().filter_map(|p| p.0).filter(|id| geos.contains_key(id)).collect();
                        if !ids.is_empty() {
                            let id = ids[st.ir_error % ids.len()];
                            st.ir_error += 1;
                            st.sel = BTreeSet::from([id]);
                            *sr = Rect::from_center_size(geos[&id].rect.center(), sr.size());
                        }
                    }
                    let general: Vec<&String> = cx.probs.iter().filter(|p| p.0.is_none()).map(|p| &p.1).collect();
                    if let Some(m) = general.first() {
                        ui.label(RichText::new(trunc(m, 60)).color(facil::ROJO_TXT));
                    }
                }
                let reach = ng::reachable(g);
                let sueltos = g.nodes.iter().filter(|n| ng::has_exec_in(n) && !reach.contains(&n.id)).count();
                if sueltos > 0 {
                    ui.label(RichText::new(format!("· {sueltos} sin conectar a un «Cuando»")).color(TEXT_MUTED))
                        .on_hover_text("Los nodos que no tienen flechas desde un nodo de evento no hacen nada (se ven más apagados).");
                }
            });
        });
    });
    let hint = if cx.facil {
        "Clic derecho: agregar · Rueda: acercar · Botón derecho arrastrando: mover · F: ver todo"
    } else {
        "Clic derecho / Espacio: agregar · Rueda: zoom · Arrastrar (der./medio): mover · Supr, Ctrl+C/V/D, F"
    };
    let fg = ctx.fonts_mut(|f| f.layout_no_wrap(hint.into(), FontId::proportional(12.), TEXT_FAINT));
    if !cx.solo_ver && outer.width() > fg.size().x + 40. {
        ctx.layer_painter(egui::LayerId::new(Order::Middle, Id::unique("nodos_ayuda"))).galley(Pos2::new(outer.right() - fg.size().x - 14., outer.top() + 10.), fg, TEXT_FAINT);
    }
}

fn menu(ctx: &egui::Context, g: &mut Graph, st: &mut Vista, edit_on: bool) -> Option<String> {
    let (id, pos) = st.menu?;
    let Some(n) = g.nodes.iter().find(|n| n.id == id).cloned() else {
        st.menu = None;
        return None;
    };
    let mut msg = None;
    let mut close = false;
    let r = egui::Area::new(Id::unique("nodos_menu")).order(Order::Foreground).fixed_pos(pos).show(ctx, |ui| {
        Frame::menu(ui.style()).show(ui, |ui| {
            menu_spacing(ui);
            let item = |ui: &mut Ui, icon: &str, t: &str| ui.add(Button::new(it(icon, t)).frame_when_inactive(false).min_size(Vec2::new(ui.available_width(), 28.))).clicked();
            if item(ui, ic::COPY, "Duplicar (Ctrl+D)") {
                st.clip = copiar(g, &st.sel);
                if let Some(c) = st.clip.clone() {
                    st.sel = pegar(g, &c, bounds(&c, None).map(|b| b.min + Vec2::splat(40.)));
                }
                close = true;
            }
            if item(ui, ic::CLIPBOARD, "Copiar (Ctrl+C)") {
                st.clip = copiar(g, &st.sel);
                close = true;
            }
            if matches!(&n.kind, NodeKind::Cond { .. }) || matches!(&n.kind, NodeKind::Event { op } if !op.is_empty()) {
                if item(ui, ic::X_CIRCLE, if n.not { "Quitar el NO" } else { "Al revés (NO)" }) {
                    if let Some(m) = g.nodes.iter_mut().find(|m| m.id == id) {
                        m.not = !m.not;
                    }
                    close = true;
                }
            }
            if !matches!(n.kind, NodeKind::Comment { .. } | NodeKind::Data { .. }) && item(ui, if n.off { ic::EYE } else { ic::EYE_SLASH }, if n.off { "Activar" } else { "Desactivar" }) {
                if let Some(m) = g.nodes.iter_mut().find(|m| m.id == id) {
                    m.off = !m.off;
                }
                close = true;
            }
            if item(ui, ic::LINK_BREAK, "Quitar sus cables") {
                g.flow.retain(|f| f.from != id && f.to != id);
                g.wires.retain(|w| w.from != id && w.to != id);
                close = true;
            }
            ui.separator();
            if ui.add(Button::new(danger_it("Borrar (Supr)")).frame_when_inactive(false).min_size(Vec2::new(ui.available_width(), 28.))).clicked() && edit_on {
                let k = borrar(g, &st.sel);
                st.sel.clear();
                msg = Some(format!("{k} nodo{} borrado{} · Ctrl+Z para deshacer", if k == 1 { "" } else { "s" }, if k == 1 { "" } else { "s" }));
                close = true;
            }
        });
    });
    let outside = ctx.input(|i| i.pointer.any_pressed()) && !r.response.contains_pointer();
    if close || outside || ctx.input(|i| i.key_pressed(Key::Escape)) {
        st.menu = None;
    }
    msg
}

/// Ventanita para cambiar un parámetro, una sub-condición o el texto de un comentario.
fn editar(ctx: &egui::Context, g: &mut Graph, st: &mut Vista, cx: &Cx, out: &mut Salida) {
    let Some(ed) = st.editar.clone() else { return };
    let Some(n) = g.nodes.iter_mut().find(|n| n.id == ed.node) else {
        st.editar = None;
        return;
    };
    let mut close = false;
    let fr = Frame::popup(&ctx.global_style()).inner_margin(14);
    let screen = ctx.content_rect();
    let pos = Pos2::new(ed.screen.x.min(screen.right() - 480.).max(screen.left() + 8.), ed.screen.y.min(screen.bottom() - 260.).max(screen.top() + 8.));
    let r = egui::Area::new(Id::unique("nodos_editar")).order(Order::Foreground).fixed_pos(pos).show(ctx, |ui| {
        fr.show(ui, |ui| {
            ui.set_max_width(if ed.k == 0 && ed.sub.is_none() && matches!(n.kind, NodeKind::Comment { .. }) { 420. } else { 460. });
            if let NodeKind::Comment { .. } = n.kind {
                facil::titulo(ui, "Texto del comentario");
                if n.args.is_empty() {
                    n.args.push(String::new());
                }
                ui.add(TextEdit::multiline(&mut n.args[0]).desired_width(400.).desired_rows(4));
                if ui.button(it(ic::CHECK, "Listo")).clicked() {
                    close = true;
                }
                return;
            }
            if let Some(j) = ed.sub {
                // Sub-condición de «O» / «Y»: sus parámetros uno debajo del otro.
                let Some(ins) = n.sub.get_mut(j) else {
                    close = true;
                    return;
                };
                let Some(m) = catalog::meta(&ins.op) else {
                    ui.label(RichText::new(format!("Condición desconocida «{}»", ins.op)).color(facil::ROJO_TXT));
                    return;
                };
                facil::titulo(ui, m.label);
                ui.label(RichText::new(catalog::sentence(ins)).color(TEXT_SOFT));
                ui.add_space(4.);
                ins.args.resize(m.params.len().max(ins.args.len()), String::new());
                let a0 = ins.args.first().cloned().unwrap_or_default();
                egui::ScrollArea::vertical().max_height(380.).show(ui, |ui| {
                    for (k, (name, t)) in m.params.iter().enumerate() {
                        ui.label(RichText::new(facil::pregunta(m.op, k, name, *t)).font(sb(14.)).color(TEXT_MUTED));
                        let acx = ArgCx { l: cx.l, op: m.op, k, obj: &a0, cond: true };
                        facil::editor(ui, *t, &mut ins.args[k], &acx, &mut out.new_var);
                        ui.add_space(4.);
                    }
                });
                ui.checkbox(&mut ins.not, "Al revés (NO)");
                if ui.button(it(ic::CHECK, "Listo")).clicked() {
                    close = true;
                }
                return;
            }
            let specs = ng::arg_specs(n);
            let Some(s) = specs.get(ed.k) else {
                close = true;
                return;
            };
            let op = op_of(n).to_string();
            let cond = matches!(n.kind, NodeKind::Cond { .. } | NodeKind::Event { .. });
            facil::titulo(ui, &if s.name.starts_with("Ingrediente") { s.name.clone() } else { facil::pregunta(&op, ed.k, &s.name, s.p) });
            ui.add_space(4.);
            let def = match &n.kind {
                NodeKind::Data { f } => ng::data_def(f).map_or(vec![], |d| d.ins.iter().map(|i| i.def.to_string()).collect()),
                _ => vec![],
            };
            while n.args.len() <= ed.k {
                n.args.push(def.get(n.args.len()).cloned().unwrap_or_default());
            }
            let a0 = n.args.first().cloned().unwrap_or_default();
            let acx = ArgCx { l: cx.l, op: &op, k: ed.k, obj: &a0, cond };
            let elegido = facil::editor(ui, s.p, &mut n.args[ed.k], &acx, &mut out.new_var);
            if s.p == P::Func {
                facil::ajustar_func(&mut n.args, cx.l);
            }
            if elegido && facil::cierra(s.p) {
                close = true;
            }
            if s.pin.is_some() {
                ui.add_space(4.);
                ui.label(RichText::new(format!("{} También puedes enchufarle un cable desde un nodo de datos.", ic::INFO)).small().color(TEXT_MUTED));
            }
        });
    });
    let fresh = ctx.cumulative_frame_nr() <= ed.abierta + 1;
    let outside = !fresh && ctx.input(|i| i.pointer.any_pressed()) && !r.response.contains_pointer() && !sobre_otro_popup(ctx);
    if close || outside || ctx.input(|i| i.key_pressed(Key::Escape)) {
        st.editar = None;
    }
}

/// ¿El puntero está sobre otra ventanita (un menú abierto desde la ventanita de edición)?
fn sobre_otro_popup(ctx: &egui::Context) -> bool {
    ctx.layer_id_at(ctx.input(|i| i.pointer.latest_pos()).unwrap_or_default()).is_some_and(|l| l.order == Order::Foreground && l.id != Id::unique("nodos_editar"))
}

// ---------- buscador de nodos ----------

struct Entrada {
    kind: NodeKind,
    args: Vec<String>,
    label: String,
    help: String,
    color: Color32,
    icon: &'static str,
    tab: usize,
    group: &'static str,
    basic: bool,
}

const TABS: [(&str, &str, &str); 5] = [
    ("Cuándo", "Cuando", "Eventos: dónde empieza cada camino de flechas"),
    ("Si...", "Si...", "Preguntas: siguen por «sí» o por «no»"),
    ("Hacer", "Acciones", "Acciones: lo que pasa en el juego"),
    ("Flujo", "Flujo", "Repetir, secuencias, esperar y tus bloques"),
    ("Datos", "Datos", "Números, cuentas, comparaciones y textos para enchufar en los parámetros"),
];

fn entradas(l: &Lists) -> Vec<Entrada> {
    let mut v = vec![];
    let cat = |op: &str| facil::cat_op(op).color;
    v.push(Entrada {
        kind: NodeKind::Event { op: String::new() }, args: vec![], label: "Cada cuadro (siempre)".into(),
        help: "Pasa todo el tiempo, muchas veces por segundo. Úsalo con nodos «Si» para preguntar cosas.".into(),
        color: CAT_TIME, icon: ic::CLOCK_CLOCKWISE, tab: 0, group: "Eventos", basic: true,
    });
    for op in ng::TRIGGERS {
        if let Some(m) = catalog::meta(op) {
            v.push(Entrada {
                kind: NodeKind::Event { op: op.to_string() }, args: facil::nuevo(m, l).args, label: m.label.into(), help: catalog::kid(op).help.into(),
                color: cat(op), icon: facil::op_icon(op), tab: 0, group: m.group, basic: true,
            });
        }
    }
    for m in CATALOG.iter().filter(|m| m.cond) {
        v.push(Entrada {
            kind: NodeKind::Cond { op: m.op.into() }, args: facil::nuevo(m, l).args, label: format!("Si: {}", m.label), help: catalog::kid(m.op).help.into(),
            color: cat(m.op), icon: facil::op_icon(m.op), tab: 1, group: m.group, basic: catalog::kid(m.op).basic || matches!(m.op, "or" | "and"),
        });
    }
    for m in CATALOG.iter().filter(|m| !m.cond && !matches!(m.op, "wait" | "call")) {
        v.push(Entrada {
            kind: NodeKind::Act { op: m.op.into() }, args: facil::nuevo(m, l).args, label: m.label.into(), help: catalog::kid(m.op).help.into(),
            color: cat(m.op), icon: facil::op_icon(m.op), tab: 2, group: m.group, basic: catalog::kid(m.op).basic,
        });
    }
    let flujo = |kind, args: Vec<String>, label: &str, help: &str, icon| Entrada { kind, args, label: label.into(), help: help.into(), color: FLOW_C, icon, tab: 3, group: "Flujo", basic: true };
    v.push(flujo(NodeKind::Seq { n: 2 }, vec![], "Secuencia", "Hace lo de la salida 1, después lo de la 2, y así.", ic::LIST_BULLETS));
    v.push(flujo(NodeKind::Repeat, vec!["3".into()], "Repetir", "Hace lo de «cada vez» varias veces seguidas.", ic::REPEAT));
    let obj = l.objs.iter().find(|o| !l.groups.contains(o)).cloned().unwrap_or_default();
    v.push(flujo(NodeKind::ForEach, vec![obj.clone()], "Para cada objeto", "Hace algo con cada copia de un objeto, una por una.", ic::STACK));
    v.push(flujo(NodeKind::While, vec![], "Mientras", "Repite mientras se cumplan los «Si» que le pongas al principio.", ic::ARROWS_CLOCKWISE));
    if let Some(m) = catalog::meta("wait") {
        v.push(Entrada { args: vec!["1".into()], color: CAT_TIME, icon: ic::HOURGLASS, ..flujo(NodeKind::Act { op: "wait".into() }, vec![], m.label, catalog::kid("wait").help, ic::HOURGLASS) });
    }
    for f in l.funcs.iter() {
        let op = if f.1 { "call_cond" } else { "call" };
        let args = std::iter::once(f.0.clone()).chain(f.2.iter().map(|_| "0".to_string())).collect();
        let kind = if f.1 { NodeKind::Cond { op: op.into() } } else { NodeKind::Act { op: op.into() } };
        v.push(Entrada { color: CAT_FUNC, ..flujo(kind, args, &format!("Mi bloque: {}", f.0), "Usa un bloque que inventaste en «Mis bloques».", ic::PUZZLE) });
    }
    v.push(flujo(NodeKind::Comment { w: 360., h: 120. }, vec!["Una nota".into()], "Comentario", "Un recuadro con una nota para explicar algo.", ic::NOTE));
    for d in ng::DATA {
        let basic = matches!(d.f, "num" | "text" | "var" | "prop" | "dt" | "add" | "sub" | "mul" | "random" | "eq" | "gt" | "lt" | "join" | "obj");
        let mut args: Vec<String> = d.ins.iter().map(|i| i.def.to_string()).collect();
        for (k, i) in d.ins.iter().enumerate() {
            match i.p {
                P::Object => args[k] = obj.clone(),
                P::Var if d.f == "global" => args[k] = l.globals.first().cloned().unwrap_or_default(),
                P::Var => args[k] = l.vars.first().or(l.locals.first()).cloned().unwrap_or_default(),
                _ => {}
            }
        }
        v.push(Entrada {
            kind: NodeKind::Data { f: d.f.into() }, args, label: d.label.into(), help: d.help.into(), color: pin_color(d.out), icon: data_icon(d.f), tab: 4,
            group: d.group, basic,
        });
    }
    v
}

fn temp(e: &Entrada) -> Node {
    Node { id: 0, kind: e.kind.clone(), x: 0., y: 0., args: e.args.clone(), not: false, sub: vec![], off: false }
}

/// ¿El nodo nuevo tiene un pin que encaje con el cable que se soltó?
fn encaja(e: &Entrada, from: Pin, geos: &HashMap<NodeId, Geo>) -> Option<Pin> {
    let n = temp(e);
    match from {
        Pin::ExecOut(..) => ng::has_exec_in(&n).then_some(Pin::ExecIn(0)),
        Pin::ExecIn(_) => (!ng::exec_outs(&n).is_empty()).then_some(Pin::ExecOut(0, 0)),
        Pin::DataOut(_) => {
            let t = data_type(geos, from)?;
            ng::arg_specs(&n).iter().position(|s| s.pin.is_some_and(|p| ng::compatible(t, p))).map(|k| Pin::DataIn(0, k as u8))
        }
        Pin::DataIn(..) => {
            let t = data_type(geos, from)?;
            ng::data_out(&n).filter(|o| ng::compatible(*o, t)).map(|_| Pin::DataOut(0))
        }
    }
}

fn paleta(ctx: &egui::Context, g: &mut Graph, st: &mut Vista, cx: &Cx, geos: &HashMap<NodeId, Geo>) {
    let Some(mut pal) = st.paleta.take() else { return };
    let all = entradas(cx.l);
    let q = facil::norm(pal.search.trim());
    let from = match pal.modo {
        PalModo::Nuevo(f) => f,
        PalModo::Sub(_) => None,
    };
    let sub = matches!(pal.modo, PalModo::Sub(_));
    let buscando = !q.is_empty();
    let mut visibles: Vec<(usize, Option<Pin>)> = all
        .iter()
        .enumerate()
        .filter(|(_, e)| !sub || (e.tab == 1 && matches!(&e.kind, NodeKind::Cond { op } if op != "call_cond")))
        .filter_map(|(k, e)| match from {
            Some(f) => encaja(e, f, geos).map(|p| (k, Some(p))),
            None => Some((k, None)),
        })
        .filter(|(k, _)| {
            let e = &all[*k];
            if buscando {
                facil::norm(&e.label).contains(&q) || facil::norm(&e.help).contains(&q) || facil::norm(e.group).contains(&q)
            } else {
                (sub || from.is_some() || e.tab == pal.tab) && (!cx.facil || pal.mas || e.basic || from.is_some())
            }
        })
        .collect();
    if buscando {
        // Primero los que tienen la palabra en el nombre; después los que la tienen en la ayuda.
        visibles.sort_by_key(|(k, _)| !facil::norm(&all[*k].label).contains(&q));
    }
    let mut pick: Option<(usize, Option<Pin>)> = None;
    let mut close = false;
    let screen = ctx.content_rect();
    let pos = Pos2::new(pal.screen.x.min(screen.right() - 470.).max(screen.left() + 8.), pal.screen.y.min(screen.bottom() - 520.).max(screen.top() + 8.));
    let r = egui::Area::new(Id::unique("nodos_paleta")).order(Order::Foreground).fixed_pos(pos).show(ctx, |ui| {
        Frame::popup(ui.style()).inner_margin(12).corner_radius(10).show(ui, |ui| {
            ui.set_width(440.);
            ui.horizontal(|ui| {
                ui.label(RichText::new(ic::PLUS).size(18.).color(ACCENT));
                let t = if sub { "Agregar una condición" } else if from.is_some() { "Agregar un nodo que encaje" } else { "Agregar un nodo" };
                ui.label(RichText::new(t).font(sb(16.)));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ib(ui, ic::X, "Cerrar (Esc)").clicked() {
                        close = true;
                    }
                });
            });
            let s = ui.add(TextEdit::singleline(&mut pal.search).hint_text(format!("{}  Buscar (por ejemplo: saltar, tecla, sumar)", ic::SEARCH)).desired_width(f32::INFINITY));
            if pal.focus {
                s.request_focus();
                pal.focus = false;
            }
            if !buscando && !sub && from.is_none() {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(4., 4.);
                    for (k, (kid, exp, tip)) in TABS.iter().enumerate() {
                        let icon = [ic::LIGHTNING, ic::GIT_BRANCH, ic::PLAY, ic::ARROWS_SPLIT, ic::MATH][k];
                        let sel = pal.tab == k;
                        let atoms = (RichText::new(icon).color(if sel { ACCENT } else { TEXT_MUTED }), RichText::new(if cx.facil { *kid } else { *exp }).font(sb(13.)));
                        if ui.add(Button::selectable(sel, atoms)).on_hover_text(*tip).clicked() {
                            pal.tab = k;
                        }
                    }
                });
            }
            ui.add_space(2.);
            egui::ScrollArea::vertical().max_height(380.).auto_shrink([false, true]).show(ui, |ui| {
                if visibles.is_empty() {
                    ui.label(RichText::new("Nada coincide.").color(TEXT_MUTED));
                }
                let mut last_group = "";
                for (k, p) in &visibles {
                    let e = &all[*k];
                    if e.group != last_group && !buscando {
                        ui.add_space(2.);
                        ui.label(RichText::new(e.group).font(sb(12.)).color(TEXT_MUTED));
                        last_group = e.group;
                    }
                    let (rect, rsp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.), Sense::click());
                    let hov = rsp.hovered();
                    let pt = ui.painter();
                    if hov {
                        pt.rect_filled(rect, 6, SURFACE_HOVER);
                    }
                    pt.rect_filled(Rect::from_min_size(rect.min + Vec2::new(2., 6.), Vec2::new(4., rect.height() - 12.)), 2, e.color);
                    pt.text(rect.left_center() + Vec2::new(16., 0.), Align2::LEFT_CENTER, e.icon, FontId::proportional(16.), e.color);
                    pt.text(rect.left_center() + Vec2::new(40., 0.), Align2::LEFT_CENTER, &e.label, FontId::proportional(14.5), TEXT);
                    if buscando {
                        pt.text(rect.right_center() - Vec2::new(8., 0.), Align2::RIGHT_CENTER, TABS[e.tab].0, FontId::proportional(12.), TEXT_FAINT);
                    }
                    let rsp = if e.help.is_empty() { rsp } else { rsp.on_hover_text(&e.help) };
                    if rsp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        pick = Some((*k, *p));
                    }
                }
            });
            if cx.facil && !buscando && from.is_none() {
                ui.checkbox(&mut pal.mas, "Ver todos los bloques (también los avanzados)");
            }
            ui.label(RichText::new("Enter elige el primero · Esc cierra").small().color(TEXT_FAINT));
        });
    });
    if pick.is_none() && ctx.input(|i| i.key_pressed(Key::Enter)) {
        pick = visibles.first().copied();
    }
    let outside = ctx.input(|i| i.pointer.any_pressed()) && !r.response.contains_pointer();
    if close || outside || ctx.input(|i| i.key_pressed(Key::Escape)) {
        return;
    }
    let Some((k, pin)) = pick else {
        st.paleta = Some(pal);
        return;
    };
    let e = &all[k];
    if let PalModo::Sub(id) = pal.modo {
        if let (Some(n), NodeKind::Cond { op }) = (g.nodes.iter_mut().find(|n| n.id == id), &e.kind) {
            n.sub.push(Instr { op: op.clone(), args: e.args.clone(), ..Default::default() });
        }
        return;
    }
    let id = next_id(g);
    let mut n = temp(e);
    n.id = id;
    let (w, _) = ng::size(&n);
    // El nodo nuevo queda con su pin justo donde se soltó el cable.
    let at = match from {
        Some(Pin::ExecIn(_)) | Some(Pin::DataIn(..)) => pal.at - Vec2::new(w, ng::HEAD_H / 2.),
        Some(_) => pal.at - Vec2::new(0., ng::HEAD_H / 2.),
        None => pal.at,
    };
    n.x = (at.x / 8.).round() * 8.;
    n.y = (at.y / 8.).round() * 8.;
    g.nodes.push(n);
    if let (Some(f), Some(p)) = (from, pin) {
        let p = match p {
            Pin::ExecIn(_) => Pin::ExecIn(id),
            Pin::ExecOut(_, k) => Pin::ExecOut(id, k),
            Pin::DataOut(_) => Pin::DataOut(id),
            Pin::DataIn(_, k) => Pin::DataIn(id, k),
        };
        let (o, i) = if is_out(f) { (f, p) } else { (p, f) };
        connect(g, o, i);
    }
    st.sel = BTreeSet::from([id]);
}

// ---------- renombrar referencias dentro de los grafos ----------

/// Renombra un objeto, escena o función en los parámetros de un grafo (como `ren_ins` en la hoja).
pub fn ren_grafo(g: &mut Graph, t: P, old: &str, new: &str, ren_expr: impl Fn(&str, &str, &str) -> String) {
    for n in &mut g.nodes {
        let specs = ng::arg_specs(n);
        for (k, a) in n.args.iter_mut().enumerate() {
            match specs.get(k).map(|s| s.p) {
                Some(x) if x == t => {
                    if a == old {
                        *a = new.into();
                    }
                }
                Some(P::Expr) if t == P::Object => *a = ren_expr(a, old, new),
                _ => {}
            }
        }
        for i in &mut n.sub {
            let ps = catalog::meta(&i.op).map_or(&[][..], |m| m.params);
            for (k, a) in i.args.iter_mut().enumerate() {
                match ps.get(k).map(|p| p.1) {
                    Some(x) if x == t && a == old => *a = new.into(),
                    Some(P::Expr) if t == P::Object => *a = ren_expr(a, old, new),
                    _ => {}
                }
            }
        }
    }
}

/// Renombra una cajita (variable) en un grafo.
pub fn ren_var_grafo(g: &mut Graph, old: &str, new: &str) {
    for n in &mut g.nodes {
        let specs = ng::arg_specs(n);
        for (k, a) in n.args.iter_mut().enumerate() {
            match specs.get(k).map(|s| s.p) {
                Some(P::Var) if a == old => *a = new.into(),
                Some(P::Expr) => *a = super::cajitas::ren_ident(a, old, new),
                _ => {}
            }
        }
        for i in &mut n.sub {
            for a in &mut i.args {
                *a = super::cajitas::ren_ident(a, old, new);
            }
        }
    }
}

// ---------- eventos compilados (solo para mirar) ----------

fn ver_eventos(ui: &mut Ui, evs: &[Event], depth: usize) {
    for e in evs {
        let tag = match e.kind {
            EventKind::Else => Some("Si no".to_string()),
            EventKind::Repeat => Some(format!("Repetir {} veces", e.text)),
            EventKind::ForEach => Some(format!("Para cada {}", e.text)),
            EventKind::While => Some("Mientras".into()),
            _ => None,
        };
        Frame::new().fill(SURFACE).stroke((1., BORDER)).corner_radius(8).inner_margin(Margin::symmetric(10, 6)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            if e.disabled {
                ui.label(RichText::new("desactivado").small().color(TEXT_MUTED));
            }
            if let Some(t) = tag {
                ui.label(RichText::new(t).font(sb(13.)).color(CAT_MOV));
            }
            for c in &e.conds {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(facil::op_icon(&c.op)).color(facil::cat_op(&c.op).color));
                    ui.label(RichText::new(catalog::sentence(c)).color(TEXT_SOFT));
                });
            }
            if e.conds.is_empty() && e.kind == EventKind::Standard {
                ui.label(RichText::new("Siempre").color(TEXT_MUTED));
            }
            for a in &e.acts {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(ic::ARROW_RIGHT).color(TEXT_FAINT));
                    ui.label(RichText::new(facil::op_icon(&a.op)).color(facil::cat_op(&a.op).color));
                    ui.label(RichText::new(catalog::sentence(a)).color(TEXT));
                });
            }
            if !e.sub.is_empty() {
                ui.horizontal_top(|ui| {
                    ui.add_space(10.);
                    ui.vertical(|ui| ver_eventos(ui, &e.sub, depth + 1));
                });
            }
        });
        ui.add_space(4.);
    }
}

fn hash<T: std::hash::Hash>(v: &T) -> u64 {
    use std::hash::Hasher;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

fn contar(evs: &[Event]) -> usize {
    evs.iter().map(|e| 1 + contar(&e.sub)).sum()
}

impl Vista {
    /// Olvida lo que se estaba haciendo en el lienzo (al deshacer, cambiar de proyecto...).
    pub fn reset(&mut self) {
        (self.drag, self.editar, self.paleta, self.menu) = (None, None, None, None);
        self.sel.clear();
        self.cache.key = 0;
    }
}

use super::eventos::Target;
use super::{seg, seg_btn, Nivel, UNDO_HINT};

impl super::App {
    fn clave(&self, t: Target) -> String {
        match t {
            Target::Scene(i) => format!("e{i}g{}", self.nv.grafo),
            Target::Func(i) => format!("f{i}"),
        }
    }

    fn hoja_de(&self, t: Target) -> &[Event] {
        match t {
            Target::Scene(i) => self.p.scenes.get(i).map_or(&[][..], |s| &s.events),
            Target::Func(i) => self.p.functions.get(i).map_or(&[][..], |f| &f.events),
        }
    }

    fn grafo_de(&self, t: Target) -> Option<&Graph> {
        match t {
            Target::Scene(i) => self.p.scenes.get(i)?.graphs.get(self.nv.grafo),
            Target::Func(i) => self.p.functions.get(i)?.graph.as_ref(),
        }
    }

    /// Pestaña Eventos (o Mis bloques): la hoja o los nodos, con el selector arriba.
    pub(super) fn eventos_o_nodos(&mut self, ui: &mut Ui, t: Target) {
        if let Target::Scene(i) = t {
            let n = self.p.scenes[i].graphs.len();
            self.nv.grafo = self.nv.grafo.min(n.saturating_sub(1));
        }
        let nodos = match t {
            Target::Scene(_) => self.nv.en_escena,
            Target::Func(_) => self.nv.en_func,
        };
        let mut set = None;
        ui.horizontal(|ui| {
            seg(ui, |ui| {
                if seg_btn(ui, !nodos, (RichText::new(ic::LIST), RichText::new("Hoja"))).on_hover_text("Eventos en filas: CUANDO / ENTONCES").clicked() {
                    set = Some(false);
                }
                if seg_btn(ui, nodos, (RichText::new(ic::GRAPH), RichText::new("Nodos"))).on_hover_text("Lógica con nodos y cables, como los Blueprints").clicked() {
                    set = Some(true);
                }
            });
            if nodos {
                self.barra_nodos(ui, t);
            } else {
                let tiene = !self.hoja_de(t).is_empty();
                let g = match t {
                    Target::Scene(i) => self.p.scenes[i].graphs.len(),
                    Target::Func(i) => self.p.functions[i].graph.is_some() as usize,
                };
                if tiene && ui.button(it(ic::EYE, "Ver la hoja como nodos")).on_hover_text("Muestra estos eventos como un grafo, sin cambiar nada").clicked() {
                    set = Some(true);
                    self.nv.ver_hoja = true;
                }
                if g > 0 {
                    ui.label(muted(format!("{} Esta hoja también tiene {g} grafo{} de nodos: se ejecuta{} después de estos eventos.", ic::INFO, if g == 1 { "" } else { "s" }, if g == 1 { "" } else { "n" })));
                }
            }
        });
        ui.add_space(6.);
        if let Some(v) = set {
            match t {
                Target::Scene(_) => self.nv.en_escena = v,
                Target::Func(_) => self.nv.en_func = v,
            }
            if !v {
                self.nv.ver_hoja = false;
            }
        }
        if nodos || set == Some(true) {
            self.nodos_ui(ui, t);
        } else {
            self.sheet(ui, t);
        }
    }

    fn barra_nodos(&mut self, ui: &mut Ui, t: Target) {
        let mut nuevo = false;
        let mut borrar_g = false;
        let mut conv = None;
        match t {
            Target::Scene(i) => {
                let names: Vec<String> = self.p.scenes[i].graphs.iter().map(|g| g.name.clone()).collect();
                if !names.is_empty() {
                    let cur = self.nv.grafo.min(names.len() - 1);
                    egui::ComboBox::from_id_salt("grafo_sel").selected_text(format!("{} {}", ic::GRAPH, names[cur])).width(150.).show_ui(ui, |ui| {
                        for (k, n) in names.iter().enumerate() {
                            if ui.selectable_label(k == cur, n).clicked() {
                                self.nv.grafo = k;
                                self.nv.ver_hoja = false;
                            }
                        }
                    });
                    let g = &mut self.p.scenes[i].graphs[cur];
                    ui.add(TextEdit::singleline(&mut g.name).desired_width(150.)).on_hover_text("Nombre del grafo");
                    borrar_g = ib(ui, ic::TRASH, "Borrar este grafo").clicked();
                }
                nuevo = ui.button(it(ic::PLUS, "Nuevo grafo")).clicked();
            }
            Target::Func(i) => {
                if self.p.functions[i].graph.is_none() {
                    nuevo = ui.button(it(ic::PLUS, "Nuevo grafo")).clicked();
                } else {
                    borrar_g = ib(ui, ic::TRASH, "Borrar el grafo de este bloque").clicked();
                }
            }
        }
        if !self.hoja_de(t).is_empty() {
            let b = Button::new((RichText::new(ic::ARROWS_SPLIT).color(ACCENT), RichText::new("Convertir la hoja"), RichText::new(ic::CARET_DOWN).color(TEXT_MUTED)));
            let (r, _) = egui::containers::menu::MenuButton::from_button(b).ui(ui, |ui| {
                menu_spacing(ui);
                if ui.button(it(ic::ARROWS_SPLIT, "Pasar la hoja a nodos (la hoja queda vacía)")).clicked() {
                    conv = Some(true);
                }
                if ui.button(it(ic::COPY, "Copiar la hoja como nodos (la hoja queda igual)")).on_hover_text("Ojo: la hoja y el grafo se ejecutan los dos").clicked() {
                    conv = Some(false);
                }
            });
            r.on_hover_text("Hacer un grafo con los eventos de la hoja");
            if seg_btn(ui, self.nv.ver_hoja, (RichText::new(ic::EYE), RichText::new("Ver la hoja"))).on_hover_text("La hoja vista como nodos, solo para mirar").clicked() {
                self.nv.ver_hoja = !self.nv.ver_hoja;
            }
        }
        if seg_btn(ui, self.nv.ver_eventos, (RichText::new(ic::LIST_BULLETS), RichText::new("Eventos generados"))).on_hover_text("Lo que el grafo le dice al juego, como hoja de eventos").clicked() {
            self.nv.ver_eventos = !self.nv.ver_eventos;
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ib(ui, ic::FIT, "Encuadrar (F)").clicked() {
                self.nv.encuadrar = true;
            }
            let dos = self.nv.sel.len() >= 2;
            let sel = self.nv.sel.clone();
            if ib_on(ui, dos, ic::ALIGN_TOP, "Alinear arriba (elige 2 o más nodos)").clicked() {
                if let Some(g) = self.grafo_mut_de(t) {
                    alinear(g, &sel, true);
                }
            }
            if ib_on(ui, dos, ic::ALIGN_LEFT, "Alinear a la izquierda (elige 2 o más nodos)").clicked() {
                if let Some(g) = self.grafo_mut_de(t) {
                    alinear(g, &sel, false);
                }
            }
        });
        if nuevo {
            self.snap_hist();
            match t {
                Target::Scene(i) => {
                    let gs = &mut self.p.scenes[i].graphs;
                    let name = (1..).map(|k| format!("Grafo {k}")).find(|n| !gs.iter().any(|g| &g.name == n)).unwrap();
                    gs.push(Graph { name, ..Default::default() });
                    self.nv.grafo = gs.len() - 1;
                }
                Target::Func(i) => self.p.functions[i].graph = Some(Graph { name: self.p.functions[i].name.clone(), ..Default::default() }),
            }
            self.nv.ver_hoja = false;
            self.nv.reset();
        }
        if borrar_g {
            self.snap_hist();
            let name = match t {
                Target::Scene(i) if self.nv.grafo < self.p.scenes[i].graphs.len() => Some(self.p.scenes[i].graphs.remove(self.nv.grafo).name),
                Target::Func(i) => self.p.functions[i].graph.take().map(|g| g.name),
                _ => None,
            };
            self.nv.grafo = self.nv.grafo.saturating_sub(1);
            self.nv.reset();
            if let Some(n) = name {
                self.say(Nivel::Info, format!("Grafo «{n}» borrado · {UNDO_HINT}"));
            }
        }
        if let Some(mover) = conv {
            self.convertir(t, mover);
        }
    }

    fn grafo_mut_de(&mut self, t: Target) -> Option<&mut Graph> {
        match t {
            Target::Scene(i) => self.p.scenes.get_mut(i)?.graphs.get_mut(self.nv.grafo),
            Target::Func(i) => self.p.functions.get_mut(i)?.graph.as_mut(),
        }
    }

    /// Hace un grafo con la hoja; `mover` deja la hoja vacía (si no, se ejecutarían las dos).
    pub(super) fn convertir(&mut self, t: Target, mover: bool) {
        self.snap_hist();
        let evs = self.hoja_de(t).to_vec();
        let name = match t {
            Target::Scene(i) => {
                let gs = &self.p.scenes[i].graphs;
                (1..).map(|k| if k == 1 { "Hoja convertida".to_string() } else { format!("Hoja convertida {k}") }).find(|n| !gs.iter().any(|g| &g.name == n)).unwrap()
            }
            Target::Func(i) => self.p.functions[i].name.clone(),
        };
        let g = ng::from_events(&name, &evs);
        let n = g.nodes.len();
        match t {
            Target::Scene(i) => {
                self.p.scenes[i].graphs.push(g);
                self.nv.grafo = self.p.scenes[i].graphs.len() - 1;
                if mover {
                    self.p.scenes[i].events.clear();
                }
            }
            Target::Func(i) => {
                let f = &mut self.p.functions[i];
                match f.graph.as_mut() {
                    Some(old) => {
                        let at = bounds(old, None).map(|b| Pos2::new(b.left(), b.bottom() + 80.));
                        pegar(old, &g, at);
                    }
                    None => f.graph = Some(g),
                }
                if mover {
                    f.events.clear();
                }
            }
        }
        match t {
            Target::Scene(_) => self.nv.en_escena = true,
            Target::Func(_) => self.nv.en_func = true,
        }
        self.nv.ver_hoja = false;
        self.nv.reset();
        self.nv.encuadrar = true;
        let extra = if mover { "; la hoja quedó vacía" } else { "; la hoja sigue igual (se ejecutan las dos)" };
        self.say(Nivel::Ok, format!("Hoja convertida en un grafo de {n} nodos{extra} · {UNDO_HINT}"));
    }

    fn nodos_ui(&mut self, ui: &mut Ui, t: Target) {
        let ctx = ui.ctx().clone();
        let l = self.kid_lists(&ctx);
        let key = self.clave(t);
        // Errores y eventos compilados: solo se recalculan cuando cambia el proyecto.
        let h = hash(&(&self.now, &key, self.facil)) | 1;
        if self.nv.cache.key != h {
            let base = self.base_dir();
            let scene = match t {
                Target::Scene(i) => self.p.scenes.get(i),
                Target::Func(_) => None,
            };
            self.nv.cache = match self.grafo_de(t) {
                Some(g) => {
                    let probs = nicostation::validate::graph_problems(&self.p, scene, g, Some(&base))
                        .into_iter()
                        .map(|(id, m)| (id, if self.facil { facil::amable(&m) } else { m }))
                        .collect();
                    Cache { key: h, probs, eventos: ng::compile_lossy(g).0 }
                }
                None => Cache { key: h, ..Default::default() },
            };
        }
        let probs = self.nv.cache.probs.clone();
        let n_ev = contar(&self.nv.cache.eventos);
        if self.nv.ver_eventos && !self.nv.ver_hoja {
            let evs = self.nv.cache.eventos.clone();
            let panel = Frame::new().fill(PANEL).inner_margin(12).stroke((1., BORDER));
            egui::Panel::right("nodos_eventos").default_size(380.).size_range(260.0..=640.).frame(panel).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(ic::LIST_BULLETS).color(ACCENT));
                    ui.label(RichText::new("Eventos generados").font(sb(15.)));
                    ui.label(muted(format!("{n_ev}")));
                });
                ui.label(muted("Así ve el juego tu grafo. Para cambiarlo, cambia los nodos."));
                ui.add_space(6.);
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    if evs.is_empty() {
                        ui.label(muted("Todavía no hay eventos: conecta un nodo «Cuando» con alguna acción."));
                    }
                    ver_eventos(ui, &evs, 0);
                });
            });
        }
        let facil_ = self.facil;
        let salida = if self.nv.ver_hoja {
            let evs = self.hoja_de(t).to_vec();
            let hk = hash(&serde_json::to_string(&evs).unwrap_or_default());
            if self.nv.prev.as_ref().is_none_or(|p| p.0 != hk) {
                self.nv.prev = Some((hk, ng::from_events("Hoja", &evs)));
            }
            let mut g = self.nv.prev.take().map(|p| p.1).unwrap_or_default();
            let k = format!("{key}-hoja");
            let cx = Cx { l: &l, facil: facil_, probs: &[], n_eventos: contar(&evs), key: &k, solo_ver: true };
            let s = lienzo(ui, &mut g, &mut self.nv, &cx);
            self.nv.prev = Some((hk, g));
            Some(s)
        } else {
            let (p, nv) = (&mut self.p, &mut self.nv);
            let g = match t {
                Target::Scene(i) => p.scenes.get_mut(i).and_then(|s| s.graphs.get_mut(nv.grafo)),
                Target::Func(i) => p.functions.get_mut(i).and_then(|f| f.graph.as_mut()),
            };
            match g {
                Some(g) => {
                    let cx = Cx { l: &l, facil: facil_, probs: &probs, n_eventos: n_ev, key: &key, solo_ver: false };
                    Some(lienzo(ui, g, nv, &cx))
                }
                None => {
                    self.sin_grafo(ui, t);
                    None
                }
            }
        };
        if let Some(s) = salida {
            if let Some(m) = s.msg {
                self.say(Nivel::Info, m);
            }
            if let Some((n, g)) = s.new_var {
                self.crear_cajita(&n, g);
            }
        }
    }

    fn sin_grafo(&mut self, ui: &mut Ui, t: Target) {
        let tiene = !self.hoja_de(t).is_empty();
        let (mut nuevo, mut conv) = (false, false);
        ui.add_space(40.);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(ic::GRAPH).size(36.).color(TEXT_FAINT));
            ui.label(RichText::new("Todavía no hay nodos aquí").font(sb(18.)));
            ui.label(muted("Un grafo es otra forma de hacer la lógica del juego: nodos unidos con flechas, como en Unreal."));
            ui.add_space(10.);
            ui.horizontal(|ui| {
                ui.add_space((ui.available_width() - if tiene { 430. } else { 170. }).max(0.) / 2.);
                ui.spacing_mut().button_padding = Vec2::new(14., 8.);
                nuevo = primary(ui, (on_accent(ic::PLUS).size(16.), on_accent("Nuevo grafo").font(sb(15.)))).clicked();
                if tiene {
                    conv = ui.button((RichText::new(ic::ARROWS_SPLIT).size(16.).color(ACCENT), RichText::new("Convertir la hoja en nodos").font(sb(15.)))).on_hover_text("Los eventos pasan al grafo y la hoja queda vacía").clicked();
                }
            });
        });
        if nuevo {
            self.snap_hist();
            match t {
                Target::Scene(i) => {
                    self.p.scenes[i].graphs.push(Graph { name: "Grafo 1".into(), ..Default::default() });
                    self.nv.grafo = self.p.scenes[i].graphs.len() - 1;
                }
                Target::Func(i) => self.p.functions[i].graph = Some(Graph { name: self.p.functions[i].name.clone(), ..Default::default() }),
            }
            self.nv.reset();
        }
        if conv {
            self.convertir(t, true);
        }
    }

    /// `--abrir nodos…`: vistas de prueba de los nodos.
    pub(super) fn abrir_nodos(&mut self, que: &str) {
        let t = Target::Scene(self.sc);
        self.nv.en_escena = true;
        match que {
            "nodos-hoja" => self.nv.ver_hoja = true,
            "nodos" => {}
            _ => self.convertir(t, true),
        }
        if que == "nodos-eventos" || que == "nodos-datos" {
            self.nv.ver_eventos = true;
        }
        if que == "nodos-datos" {
            // Arriba de todo, un camino con nodos de datos: mover al héroe con 80 * dt y mostrar «Puntos: » + puntos.
            let obj = self.p.objects_of(&self.p.scenes[self.sc]).map(|o| o.name.clone()).next().unwrap_or_default();
            let txt = self.p.objects_of(&self.p.scenes[self.sc]).find(|o| matches!(o.kind, Kind::Text { .. })).map_or(obj.clone(), |o| o.name.clone());
            if let Some(g) = self.grafo_mut_de(t) {
                let top = bounds(g, None).map_or(0., |b| b.top()) - 360.;
                let id = next_id(g);
                let mk = |k: u32, kind: NodeKind, x: f32, y: f32, args: &[&str]| Node { id: id + k, kind, x, y: top + y, args: args.iter().map(|a| a.to_string()).collect(), not: false, sub: vec![], off: false };
                g.nodes.extend([
                    mk(0, NodeKind::Event { op: "key_down".into() }, 0., 0., &["Right"]),
                    mk(1, NodeKind::Act { op: "add".into() }, 310., 0., &[&obj, "x", "0"]),
                    mk(2, NodeKind::Act { op: "set_text".into() }, 620., 0., &[&txt, "\"\""]),
                    mk(3, NodeKind::Data { f: "mul".into() }, 0., 130., &["80", "1"]),
                    mk(4, NodeKind::Data { f: "dt".into() }, -290., 190., &[]),
                    mk(5, NodeKind::Data { f: "join".into() }, 310., 150., &["\"Puntos: \"", "0"]),
                    mk(6, NodeKind::Data { f: "var".into() }, 0., 250., &["puntos"]),
                ]);
                g.flow.extend([Flow { from: id, out: 0, to: id + 1 }, Flow { from: id + 1, out: 0, to: id + 2 }]);
                g.wires.extend([Wire { from: id + 3, to: id + 1, arg: 2 }, Wire { from: id + 4, to: id + 3, arg: 1 }, Wire { from: id + 5, to: id + 2, arg: 1 }, Wire { from: id + 6, to: id + 5, arg: 1 }]);
                self.nv.sel = (id..id + 7).collect();
            }
        }
        if que == "nodos-paleta" {
            let at = self.grafo_de(t).and_then(|g| bounds(g, None)).map_or(Pos2::ZERO, |b| Pos2::new(b.left() + 300., b.top() + 160.));
            self.nv.paleta = Some(Paleta { screen: Pos2::new(560., 170.), at, modo: PalModo::Nuevo(None), search: String::new(), tab: 2, mas: false, focus: true });
        }
        if que == "nodos-error" {
            // Rompe dos nodos a propósito: un objeto sin elegir y una cuenta mal escrita.
            if let Some(g) = self.grafo_mut_de(t) {
                let mut rotos = BTreeSet::new();
                if let Some(n) = g.nodes.iter_mut().find(|n| matches!(&n.kind, NodeKind::Act { op } if op == "flip_x")) {
                    n.args[0].clear();
                    rotos.insert(n.id);
                }
                if let Some(n) = g.nodes.iter_mut().filter(|n| matches!(&n.kind, NodeKind::Act { op } if op == "flip_x")).nth(1) {
                    n.args[1] = "1 +".into();
                    rotos.insert(n.id);
                }
                self.nv.sel = rotos;
            }
        }
    }
}
