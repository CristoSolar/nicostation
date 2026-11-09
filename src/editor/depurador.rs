//! Depurador de la vista previa: instancias, cajitas, registro y rendimiento.
//! En modo fácil es «¿Qué está pasando?» (solo mirar, con palabras simples); en modo experto se puede editar en vivo.

use crate::iconos as ic;
use crate::tema::*;
use crate::vista::{Pestana, Sesion, Tipo, HISTORIA};
use eframe::egui::{self, Align2, Color32, DragValue, FontId, Frame, Margin, Pos2, Rect, RichText, Sense, Stroke, TextEdit, Ui, Vec2};
use nicostation::model::{Kind, Value, Vars};
use std::collections::HashMap;

pub fn panel(ui: &mut Ui, s: &mut Sesion, facil: bool) {
    if facil {
        return que_pasa(ui, s);
    }
    let n = s.world.insts.len();
    ui.horizontal(|ui| {
        ui.label(RichText::new(ic::BUG).size(16.).color(ACCENT));
        ui.label(RichText::new("Depurador").font(sb(15.)));
    });
    ui.add_space(2.);
    let avisos = s.registro.iter().filter(|l| l.tipo == Tipo::Aviso).count();
    Frame::new().fill(BG).stroke((1., BORDER)).corner_radius(R).inner_margin(2).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(2.);
                ui.spacing_mut().button_padding = Vec2::new(7., 3.);
                for (p, t) in [
                    (Pestana::Cosas, format!("Instancias {n}")),
                    (Pestana::Cajitas, "Variables".into()),
                    (Pestana::Registro, if avisos > 0 { format!("Registro ({avisos})") } else { "Registro".into() }),
                    (Pestana::Rendimiento, "Rendimiento".into()),
                    (Pestana::Perfil, "Perfilador".into()),
                ] {
                    if ui.add(egui::Button::selectable(s.pestana == p, t).frame_when_inactive(s.pestana == p)).clicked() {
                        s.pestana = p;
                    }
                }
        });
    });
    ui.add_space(4.);
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match s.pestana {
        Pestana::Cosas => {
            // La elegida arriba (se ve sin bajar), y la lista debajo.
            if s.elegida().is_some() {
                propiedades(ui, s);
                ui.add_space(4.);
                ui.separator();
            } else {
                ui.label(muted("Elige una instancia de la lista, o haz clic en el juego en pausa (Alt+clic mientras corre)."));
            }
            lista(ui, s, false);
        }
        Pestana::Cajitas => cajitas(ui, s, true),
        Pestana::Registro => registro(ui, s, false),
        Pestana::Rendimiento => rendimiento(ui, s),
        Pestana::Perfil => perfilador(ui, s),
    });
}

/// Vista simple para niños.
fn que_pasa(ui: &mut Ui, s: &mut Sesion) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(ic::QUESTION).size(20.).color(ACCENT));
        ui.label(RichText::new("¿Qué está pasando?").font(sb(17.)));
    });
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.label(RichText::new("Estas son las cosas que hay en tu juego ahora mismo. Toca una para verla brillar.").color(TEXT_SOFT));
        ui.add_space(4.);
        if let Some(i) = s.elegida() {
            ficha_simple(ui, s, i);
            ui.add_space(6.);
        }
        lista(ui, s, true);
        ui.add_space(10.);
        cajitas(ui, s, false);
        ui.add_space(10.);
        ui.label(RichText::new("Lo que pasó").font(sb(15.)));
        ui.label(RichText::new("Cambios de escena, sonidos y partidas guardadas, del más nuevo al más viejo.").color(TEXT_MUTED));
        registro(ui, s, true);
    });
}

