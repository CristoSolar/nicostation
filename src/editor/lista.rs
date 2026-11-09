//! Editor amable de listas y estructuras (cajitas con varias cosas adentro): filas con tipo, valor,
//! subir/bajar, quitar y agregar; un nivel de listas o estructuras dentro de otra. En modo experto
//! también se puede ver y escribir como JSON.

use super::iconos as ic;
use super::tema::*;
use eframe::egui::{self, Button, DragValue, FontId, Frame, Margin, RichText, TextEdit, Ui, Vec2};
use nicostation::model::Value;
use std::collections::BTreeMap;

// ---------- operaciones puras ----------

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tipo {
    Num,
    Txt,
    SiNo,
    Lista,
    Estructura,
}

pub const TIPOS: [(Tipo, &str); 5] = [(Tipo::Num, "Número"), (Tipo::Txt, "Texto"), (Tipo::SiNo, "Sí / no"), (Tipo::Lista, "Lista"), (Tipo::Estructura, "Estructura")];

pub fn tipo(v: &Value) -> Tipo {
    match v {
        Value::Num(_) => Tipo::Num,
        Value::Str(_) => Tipo::Txt,
        Value::Bool(_) => Tipo::SiNo,
        Value::List(_) => Tipo::Lista,
        Value::Map(_) => Tipo::Estructura,
    }
}

pub fn icono(t: Tipo) -> &'static str {
    match t {
        Tipo::Num => ic::HASH,
        Tipo::Txt => ic::TEXT_T,
        Tipo::SiNo => ic::TOGGLE_RIGHT,
        Tipo::Lista => ic::LIST,
        Tipo::Estructura => ic::BRACES,
    }
}

pub fn nombre(t: Tipo) -> &'static str {
    TIPOS.iter().find(|x| x.0 == t).map_or("", |x| x.1)
}

/// Valor vacío de un tipo.
pub fn vacio(t: Tipo) -> Value {
    match t {
        Tipo::Num => Value::Num(0.),
        Tipo::Txt => Value::Str(String::new()),
        Tipo::SiNo => Value::Bool(false),
        Tipo::Lista => Value::List(vec![]),
        Tipo::Estructura => Value::Map(BTreeMap::new()),
    }
}

/// Cambia el tipo conservando lo que se pueda.
pub fn convertir(v: &Value, t: Tipo) -> Value {
    match t {
        Tipo::Num => Value::Num(v.num()),
        Tipo::Txt => Value::Str(match v {
            Value::Bool(b) => if *b { "sí".into() } else { "no".into() },
            _ => v.text(),
        }),
        Tipo::SiNo => Value::Bool(v.truthy()),
        Tipo::Lista => match v {
            Value::List(_) => v.clone(),
            Value::Map(m) => Value::List(m.values().cloned().collect()),
            _ => Value::List(vec![v.clone()]),
        },
        Tipo::Estructura => match v {
            Value::Map(_) => v.clone(),
            Value::List(l) => Value::Map(l.iter().enumerate().map(|(k, x)| (k.to_string(), x.clone())).collect()),
            _ => Value::Map([("valor".to_string(), v.clone())].into()),
        },
    }
}

/// Cuántas cosas tiene.
pub fn largo(v: &Value) -> usize {
    match v {
        Value::List(l) => l.len(),
        Value::Map(m) => m.len(),
        _ => 0,
    }
}

/// Agrega un elemento vacío del tipo dado (en una estructura, con un nombre libre).
pub fn agregar(v: &mut Value, t: Tipo) {
    match v {
        Value::List(l) => l.push(vacio(t)),
        Value::Map(m) => {
            let k = clave_libre(m, "campo");
            m.insert(k, vacio(t));
        }
        _ => {}
    }
}

pub fn clave_libre(m: &BTreeMap<String, Value>, base: &str) -> String {
    (1..).map(|k| if k == 1 { base.to_string() } else { format!("{base}{k}") }).find(|k| !m.contains_key(k)).unwrap()
}

/// Quita el elemento `i` (en una estructura, el i-ésimo campo en orden).
pub fn quitar(v: &mut Value, i: usize) {
    match v {
        Value::List(l) if i < l.len() => drop(l.remove(i)),
        Value::Map(m) => {
            if let Some(k) = m.keys().nth(i).cloned() {
                m.remove(&k);
            }
        }
        _ => {}
    }
}

