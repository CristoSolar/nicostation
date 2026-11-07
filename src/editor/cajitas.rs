//! Cajitas (variables) en el modo fácil: tarjetas con tipo, valor y dónde se usan.

use super::facil::{self, has_ident, ident_fix, ident_ok};
use super::iconos as ic;
use super::tema::*;
use eframe::egui::{self, Align, Button, CornerRadius, DragValue, FontId, Frame, Key, Layout, Margin, Rect, RichText, TextEdit, Ui, Vec2};
use nicostation::catalog::{self, P};
use nicostation::model::*;

// ---------- uso y renombrado ----------

/// ¿La instrucción usa la cajita `name` (como cajita o dentro de una cuenta)?
pub fn usa_var(i: &Instr, name: &str) -> bool {
    let ps = catalog::meta(&i.op).map_or(&[][..], |m| m.params);
    i.args.iter().enumerate().any(|(k, a)| match ps.get(k).map(|p| p.1) {
        Some(P::Var) => a == name,
        Some(P::Expr) | None => has_ident(a, name),
        _ => false,
    }) || i.sub.iter().any(|s| usa_var(s, name))
}

/// ¿La instrucción usa la cajita `var` del objeto `obj`?
pub fn usa_obj_var(i: &Instr, obj: &str, var: &str) -> bool {
    let ps = catalog::meta(&i.op).map_or(&[][..], |m| m.params);
    let dotted = format!("{obj}.{var}");
    i.args.iter().enumerate().any(|(k, a)| match ps.get(k).map(|p| p.1) {
        Some(P::Prop) => a == var && i.args.first().is_some_and(|o| o == obj),
        Some(P::Expr) | None => a.contains(&dotted),
        _ => false,
    }) || i.sub.iter().any(|s| usa_obj_var(s, obj, var))
}

pub fn contar(evs: &[Event], f: &dyn Fn(&Instr) -> bool) -> usize {
    evs.iter().map(|e| e.conds.iter().chain(&e.acts).filter(|i| f(i)).count() + contar(&e.sub, f)).sum()
}

/// Veces que se usa una cajita: la de una escena (en esa escena y en Mis bloques) o la del juego (en todas partes).
pub fn usos(p: &Project, scene: Option<usize>, name: &str) -> usize {
    let f = |i: &Instr| usa_var(i, name);
    let sc: usize = p.scenes.iter().enumerate().filter(|(k, _)| scene.is_none_or(|s| s == *k)).map(|(_, s)| contar(&s.events, &f)).sum();
    sc + p.functions.iter().map(|x| contar(&x.events, &f)).sum::<usize>()
}