/// Miniatura de un objeto (su primera imagen, su color o un ícono).
fn mini(ui: &mut Ui, s: &Sesion, obj: &str, lado: f32) {
    let (r, _) = ui.allocate_exact_size(Vec2::splat(lado), Sense::hover());
    let p = ui.painter();
    p.rect_filled(r, 4, SURFACE_HOVER);
    let full = Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.));
    match s.world.object_def(obj).map(|d| &d.kind) {
        Some(Kind::Background { .. }) => drop(p.text(r.center(), Align2::CENTER_CENTER, ic::MOUNTAINS, FontId::proportional(lado * 0.6), TEXT_SOFT)),
        Some(Kind::Particles(_)) => drop(p.text(r.center(), Align2::CENTER_CENTER, ic::SPARKLE, FontId::proportional(lado * 0.6), TEXT_SOFT)),
        Some(Kind::Mesh { .. }) => drop(p.text(r.center(), Align2::CENTER_CENTER, ic::CUBE, FontId::proportional(lado * 0.6), TEXT_SOFT)),
        Some(Kind::Light { .. }) => drop(p.text(r.center(), Align2::CENTER_CENTER, ic::LIGHTBULB, FontId::proportional(lado * 0.6), TEXT_SOFT)),
        Some(Kind::Camera3d { .. }) => drop(p.text(r.center(), Align2::CENTER_CENTER, ic::CAMERA, FontId::proportional(lado * 0.6), TEXT_SOFT)),
        Some(Kind::Sprite { animations, .. }) => match animations.first().and_then(|a| a.frames.first()).and_then(|f| s.tex.get(f)) {
            Some(t) => {
                let sz = t.size_vec2();
                let k = (r.width() - 4.) / sz.x.max(sz.y);
                p.image(t.id(), Rect::from_center_size(r.center(), sz * k), full, Color32::WHITE);
            }
            None => drop(p.text(r.center(), Align2::CENTER_CENTER, ic::IMAGE, FontId::proportional(lado * 0.6), TEXT_FAINT)),
        },
        Some(Kind::Box { color, .. }) => drop(p.rect_filled(r.shrink(3.), 2, Color32::from_rgb(color[0], color[1], color[2]))),
        Some(Kind::Text { color, .. }) => drop(p.text(r.center(), Align2::CENTER_CENTER, ic::TEXT_T, FontId::proportional(lado * 0.6), Color32::from_rgb(color[0], color[1], color[2]))),
        Some(Kind::Tilemap { .. }) => drop(p.text(r.center(), Align2::CENTER_CENTER, ic::MAP, FontId::proportional(lado * 0.6), CAT_SCENE)),
        None => {}
    }
}

/// Instancias agrupadas por objeto, con cantidad; un botón por instancia.
fn lista(ui: &mut Ui, s: &mut Sesion, facil: bool) {
    let mut grupos: Vec<(String, Vec<usize>)> = vec![];
    for (i, o) in s.world.insts.iter().enumerate() {
        match grupos.iter_mut().find(|g| g.0 == o.obj) {
            Some(g) => g.1.push(i),
            None => grupos.push((o.obj.clone(), vec![i])),
        }
    }
    if grupos.is_empty() {
        ui.label(muted(if facil { "No hay nada en la escena." } else { "Sin instancias." }));
    }
    let elegida = s.elegida();
    let mut nueva = None;
    for (obj, idx) in &grupos {
        Frame::new().fill(SURFACE).corner_radius(8).inner_margin(Margin::symmetric(8, 6)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                mini(ui, s, obj, if facil { 26. } else { 20. });
                ui.label(RichText::new(obj).font(sb(if facil { 15. } else { 14. })));
                let n = idx.len();
                let t = if facil { if n == 1 { "hay 1".to_string() } else { format!("hay {n}") } } else { format!("{n}") };
                ui.label(RichText::new(t).color(TEXT_MUTED));
            });
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(4.);
                ui.spacing_mut().button_padding = Vec2::new(6., 2.);
                for (k, &i) in idx.iter().enumerate().take(60) {
                    let sel = elegida == Some(i);
                    let t = if idx.len() == 1 { if facil { "Ver".to_string() } else { format!("#{}", s.world.insts[i].id) } } else { format!("{}", k + 1) };
                    let tip = format!("x {:.0}, y {:.0}", s.world.insts[i].x, s.world.insts[i].y);
                    if ui.add(egui::Button::selectable(sel, t)).on_hover_text(tip).clicked() {
                        nueva = Some(if sel { None } else { Some(s.world.insts[i].id) });
                    }
                }
                if idx.len() > 60 {
                    ui.label(muted(format!("y {} más", idx.len() - 60)));
                }
            });
        });
        ui.add_space(2.);
    }
    if let Some(n) = nueva {
        s.sel = n;
    }
}