/// Mueve el elemento `i` de una lista una posición (-1 arriba, +1 abajo). Devuelve si se movió.
pub fn mover(v: &mut Value, i: usize, d: i32) -> bool {
    let Value::List(l) = v else { return false };
    let j = i as i64 + d as i64;
    if i >= l.len() || j < 0 || j as usize >= l.len() {
        return false;
    }
    l.swap(i, j as usize);
    true
}

/// Cambia el nombre de un campo de una estructura. No pisa otro campo ni acepta nombres vacíos.
pub fn renombrar(m: &mut BTreeMap<String, Value>, viejo: &str, nuevo: &str) -> bool {
    let nuevo = nuevo.trim();
    if nuevo.is_empty() || nuevo == viejo || m.contains_key(nuevo) {
        return false;
    }
    match m.remove(viejo) {
        Some(x) => {
            m.insert(nuevo.to_string(), x);
            true
        }
        None => false,
    }
}

// ---------- interfaz ----------

/// Valor en palabras (sí / no en vez de true / false).
pub fn amable(v: &Value) -> String {
    match v {
        Value::Bool(b) => if *b { "sí".into() } else { "no".into() },
        Value::List(l) => format!("[{}]", l.iter().map(amable).collect::<Vec<_>>().join(", ")),
        Value::Map(m) => format!("{{{}}}", m.iter().map(|(k, v)| format!("{k}: {}", amable(v))).collect::<Vec<_>>().join(", ")),
        _ => v.text(),
    }
}

/// Resumen corto: «3 cosas: 1, 2, 3».
pub fn resumen(v: &Value) -> String {
    let n = largo(v);
    let t = amable(v);
    let t = if t.chars().count() > 40 { format!("{}…", t.chars().take(39).collect::<String>()) } else { t };
    match (v, n) {
        (Value::List(_), 0) => "lista vacía".into(),
        (Value::Map(_), 0) => "estructura vacía".into(),
        (_, 1) => format!("1 cosa: {t}"),
        _ => format!("{n} cosas: {t}"),
    }
}

/// Editor de una lista o estructura. `json`: ofrece el botón para verla como JSON (modo experto).
/// Devuelve true si cambió algo.
pub fn editor(ui: &mut Ui, id: impl std::hash::Hash + std::fmt::Debug, v: &mut Value, json: bool) -> bool {
    let id = ui.make_persistent_id(id);
    let mut ch = false;
    let como_json = json && ui.data_mut(|d| *d.get_temp_mut_or(id.with("json"), false));
    Frame::new().fill(super::tema::BG).stroke((1., BORDER)).corner_radius(8).inner_margin(Margin::same(8)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        if json {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(Button::selectable(como_json, RichText::new("JSON").small())).on_hover_text("Ver y escribir como JSON").clicked() {
                        ui.data_mut(|d| d.insert_temp(id.with("json"), !como_json));
                    }
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.add(egui::Label::new(muted(resumen(v))).truncate());
                    });
                });
            });
        }
        if como_json {
            ch |= editor_json(ui, id, v);
        } else {
            ch |= filas(ui, id, v, 0);
        }
    });
    ch
}

fn editor_json(ui: &mut Ui, id: egui::Id, v: &mut Value) -> bool {
    let bid = id.with("buf");
    let mut buf: String = ui.data_mut(|d| d.get_temp(bid)).unwrap_or_else(|| serde_json::to_string_pretty(v).unwrap_or_default());
    let r = ui.add(TextEdit::multiline(&mut buf).desired_width(f32::INFINITY).desired_rows(3).font(FontId::monospace(13.)));
    let mut ch = false;
    match serde_json::from_str::<Value>(&buf) {
        Ok(nv @ (Value::List(_) | Value::Map(_))) if r.changed() => {
            *v = nv;
            ch = true;
        }
        Ok(Value::List(_) | Value::Map(_)) => {}
        _ => drop(ui.label(RichText::new(format!("{} JSON inválido: tiene que ser [ ... ] o {{ ... }}", ic::WARNING)).color(DANGER).small())),
    }
    if r.has_focus() {
        ui.data_mut(|d| d.insert_temp(bid, buf));
    } else {
        ui.data_mut(|d| d.remove::<String>(bid));
    }
    ch
}

