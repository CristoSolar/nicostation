//! Hoja de eventos (dos columnas, como GDevelop) y ventana de edición de instrucciones.

use super::iconos as ic;
use super::tema::*;
use super::{combo, rel};
use eframe::egui::{self, text::LayoutJob, Align, Button, CornerRadius, FontId, Key, Layout, Margin, Rect, RichText, Sense, Shape, TextEdit, TextFormat, Ui, Vec2};
use nicostation::catalog::{self, Meta, CATALOG, P};
use nicostation::model::*;
use std::collections::HashMap;
use std::path::PathBuf;

/// Lista de eventos que se edita: la de una escena o la de una función.
#[derive(Clone, Copy, PartialEq)]
pub enum Target {
    Scene(usize),
    Func(usize),
}

/// Ubicación de una instrucción: ruta del evento, columna y ruta dentro de `sub`.
/// Con `new`, `ip` apunta a la lista donde se agrega.
pub struct Open {
    pub ev: Vec<usize>,
    pub cond: bool,
    pub ip: Vec<usize>,
    pub new: bool,
}

pub struct InstrEdit {
    pub at: Open,
    pub target: Target,
    pub ins: Instr,
    pub search: String,
    /// Enfocar el buscador en el primer cuadro.
    pub focus: bool,
}

/// Nombres disponibles para los combos de parámetros.
pub struct Lists {
    pub objs: Vec<String>,
    pub layers: Vec<String>,
    pub scenes: Vec<String>,
    pub funcs: Vec<(String, bool, Vec<String>)>,
    pub anims: HashMap<String, Vec<String>>,
    pub base: PathBuf,
    // ---- extras del modo fácil ----
    /// Nombres de grupos (también están en `objs`).
    pub groups: Vec<String>,
    /// Miniatura de cada objeto.
    pub thumbs: HashMap<String, Thumb>,
    /// Primera imagen de cada animación: (objeto, animación).
    pub anim_tex: HashMap<(String, String), egui::TextureHandle>,
    /// Cajitas de la escena y del juego.
    pub vars: Vec<String>,
    pub globals: Vec<String>,
    /// Variables propias de cada objeto.
    pub obj_vars: HashMap<String, Vec<String>>,
    /// Imágenes y sonidos dentro de la carpeta del proyecto.
    pub files: Vec<String>,
    /// Ingredientes del bloque que se está editando (cajitas locales).
    pub locals: Vec<String>,
}

/// Cómo dibujar un objeto en miniatura.
#[derive(Clone)]
pub enum Thumb {
    Tex(egui::TextureHandle),
    Color([u8; 3]),
    Text([u8; 3]),
    /// Sprite sin imagen.
    Empty,
}

/// Lo que la hoja pide al editor: abrir una instrucción o mostrar un mensaje.
#[derive(Default)]
pub struct Salida {
    pub open: Option<Open>,
    pub msg: Option<String>,
    /// Cajita a crear si no existe: (nombre, del juego).
    pub new_var: Option<(String, bool)>,
    /// Abrir el recetario.
    pub recetas: bool,
}

#[derive(PartialEq)]
pub enum Res {
    Ok,
    Cancel,
    Del,
}

enum A {
    Below,
    Dup,
    Del,
    Up,
    Down,
}

const KINDS: [(EventKind, &str); 7] = [
    (EventKind::Standard, "Estándar"),
    (EventKind::Else, "Si no"),
    (EventKind::Repeat, "Repetir"),
    (EventKind::ForEach, "Para cada objeto"),
    (EventKind::While, "Mientras"),
    (EventKind::Group, "Grupo"),
    (EventKind::Comment, "Comentario"),
];

pub const GUTTER: f32 = 26.;
const UNDO_HINT: &str = "Ctrl+Z para deshacer";

/// Encabezado de columnas de la hoja.
pub fn cabecera(ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.add_space(GUTTER + ui.spacing().item_spacing.x);
        ui.spacing_mut().item_spacing.x = 0.;
        ui.columns(2, |c| {
            for (i, t) in ["Condiciones", "Acciones"].into_iter().enumerate() {
                c[i].horizontal(|ui| {
                    ui.add_space(10.);
                    ui.label(RichText::new(t).font(sb(12.)).color(TEXT_MUTED));
                });
            }
        });
    });
}