fn si_no(b: bool) -> &'static str {
    if b { "sí" } else { "no" }
}

/// Ficha de la instancia elegida, en palabras simples (solo mirar).
fn ficha_simple(ui: &mut Ui, s: &Sesion, i: usize) {
    let o = &s.world.insts[i];
    let anim = s.world.get(i, "anim").text();
    Frame::new().fill(blend(ACCENT, SURFACE, 0.12)).stroke((1.5, ACCENT)).corner_radius(10).inner_margin(10).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            mini(ui, s, &o.obj, 32.);
            ui.label(RichText::new(&o.obj).font(sb(17.)));
        });
        let fila = |ui: &mut Ui, icon: &str, t: String| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon).color(TEXT_MUTED));
                ui.label(RichText::new(t).size(15.));
            });
        };
        fila(ui, ic::CROSSHAIR, format!("Está en x {:.0}, y {:.0}", o.x, o.y));
        fila(ui, ic::SQUARE, format!("Mide {:.0} de ancho y {:.0} de alto", o.w, o.h));
        if !anim.is_empty() {
            fila(ui, ic::FILM, format!("Animación «{anim}»"));
        }
        let plataformas = s.world.object_def(&o.obj).is_some_and(|d| d.behaviors.iter().any(|b| matches!(b, nicostation::model::Behavior::Platformer { .. })));
        if plataformas {
            fila(ui, ic::RUN, format!("Tocando el suelo: {} · Saltando: {}", si_no(o.on_floor), si_no(o.jumping)));
        }
        if !o.visible {
            fila(ui, ic::EYE_SLASH, "Ahora está escondido".into());
        }
        if o.opacity < 255. {
            fila(ui, ic::EYE, format!("Transparencia: {:.0} de 255", o.opacity));
        }
        if !o.vars.is_empty() {
            ui.add_space(4.);
            ui.label(RichText::new("Sus cajitas").font(sb(14.)));
            for (k, v) in &o.vars {
                caja_simple(ui, k, v);
            }
        }
    });
}

fn caja_simple(ui: &mut Ui, k: &str, v: &Value) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(ic::ARCHIVE_BOX).color(CAT_VAR));
        ui.label(RichText::new(k).size(15.));
        Frame::new().fill(CHIP).corner_radius(10).inner_margin(Margin::symmetric(8, 1)).show(ui, |ui| {
            ui.label(RichText::new(v.text()).size(15.).color(CHIP_TEXT));
        });
    });
}

/// Un valor editable que no se pisa mientras se escribe.
fn valor(ui: &mut Ui, editando: &mut HashMap<String, String>, clave: String, v: &mut Value) {
    let mut t = editando.get(&clave).cloned().unwrap_or_else(|| v.text());
    let r = ui.add(TextEdit::singleline(&mut t).desired_width(ui.available_width().clamp(60., 140.)).font(egui::TextStyle::Monospace));
    if r.changed() {
        *v = crate::parse_val(&t);
    }
    if r.has_focus() {
        editando.insert(clave, t);
    } else {
        editando.remove(&clave);
    }
}

fn tabla_vars(ui: &mut Ui, id: &str, vars: &mut Vars, editar: bool, editando: &mut HashMap<String, String>) {
    if vars.is_empty() {
        ui.label(muted("(ninguna)"));
        return;
    }
    egui::Grid::new(id).num_columns(2).spacing([10., 4.]).show(ui, |ui| {
        for (k, v) in vars.iter_mut() {
            ui.label(k);
            if editar {
                valor(ui, editando, format!("{id}/{k}"), v);
            } else {
                ui.label(RichText::new(v.text()).monospace());
            }
            ui.end_row();
        }
    });
}