/// Reemplaza el identificador `old` por `new` (no dentro de comillas ni después de un punto).
pub fn ren_ident(src: &str, old: &str, new: &str) -> String {
    let b = src.as_bytes();
    let (mut i, mut q, mut out) = (0, false, String::with_capacity(src.len()));
    while i < b.len() {
        let c = b[i];
        if c == b'"' {
            q = !q;
        }
        if !q && (c.is_ascii_alphabetic() || c == b'_') {
            let st = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            let w = &src[st..i];
            out.push_str(if w == old && (st == 0 || b[st - 1] != b'.') { new } else { w });
            continue;
        }
        let ch = src[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn ren_ins(l: &mut [Instr], old: &str, new: &str) {
    for i in l {
        let ps = catalog::meta(&i.op).map_or(&[][..], |m| m.params);
        for (k, a) in i.args.iter_mut().enumerate() {
            match ps.get(k).map(|p| p.1) {
                Some(P::Var) if a == old => *a = new.into(),
                Some(P::Expr) | None => *a = ren_ident(a, old, new),
                _ => {}
            }
        }
        ren_ins(&mut i.sub, old, new);
    }
}

pub fn ren_var(evs: &mut [Event], old: &str, new: &str) {
    for e in evs {
        if e.kind == EventKind::Repeat {
            e.text = ren_ident(&e.text, old, new);
        }
        ren_ins(&mut e.conds, old, new);
        ren_ins(&mut e.acts, old, new);
        ren_var(&mut e.sub, old, new);
    }
}

// ---------- tarjetas ----------

#[derive(Clone, Copy, PartialEq)]
pub enum Tipo {
    Num,
    Txt,
    SiNo,
}

pub fn tipo(v: &Value) -> Tipo {
    match v {
        Value::Num(_) => Tipo::Num,
        Value::Str(_) => Tipo::Txt,
        Value::Bool(_) => Tipo::SiNo,
    }
}

fn convertir(v: &Value, t: Tipo) -> Value {
    match t {
        Tipo::Num => Value::Num(v.num()),
        Tipo::Txt => Value::Str(match v {
            Value::Bool(b) => if *b { "sí".into() } else { "no".into() },
            _ => v.text(),
        }),
        Tipo::SiNo => Value::Bool(v.truthy()),
    }
}

fn tipo_icon(t: Tipo) -> &'static str {
    match t {
        Tipo::Num => ic::HASH,
        Tipo::Txt => ic::TEXT_T,
        Tipo::SiNo => ic::TOGGLE_RIGHT,
    }
}

const TIPOS: [(Tipo, &str); 3] = [(Tipo::Num, "Número"), (Tipo::Txt, "Texto"), (Tipo::SiNo, "Sí / no")];

/// Editor del valor según su tipo.
fn valor(ui: &mut Ui, v: &mut Value) {
    match v {
        Value::Num(n) => {
            if ui.add(Button::new(RichText::new(ic::MINUS).size(16.)).min_size(Vec2::splat(32.))).on_hover_text("Restar 1").clicked() {
                *n -= 1.;
            }
            ui.add(DragValue::new(n).speed(0.5).custom_formatter(|x, _| Value::Num(x).text()).min_decimals(0)).on_hover_text("Arrastra o escribe un número");
            if ui.add(Button::new(RichText::new(ic::PLUS).size(16.)).min_size(Vec2::splat(32.))).on_hover_text("Sumar 1").clicked() {
                *n += 1.;
            }
        }
        Value::Str(s) => drop(ui.add(TextEdit::singleline(s).desired_width(180.).font(FontId::proportional(15.)).hint_text("texto"))),
        Value::Bool(b) => {
            let (icon, t, c) = if *b { (ic::TOGGLE_RIGHT, "sí", OK) } else { (ic::TOGGLE_LEFT, "no", TEXT_MUTED) };
            if ui.add(Button::new((RichText::new(icon).size(26.).color(c), RichText::new(t).font(sb(15.)))).frame_when_inactive(false).min_size(Vec2::new(0., 32.))).clicked() {
                *b = !*b;
            }
        }
    }
}

pub enum CardAct {
    Borrar,
    Renombrar(String),
}

/// Tarjeta de una cajita. `editable`: se puede cambiar el nombre.
pub fn tarjeta(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, name: &str, v: &mut Value, usos: usize, editable: bool, taken: &dyn Fn(&str) -> bool) -> Option<CardAct> {
    let mut act = None;
    let t = tipo(v);
    ui.push_id(id, |ui| {
        let r = Frame::new().fill(tint(CAT_VAR)).corner_radius(10).inner_margin(Margin { left: 16, right: 8, top: 8, bottom: 8 }).show(ui, |ui| {
            ui.set_width(ui.available_width());
            // Ancho: valor en la misma fila; angosto: el valor baja a una segunda fila.
            let wide = ui.available_width() >= 560.;
            let mut renombrar = None;
            let mut nombre = |ui: &mut Ui| {
                ui.label(RichText::new(tipo_icon(t)).size(22.).color(CAT_VAR)).on_hover_text(TIPOS.iter().find(|x| x.0 == t).map_or("", |x| x.1));
                if editable {
                    let bid = ui.make_persistent_id("buf");
                    let mut buf: String = ui.data_mut(|d| d.get_temp(bid)).unwrap_or_else(|| name.to_string());
                    let w = (ui.available_width() - 40.).clamp(80., 150.);
                    let r = ui.add(TextEdit::singleline(&mut buf).font(sb(16.)).desired_width(w)).on_hover_text("Nombre de la cajita");
                    if r.lost_focus() {
                        let fixed = buf.trim().to_string();
                        if fixed != name && ident_ok(&fixed) && !taken(&fixed) {
                            renombrar = Some(fixed);
                        }
                        ui.data_mut(|d| d.remove::<String>(bid));
                    } else if r.has_focus() {
                        ui.data_mut(|d| d.insert_temp(bid, buf.clone()));
                    }
                    if r.has_focus() && buf.trim() != name && (!ident_ok(buf.trim()) || taken(buf.trim())) {
                        ui.add(egui::Label::new(RichText::new(if taken(buf.trim()) { "ya existe" } else { "solo letras, números y _" }).color(facil::ROJO_TXT)).truncate());
                    }
                } else {
                    ui.add(egui::Label::new(RichText::new(name).font(sb(16.))).truncate());
                }
            };
            let fila_valor = |ui: &mut Ui, v: &mut Value| {
                ui.label(RichText::new("vale").color(TEXT_SOFT));
                valor(ui, v);
            };
            ui.horizontal(|ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.add(Button::new(RichText::new(ic::TRASH).size(17.).color(DANGER)).frame_when_inactive(false).min_size(Vec2::splat(32.))).on_hover_text("Borrar cajita").clicked() {
                        act = Some(CardAct::Borrar);
                    }
                    let tt = TIPOS.iter().find(|x| x.0 == t).map_or("", |x| x.1);
                    ui.menu_button(RichText::new(format!("{tt} {}", ic::CARET_DOWN)).color(TEXT_SOFT), |ui| {
                        menu_spacing(ui);
                        ui.label(RichText::new("¿Qué guarda esta cajita?").color(TEXT_MUTED));
                        for (k, n) in TIPOS {
                            if ui.add(Button::selectable(t == k, (RichText::new(tipo_icon(k)).color(CAT_VAR), RichText::new(n))).min_size(Vec2::new(0., 30.))).clicked() {
                                *v = convertir(v, k);
                                ui.close();
                            }
                        }
                    }).response.on_hover_text("Cambiar el tipo");
                    ui.add_space(8.);
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        nombre(ui);
                        if wide {
                            fila_valor(ui, v);
                        }
                    });
                });
            });
            if !wide {
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(30.);
                    fila_valor(ui, v);
                });
            }
            if let Some(n) = renombrar {
                act = Some(CardAct::Renombrar(n));
            }
            let u = match usos {
                0 => "Todavía no se usa en ningún bloque.".to_string(),
                1 => "Se usa en 1 bloque.".into(),
                n => format!("Se usa en {n} bloques."),
            };
            ui.label(RichText::new(u).size(15.).color(TEXT_SOFT));
        });
        let rr = r.response.rect;
        ui.painter().rect_filled(Rect::from_min_size(rr.min, Vec2::new(5., rr.height())), CornerRadius { nw: 10, sw: 10, ne: 0, se: 0 }, CAT_VAR);
    });
    act
}