pub fn sheet(ui: &mut Ui, evs: &mut Vec<Event>, path: &[usize], objs: &[String], out: &mut Salida) {
    let mut act = None;
    for (i, e) in evs.iter_mut().enumerate() {
        let p: Vec<usize> = path.iter().copied().chain([i]).collect();
        ui.push_id(i, |ui| {
            if let Some(a) = event_ui(ui, e, &p, objs, out) {
                act = Some((i, a));
            }
        });
    }
    match act {
        Some((i, A::Below)) => evs.insert(i + 1, Event::default()),
        Some((i, A::Dup)) => {
            let e = evs[i].clone();
            evs.insert(i + 1, e);
        }
        Some((i, A::Del)) => {
            let e = evs.remove(i);
            let what = match e.kind {
                EventKind::Comment => "el comentario",
                EventKind::Group => "el grupo",
                _ => "el evento",
            };
            out.msg = Some(format!("Eliminado {what} · {UNDO_HINT}"));
        }
        Some((i, A::Up)) if i > 0 => evs.swap(i, i - 1),
        Some((i, A::Down)) if i + 1 < evs.len() => evs.swap(i, i + 1),
        _ => {}
    }
}

fn event_ui(ui: &mut Ui, e: &mut Event, p: &[usize], objs: &[String], out: &mut Salida) -> Option<A> {
    let mut act = None;
    ui.scope(|ui| {
        if e.disabled {
            ui.multiply_opacity(0.45);
        }
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(Vec2::new(GUTTER, 26.), Layout::top_down(Align::Min), |ui| {
                ui.set_width(GUTTER);
                ui.spacing_mut().button_padding = Vec2::new(5., 5.);
                let b = Button::new(RichText::new(ic::DOTS).size(16.).color(TEXT_MUTED)).frame_when_inactive(false).min_size(Vec2::splat(GUTTER));
                let (r, _) = egui::containers::menu::MenuButton::from_button(b).ui(ui, |ui| menu(ui, e, &mut act));
                r.on_hover_text("Opciones del evento");
            });
            ui.vertical(|ui| match e.kind {
                EventKind::Comment => comment_ui(ui, e),
                EventKind::Group => group_ui(ui, e, p, objs, out),
                _ => standard_ui(ui, e, p, objs, out),
            });
        });
    });
    act
}

fn disabled_tag(ui: &mut Ui, e: &Event) {
    if e.disabled {
        tag(ui, "Deshabilitado", TEXT_MUTED);
    }
}

fn comment_ui(ui: &mut Ui, e: &mut Event) {
    let r = egui::Frame::new().fill(SURFACE).corner_radius(R).inner_margin(Margin { left: 12, right: 8, top: 6, bottom: 6 }).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(RichText::new(ic::NOTE).color(WARN));
            disabled_tag(ui, e);
            ui.add(TextEdit::multiline(&mut e.text).text_color(TEXT).frame(egui::Frame::NONE)
                .desired_width(f32::INFINITY).desired_rows(1).hint_text("Comentario…"));
        });
    });
    let rr = r.response.rect;
    ui.painter().rect_filled(Rect::from_min_size(rr.min, Vec2::new(2., rr.height())), CornerRadius { nw: 2, sw: 2, ne: 0, se: 0 }, WARN);
}

fn group_ui(ui: &mut Ui, e: &mut Event, p: &[usize], objs: &[String], out: &mut Salida) {
    let fold_id = ui.make_persistent_id("abierto");
    let mut unfolded = ui.ctx().data_mut(|d| *d.get_temp_mut_or(fold_id, true));
    egui::Frame::new().stroke((1., BORDER)).corner_radius(R).show(ui, |ui| {
        ui.set_width(ui.available_width());
        egui::Frame::new().fill(SURFACE).corner_radius(CornerRadius { nw: R, ne: R, sw: if unfolded { 0 } else { R }, se: if unfolded { 0 } else { R } })
            .inner_margin(Margin::symmetric(6, 4)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let caret = if unfolded { ic::CARET_DOWN } else { ic::CARET_RIGHT };
                    if ib(ui, caret, if unfolded { "Plegar grupo" } else { "Desplegar grupo" }).clicked() {
                        unfolded = !unfolded;
                        ui.ctx().data_mut(|d| d.insert_temp(fold_id, unfolded));
                    }
                    ui.label(RichText::new(ic::FOLDERS).color(TEXT_MUTED));
                    disabled_tag(ui, e);
                    ui.add(TextEdit::singleline(&mut e.text).hint_text("Título del grupo").font(sb(14.)).frame(egui::Frame::NONE)
                        .desired_width(ui.available_width() - 80.));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(format!("{}", e.sub.len())).color(TEXT_MUTED)).on_hover_text("Eventos en el grupo");
                    });
                });
            });
        if unfolded {
            egui::Frame::new().inner_margin(Margin { left: 8, right: 6, top: 6, bottom: 6 }).show(ui, |ui| {
                ui.set_width(ui.available_width());
                sheet(ui, &mut e.sub, p, objs, out);
                if add_btn(ui, "Evento en el grupo").clicked() {
                    e.sub.push(Event::default());
                }
            });
        }
    });
}

