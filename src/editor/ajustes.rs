//! Ajustes del juego (Archivo > Ajustes del juego…): nombre, tamaño, escalado, pixel art, FPS, ícono e imagen de carga.
//! También la carga de proyectos con migración del formato y aviso si el juego viene de una versión más nueva.

use super::iconos as ic;
use super::tema::*;
use eframe::egui::{self, Align, Button, Color32, DragValue, Frame, Layout, Pos2, Rect, RichText, Sense, TextEdit, Ui, Vec2};
use nicostation::model::{parse_project, Project, FORMAT_VERSION};
use std::collections::HashMap;
use std::path::Path;

/// Lee un proyecto: si es de un formato viejo lo migra; si es de uno más nuevo, lo deja como está y avisa.
pub fn leer(json: &str) -> Result<(Project, Option<String>), String> {
    let p = parse_project(json.as_bytes())?;
    let aviso = (p.version > FORMAT_VERSION).then(|| {
        format!(
            "Este juego se hizo con una versión más nueva de NicoStation (formato {}; este editor entiende hasta el {FORMAT_VERSION}). Puede que algunas cosas no se vean o se pierdan al guardar.",
            p.version
        )
    });
    Ok((p, aviso))
}

pub fn cargar(path: &Path) -> Result<(Project, Option<String>), String> {
    leer(&std::fs::read_to_string(path).map_err(|e| e.to_string())?)
}

pub const MODOS: [(&str, &str, &str, &str); 3] = [
    ("fit", "Ajustar", "Que entre", "Se agranda todo lo que puede sin deformarse; si la ventana tiene otra forma, quedan franjas negras."),
    ("stretch", "Estirar", "Llenar todo", "Llena toda la ventana aunque el dibujo se estire o se aplaste."),
    ("integer", "Pixel perfecto", "Pixeles parejos", "Solo agranda al doble, triple... así cada pixel queda del mismo tamaño (ideal para pixel art). Puede dejar más borde."),
];

/// Lo que el diálogo pide al editor (elegir archivos se hace afuera: hay que copiarlos al juego).
pub enum Pedido {
    Cerrar,
    ElegirIcono,
    ElegirCarga,
}

pub fn dialogo(ctx: &egui::Context, p: &mut Project, imgs: &[String], tex: &HashMap<String, egui::TextureHandle>, facil: bool) -> Option<Pedido> {
    let mut ped = None;
    let frame = Frame::window(&ctx.global_style()).inner_margin(20).corner_radius(12);
    let r = egui::Modal::new(egui::Id::unique("ajustes_juego")).frame(frame).show(ctx, |ui| {
        ui.set_width(600.);
        ui.horizontal(|ui| {
            ui.label(RichText::new(ic::GEAR).size(22.).color(ACCENT));
            ui.label(RichText::new("Ajustes del juego").font(sb(20.)));
        });
        ui.label(muted(if facil { "Cómo se llama tu juego, de qué tamaño es y cómo se ve en la pantalla." } else { "Se guardan en el proyecto (settings) y los usa el reproductor y la exportación." }));
        ui.add_space(8.);
        egui::ScrollArea::vertical().max_height(560.).show(ui, |ui| {
            egui::Grid::new("ajustes_grid").num_columns(2).spacing([16., 10.]).min_col_width(130.).show(ui, |ui| {
                ui.label(muted("Nombre"));
                ui.add(TextEdit::singleline(&mut p.name).desired_width(280.));
                ui.end_row();
                ui.label(muted(if facil { "Tamaño de la pantalla" } else { "Resolución" }));
                ui.horizontal(|ui| {
                    ui.add(DragValue::new(&mut p.width).range(16..=8192).suffix(" px"));
                    ui.label(muted("×"));
                    ui.add(DragValue::new(&mut p.height).range(16..=8192).suffix(" px"));
                    ui.menu_button(it(ic::CARET_DOWN, "Comunes"), |ui| {
                        menu_spacing(ui);
                        for (w, h, n) in [(320., 180., "Pixel art chico (16:9)"), (640., 360., "Pixel art (16:9)"), (800., 600., "Clásico (4:3)"), (1280., 720., "HD (16:9)"), (1920., 1080., "Full HD"), (600., 800., "Vertical (celular)")] {
                            if ui.button(format!("{n}  {w:.0} × {h:.0}")).clicked() {
                                (p.width, p.height) = (w, h);
                                ui.close();
                            }
                        }
                    });
                });
                ui.end_row();
                ui.label(muted(if facil { "Si la ventana es más grande" } else { "Escalado" }));
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        super::seg(ui, |ui| {
                            for (id, exp, kid, _) in MODOS {
                                if super::seg_btn(ui, p.settings.scale_mode == id, if facil { kid } else { exp }).clicked() {
                                    p.settings.scale_mode = id.into();
                                }
                            }
                        })
                    });
                    let ex = MODOS.iter().find(|m| m.0 == p.settings.scale_mode).map_or("", |m| m.3);
                    ui.add(egui::Label::new(RichText::new(ex).color(TEXT_SOFT)).wrap());
                    escala_dibujo(ui, &p.settings.scale_mode, p.width / p.height.max(1.));
                });
                ui.end_row();
                ui.label(muted("Pixel art"));
                ui.vertical(|ui| {
                    ui.checkbox(&mut p.settings.pixel_art, if facil { "Pixeles nítidos (sin borronear)" } else { "Dibujo nítido (filtro «nearest»)" });
                    ui.label(RichText::new(if p.settings.pixel_art { "Al agrandar, cada pixel se ve como un cuadradito." } else { "Al agrandar, las imágenes se suavizan (mejor para dibujos grandes o fotos)." }).small().color(TEXT_MUTED));
                });
                ui.end_row();
                ui.label(muted(if facil { "Velocidad máxima" } else { "FPS máximos" }));
                ui.horizontal(|ui| {
                    super::seg(ui, |ui| {
                        for (v, t) in [(0, "Pantalla"), (30, "30"), (60, "60"), (120, "120")] {
                            if super::seg_btn(ui, p.settings.max_fps == v, t).on_hover_text(if v == 0 { "Lo que dé la pantalla (recomendado)" } else { "Cuadros por segundo como máximo" }).clicked() {
                                p.settings.max_fps = v;
                            }
                        }
                    });
                    if !facil {
                        ui.add(DragValue::new(&mut p.settings.max_fps).range(0..=1000).suffix(" fps")).on_hover_text("0 = los de la pantalla");
                    }
                });
                ui.end_row();
                ui.label(muted("Ícono"));
                if imagen(ui, "icono", &mut p.settings.icon, imgs, tex, 48.) {
                    ped = Some(Pedido::ElegirIcono);
                }
                ui.end_row();
                ui.label(muted(if facil { "Imagen mientras carga" } else { "Pantalla de carga" }));
                if imagen(ui, "carga", &mut p.settings.loading_image, imgs, tex, 96.) {
                    ped = Some(Pedido::ElegirCarga);
                }
                ui.end_row();
                if !facil {
                    ui.label(muted("Formato"));
                    ui.label(muted(format!("versión {} (este editor: {FORMAT_VERSION})", p.version)));
                    ui.end_row();
                }
            });
        });
        ui.add_space(10.);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if primary(ui, (on_accent(ic::CHECK), on_accent("Listo").font(sb(14.)))).clicked() {
                ped = Some(Pedido::Cerrar);
            }
            ui.label(muted("Los cambios se aplican enseguida · Ctrl+Z deshace"));
        });
    });
    if ped.is_none() && r.should_close() {
        ped = Some(Pedido::Cerrar);
    }
    ped
}

