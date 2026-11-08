//! «Mis bloques» (funciones) en el modo fácil: lista de bloques inventados, asistente para crear uno y cabecera de edición.

use super::facil::{self, ident_fix, ident_ok, pill, pill_style, Ficha, Pill};
use super::iconos as ic;
use super::tema::*;
use eframe::egui::{self, Align, Button, CornerRadius, Frame, Key, Layout, Margin, Rect, RichText, Sense, Stroke, TextEdit, Ui, UiBuilder, Vec2};
use nicostation::model::Function;

/// Dibujo del bloque como se verá en la hoja: «comer [n]».
pub fn dibujo(ui: &mut Ui, name: &str, params: &[String], cond: bool) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(5., 4.);
        ui.label(RichText::new(ic::PUZZLE).size(18.).color(CAT_FUNC));
        pill(ui, if name.is_empty() { "nombre" } else { name }, &pill_style(Ficha::Mio, CAT_FUNC), Sense::hover());
        for p in params {
            pill(ui, p, &pill_style(Ficha::Hueco, CAT_FUNC), Sense::hover());
        }
        if cond {
            ui.label(RichText::new("?").font(sb(18.)).color(TEXT));
        }
    });
}

fn tipo_txt(cond: bool) -> &'static str {
    if cond { "pregunta (sí / no)" } else { "acción" }
}

pub enum Act {
    Sel(usize),
    Nuevo,
    Borrar(usize),
    Renombrar(usize, String),
}

/// Lista de bloques como tarjetas rosadas.
pub fn lista(ui: &mut Ui, fs: &[Function], sel: usize) -> Option<Act> {
    let mut act = None;
    ui.horizontal(|ui| {
        ui.label(RichText::new(ic::PUZZLE).size(24.).color(CAT_FUNC));
        ui.label(RichText::new("Mis bloques").font(sb(20.)));
    });
    ui.label(RichText::new("Inventa tu propio bloque: junta varios bloques en uno y úsalo donde quieras.").size(15.).color(TEXT_SOFT));
    ui.add_space(8.);
    ui.spacing_mut().button_padding = Vec2::new(14., 8.);
    // Secundario: al lado, la hoja del bloque ya tiene su botón principal («Nuevo evento»).
    if ui.add(Button::new((RichText::new(ic::SPARKLE).size(17.).color(CAT_FUNC), RichText::new("Inventar un bloque").font(sb(15.))))).clicked() {
        act = Some(Act::Nuevo);
    }
    ui.add_space(8.);
    for (i, f) in fs.iter().enumerate() {
        let s = i == sel;
        let r = ui.scope_builder(UiBuilder::new().sense(Sense::click()).layout(Layout::top_down(Align::Min)).id_salt(i), |ui| {
            Frame::new().fill(if s { blend(CAT_FUNC, SURFACE, 0.3) } else { tint(CAT_FUNC) }).corner_radius(10)
                .stroke(Stroke::new(if s { 2. } else { 0. }, CAT_FUNC)).inner_margin(Margin { left: 14, right: 8, top: 8, bottom: 8 }).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    dibujo(ui, &f.name, &f.params, f.is_condition);
                    ui.label(RichText::new(format!("Es una {} · {}", tipo_txt(f.is_condition), if f.events.len() == 1 { "1 evento".to_string() } else { format!("{} eventos", f.events.len()) })).size(15.).color(TEXT_SOFT));
                });
        });
        let rr = r.response.rect;
        ui.painter().rect_filled(Rect::from_min_size(rr.min, Vec2::new(5., rr.height())), CornerRadius { nw: 10, sw: 10, ne: 0, se: 0 }, CAT_FUNC);
        if r.response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            act = Some(Act::Sel(i));
        }
        ui.add_space(2.);
    }
    if fs.is_empty() {
        ui.add_space(6.);
        ui.label(RichText::new("Todavía no inventaste ningún bloque.").color(TEXT_MUTED));
    }
    act
}