/// Propiedades en vivo de la instancia elegida (modo experto: se pueden cambiar).
fn propiedades(ui: &mut Ui, s: &mut Sesion) {
    let Some(i) = s.elegida() else { return };
    let anim = s.world.get(i, "anim").text();
    let Sesion { world, editando, .. } = s;
    let o = &mut world.insts[i];
    ui.horizontal(|ui| {
        ui.label(RichText::new(ic::CROSSHAIR).color(ACCENT));
        ui.label(RichText::new(&o.obj).font(sb(15.)));
        ui.label(muted(format!("id {}", o.id)));
    });
    egui::Grid::new("dep_props").num_columns(2).spacing([10., 4.]).show(ui, |ui| {
        for (n, v, sp) in [("x", &mut o.x, 1.), ("y", &mut o.y, 1.), ("ángulo", &mut o.angle, 1.), ("z", &mut o.z, 0.1), ("opacidad", &mut o.opacity, 1.)] {
            lbl(ui, n);
            ui.add(DragValue::new(v).speed(sp).max_decimals(2));
            ui.end_row();
        }
        let solo = |ui: &mut Ui, n: &str, t: String| {
            lbl(ui, n);
            ui.label(RichText::new(t).monospace());
            ui.end_row();
        };
        lbl(ui, "visible");
        ui.checkbox(&mut o.visible, "");
        ui.end_row();
        solo(ui, "ancho × alto", format!("{:.1} × {:.1}", o.w, o.h));
        if o.tint != [255; 3] {
            solo(ui, "color", format!("#{:02x}{:02x}{:02x}", o.tint[0], o.tint[1], o.tint[2]));
        }
        solo(ui, "capa", if o.layer.is_empty() { "Base".into() } else { o.layer.clone() });
        if !anim.is_empty() || o.anim > 0 {
            solo(ui, "animación", format!("{} «{anim}» cuadro {}", o.anim, o.frame));
        }
        solo(ui, "en el suelo", si_no(o.on_floor).into());
        solo(ui, "saltando", si_no(o.jumping).into());
        if o.flip_x {
            solo(ui, "volteado", "sí".into());
        }
        if !o.text.is_empty() {
            solo(ui, "texto", o.text.clone());
        }
    });
    ui.add_space(4.);
    ui.label(RichText::new("Variables de la instancia").font(sb(13.)).color(TEXT_MUTED));
    tabla_vars(ui, "dep_ivars", &mut o.vars, true, editando);
}

fn lbl(ui: &mut Ui, t: &str) {
    ui.label(RichText::new(t).color(TEXT_MUTED));
}

/// Cajitas de la escena, globales y partida guardada.
fn cajitas(ui: &mut Ui, s: &mut Sesion, editar: bool) {
    let Sesion { world, editando, .. } = s;
    let secciones: [(&str, &str, &str, &mut Vars); 3] = if editar {
        [
            ("dep_scene", "Variables de la escena", "Vuelven a empezar al cambiar de escena.", &mut world.vars),
            ("dep_glob", "Variables globales", "Duran todo el juego.", &mut world.globals),
            ("dep_store", "Partida guardada", "Lo que se guarda con «guardar».", &mut world.storage),
        ]
    } else {
        [
            ("dep_scene", "Cajitas de esta escena", "", &mut world.vars),
            ("dep_glob", "Cajitas de todo el juego", "", &mut world.globals),
            ("dep_store", "Partida guardada", "", &mut world.storage),
        ]
    };
    for (id, t, sub, vars) in secciones {
        if !editar && id == "dep_store" && vars.is_empty() {
            continue;
        }
        ui.label(RichText::new(t).font(sb(if editar { 14. } else { 15. })));
        if !sub.is_empty() {
            ui.label(RichText::new(sub).small().color(TEXT_MUTED));
        }
        if editar {
            tabla_vars(ui, id, vars, true, editando);
        } else if vars.is_empty() {
            ui.label(muted("No hay cajitas."));
        } else {
            for (k, v) in vars.iter() {
                caja_simple(ui, k, v);
            }
        }
        ui.add_space(6.);
    }
}

fn tipo_icono(t: Tipo) -> (&'static str, Color32) {
    match t {
        Tipo::Escena => (ic::FLAG, CAT_SCENE),
        Tipo::Sonido => (ic::SPEAKER, CAT_AUDIO),
        Tipo::Guardado => (ic::FLOPPY, CAT_MOV),
        Tipo::Salir => (ic::DOOR_OPEN, DANGER),
        Tipo::Aviso => (ic::WARNING, WARN),
        Tipo::Info => (ic::CHECK, OK),
    }
}