fn standard_ui(ui: &mut Ui, e: &mut Event, p: &[usize], objs: &[String], out: &mut Salida) {
    let bg = [ui.painter().add(Shape::Noop), ui.painter().add(Shape::Noop), ui.painter().add(Shape::Noop)];
    let rects = ui.scope(|ui| {
        ui.spacing_mut().item_spacing.x = 0.;
        ui.columns(2, |c| {
            let m = Margin { left: 10, right: 8, top: 6, bottom: 6 };
            let r0 = egui::Frame::new().inner_margin(m).show(&mut c[0], |ui| {
                ui.spacing_mut().item_spacing.x = 8.;
                ui.set_width(ui.available_width());
                kind_header(ui, e, objs);
                instrs(ui, &mut e.conds, p, true, &[], out);
                if add_btn(ui, "Condición").clicked() {
                    out.open = Some(Open { ev: p.to_vec(), cond: true, ip: vec![], new: true });
                }
            });
            let r1 = egui::Frame::new().inner_margin(m).show(&mut c[1], |ui| {
                ui.spacing_mut().item_spacing.x = 8.;
                ui.set_width(ui.available_width());
                instrs(ui, &mut e.acts, p, false, &[], out);
                if add_btn(ui, "Acción").clicked() {
                    out.open = Some(Open { ev: p.to_vec(), cond: false, ip: vec![], new: true });
                }
            });
            [r0.response.rect, r1.response.rect]
        })
    }).inner;
    let bottom = rects[0].bottom().max(rects[1].bottom());
    let (a, b) = (rects[0].with_max_y(bottom), rects[1].with_max_y(bottom));
    let pa = ui.painter();
    pa.set(bg[0], egui::epaint::RectShape::filled(a, CornerRadius { nw: R, sw: R, ne: 0, se: 0 }, COND_BG));
    pa.set(bg[1], egui::epaint::RectShape::filled(b, CornerRadius { nw: 0, sw: 0, ne: R, se: R }, ACT_BG));
    pa.set(bg[2], egui::epaint::RectShape::filled(Rect::from_min_size(a.min, Vec2::new(2., a.height())), CornerRadius { nw: 2, sw: 2, ne: 0, se: 0 }, TEXT_FAINT));
    if !e.sub.is_empty() {
        ui.add_space(2.);
        ui.indent("sub", |ui| sheet(ui, &mut e.sub, p, objs, out));
    }
}

/// Línea superior de eventos especiales (repetir, para cada, mientras) y etiqueta de deshabilitado.
fn kind_header(ui: &mut Ui, e: &mut Event, objs: &[String]) {
    if e.kind == EventKind::Standard && !e.disabled {
        return;
    }
    let lbl = |ui: &mut Ui, t: &str| drop(ui.label(RichText::new(t).font(sb(13.)).color(TEXT_MUTED)));
    ui.horizontal_wrapped(|ui| {
        match e.kind {
            EventKind::Repeat => {
                lbl(ui, "Repetir");
                ui.add(TextEdit::singleline(&mut e.text).desired_width(70.).hint_text("3"));
                lbl(ui, "veces");
            }
            EventKind::ForEach => {
                lbl(ui, "Para cada");
                combo(ui, "foreach", &mut e.text, objs, "(elegir)");
            }
            EventKind::While => lbl(ui, "Mientras se cumpla"),
            _ => {}
        }
        disabled_tag(ui, e);
    });
    if e.kind == EventKind::Repeat {
        expr_error(ui, &e.text);
    }
}