// ---------- pestaña ----------

pub struct Nueva {
    pub global: bool,
    pub name: String,
    pub tipo: Tipo,
    pub valor: String,
    /// Poner el cursor en el nombre al abrir.
    pub foco: bool,
}

pub enum Accion {
    Borrar { global: bool, name: String },
    Renombrar { global: bool, old: String, new: String },
}

/// Formulario de cajita nueva. Devuelve (nombre, valor) al crear.
fn formulario(ui: &mut Ui, n: &mut Nueva, taken: &dyn Fn(&str) -> bool) -> (Option<(String, Value)>, bool) {
    let (mut crear, mut cancel) = (None, false);
    Frame::new().fill(PANEL).stroke((1., CAT_VAR)).corner_radius(10).inner_margin(12).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new("Nueva cajita").font(sb(16.)));
        egui::Grid::new("nueva").num_columns(2).spacing([12., 8.]).show(ui, |ui| {
            ui.label(RichText::new("Nombre").color(TEXT_SOFT));
            let r = ui.add(TextEdit::singleline(&mut n.name).hint_text("por ejemplo: vidas").desired_width(200.).font(FontId::proportional(15.)));
            if std::mem::take(&mut n.foco) {
                r.request_focus();
            }
            ui.end_row();
            ui.label(RichText::new("Guarda un").color(TEXT_SOFT));
            ui.horizontal(|ui| {
                for (k, t) in TIPOS {
                    if ui.add(Button::selectable(n.tipo == k, (RichText::new(tipo_icon(k)).color(CAT_VAR), RichText::new(t))).min_size(Vec2::new(0., 32.))).clicked() {
                        n.tipo = k;
                        n.valor = match k {
                            Tipo::Num => "0".into(),
                            Tipo::Txt => String::new(),
                            Tipo::SiNo => "no".into(),
                        };
                    }
                }
            });
            ui.end_row();
            ui.label(RichText::new("Empieza en").color(TEXT_SOFT));
            match n.tipo {
                Tipo::SiNo => drop(ui.horizontal(|ui| {
                    for t in ["sí", "no"] {
                        if ui.add(Button::selectable(n.valor == t, t).min_size(Vec2::new(60., 32.))).clicked() {
                            n.valor = t.into();
                        }
                    }
                })),
                _ => drop(ui.add(TextEdit::singleline(&mut n.valor).desired_width(200.).font(FontId::proportional(15.)))),
            }
            ui.end_row();
        });
        let nm = n.name.trim().to_string();
        let nm = nm.as_str();
        let ok = ident_ok(nm) && !taken(nm);
        if !nm.is_empty() && !ok {
            ui.horizontal_wrapped(|ui| {
                if taken(nm) {
                    ui.label(RichText::new("Ya hay una cajita con ese nombre.").color(facil::ROJO_TXT));
                } else {
                    ui.label(RichText::new("Usa solo letras sin acento, números y _ (sin espacios).").color(facil::ROJO_TXT));
                    let fix = ident_fix(nm);
                    if !fix.is_empty() && !taken(&fix) && ui.button(format!("Usar «{fix}»")).clicked() {
                        n.name = fix;
                    }
                }
            });
        }
        let num_ok = n.tipo != Tipo::Num || n.valor.trim().parse::<f64>().is_ok();
        if !num_ok {
            ui.label(RichText::new("El valor tiene que ser un número.").color(facil::ROJO_TXT));
        }
        ui.horizontal(|ui| {
            let enter = ui.input(|i| i.key_pressed(Key::Enter));
            if ui.add_enabled_ui(ok && num_ok, |ui| primary(ui, (on_accent(ic::CHECK), on_accent("Crear cajita").font(sb(15.))))).inner.clicked() || (enter && ok && num_ok) {
                let v = match n.tipo {
                    Tipo::Num => Value::Num(n.valor.trim().parse().unwrap_or(0.)),
                    Tipo::Txt => Value::Str(n.valor.clone()),
                    Tipo::SiNo => Value::Bool(n.valor == "sí"),
                };
                crear = Some((nm.to_string(), v));
            }
            if ui.button("Cancelar").clicked() {
                cancel = true;
            }
        });
    });
    (crear, cancel)
}