/// Chips de ingredientes con botón para quitar, y campo para agregar.
fn ingredientes(ui: &mut Ui, params: &mut Vec<String>, nuevo: &mut String) {
    let mut del = None;
    ui.horizontal_wrapped(|ui| {
        for (k, p) in params.iter().enumerate() {
            let st = Pill { icon: Some(ic::X), ..pill_style(Ficha::Cajita, CAT_VAR) };
            if pill(ui, p, &st, Sense::click()).on_hover_text("Quitar este ingrediente").clicked() {
                del = Some(k);
            }
        }
        let r = ui.add(TextEdit::singleline(nuevo).hint_text("nuevo ingrediente").desired_width(ui.available_width().clamp(100., 150.)));
        let enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        let nm = nuevo.trim().to_string();
        let ok = ident_ok(&nm) && !params.contains(&nm);
        if (ui.add_enabled(ok, Button::new(it(ic::PLUS, "Agregar")).min_size(Vec2::new(0., 30.))).clicked() || enter) && ok {
            params.push(nm);
            nuevo.clear();
            r.request_focus();
        }
    });
    if let Some(k) = del {
        params.remove(k);
    }
    let nm = nuevo.trim();
    if !nm.is_empty() && !ident_ok(nm) {
        let fix = ident_fix(nm);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Usa solo letras sin acento, números y _.").color(facil::ROJO_TXT));
            if !fix.is_empty() && ui.button(format!("Usar «{fix}»")).clicked() {
                *nuevo = fix;
            }
        });
    }
}

fn tipo_opciones(ui: &mut Ui, cond: &mut bool, w: f32) {
    ui.horizontal_top(|ui| {
        for (c, icon, t, sub) in [
            (false, ic::LIGHTNING, "Una acción", "Hace algo. Ejemplo: «saltar_alto» o «perder_vida». Va en ENTONCES."),
            (true, ic::QUESTION, "Una pregunta", "Responde sí o no. Ejemplo: «tiene_vidas». Va en CUANDO."),
        ] {
            let sel = *cond == c;
            let r = ui.scope_builder(UiBuilder::new().sense(Sense::click()).layout(Layout::top_down(Align::Min)).id_salt(c), |ui| {
                Frame::new().fill(if sel { blend(ACCENT, SURFACE, 0.18) } else { SURFACE }).corner_radius(10)
                    .stroke(Stroke::new(if sel { 2. } else { 1. }, if sel { ACCENT } else { BORDER_STRONG })).inner_margin(12).show(ui, |ui| {
                        ui.set_width(w);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(icon).size(22.).color(if sel { ACCENT } else { TEXT_SOFT }));
                            ui.label(RichText::new(t).font(sb(17.)));
                        });
                        ui.add(egui::Label::new(RichText::new(sub).size(15.).color(TEXT_SOFT)).wrap().selectable(false));
                    });
            });
            if r.response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                *cond = c;
            }
        }
    });
}

/// Validación del nombre; devuelve el mensaje de error (y el arreglo sugerido).
fn nombre_error(n: &str, taken: &dyn Fn(&str) -> bool) -> Option<(String, Option<String>)> {
    if n.is_empty() {
        return Some(("Escribe un nombre.".into(), None));
    }
    if !ident_ok(n) {
        let f = ident_fix(n);
        return Some(("Usa solo letras sin acento, números y _ (sin espacios).".into(), (!f.is_empty() && !taken(&f)).then_some(f)));
    }
    taken(n).then(|| ("Ya tienes un bloque con ese nombre.".into(), None))
}

pub struct Asistente {
    pub name: String,
    pub cond: bool,
    pub params: Vec<String>,
    pub nuevo: String,
    pub foco: bool,
}

impl Default for Asistente {
    fn default() -> Self {
        Asistente { name: String::new(), cond: false, params: vec![], nuevo: String::new(), foco: true }
    }
}