fn registro(ui: &mut Ui, s: &mut Sesion, facil: bool) {
    ui.horizontal(|ui| {
        ui.label(muted(format!("{} anotaciones", s.registro.len())));
        if ui.add_enabled(!s.registro.is_empty(), egui::Button::new(it(ic::BROOM, "Limpiar"))).clicked() {
            s.registro.clear();
        }
    });
    if s.registro.is_empty() {
        ui.label(muted(if facil { "Todavía no pasó nada especial." } else { "Sin anotaciones." }));
        return;
    }
    for l in s.registro.iter().rev().take(if facil { 30 } else { 300 }) {
        let (icon, c) = tipo_icono(l.tipo);
        ui.horizontal_top(|ui| {
            ui.label(RichText::new(format!("{:>6.1} s", l.t)).monospace().color(TEXT_FAINT)).on_hover_text("Tiempo de la escena (world.time)");
            ui.label(RichText::new(icon).color(c));
            let t = if facil && l.tipo == Tipo::Aviso { format!("Algo no funcionó: {}", l.texto) } else { l.texto.clone() };
            ui.add(egui::Label::new(RichText::new(t).color(if l.tipo == Tipo::Aviso { WARN } else { TEXT })).wrap());
        });
    }
}

/// Milisegundos en la unidad más legible (µs si es muy poco).
fn ms_txt(ms: f32) -> String {
    if ms < 0.1 { format!("{:.0} µs", ms * 1000.) } else { format!("{ms:.2} ms") }
}

/// Tabla del perfilador: qué se lleva el tiempo de cada paso (eventos de primer nivel y comportamientos).
fn perfilador(ui: &mut Ui, s: &mut Sesion) {
    let ult = s.pasos_ms.back().copied().unwrap_or(0.);
    let prom = if s.pasos_ms.is_empty() { 0. } else { s.pasos_ms.iter().sum::<f32>() / s.pasos_ms.len() as f32 };
    let medido: f32 = s.perfil.iter().map(|e| e.1).sum();
    egui::Grid::new("perf_tot").num_columns(2).spacing([12., 4.]).show(ui, |ui| {
        for (n, t) in [
            ("Paso completo (último)", ms_txt(ult)),
            ("Paso completo (promedio)", ms_txt(prom)),
            ("Medido por partes", ms_txt(medido)),
            ("Resto del motor", ms_txt((prom - medido).max(0.))),
        ] {
            lbl(ui, n);
            ui.label(RichText::new(t).monospace());
            ui.end_row();
        }
    });
    let presupuesto = 1000. / 60.;
    let uso = prom / presupuesto;
    ui.add(egui::ProgressBar::new(uso.min(1.)).text(format!("{:.1} % de un cuadro de 60 fps", uso * 100.)).fill(if uso > 0.8 { DANGER } else if uso > 0.5 { WARN } else { OK }));
    ui.add_space(8.);
    if s.perfil.is_empty() {
        ui.label(muted("El motor todavía no mandó tiempos (se miden mientras el juego corre)."));
        return;
    }
    let mut filas: Vec<&(String, f32, f32, f32)> = s.perfil.iter().collect();
    filas.sort_by(|a, b| b.1.total_cmp(&a.1));
    let tope = filas.first().map_or(1., |e| e.1.max(e.2)).max(0.001);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Qué").small().color(TEXT_MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new("promedio / máximo").small().color(TEXT_MUTED));
        });
    });
    for (k, (n, avg, max, _)) in filas.into_iter().enumerate() {
        let (r, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 22.), Sense::hover());
        let p = ui.painter();
        if k % 2 == 0 {
            p.rect_filled(r, 3, SURFACE);
        }
        let barra = Rect::from_min_size(r.min, Vec2::new(r.width() * (avg / tope).clamp(0., 1.), r.height()));
        p.rect_filled(barra, 3, ACCENT.gamma_multiply(0.28));
        let pico = r.left() + r.width() * (max / tope).clamp(0., 1.);
        p.line_segment([Pos2::new(pico, r.top() + 3.), Pos2::new(pico, r.bottom() - 3.)], Stroke::new(1.5, ACCENT));
        let t = p.layout_no_wrap(n.clone(), FontId::proportional(13.), TEXT);
        let tw = r.width() - 110.;
        let t = if t.size().x > tw { p.layout(n.clone(), FontId::proportional(12.), TEXT, tw) } else { t };
        p.galley(Pos2::new(r.left() + 6., r.center().y - t.size().y / 2.), t, TEXT);
        p.text(r.right_center() - Vec2::new(6., 0.), Align2::RIGHT_CENTER, format!("{} / {}", ms_txt(*avg), ms_txt(*max)), FontId::monospace(12.), TEXT_SOFT);
        resp.on_hover_text(format!("{n}\npromedio {avg:.4} ms · máximo reciente {max:.4} ms\n{:.1} % del paso", if prom > 0. { avg / prom * 100. } else { 0. }));
    }
    ui.add_space(6.);
    ui.label(RichText::new("La barra es el promedio; la rayita, el máximo reciente.").small().color(TEXT_MUTED));
}