fn add_btn(ui: &mut Ui, t: &str) -> egui::Response {
    ui.add(Button::new((RichText::new(ic::PLUS).color(TEXT_MUTED), RichText::new(t).color(TEXT_MUTED))).frame_when_inactive(false).small())
}

fn menu(ui: &mut Ui, e: &mut Event, act: &mut Option<A>) {
    menu_spacing(ui);
    if ui.button(it(ic::PLUS, "Añadir evento debajo")).clicked() {
        *act = Some(A::Below);
    }
    if e.kind != EventKind::Comment && ui.button(it(ic::TREE, "Añadir sub-evento")).clicked() {
        e.sub.push(Event::default());
    }
    if ui.button(it(ic::COPY, "Duplicar")).clicked() {
        *act = Some(A::Dup);
    }
    ui.separator();
    if ui.button(it(ic::CARET_UP, "Mover arriba")).clicked() {
        *act = Some(A::Up);
    }
    if ui.button(it(ic::CARET_DOWN, "Mover abajo")).clicked() {
        *act = Some(A::Down);
    }
    let (i, t) = if e.disabled { (ic::EYE, "Habilitar") } else { (ic::EYE_SLASH, "Deshabilitar") };
    if ui.button(it(i, t)).clicked() {
        e.disabled = !e.disabled;
    }
    ui.menu_button(it(ic::LIST, "Cambiar tipo"), |ui| {
        menu_spacing(ui);
        for (k, n) in KINDS.iter() {
            if ui.radio(e.kind == *k, *n).clicked() {
                e.kind = k.clone();
            }
        }
    });
    ui.separator();
    if ui.button(danger_it("Borrar evento")).clicked() {
        *act = Some(A::Del);
    }
}

/// Frase con los argumentos resaltados (mismo texto que `catalog::sentence`).
fn frase(i: &Instr) -> LayoutJob {
    let mut j = LayoutJob::default();
    let f = FontId::proportional(14.);
    let mut push = |t: &str, c, font: FontId| j.append(t, 0., TextFormat::simple(font, c));
    if i.not {
        push("NO ", WARN, sb(14.));
    }
    let Some(m) = catalog::meta(&i.op) else {
        push(&format!("¿{}? {:?}", i.op, i.args), DANGER, f);
        return j;
    };
    let arg = |n: usize| i.args.get(n).cloned().unwrap_or_default();
    let s = m.sentence;
    let (mut k, mut lit) = (0, String::new());
    while k < s.len() {
        let rest = &s[k..];
        let tok = if rest.starts_with("_args_") {
            Some((i.args.iter().skip(1).cloned().collect::<Vec<_>>().join(", "), 6))
        } else if let Some(d) = rest.strip_prefix('_').map(|r| r.bytes().take_while(u8::is_ascii_digit).count()).filter(|&d| d > 0 && rest[1 + d..].starts_with('_')) {
            Some((arg(rest[1..1 + d].parse().unwrap_or(0)), d + 2))
        } else {
            None
        };
        match tok {
            Some((a, len)) => {
                push(&std::mem::take(&mut lit), TEXT_MUTED, f.clone());
                if a.is_empty() {
                    push("(vacío)", TEXT_FAINT, f.clone());
                } else {
                    push(&a, TEXT, f.clone());
                }
                k += len;
            }
            None => {
                let c = rest.chars().next().unwrap();
                lit.push(c);
                k += c.len_utf8();
            }
        }
    }
    push(&lit, TEXT_MUTED, f);
    j
}