/// Asistente para inventar un bloque. `Some(None)` = cancelado.
pub fn asistente(ctx: &egui::Context, a: &mut Asistente, taken: &dyn Fn(&str) -> bool) -> Option<Option<Function>> {
    let mut res = None;
    let frame = Frame::window(&ctx.global_style()).inner_margin(20).corner_radius(12);
    let r = egui::Modal::new(egui::Id::unique("asistente_bloque")).frame(frame).show(ctx, |ui| {
        let w = (ctx.content_rect().width() - 80.).min(640.);
        ui.set_width(w);
        ui.horizontal(|ui| {
            ui.label(RichText::new(ic::SPARKLE).size(26.).color(CAT_FUNC));
            ui.label(RichText::new("Inventar un bloque").font(sb(24.)));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ib(ui, ic::X, "Cerrar (Esc)").clicked() {
                    res = Some(None);
                }
            });
        });
        ui.add_space(10.);
        ui.label(RichText::new("1. ¿Cómo se llama tu bloque?").font(sb(17.)));
        let r = ui.add(TextEdit::singleline(&mut a.name).hint_text("por ejemplo: ganar_puntos").desired_width(w.min(320.)).font(egui::FontId::proportional(17.)));
        if std::mem::take(&mut a.foco) {
            r.request_focus();
        }
        let err = nombre_error(a.name.trim(), taken);
        if let (Some((e, fix)), false) = (&err, a.name.trim().is_empty()) {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(e).color(facil::ROJO_TXT));
                if let Some(f) = fix {
                    if ui.button(format!("Usar «{f}»")).clicked() {
                        a.name = f.clone();
                    }
                }
            });
        }
        ui.add_space(12.);
        ui.label(RichText::new("2. ¿Es una acción (hace algo) o una pregunta (sí / no)?").font(sb(17.)));
        ui.add_space(2.);
        tipo_opciones(ui, &mut a.cond, ((w - 56.) / 2.).floor());
        ui.add_space(12.);
        ui.label(RichText::new("3. Ingredientes (si quieres)").font(sb(17.)));
        ui.label(RichText::new("Datos que le das al bloque cuando lo usas. Por ejemplo, cuántos puntos sumar. Dentro del bloque son cajitas.").color(TEXT_SOFT));
        ingredientes(ui, &mut a.params, &mut a.nuevo);
        ui.add_space(12.);
        Frame::new().fill(BG).corner_radius(10).inner_margin(12).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("Así se va a ver:").color(TEXT_MUTED));
            Frame::new().fill(tint(CAT_FUNC)).corner_radius(8).inner_margin(Margin { left: 14, right: 8, top: 6, bottom: 6 }).show(ui, |ui| {
                dibujo(ui, a.name.trim(), &a.params, a.cond);
            });
        });
        ui.add_space(10.);
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Cancelar").clicked() {
                res = Some(None);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().button_padding = Vec2::new(18., 8.);
                let ok = err.is_none();
                if ui.add_enabled_ui(ok, |ui| primary(ui, (on_accent(ic::CHECK).size(18.), on_accent("Crear bloque").font(sb(16.))))).inner.clicked() {
                    res = Some(Some(Function { name: a.name.trim().into(), is_condition: a.cond, params: a.params.clone(), events: vec![], graph: None }));
                }
            });
        });
    });
    if res.is_none() && r.should_close() {
        res = Some(None);
    }
    res
}