/// Filas de una colección. `nivel` 0 permite listas/estructuras adentro (un nivel); en el 1 se muestran como resumen editable en JSON corto.
fn filas(ui: &mut Ui, id: egui::Id, v: &mut Value, nivel: usize) -> bool {
    let mut ch = false;
    let es_lista = matches!(v, Value::List(_));
    let n = largo(v);
    let mut accion: Option<(usize, i32)> = None; // (i, -1 subir, +1 bajar, 0 quitar)
    let mut ren: Option<(String, String)> = None;
    if n == 0 {
        ui.label(muted(if es_lista { "La lista está vacía. Agrega algo:" } else { "Sin campos. Agrega uno:" }));
    }
    let claves: Vec<String> = match v {
        Value::Map(m) => m.keys().cloned().collect(),
        _ => vec![],
    };
    for i in 0..n {
        let rid = id.with(("fila", nivel, i));
        ui.push_id(rid, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.;
                // Nombre o número.
                if es_lista {
                    ui.add_sized([26., 24.], egui::Label::new(RichText::new(format!("{}", i)).monospace().color(TEXT_MUTED)))
                        .on_hover_text(format!("Lugar {i} (se cuenta desde 0)"));
                } else {
                    let k = &claves[i];
                    let bid = rid.with("clave");
                    let mut buf: String = ui.data_mut(|d| d.get_temp(bid)).unwrap_or_else(|| k.clone());
                    let r = ui.add(TextEdit::singleline(&mut buf).desired_width(80.).font(FontId::proportional(14.))).on_hover_text("Nombre del campo");
                    if r.has_focus() {
                        ui.data_mut(|d| d.insert_temp(bid, buf.clone()));
                    } else if r.lost_focus() {
                        ui.data_mut(|d| d.remove::<String>(bid));
                        if buf.trim() != k {
                            ren = Some((k.clone(), buf.trim().to_string()));
                        }
                    }
                }
                let item = match v {
                    Value::List(l) => &mut l[i],
                    Value::Map(m) => m.get_mut(&claves[i]).unwrap(),
                    _ => unreachable!(),
                };
                // Tipo.
                let t = tipo(item);
                ui.menu_button(RichText::new(icono(t)).color(CAT_VAR), |ui| {
                    menu_spacing(ui);
                    for (k, nm) in TIPOS {
                        if nivel > 0 && matches!(k, Tipo::Lista | Tipo::Estructura) {
                            continue;
                        }
                        if ui.add(Button::selectable(t == k, (RichText::new(icono(k)).color(CAT_VAR), RichText::new(nm)))).clicked() {
                            *item = convertir(item, k);
                            ch = true;
                            ui.close();
                        }
                    }
                }).response.on_hover_text(format!("{}: cambiar el tipo", nombre(t)));
                // Valor.
                ch |= valor_simple(ui, item, nivel);
                // Botones a la derecha.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ib(ui, ic::X, "Quitar").clicked() {
                        accion = Some((i, 0));
                    }
                    if es_lista {
                        if ib_on(ui, i + 1 < n, ic::CARET_DOWN, "Bajar").clicked() {
                            accion = Some((i, 1));
                        }
                        if ib_on(ui, i > 0, ic::CARET_UP, "Subir").clicked() {
                            accion = Some((i, -1));
                        }
                    }
                });
            });
            // Un nivel adentro.
            if nivel == 0 {
                let item = match v {
                    Value::List(l) => &mut l[i],
                    Value::Map(m) => m.get_mut(&claves[i]).unwrap(),
                    _ => unreachable!(),
                };
                if matches!(item, Value::List(_) | Value::Map(_)) {
                    ui.horizontal(|ui| {
                        ui.add_space(30.);
                        ui.vertical(|ui| {
                            Frame::new().stroke((1., BORDER)).corner_radius(6).inner_margin(Margin::same(6)).show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ch |= filas(ui, rid, item, 1);
                            });
                        });
                    });
                }
            }
        });
    }
    match accion {
        Some((i, 0)) => {
            quitar(v, i);
            ch = true;
        }
        Some((i, d)) => ch |= mover(v, i, d),
        None => {}
    }
    if let (Some((a, b)), Value::Map(m)) = (ren, &mut *v) {
        ch |= renombrar(m, &a, &b);
    }
    // Agregar.
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.;
        let b = Button::new((RichText::new(ic::PLUS).color(CAT_VAR), RichText::new(if es_lista { "Agregar" } else { "Campo" })));
        if ui.add(b).on_hover_text(if es_lista { "Agregar otro del mismo tipo que el último" } else { "Agregar un campo con un número" }).clicked() {
            let t = match v {
                // Igual que el último, así una lista de números sigue siendo de números.
                Value::List(l) => l.last().map_or(Tipo::Num, tipo),
                _ => Tipo::Num,
            };
            agregar(v, if nivel > 0 && matches!(t, Tipo::Lista | Tipo::Estructura) { Tipo::Num } else { t });
            ch = true;
        }
        ui.menu_button(RichText::new(ic::CARET_DOWN).color(TEXT_MUTED), |ui| {
            menu_spacing(ui);
            ui.label(muted("Agregar un..."));
            for (k, nm) in TIPOS {
                if nivel > 0 && matches!(k, Tipo::Lista | Tipo::Estructura) {
                    continue;
                }
                if ui.button((RichText::new(icono(k)).color(CAT_VAR), RichText::new(nm))).clicked() {
                    agregar(v, k);
                    ch = true;
                    ui.close();
                }
            }
        }).response.on_hover_text("Elegir qué agregar");
    });
    ch
}