fn rendimiento(ui: &mut Ui, s: &mut Sesion) {
    let ms: Vec<f32> = s.pasos_ms.iter().copied().collect();
    let ult = ms.last().copied().unwrap_or(0.);
    let prom = if ms.is_empty() { 0. } else { ms.iter().sum::<f32>() / ms.len() as f32 };
    let max = ms.iter().copied().fold(0., f32::max);
    egui::Grid::new("dep_perf").num_columns(2).spacing([12., 4.]).show(ui, |ui| {
        for (n, t) in [
            ("Cuadros por segundo", format!("{:.0}", s.fps)),
            ("Paso del motor", format!("{ult:.2} ms")),
            ("Promedio / máximo", format!("{prom:.2} / {max:.2} ms")),
            ("Instancias", format!("{}", s.world.insts.len())),
            ("Pasos dados", format!("{}", s.pasos)),
            ("Tiempo de la escena", format!("{:.1} s", s.world.time)),
        ] {
            lbl(ui, n);
            ui.label(RichText::new(t).monospace());
            ui.end_row();
        }
    });
    ui.add_space(6.);
    ui.label(RichText::new(format!("Tiempo de cada paso (últimos {HISTORIA})")).small().color(TEXT_MUTED));
    let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 56.), Sense::hover());
    let p = ui.painter();
    p.rect_filled(r, 4, BG);
    p.rect_stroke(r, 4, Stroke::new(1., BORDER), egui::StrokeKind::Inside);
    // La línea de 16.7 ms es todo el tiempo que hay en un cuadro de 60 por segundo.
    let tope = max.max(2.);
    let presupuesto = (1000. / 60.) as f32;
    if presupuesto <= tope {
        let y = r.bottom() - presupuesto / tope * r.height();
        p.line_segment([Pos2::new(r.left(), y), Pos2::new(r.right(), y)], Stroke::new(1., DANGER.gamma_multiply(0.6)));
    }
    if ms.len() >= 2 {
        let pts: Vec<Pos2> = ms.iter().enumerate()
            .map(|(k, v)| Pos2::new(r.left() + k as f32 / (HISTORIA - 1) as f32 * r.width(), r.bottom() - 2. - v / tope * (r.height() - 4.)))
            .collect();
        p.add(egui::Shape::line(pts, Stroke::new(1.5, ACCENT)));
    }
    p.text(r.right_top() + Vec2::new(-4., 3.), Align2::RIGHT_TOP, format!("{tope:.1} ms"), FontId::proportional(11.), TEXT_FAINT);
    ui.add_space(10.);
    ui.label(RichText::new("Cámara").font(sb(14.)));
    ui.checkbox(&mut s.ver_todo, "Ver toda la escena").on_hover_text("Aleja la vista para mostrar todo el nivel; el rectángulo azul es lo que ve la cámara del juego");
    let w = &s.world;
    ui.label(RichText::new(format!("centro ({:.0}, {:.0}) · zoom {:.2}", w.camera.0, w.camera.1, w.zoom)).monospace().color(TEXT_MUTED));
}