/// Cabecera del bloque elegido: nombre, tipo, ingredientes y ayuda.
pub fn cabecera(ui: &mut Ui, i: usize, f: &mut Function, nuevo: &mut String, taken: &dyn Fn(&str) -> bool) -> Option<Act> {
    let mut act = None;
    Frame::new().fill(PANEL).stroke((1., BORDER)).corner_radius(12).inner_margin(14).show(ui, |ui| {
        ui.set_width(ui.available_width());
        // Fila del dibujo con «Borrar» a la derecha; si no entra, «Borrar» va debajo.
        let narrow = ui.available_width() < 460.;
        let borrar = |ui: &mut Ui, act: &mut Option<Act>| {
            if ui.add(Button::new(danger_it("Borrar bloque")).min_size(Vec2::new(0., 32.))).clicked() {
                *act = Some(Act::Borrar(i));
            }
        };
        if narrow {
            Frame::new().fill(tint(CAT_FUNC)).corner_radius(8).inner_margin(Margin { left: 14, right: 10, top: 6, bottom: 6 }).show(ui, |ui| {
                dibujo(ui, &f.name, &f.params, f.is_condition);
            });
            borrar(ui, &mut act);
        } else {
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    borrar(ui, &mut act);
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        Frame::new().fill(tint(CAT_FUNC)).corner_radius(8).inner_margin(Margin { left: 14, right: 10, top: 6, bottom: 6 }).show(ui, |ui| {
                            dibujo(ui, &f.name, &f.params, f.is_condition);
                        });
                    });
                });
            });
        }
        ui.add_space(8.);
        // Filas «etiqueta: control» que se acomodan al ancho (sin rejilla fija).
        let fila = |ui: &mut Ui, t: &str, body: &mut dyn FnMut(&mut Ui)| {
            ui.horizontal_wrapped(|ui| {
                ui.allocate_ui_with_layout(Vec2::new(100., 32.), Layout::left_to_right(Align::Center), |ui| {
                    ui.set_min_width(100.);
                    ui.label(RichText::new(t).color(TEXT_SOFT));
                });
                body(ui);
            });
        };
        fila(ui, "Nombre", &mut |ui| {
            let bid = ui.make_persistent_id(("fname", i));
            let mut buf: String = ui.data_mut(|d| d.get_temp(bid)).unwrap_or_else(|| f.name.clone());
            let w = ui.available_width().clamp(120., 200.);
            let r = ui.add(TextEdit::singleline(&mut buf).font(sb(16.)).desired_width(w));
            if r.lost_focus() {
                let n = buf.trim().to_string();
                if n != f.name && nombre_error(&n, taken).is_none() {
                    act = Some(Act::Renombrar(i, n));
                }
                ui.data_mut(|d| d.remove::<String>(bid));
            } else if r.has_focus() {
                if let Some((e, _)) = (buf.trim() != f.name).then(|| nombre_error(buf.trim(), taken)).flatten() {
                    ui.label(RichText::new(e).color(facil::ROJO_TXT));
                }
                ui.data_mut(|d| d.insert_temp(bid, buf));
            }
        });
        fila(ui, "Es una", &mut |ui| {
            for (c, t) in [(false, "Acción"), (true, "Pregunta (sí / no)")] {
                if ui.add(Button::selectable(f.is_condition == c, RichText::new(t).size(15.)).min_size(Vec2::new(0., 32.))).clicked() {
                    f.is_condition = c;
                }
            }
        });
        ui.label(RichText::new("Ingredientes").color(TEXT_SOFT));
        ingredientes(ui, &mut f.params, nuevo);
        ui.add_space(6.);
        let info = |ui: &mut Ui, icon: &str, t: String| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(icon).color(CAT_FUNC));
                ui.label(RichText::new(t).color(TEXT_SOFT));
            });
        };
        if !f.params.is_empty() {
            info(ui, ic::ARCHIVE_BOX, format!(
                "Los ingredientes ({}) son cajitas que solo existen dentro de este bloque. Escribe su nombre donde quieras usar su valor.",
                f.params.join(", ")
            ));
        }
        if f.is_condition {
            info(ui, ic::QUESTION, "Es una pregunta: usa el bloque «Responder» (en ENTONCES, categoría Mis bloques) para decir sí o no.".into());
        }
        info(ui, ic::LIGHTBULB, format!(
            "Para usarlo, ve a Eventos y búscalo en la categoría «Mis bloques», en {}.",
            if f.is_condition { "CUANDO" } else { "ENTONCES" }
        ));
    });
    act
}

/// Estado vacío cuando todavía no hay bloques.
pub fn vacio(ui: &mut Ui) -> bool {
    let mut go = false;
    ui.add_space(60.);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(ic::PUZZLE).size(48.).color(CAT_FUNC));
        ui.add_space(6.);
        ui.label(RichText::new("Inventa tu propio bloque").font(sb(22.)));
        ui.label(RichText::new("Junta varios bloques en uno, ponle nombre y úsalo donde quieras.").size(16.).color(TEXT_SOFT));
        ui.label(RichText::new("Por ejemplo, «ganar_puntos» que suma puntos y hace sonar una moneda.").size(15.).color(TEXT_MUTED));
        ui.add_space(14.);
        ui.spacing_mut().button_padding = Vec2::new(16., 10.);
        go = primary(ui, (on_accent(ic::SPARKLE).size(18.), on_accent("Inventar un bloque").font(sb(16.)))).clicked();
    });
    go
}