/// Elegir una imagen del proyecto con vista previa. Devuelve true si pide buscar un archivo afuera.
fn imagen(ui: &mut Ui, id: &str, v: &mut String, imgs: &[String], tex: &HashMap<String, egui::TextureHandle>, lado: f32) -> bool {
    let mut out = false;
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(lado), Sense::hover());
        super::recursos::damero(ui.painter(), rect);
        match tex.get(v.as_str()) {
            Some(t) => {
                let s = t.size_vec2();
                let k = (rect.width() / s.x).min(rect.height() / s.y);
                ui.painter().image(t.id(), Rect::from_center_size(rect.center(), s * k), Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)), Color32::WHITE);
            }
            None => drop(ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, ic::IMAGE, egui::FontId::proportional(lado * 0.4), TEXT_FAINT)),
        }
        ui.vertical(|ui| {
            let shown = if v.is_empty() { "(ninguna)".to_string() } else { v.clone() };
            egui::ComboBox::from_id_salt(id).selected_text(shown).width(260.).show_ui(ui, |ui| {
                if ui.selectable_label(v.is_empty(), "(ninguna)").clicked() {
                    v.clear();
                }
                for f in imgs {
                    if ui.selectable_label(v == f, f.as_str()).clicked() {
                        *v = f.clone();
                    }
                }
            });
            ui.horizontal(|ui| {
                if ui.button(it(ic::FOLDER_OPEN, "Elegir archivo…")).clicked() {
                    out = true;
                }
                if !v.is_empty() && ui.add(Button::new(it(ic::X, "Quitar")).frame_when_inactive(false)).clicked() {
                    v.clear();
                }
            });
        });
    });
    out
}

/// Dibujito que muestra cómo queda el juego en una ventana ancha con el modo elegido.
fn escala_dibujo(ui: &mut Ui, modo: &str, aspecto: f32) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(200., 70.), Sense::hover());
    let p = ui.painter();
    p.rect_filled(r, 4, Color32::BLACK);
    p.rect_stroke(r, 4, egui::Stroke::new(1., BORDER_STRONG), egui::StrokeKind::Inside);
    let juego = match modo {
        "stretch" => r.shrink(1.),
        "integer" => {
            let h = (r.height() * 0.5).floor();
            Rect::from_center_size(r.center(), Vec2::new(h * aspecto, h))
        }
        _ => Rect::from_center_size(r.center(), Vec2::new(r.height() * aspecto, r.height()).min(r.size())),
    };
    p.rect_filled(juego, 2, blend(CAT_MOV, SURFACE, 0.6));
    p.text(juego.center(), egui::Align2::CENTER_CENTER, ic::GAMEPAD, egui::FontId::proportional(18.), TEXT);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migra_proyectos_viejos() {
        let (p, aviso) = leer(r#"{"name":"t","width":10,"height":10,"first_scene":"E","scenes":[{"name":"E"}]}"#).unwrap();
        assert!(aviso.is_none());
        assert_eq!(p.version, FORMAT_VERSION);
        assert_eq!(p.settings.scale_mode, "fit");
        assert!(p.settings.pixel_art);
    }

    #[test]
    fn avisa_si_es_mas_nuevo() {
        let json = format!(r#"{{"name":"t","width":10,"height":10,"first_scene":"E","version":{},"scenes":[{{"name":"E"}}]}}"#, FORMAT_VERSION + 1);
        let (p, aviso) = leer(&json).unwrap();
        assert!(aviso.unwrap().contains("versión más nueva"));
        // No se baja la versión.
        assert_eq!(p.version, FORMAT_VERSION + 1);
        assert!(leer("{").is_err());
    }
}