fn instrs(ui: &mut Ui, l: &mut Vec<Instr>, ev: &[usize], cond: bool, ip: &[usize], out: &mut Salida) {
    // (índice, -1 subir / 1 bajar / 0 borrar / 2 duplicar)
    let mut act: Option<(usize, i8)> = None;
    for (k, ins) in l.iter_mut().enumerate() {
        let ipk: Vec<usize> = ip.iter().copied().chain([k]).collect();
        ui.push_id(k, |ui| {
            let idx = ui.painter().add(Shape::Noop);
            let r = ui.add(egui::Label::new(frase(ins)).wrap().sense(Sense::click()))
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text("Clic: editar. Clic derecho: más opciones");
            let full = Rect::from_x_y_ranges(ui.max_rect().x_range(), r.rect.y_range()).expand2(Vec2::new(4., 2.));
            if r.hovered() || r.context_menu_opened() {
                ui.painter().set(idx, egui::epaint::RectShape::filled(full, 4, SURFACE_HOVER));
            }
            if r.clicked() {
                out.open = Some(Open { ev: ev.to_vec(), cond, ip: ipk.clone(), new: false });
            }
            r.context_menu(|ui| {
                menu_spacing(ui);
                for (t, i, a) in [("Duplicar", ic::COPY, 2), ("Subir", ic::CARET_UP, -1), ("Bajar", ic::CARET_DOWN, 1)] {
                    if ui.button(it(i, t)).clicked() {
                        act = Some((k, a));
                    }
                }
                ui.separator();
                if ui.button(danger_it("Borrar")).clicked() {
                    act = Some((k, 0));
                }
            });
            if cond && matches!(ins.op.as_str(), "or" | "and") {
                ui.indent("sub", |ui| {
                    instrs(ui, &mut ins.sub, ev, true, &ipk, out);
                    if add_btn(ui, "Sub-condición").clicked() {
                        out.open = Some(Open { ev: ev.to_vec(), cond: true, ip: ipk.clone(), new: true });
                    }
                });
            }
        });
    }
    match act {
        Some((k, 0)) => {
            l.remove(k);
            out.msg = Some(format!("Eliminada la {} · {UNDO_HINT}", if cond { "condición" } else { "acción" }));
        }
        Some((k, 2)) => {
            let i = l[k].clone();
            l.insert(k + 1, i);
        }
        Some((k, -1)) if k > 0 => l.swap(k, k - 1),
        Some((k, 1)) if k + 1 < l.len() => l.swap(k, k + 1),
        _ => {}
    }
}

pub fn ev_mut<'a>(evs: &'a mut [Event], p: &[usize]) -> Option<&'a mut Event> {
    let (f, rest) = p.split_first()?;
    let e = evs.get_mut(*f)?;
    if rest.is_empty() { Some(e) } else { ev_mut(&mut e.sub, rest) }
}

fn ilist<'a>(l: &'a mut Vec<Instr>, ip: &[usize]) -> Option<&'a mut Vec<Instr>> {
    match ip.split_first() {
        None => Some(l),
        Some((f, r)) => ilist(&mut l.get_mut(*f)?.sub, r),
    }
}

/// Lista que contiene la instrucción y su índice (None = agregar al final).
pub fn locate<'a>(evs: &'a mut [Event], o: &Open) -> Option<(&'a mut Vec<Instr>, Option<usize>)> {
    let e = ev_mut(evs, &o.ev)?;
    let l = if o.cond { &mut e.conds } else { &mut e.acts };
    if o.new {
        return Some((ilist(l, &o.ip)?, None));
    }
    let (k, par) = o.ip.split_last()?;
    let l = ilist(l, par)?;
    (*k < l.len()).then_some((l, Some(*k)))
}

pub fn get(evs: &mut [Event], o: &Open) -> Option<Instr> {
    locate(evs, o).and_then(|(l, k)| k.map(|k| l[k].clone()))
}

pub fn apply(evs: &mut [Event], ed: &InstrEdit, del: bool) {
    match locate(evs, &ed.at) {
        Some((l, None)) if !del => l.push(ed.ins.clone()),
        Some((l, Some(k))) if del => drop(l.remove(k)),
        Some((l, Some(k))) => l[k] = ed.ins.clone(),
        _ => {}
    }
}

pub fn default_arg(t: P, l: &Lists) -> String {
    match t {
        P::Object => l.objs.first().cloned().unwrap_or_default(),
        P::Scene => l.scenes.first().cloned().unwrap_or_default(),
        P::Key => "Space".into(),
        P::Mouse => "Left".into(),
        P::Op => "=".into(),
        P::Expr => "0".into(),
        P::Prop => "x".into(),
        P::Ease => "ease_out".into(),
        P::Code => "// tu código aquí\n".into(),
        _ => String::new(),
    }
}

fn set_op(ins: &mut Instr, m: &Meta, l: &Lists) {
    if ins.op == m.op {
        return;
    }
    ins.op = m.op.into();
    ins.args = m.params.iter().map(|p| default_arg(p.1, l)).collect();
    ins.not &= m.cond;
    if !matches!(m.op, "or" | "and") {
        ins.sub.clear();
    }
}