fn valor_simple(ui: &mut Ui, v: &mut Value, nivel: usize) -> bool {
    match v {
        Value::Num(n) => ui.add(DragValue::new(n).speed(0.5).custom_formatter(|x, _| Value::Num(x).text()).min_decimals(0)).changed(),
        Value::Str(s) => {
            let w = (ui.available_width() - 100.).clamp(50., 140.);
            ui.add(TextEdit::singleline(s).desired_width(w).hint_text("texto")).changed()
        }
        Value::Bool(b) => {
            let (icon, t, c) = if *b { (ic::TOGGLE_RIGHT, "sí", OK) } else { (ic::TOGGLE_LEFT, "no", TEXT_MUTED) };
            let r = ui.add(Button::new((RichText::new(icon).size(20.).color(c), RichText::new(t))).frame_when_inactive(false).min_size(Vec2::new(0., 24.)));
            if r.clicked() {
                *b = !*b;
            }
            r.clicked()
        }
        Value::List(_) | Value::Map(_) => {
            ui.label(muted(resumen(v)));
            let _ = nivel;
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operaciones_de_lista() {
        let mut v = Value::List(vec![]);
        agregar(&mut v, Tipo::Num);
        agregar(&mut v, Tipo::Txt);
        agregar(&mut v, Tipo::Lista);
        assert_eq!(largo(&v), 3);
        assert_eq!(v, Value::List(vec![Value::Num(0.), Value::Str(String::new()), Value::List(vec![])]));
        assert!(mover(&mut v, 0, 1));
        assert_eq!(tipo(&match &v { Value::List(l) => l[1].clone(), _ => panic!() }), Tipo::Num);
        assert!(!mover(&mut v, 0, -1));
        assert!(!mover(&mut v, 2, 1));
        quitar(&mut v, 2);
        assert_eq!(largo(&v), 2);
        quitar(&mut v, 9);
        assert_eq!(largo(&v), 2);
    }

    #[test]
    fn operaciones_de_estructura() {
        let mut v = Value::Map(BTreeMap::new());
        agregar(&mut v, Tipo::Num);
        agregar(&mut v, Tipo::SiNo);
        let Value::Map(m) = &mut v else { panic!() };
        assert_eq!(m.keys().cloned().collect::<Vec<_>>(), vec!["campo", "campo2"]);
        assert!(renombrar(m, "campo", "vida"));
        assert!(!renombrar(m, "vida", "campo2"));
        assert!(!renombrar(m, "vida", "  "));
        assert!(m.contains_key("vida"));
        quitar(&mut v, 0);
        let Value::Map(m) = &v else { panic!() };
        assert_eq!(m.keys().cloned().collect::<Vec<_>>(), vec!["vida"]);
        // Mover no hace nada en una estructura (va por orden alfabético).
        assert!(!mover(&mut v, 0, 1));
    }

    #[test]
    fn convertir_tipos() {
        assert_eq!(convertir(&Value::Str("3".into()), Tipo::Num), Value::Num(3.));
        assert_eq!(convertir(&Value::Num(2.), Tipo::Lista), Value::List(vec![Value::Num(2.)]));
        let l = Value::List(vec![Value::Num(1.), Value::Num(2.)]);
        let m = convertir(&l, Tipo::Estructura);
        assert_eq!(largo(&m), 2);
        assert_eq!(convertir(&m, Tipo::Lista), l);
        assert_eq!(resumen(&l), "2 cosas: [1, 2]");
        assert_eq!(resumen(&Value::List(vec![])), "lista vacía");
    }
}
