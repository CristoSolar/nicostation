//! Recetario: trozos de juego listos. Se elige una receta, se contestan sus preguntas y se agregan los eventos.

use super::eventos::Lists;
use super::facil::{self, ArgCx};
use super::iconos as ic;
use super::tema::*;
use eframe::egui::{self, Align, Button, CornerRadius, Frame, Layout, Margin, Rect, RichText, Sense, Stroke, Ui, UiBuilder, Vec2};
use nicostation::catalog::P;
use nicostation::model::Event;
use nicostation::recetas::RECETAS;

#[derive(Default)]
pub struct Recetario {
    pub sel: Option<usize>,
    pub vals: Vec<String>,
    pub err: Option<String>,
}

impl Recetario {
    pub fn elegir(&mut self, i: usize) {
        self.sel = Some(i);
        // Los objetos y escenas se eligen; teclas y números traen un valor de ejemplo.
        self.vals = RECETAS[i].huecos.iter().map(|(_, t, d)| if matches!(t, P::Object | P::Scene) { String::new() } else { d.to_string() }).collect();
        self.err = None;
    }
}

pub enum RecRes {
    Agregar(usize, Vec<Event>),
    Cerrar,
}

fn tarjeta(ui: &mut Ui, w: f32, i: usize) -> bool {
    let r = &RECETAS[i];
    let c = facil::cat(r.grupo);
    let resp = ui.scope_builder(UiBuilder::new().sense(Sense::click()).layout(Layout::top_down(Align::Min)), |ui| {
        ui.set_width(w);
        Frame::new().fill(tint(c.color)).corner_radius(10).inner_margin(Margin { left: 18, right: 12, top: 12, bottom: 12 }).show(ui, |ui| {
            ui.set_width(w - 30.);
            ui.set_min_height(72.);
            ui.horizontal(|ui| {
                ui.label(RichText::new(c.icon).size(22.).color(c.color));
                ui.add(egui::Label::new(RichText::new(r.nombre).font(sb(17.)).color(TEXT)).selectable(false));
            });
            ui.add(egui::Label::new(RichText::new(r.descripcion).size(15.).color(TEXT_SOFT)).selectable(false).wrap());
        });
    });
    let rect = resp.response.rect;
    let p = ui.painter();
    p.rect_filled(Rect::from_min_size(rect.min, Vec2::new(6., rect.height())), CornerRadius { nw: 10, sw: 10, ne: 0, se: 0 }, c.color);
    if resp.response.hovered() {
        p.rect_stroke(rect, 10, Stroke::new(2., c.color), egui::StrokeKind::Inside);
    }
    resp.response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

pub fn recetario(ctx: &egui::Context, st: &mut Recetario, l: &Lists) -> Option<RecRes> {
    let mut res = None;
    let scr = ctx.content_rect();
    let w = (scr.width() - 80.).min(880.);
    let hgt = (scr.height() - 240.).clamp(240., 540.);
    let frame = Frame::window(&ctx.global_style()).inner_margin(20).corner_radius(12);
    let r = egui::Modal::new(egui::Id::unique("recetario")).frame(frame).show(ctx, |ui| {
        ui.set_width(w);
        match st.sel {
            None => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(ic::COOKING_POT).size(26.).color(CAT_FUNC));
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Recetas").font(sb(24.)));
                        ui.label(RichText::new("Trozos de juego listos. Elige uno, contesta unas preguntas y se agrega a tu juego.").color(TEXT_SOFT));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        if ib(ui, ic::X, "Cerrar (Esc)").clicked() {
                            res = Some(RecRes::Cerrar);
                        }
                    });
                });
                ui.add_space(10.);
                let cols = if w >= 560. { 2 } else { 1 };
                let tw = ((w - 13. * cols as f32) / cols as f32).floor();
                egui::ScrollArea::vertical().max_height(hgt + 60.).min_scrolled_height(hgt + 60.).auto_shrink([false, false]).show(ui, |ui| {
                    for pair in (0..RECETAS.len()).collect::<Vec<_>>().chunks(cols) {
                        ui.horizontal_top(|ui| {
                            for &i in pair {
                                if tarjeta(ui, tw, i) {
                                    st.elegir(i);
                                }
                            }
                        });
                        ui.add_space(4.);
                    }
                });
            }
            Some(i) => {
                let rc = &RECETAS[i];
                let c = facil::cat(rc.grupo);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(c.icon).size(26.).color(c.color));
                    ui.vertical(|ui| {
                        ui.label(RichText::new(rc.nombre).font(sb(24.)));
                        ui.label(RichText::new(rc.descripcion).color(TEXT_SOFT));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        if ib(ui, ic::X, "Cerrar (Esc)").clicked() {
                            res = Some(RecRes::Cerrar);
                        }
                    });
                });
                ui.add_space(10.);
                egui::ScrollArea::vertical().max_height(hgt).auto_shrink([false, true]).show(ui, |ui| {
                    if rc.huecos.is_empty() {
                        ui.label(RichText::new("Esta receta no necesita preguntas. Solo agrégala.").size(16.));
                    }
                    let mut nv = None;
                    for (k, (q, t, _)) in rc.huecos.iter().enumerate() {
                        ui.push_id(k, |ui| {
                            Frame::new().fill(PANEL).corner_radius(10).inner_margin(12).show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.label(RichText::new(format!("{}. {q}", k + 1)).font(sb(17.)));
                                ui.add_space(4.);
                                let cx = ArgCx { l, op: "", k, obj: "", cond: false };
                                facil::editor(ui, *t, &mut st.vals[k], &cx, &mut nv);
                            });
                        });
                        ui.add_space(6.);
                    }
                    if !rc.variables.is_empty() {
                        let n: Vec<&str> = rc.variables.iter().map(|v| v.0).collect();
                        ui.label(RichText::new(format!("{} Se va a crear la cajita «{}» si no existe.", ic::ARCHIVE_BOX, n.join("», «"))).color(TEXT_SOFT));
                    }
                });
                if let Some(e) = &st.err {
                    ui.add_space(4.);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(ic::WARNING).color(facil::ROJO_TXT));
                        ui.label(RichText::new(e).color(facil::ROJO_TXT).size(15.));
                    });
                }
                ui.add_space(8.);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.add(Button::new(it(ic::ARROW_LEFT, "Ver otras recetas")).min_size(Vec2::new(0., 34.))).clicked() {
                        st.sel = None;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().button_padding = Vec2::new(18., 8.);
                        if primary(ui, (on_accent(ic::PLUS).size(18.), on_accent("Agregar al juego").font(sb(16.)))).clicked() {
                            match rc.aplicar(&st.vals) {
                                Ok(evs) => res = Some(RecRes::Agregar(i, evs)),
                                Err(e) => {
                                    st.err = Some(match e.strip_prefix("falta responder: ") {
                                        Some(q) => format!("Falta contestar: {q}"),
                                        None => e,
                                    })
                                }
                            }
                        }
                    });
                });
            }
        }
    });
    if res.is_none() && r.should_close() {
        res = Some(RecRes::Cerrar);
    }
    res
}