/// Error de sintaxis de una expresión, en rojo debajo del campo.
fn expr_error(ui: &mut Ui, s: &str) {
    if let Err(e) = nicostation::expr::parse(s) {
        ui.label(RichText::new(format!("{} {e}", ic::WARNING)).small().color(DANGER));
    }
}

pub fn window(ctx: &egui::Context, ed: &mut InstrEdit, l: &Lists) -> Option<Res> {
    let mut res = None;
    let cond = ed.at.cond;
    let q = ed.search.to_lowercase();
    let visible: Vec<&Meta> = CATALOG.iter()
        .filter(|m| m.cond == cond && (q.is_empty() || m.label.to_lowercase().contains(&q) || m.op.contains(&q) || m.group.to_lowercase().contains(&q)))
        .collect();
    let mut groups: Vec<&str> = vec![];
    visible.iter().for_each(|m| if !groups.contains(&m.group) { groups.push(m.group) });
    // Lista en el orden en que se muestra (por grupo), para moverse con las flechas.
    let order: Vec<&Meta> = groups.iter().flat_map(|g| visible.iter().copied().filter(move |m| m.group == *g)).collect();
    let frame = egui::Frame::window(&ctx.global_style()).inner_margin(16);
    let r = egui::Modal::new(egui::Id::unique("editor_instr")).frame(frame).show(ctx, |ui| {
        ui.set_width(760.);
        let kind = if cond { "condición" } else { "acción" };
        ui.horizontal(|ui| {
            ui.label(RichText::new(ic::FUNCTION).size(18.).color(ACCENT));
            ui.heading(if ed.at.new { format!("Nueva {kind}") } else { format!("Editar {kind}") });
        });
        ui.add_space(4.);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(250.);
                let s = ui.add(TextEdit::singleline(&mut ed.search).hint_text(format!("{}  Buscar", ic::SEARCH)).desired_width(f32::INFINITY));
                if ed.focus {
                    s.request_focus();
                    ed.focus = false;
                }
                // Flechas: recorrer las instrucciones visibles.
                let (dn, up) = ctx.input_mut(|i| (i.consume_key(egui::Modifiers::NONE, Key::ArrowDown), i.consume_key(egui::Modifiers::NONE, Key::ArrowUp)));
                let mut scroll_to = false;
                if (dn || up) && !order.is_empty() {
                    let cur = order.iter().position(|m| m.op == ed.ins.op);
                    let n = match (cur, dn) {
                        (None, _) => 0,
                        (Some(c), true) => (c + 1).min(order.len() - 1),
                        (Some(c), false) => c.saturating_sub(1),
                    };
                    set_op(&mut ed.ins, order[n], l);
                    scroll_to = true;
                }
                if !q.is_empty() && order.len() == 1 && ed.ins.op != order[0].op {
                    set_op(&mut ed.ins, order[0], l);
                }
                egui::Frame::new().fill(BG).corner_radius(R).inner_margin(6).show(ui, |ui| {
                    egui::ScrollArea::vertical().id_salt("ops").max_height(400.).min_scrolled_height(400.).auto_shrink([false, false]).show(ui, |ui| {
                        if order.is_empty() {
                            ui.label(RichText::new("Nada coincide con la búsqueda.").color(TEXT_MUTED));
                        }
                        for g in &groups {
                            ui.add_space(4.);
                            ui.label(RichText::new(*g).font(sb(12.)).color(TEXT_MUTED));
                            for m in visible.iter().filter(|m| m.group == *g) {
                                let sel = ed.ins.op == m.op;
                                let r = ui.add(Button::selectable(sel, RichText::new(m.label).color(if sel { ACCENT } else { TEXT }))
                                    .frame_when_inactive(sel).min_size(Vec2::new(ui.available_width(), 26.)));
                                if r.clicked() {
                                    set_op(&mut ed.ins, m, l);
                                }
                                if sel && scroll_to {
                                    r.scroll_to_me(None);
                                }
                            }
                        }
                    });
                });
            });
            ui.add_space(8.);
            ui.vertical(|ui| params_ui(ui, &mut ed.ins, l));
        });
        ui.add_space(8.);
        ui.separator();
        ui.horizontal(|ui| {
            if !ed.at.new && ui.button(danger_it("Borrar")).clicked() {
                res = Some(Res::Del);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let ok = catalog::meta(&ed.ins.op).is_some();
                if ui.add_enabled_ui(ok, |ui| primary(ui, (RichText::new(ic::CHECK), RichText::new("Aceptar").font(sb(14.))))).inner.clicked() {
                    res = Some(Res::Ok);
                }
                if ui.button("Cancelar").clicked() {
                    res = Some(Res::Cancel);
                }
                ui.label(RichText::new("Enter: aceptar. Esc: cancelar").small().color(TEXT_FAINT));
            });
        });
    });
    if res.is_none() && catalog::meta(&ed.ins.op).is_some() && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Enter)) {
        res = Some(Res::Ok);
    }
    if res.is_none() && r.should_close() {
        res = Some(Res::Cancel);
    }
    res
}