fn seccion(ui: &mut Ui, p: &mut Project, sc: usize, global: bool, nueva: &mut Option<Nueva>, act: &mut Option<Accion>) {
    let (icon, t, sub) = if global {
        (ic::GLOBE, "Cajitas de todo el juego", "No se borran al cambiar de escena. Sirven para el récord o las vidas que pasan de nivel a nivel.")
    } else {
        (ic::ARCHIVE_BOX, "Cajitas de esta escena", "Vuelven a su valor inicial cada vez que empieza la escena.")
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(icon).size(22.).color(CAT_VAR));
        ui.label(RichText::new(t).font(sb(19.)));
        if !global {
            ui.label(RichText::new(format!("«{}»", p.scenes[sc].name)).color(TEXT_MUTED).size(15.));
        }
    });
    ui.label(RichText::new(sub).color(TEXT_SOFT));
    ui.add_space(6.);
    let names: Vec<String> = if global { p.globals.keys().cloned().collect() } else { p.scenes[sc].vars.keys().cloned().collect() };
    let usos_list: Vec<usize> = names.iter().map(|n| usos(p, if global { None } else { Some(sc) }, n)).collect();
    let taken_list = names.clone();
    let taken = |n: &str| taken_list.iter().any(|x| x == n);
    if names.is_empty() {
        ui.label(RichText::new("Todavía no hay cajitas aquí.").color(TEXT_MUTED).size(15.));
    }
    let vars = if global { &mut p.globals } else { &mut p.scenes[sc].vars };
    for (k, (name, v)) in vars.iter_mut().enumerate() {
        match tarjeta(ui, (global, k), name, v, usos_list[k], true, &taken) {
            Some(CardAct::Borrar) => *act = Some(Accion::Borrar { global, name: name.clone() }),
            Some(CardAct::Renombrar(n)) => *act = Some(Accion::Renombrar { global, old: name.clone(), new: n }),
            None => {}
        }
        ui.add_space(2.);
    }
    ui.add_space(4.);
    match nueva {
        Some(n) if n.global == global => {
            let (crear, cancel) = formulario(ui, n, &taken);
            if let Some((name, v)) = crear {
                vars.insert(name, v);
                *nueva = None;
            } else if cancel {
                *nueva = None;
            }
        }
        _ => {
            let b = Button::new((RichText::new(ic::PLUS).size(17.).color(CAT_VAR), RichText::new("Nueva cajita").font(sb(15.))))
                .stroke((1., CAT_VAR)).min_size(Vec2::new(0., 36.));
            if ui.add(b).clicked() {
                *nueva = Some(Nueva { global, name: String::new(), tipo: Tipo::Num, valor: "0".into(), foco: true });
            }
        }
    }
}

pub fn pestana(ui: &mut Ui, p: &mut Project, sc: usize, nueva: &mut Option<Nueva>) -> Option<Accion> {
    let mut act = None;
    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(ic::ARCHIVE_BOX).size(30.).color(CAT_VAR));
            ui.vertical(|ui| {
                ui.label(RichText::new("Cajitas").font(sb(24.)));
                ui.label(RichText::new("Una cajita guarda algo para que el juego lo recuerde: puntos, vidas, el nombre del jugador.").size(16.).color(TEXT_SOFT));
            });
        });
        ui.add_space(12.);
        let wide = ui.available_width() > 980.;
        if wide {
            ui.columns(2, |c| {
                c[0].add_space(0.);
                seccion(&mut c[0], p, sc, false, nueva, &mut act);
                seccion(&mut c[1], p, sc, true, nueva, &mut act);
            });
        } else {
            seccion(ui, p, sc, false, nueva, &mut act);
            ui.add_space(18.);
            seccion(ui, p, sc, true, nueva, &mut act);
        }
        ui.add_space(24.);
    });
    act
}

/// Cajitas de un objeto (panel de propiedades) con el mismo estilo de tarjeta.
pub fn del_objeto(ui: &mut Ui, obj: &str, vars: &mut Vars, usos: &dyn Fn(&str) -> usize, nb: &mut String) {
    let mut del = None;
    if vars.is_empty() {
        ui.label(muted("Sin cajitas. Una cajita del objeto guarda algo de cada copia, por ejemplo su dirección."));
    }
    for (k, (name, v)) in vars.iter_mut().enumerate() {
        if let Some(CardAct::Borrar) = tarjeta(ui, ("ov", obj, k), name, v, usos(name), false, &|_| true) {
            del = Some(name.clone());
        }
        ui.add_space(2.);
    }
    if let Some(k) = del {
        vars.remove(&k);
    }
    ui.horizontal(|ui| {
        let r = ui.add(TextEdit::singleline(nb).hint_text("nueva cajita").desired_width(140.));
        let enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        let ok = ident_ok(nb.trim()) && !vars.contains_key(nb.trim());
        if (ui.add_enabled(ok, Button::new(it(ic::PLUS, "Agregar"))).clicked() || (enter && ok)) && ok {
            vars.insert(std::mem::take(nb).trim().to_string(), Value::Num(0.));
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn renombra_identificadores() {
        assert_eq!(super::ren_ident("puntos + Heroe.puntos + \"puntos\" + puntos2", "puntos", "pts"), "pts + Heroe.puntos + \"puntos\" + puntos2");
    }
}