fn params_ui(ui: &mut Ui, ins: &mut Instr, l: &Lists) {
    let Some(m) = catalog::meta(&ins.op) else {
        ui.add_space(40.);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(ic::CURSOR).size(28.).color(TEXT_FAINT));
            ui.label(RichText::new("Elige una instrucción de la lista.").color(TEXT_MUTED));
        });
        return;
    };
    ui.label(RichText::new(m.label).font(sb(15.)));
    // Vista previa con la frase del catálogo.
    egui::Frame::new().fill(BG).corner_radius(R).inner_margin(Margin::symmetric(10, 8)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(catalog::sentence(ins)).color(TEXT));
    });
    ui.add_space(6.);
    let a = &mut ins.args;
    if a.len() < m.params.len() {
        a.resize(m.params.len(), String::new());
    }
    if m.params.is_empty() {
        ui.label(RichText::new("Sin parámetros.").color(TEXT_MUTED));
    }
    let fw = 260.;
    egui::Grid::new("params").num_columns(2).spacing([12., 8.]).min_col_width(110.).show(ui, |ui| {
        for (k, &(name, t)) in m.params.iter().enumerate() {
            ui.label(RichText::new(name).color(TEXT_MUTED));
            let a0 = a[0].clone();
            let v = &mut a[k];
            match t {
                P::Object => drop(combo(ui, k, v, &l.objs, "(elegir)")),
                P::Key => drop(combo(ui, k, v, catalog::KEYS, "(elegir)")),
                P::Mouse => drop(combo(ui, k, v, catalog::MOUSE, "(elegir)")),
                P::Op => drop(combo(ui, k, v, catalog::OPS, "(elegir)")),
                P::Ease => drop(combo(ui, k, v, catalog::EASES, "(elegir)")),
                P::Scene => drop(combo(ui, k, v, &l.scenes, "(elegir)")),
                P::Layer => drop(combo(ui, k, v, &l.layers, "(base)")),
                P::Func => {
                    let fs: Vec<String> = l.funcs.iter().filter(|f| f.1 == m.cond).map(|f| f.0.clone()).collect();
                    combo(ui, k, v, &fs, "(elegir)");
                }
                P::Anim => match l.anims.get(&a0) {
                    Some(an) if !an.is_empty() => drop(combo(ui, k, v, an, "(elegir)")),
                    _ => drop(ui.add(TextEdit::singleline(v).desired_width(fw))),
                },
                P::Prop => drop(ui.horizontal(|ui| {
                    ui.add(TextEdit::singleline(v).desired_width(fw - 40.));
                    egui::ComboBox::from_id_salt(("prop", k)).selected_text("").width(28.).show_ui(ui, |ui| {
                        for p in catalog::PROPS {
                            ui.selectable_value(v, p.to_string(), *p);
                        }
                    }).response.on_hover_text("Propiedades conocidas");
                })),
                P::File => drop(ui.horizontal(|ui| {
                    ui.add(TextEdit::singleline(v).desired_width(fw - 40.));
                    if ib(ui, ic::FOLDER_OPEN, "Elegir archivo").clicked() {
                        if let Some(f) = rfd::FileDialog::new().set_directory(&l.base).pick_file() {
                            *v = rel(&l.base, &f);
                        }
                    }
                })),
                P::Expr => drop(ui.vertical(|ui| {
                    ui.add(TextEdit::singleline(v).font(egui::TextStyle::Monospace).desired_width(fw));
                    expr_error(ui, v);
                })),
                P::Text | P::Var => drop(ui.add(TextEdit::singleline(v).desired_width(fw))),
                P::Code => codigo(ui, v, fw + 160.),
            }
            ui.end_row();
            if t == P::Func {
                let ps = l.funcs.iter().find(|f| f.0 == a[k]).map(|f| f.2.clone()).unwrap_or_default();
                a.resize(k + 1 + ps.len(), "0".into());
                for (j, pn) in ps.iter().enumerate() {
                    ui.label(RichText::new(pn).color(TEXT_MUTED));
                    let v = &mut a[k + 1 + j];
                    ui.vertical(|ui| {
                        ui.add(TextEdit::singleline(v).font(egui::TextStyle::Monospace).desired_width(fw));
                        expr_error(ui, v);
                    });
                    ui.end_row();
                }
            }
        }
    });
    if m.cond {
        ui.add_space(4.);
        ui.checkbox(&mut ins.not, "Invertir (NO)");
    }
    if matches!(m.op, "or" | "and") {
        ui.label(RichText::new("Las sub-condiciones se agregan en la hoja con «+ Sub-condición».").color(TEXT_MUTED));
    }
}

/// Funciones que se pueden usar dentro del código Rhai (para la ayuda).
pub const AYUDA_CODIGO: &str = "Dentro del código puedes usar:\n\
get_var(n) / set_var(n, v), get_global(n) / set_global(n, v), saved(n) / save(n, v)\n\
picked(obj) / all(obj) (ids), count(obj), object(id), exists(id)\n\
prop(id, p) / set_prop(id, p, v), create(obj, x, y) (id), destroy(id)\n\
key_down(k) / key_pressed(k), mouse_x() / mouse_y()\n\
random(a, b), dt(), time(), dsin / dcos / point_angle / point_distance\n\
play_sound(f), change_scene(n), log(msg)";

/// Colorea código Rhai: palabras clave, textos, números y comentarios.
fn colorear_codigo(src: &str, font: FontId) -> LayoutJob {
    const CLAVES: &[&str] = &["let", "const", "if", "else", "fn", "for", "in", "while", "loop", "return", "break", "continue", "true", "false", "switch"];
    let mut j = LayoutJob::default();
    let fmt = |c: egui::Color32| TextFormat { font_id: font.clone(), color: c, ..Default::default() };
    let (mut i, b) = (0, src.as_bytes());
    while i < src.len() {
        let rest = &src[i..];
        let (len, c) = if rest.starts_with("//") {
            (rest.find('\n').unwrap_or(rest.len()), TEXT_FAINT)
        } else if rest.starts_with('"') {
            (rest[1..].find('"').map_or(rest.len(), |k| k + 2), OK)
        } else if b[i].is_ascii_digit() {
            (rest.find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '.' || ch == '_')).unwrap_or(rest.len()), CAT_MOV)
        } else if b[i].is_ascii_alphabetic() || b[i] == b'_' {
            let n = rest.find(|ch: char| !(ch.is_alphanumeric() || ch == '_')).unwrap_or(rest.len());
            (n, if CLAVES.contains(&&rest[..n]) { ACCENT } else { TEXT })
        } else {
            (rest.chars().next().map_or(1, |ch| ch.len_utf8()), TEXT_SOFT)
        };
        j.append(&rest[..len], 0., fmt(c));
        i += len;
    }
    j
}

/// Editor de código Rhai de varias líneas, con colores y revisión en vivo.
pub fn codigo(ui: &mut Ui, v: &mut String, w: f32) {
    let font = FontId::monospace(13.);
    let mut lay = |ui: &Ui, buf: &dyn egui::TextBuffer, wrap: f32| {
        let mut j = colorear_codigo(buf.as_str(), font.clone());
        j.wrap.max_width = wrap;
        ui.fonts_mut(|f| f.layout_job(j))
    };
    ui.vertical(|ui| {
        ui.add(TextEdit::multiline(v).code_editor().desired_rows(6).desired_width(w).layouter(&mut lay));
        ui.horizontal_wrapped(|ui| {
            match nicostation::script::check(v) {
                Ok(()) => drop(ui.label(RichText::new(format!("{}  El código se entiende", ic::CHECK)).color(OK))),
                Err(e) => drop(ui.label(RichText::new(format!("{}  {e}", ic::WARNING)).color(DANGER))),
            }
            ui.label(RichText::new(ic::QUESTION).color(TEXT_MUTED)).on_hover_text(AYUDA_CODIGO);
        });
    });
}
