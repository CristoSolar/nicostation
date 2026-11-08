//! Modo fácil (para niños): la hoja de eventos se lee como frases con bloques de colores,
//! los argumentos son fichas que se cambian con un clic, y los bloques se eligen de un catálogo con dibujos.

use super::eventos::{self, Lists, Open, Salida, Target, Thumb};
use super::iconos as ic;
use super::tema::*;
use eframe::egui::{
    self, Align, Align2, Button, Color32, CornerRadius, FontId, Frame, Key, Layout, Margin, PopupCloseBehavior, Pos2, Rect, RichText, Sense,
    Stroke, TextEdit, Ui, UiBuilder, Vec2,
};
use nicostation::catalog::{self, Meta, CATALOG, P};
use nicostation::model::*;

/// Prueba de interfaz (`--abrir ficha`): abre la ventanita de la primera ficha de objeto.
pub static ABRIR_FICHA: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
/// Prueba de interfaz (`--abrir arreglar`): muestra el primer problema como si se pulsara «Mostrar».
pub static MOSTRAR_PRIMERO: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static FICHA_AHORA: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Texto rojo claro legible sobre superficies oscuras.
pub const ROJO_TXT: Color32 = Color32::from_rgb(0xff, 0x9a, 0x9a);
const UNDO_HINT: &str = "Ctrl+Z para deshacer";

// ---------- categorías ----------

pub struct Cat {
    pub name: &'static str,
    pub icon: &'static str,
    pub color: Color32,
    pub groups: &'static [&'static str],
}

pub const CATS: &[Cat] = &[
    Cat { name: "Movimiento", icon: ic::RUN, color: CAT_MOV, groups: &["Movimiento"] },
    Cat { name: "Teclado y mouse", icon: ic::KEYBOARD, color: CAT_INPUT, groups: &["Teclado", "Mouse"] },
    Cat { name: "Objetos", icon: ic::SHAPES, color: CAT_OBJ, groups: &["Objetos"] },
    Cat { name: "Cajitas (variables)", icon: ic::ARCHIVE_BOX, color: CAT_VAR, groups: &["Variables"] },
    Cat { name: "Sonido", icon: ic::SPEAKER, color: CAT_AUDIO, groups: &["Audio"] },
    Cat { name: "Tiempo", icon: ic::TIMER, color: CAT_TIME, groups: &["Tiempo"] },
    Cat { name: "Escena", icon: ic::FILM, color: CAT_SCENE, groups: &["Escena"] },
    Cat { name: "Mis bloques", icon: ic::PUZZLE, color: CAT_FUNC, groups: &["Funciones"] },
    Cat { name: "Control", icon: ic::FLOW, color: CAT_TIME, groups: &["Control"] },
];
const MIS_BLOQUES: usize = 7;

pub fn cat(group: &str) -> &'static Cat {
    CATS.iter().find(|c| c.groups.contains(&group)).unwrap_or(&CATS[8])
}

pub fn cat_op(op: &str) -> &'static Cat {
    catalog::meta(op).map_or(&CATS[8], |m| cat(m.group))
}

pub fn op_icon(op: &str) -> &'static str {
    match op {
        "start" => ic::FLAG,
        "trigger_once" => ic::LIGHTNING,
        "every" | "timer" | "reset_timer" => ic::TIMER,
        "play_music" | "stop_music" => ic::MUSIC,
        "mouse_down" | "mouse_clicked" | "cursor_on" => ic::CURSOR_CLICK,
        "return" => ic::CHECK_CIRCLE,
        _ => cat_op(op).icon,
    }
}

/// Frase del bloque para el modo fácil (algunas del catálogo se reescriben para que se lean mejor).
fn frase_de(m: &Meta) -> &'static str {
    match m.op {
        "trigger_once" => "Solo una vez (no a cada rato)",
        "timer" => "El cronómetro _0_ pasó los _1_ segundos",
        "set_text" => "Cambiar el texto de _0_ a _1_",
        "set_animation" => "Cambiar la animación de _0_ a _1_",
        "flip_x" => "Voltear _0_ como un espejo: _1_",
        "look_at" => "Girar _0_ para que mire a X _1_ Y _2_",
        "set_var" => "Poner la cajita _0_ en _1_",
        "add_var" => "Sumar _1_ a la cajita _0_",
        "set_global" => "Poner la cajita del juego _0_ en _1_",
        "add_global" => "Sumar _1_ a la cajita del juego _0_",
        "reset_timer" => "Poner en cero el cronómetro _0_",
        "create" => "Crear _0_ en X _1_ Y _2_ (capa _3_)",
        "camera" => "Centrar la cámara en X _0_ Y _1_",
        "camera_zoom" => "Poner el zoom de la cámara en _0_",
        "play_sound" => "Hacer sonar _0_ (volumen _1_)",
        "play_music" => "Poner la música _0_ (volumen _1_)",
        "pick_nearest" => "Elegir el _0_ más cercano a X _1_ Y _2_",
        "call" | "call_cond" => "_0_ _args_",
        "return" => "Responder _0_",
        _ => m.sentence,
    }
}

/// Parámetros que valen sí (1) o no (0).
fn es_si_no(op: &str, k: usize) -> bool {
    matches!((op, k), ("flip_x", 1) | ("return", 0))
}

/// Pregunta amable para cada parámetro.
pub fn pregunta(op: &str, k: usize, name: &str, t: P) -> String {
    if es_si_no(op, k) {
        return if op == "return" { "¿Qué responde?".into() } else { "¿Lo das vuelta?".into() };
    }
    match (t, name) {
        (P::Object, "Otro") => "¿Con cuál otro objeto?",
        (P::Object, "Obstáculo") => "¿De qué obstáculo?",
        (P::Object, _) => "¿Qué objeto?",
        (P::Key, _) => "¿Qué tecla?",
        (P::Mouse, _) => "¿Qué botón del mouse?",
        (P::Op, _) => "¿Cómo se comparan?",
        (P::Scene, _) => "¿Qué escena?",
        (P::Prop, _) => "¿Qué dato del objeto?",
        (P::Anim, _) => "¿Qué animación?",
        (P::File, _) => "¿Qué sonido?",
        (P::Layer, _) => "¿En qué capa? (base es la normal)",
        (P::Var, _) => "¿Qué cajita?",
        (P::Func, _) => "¿Cuál de tus bloques?",
        (P::Text, _) => "¿Cómo se llama el cronómetro?",
        (P::Ease, _) => "¿Cómo se mueve?",
        (P::Code, _) => "Código Rhai (para programadores)",
        (_, "Segundos") => "¿Cuántos segundos?",
        (_, "X") => "Posición X (izquierda a derecha)",
        (_, "Y") => "Posición Y (arriba a abajo)",
        (_, "Distancia") => "¿A qué distancia? (en píxeles)",
        (_, "Volumen 0-100") => "¿Qué tan fuerte? (0 a 100)",
        (_, "Zoom") => "¿Cuánto zoom? (1 es normal)",
        (_, "Texto") => "¿Qué texto? (entre comillas)",
        (_, "Otro valor") => "¿Con qué lo comparas?",
        (_, "Valor") => "¿Qué valor?",
        _ => return format!("{name}?"),
    }
    .into()
}

/// Palabra que se muestra en la ficha vacía del catálogo de bloques.
fn hueco(name: &str, t: P) -> String {
    match (t, name) {
        (P::Object, "Objeto") => "objeto".into(),
        (P::Object, _) => "otro objeto".into(),
        (P::Var, _) => "cajita".into(),
        (P::Func, _) => "mi bloque".into(),
        (P::Op, _) => "comparación".into(),
        (_, "Sí (1) / No (0)" | "Verdadero/falso") => "sí / no".into(),
        (_, "Volumen 0-100") => "volumen".into(),
        _ => name.to_lowercase(),
    }
}

pub fn prop_corta(p: &str) -> String {
    match p {
        "x" => "X".into(),
        "y" => "Y".into(),
        "cx" => "centro X".into(),
        "cy" => "centro Y".into(),
        "w" => "ancho".into(),
        "h" => "alto".into(),
        "angle" => "ángulo".into(),
        "opacity" => "transparencia".into(),
        "z" => "adelante/atrás".into(),
        "speed" => "velocidad".into(),
        _ => p.into(),
    }
}

// ---------- textos amables ----------

/// Quita acentos y pasa a minúsculas (para buscar).
pub fn norm(s: &str) -> String {
    s.to_lowercase().chars().map(sin_acento).collect()
}

pub fn sin_acento(c: char) -> char {
    match c {
        'á' | 'à' | 'ä' | 'â' => 'a',
        'é' | 'è' | 'ë' | 'ê' => 'e',
        'í' | 'ì' | 'ï' | 'î' => 'i',
        'ó' | 'ò' | 'ö' | 'ô' => 'o',
        'ú' | 'ù' | 'ü' | 'û' => 'u',
        'Á' | 'À' | 'Ä' | 'Â' => 'A',
        'É' | 'È' | 'Ë' | 'Ê' => 'E',
        'Í' | 'Ì' | 'Ï' | 'Î' => 'I',
        'Ó' | 'Ò' | 'Ö' | 'Ô' => 'O',
        'Ú' | 'Ù' | 'Ü' | 'Û' => 'U',
        'ñ' => 'n',
        'Ñ' => 'N',
        _ => c,
    }
}

/// Nombre válido para cajitas y bloques: letras sin acento, números y _, sin empezar con número.
pub fn ident_ok(s: &str) -> bool {
    let mut c = s.chars();
    c.next().is_some_and(|f| f.is_ascii_alphabetic() || f == '_') && c.all(|x| x.is_ascii_alphanumeric() || x == '_')
}

/// Arreglo automático: espacios a _, sin acentos ni símbolos.
pub fn ident_fix(s: &str) -> String {
    let mut o: String = s.trim().chars().map(sin_acento).map(|c| if c == ' ' || c == '-' { '_' } else { c })
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
    if o.starts_with(|c: char| c.is_ascii_digit()) {
        o.insert(0, '_');
    }
    o
}

fn mayus(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

/// Error del analizador de expresiones en palabras sencillas.
pub fn expr_amable(e: &str) -> String {
    let entre = |s: &str| s.split('«').nth(1).and_then(|r| r.split('»').next()).unwrap_or("").to_string();
    if e.contains("termina antes de tiempo") {
        "falta algo al final (¿un número después del signo?)".into()
    } else if e.starts_with("falta «)»") {
        "falta cerrar un paréntesis «)»".into()
    } else if e.contains("comillas") {
        "falta cerrar las comillas \" del texto".into()
    } else if e.starts_with("sobra texto") {
        format!("sobra «{}». ¿Falta un signo como + entre medio? Si es un texto, ponlo entre comillas: \"Hola\"", entre(e))
    } else if e.starts_with("no esperaba") {
        format!("no entiendo el «{}». Revisa lo que escribiste", entre(e))
    } else if e.starts_with("número inválido") {
        format!("el número «{}» está mal escrito", entre(e))
    } else if e.starts_with("falta la propiedad") {
        format!("después de «{}» va un dato, por ejemplo Heroe.x", entre(e))
    } else if e.starts_with("falta «,»") {
        "separa con comas lo que va dentro de los paréntesis".into()
    } else {
        e.into()
    }
}

/// Mensaje de `validate` (sin la ubicación) explicado para un niño.
pub fn amable(t: &str) -> String {
    let entre = |s: &str| s.split('«').nth(1).and_then(|r| r.split('»').next()).unwrap_or("").to_string();
    if let Some(r) = t.strip_prefix("expresión «") {
        let (src, err) = r.split_once("»: ").unwrap_or((r, ""));
        return format!("Hay un error en «{src}»: {}.", expr_amable(err));
    }
    if t.starts_with("argumento ") {
        if let Some((head, what)) = t.split_once("»: ") {
            let v = head.split_once('«').map_or("", |x| x.1);
            let s = if what.starts_with("objeto o grupo desconocido") {
                if v.is_empty() { "Falta elegir el objeto.".into() } else { format!("Este bloque usa «{v}», que no existe. Elige otro objeto.") }
            } else if what.starts_with("escena desconocida") {
                if v.is_empty() { "Falta elegir la escena.".into() } else { format!("La escena «{v}» no existe. Elige otra escena.") }
            } else if what.starts_with("tecla desconocida") {
                format!("La tecla «{v}» no existe. Elige una del teclado.")
            } else if what.starts_with("botón desconocido") {
                "Elige un botón del mouse.".into()
            } else if what.starts_with("operador inválido") {
                "Elige cómo comparar, por ejemplo «es igual a».".into()
            } else if what.starts_with("capa desconocida") {
                format!("La capa «{v}» no existe. Elige otra capa.")
            } else if what.starts_with("el archivo no existe") {
                if v.is_empty() { "Falta elegir el archivo.".into() } else { format!("No encuentro el archivo «{v}». Elige otro.") }
            } else if what.starts_with("animación desconocida") {
                format!("«{v}» no es una animación de ese objeto. Elige otra.")
            } else if what.starts_with("función desconocida") {
                format!("El bloque «{v}» no existe en Mis bloques.")
            } else if what.contains("función-condición") {
                format!("«{v}» es una pregunta: va en la columna CUANDO.")
            } else if what.contains("función-acción") {
                format!("«{v}» es una acción: va en la columna ENTONCES.")
            } else if what.starts_with("espera ") {
                "Este bloque necesita otros ingredientes. Haz clic para revisarlo.".into()
            } else {
                mayus(what)
            };
            return s;
        }
    }
    if t.contains("es una condición; va en las acciones") {
        "Este bloque es un CUANDO: va en la columna de la izquierda.".into()
    } else if t.contains("es una acción; va en las condiciones") {
        "Este bloque es un ENTONCES: va en la columna de la derecha.".into()
    } else if t.contains("argumentos") && t.contains("necesita") {
        "A este bloque le faltan datos. Haz clic para completarlo.".into()
    } else if t.starts_with("instrucción desconocida") {
        format!("No conozco el bloque «{}».", entre(t))
    } else if t.contains("«para cada» con objeto desconocido") {
        format!("«Para cada» usa «{}», que no existe.", entre(t.split("desconocido").nth(1).unwrap_or("")))
    } else if t.contains("«mientras» sin condiciones") {
        "Un «mientras» necesita al menos un CUANDO; si no, se repetiría para siempre.".into()
    } else {
        mayus(t)
    }
}

// ---------- problemas de validate ubicados en la hoja ----------

#[derive(Clone, Debug)]
pub struct Problema {
    /// Ruta del evento (base 0).
    pub ev: Vec<usize>,
    /// Columna (true = CUANDO) e índice del bloque.
    pub at: Option<(bool, usize)>,
    pub msg: String,
}

/// Separa los problemas de una hoja (`owner` = "Escena «X»" o "Función «f»") del resto.
pub fn problemas(all: &[String], owner: &str) -> (Vec<Problema>, Vec<String>) {
    let (mut here, mut otros) = (vec![], vec![]);
    for m in all {
        let Some(rest) = m.strip_prefix(owner).and_then(|r| r.strip_prefix(" › evento ")) else {
            otros.push(m.clone());
            continue;
        };
        let (loc, text) = rest.split_once(": ").unwrap_or((rest, ""));
        let mut parts = loc.split(" › ");
        let ev: Vec<usize> = parts.next().unwrap_or("").split('.').filter_map(|n| n.parse::<usize>().ok()).map(|n| n.saturating_sub(1)).collect();
        let at = parts.next().and_then(|p| {
            let (c, n) = if let Some(n) = p.strip_prefix("condición ") { (true, n) } else { (false, p.strip_prefix("acción ")?) };
            Some((c, n.parse::<usize>().ok()?.saturating_sub(1)))
        });
        // "› argumento 2: expresión ..." queda en la ubicación; se lo devolvemos al texto.
        let text = match parts.next() {
            Some(_) if text.starts_with("expresión") => text.to_string(),
            Some(a) => format!("{a}: {text}"),
            None => text.to_string(),
        };
        here.push(Problema { ev, at, msg: amable(&text) });
    }
    (here, otros)
}

// ---------- fichas ----------

#[derive(Clone, Copy, PartialEq)]
pub enum Ficha {
    Normal,
    Cajita,
    Mio,
    Vacia,
    Hueco,
}

pub struct Pill {
    pub fill: Color32,
    pub fg: Color32,
    pub stroke: Option<Color32>,
    pub icon: Option<&'static str>,
}

pub fn pill_style(f: Ficha, c: Color32) -> Pill {
    match f {
        Ficha::Normal => Pill { fill: CHIP, fg: CHIP_TEXT, stroke: None, icon: None },
        Ficha::Cajita => Pill { fill: CAT_VAR, fg: ON_CAT, stroke: None, icon: Some(ic::ARCHIVE_BOX) },
        Ficha::Mio => Pill { fill: CAT_FUNC, fg: ON_CAT, stroke: None, icon: None },
        Ficha::Vacia => Pill { fill: blend(ACCENT, SURFACE, 0.22), fg: TEXT, stroke: Some(ACCENT), icon: Some(ic::CURSOR_CLICK) },
        Ficha::Hueco => Pill { fill: blend(Color32::WHITE, tint(c), 0.16), fg: TEXT, stroke: None, icon: None },
    }
}

/// Píldora redondeada con ícono opcional; se dibuja a mano para controlar el contraste.
pub fn pill(ui: &mut Ui, text: &str, st: &Pill, sense: Sense) -> egui::Response {
    pill_at(ui, text, st, sense, None)
}

/// Como `pill`, con un id fijo (para que su ventanita no se cierre si cambia el diseño de la hoja).
pub fn pill_at(ui: &mut Ui, text: &str, st: &Pill, sense: Sense, id: Option<egui::Id>) -> egui::Response {
    let font = sb(15.);
    let g = ui.painter().layout_no_wrap(text.to_string(), font, st.fg);
    let ig = st.icon.map(|i| ui.painter().layout_no_wrap(i.to_string(), FontId::proportional(15.), st.fg));
    let w = g.size().x + 20. + ig.as_ref().map_or(0., |g| g.size().x + 5.);
    let (rect, r) = match id {
        Some(id) => {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 26.), Sense::hover());
            (rect, ui.interact(rect, id, sense))
        }
        None => ui.allocate_exact_size(Vec2::new(w, 26.), sense),
    };
    let p = ui.painter();
    let click = sense.senses_click();
    let hov = click && r.hovered();
    p.rect_filled(rect, 13, if hov { blend(Color32::WHITE, st.fill, 0.25) } else { st.fill });
    if let Some(s) = st.stroke {
        p.rect_stroke(rect, 13, Stroke::new(1.5, s), egui::StrokeKind::Inside);
    }
    if hov || r.has_focus() {
        p.rect_stroke(rect.expand(1.5), 15, Stroke::new(2., ACCENT), egui::StrokeKind::Outside);
    }
    let mut x = rect.left() + 10.;
    if let Some(ig) = ig {
        let sz = ig.size();
        p.galley(Pos2::new(x, rect.center().y - sz.y / 2.), ig, st.fg);
        x += sz.x + 5.;
    }
    let sz = g.size();
    p.galley(Pos2::new(x, rect.center().y - sz.y / 2.), g, st.fg);
    if click {
        r.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        r
    }
}

/// Texto y tipo de ficha para un argumento ya completado.
pub fn ficha(op: &str, k: usize, t: P, v: &str, l: &Lists) -> (String, Ficha, Option<String>) {
    if v.is_empty() && t != P::Layer {
        return ("elegir".into(), Ficha::Vacia, Some("Falta completar esto. Haz clic para elegir.".into()));
    }
    match t {
        P::Object if l.groups.iter().any(|g| g == v) => (v.into(), Ficha::Normal, Some(format!("«{v}» es un grupo de objetos"))),
        P::Key => (catalog::key_label(v), Ficha::Normal, None),
        P::Mouse => (catalog::mouse_label(v).into(), Ficha::Normal, None),
        P::Op => (catalog::op_words(v).into(), Ficha::Normal, Some(format!("Comparación «{v}»"))),
        P::Ease => (catalog::ease_label(v).into(), Ficha::Normal, Some(format!("Curva «{v}»"))),
        P::Code => {
            let l1 = v.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
            let l1 = if l1.chars().count() > 24 { format!("{}...", l1.chars().take(22).collect::<String>()) } else { l1.to_string() };
            (format!("código: {l1}"), Ficha::Normal, Some(v.to_string()))
        }
        P::Prop if catalog::PROPS.contains(&v) => (prop_corta(v), Ficha::Normal, Some(mayus(&catalog::prop_label(v)))),
        P::Prop => (v.into(), Ficha::Cajita, Some(format!("Cajita «{v}» del objeto"))),
        P::Layer if v.is_empty() => ("base".into(), Ficha::Normal, Some("La capa normal".into())),
        P::File => (v.rsplit('/').next().unwrap_or(v).into(), Ficha::Normal, Some(v.into())),
        P::Var => (v.into(), Ficha::Cajita, Some(format!("Cajita «{v}»"))),
        P::Func => (v.into(), Ficha::Mio, Some("Un bloque de «Mis bloques»".into())),
        P::Expr if es_si_no(op, k) && matches!(v.trim(), "1" | "0" | "true" | "false") => {
            ((if matches!(v.trim(), "1" | "true") { "sí" } else { "no" }).into(), Ficha::Normal, None)
        }
        P::Expr => {
            let simple = ident_ok(v.trim()) && (l.vars.iter().chain(&l.globals).chain(&l.locals).any(|x| x == v.trim()));
            let dt = has_ident(v, "dt");
            let mut tip = dt.then(|| "dt es el tiempo de un cuadro. Multiplicar por dt quiere decir «por segundo»: 80 * dt es «80 por segundo».".to_string());
            let txt = if v.chars().count() > 30 {
                tip = Some(format!("{v}{}", tip.map(|t| format!("\n\n{t}")).unwrap_or_default()));
                format!("{}...", v.chars().take(28).collect::<String>())
            } else {
                v.to_string()
            };
            (txt, if simple { Ficha::Cajita } else { Ficha::Normal }, tip)
        }
        _ => (v.into(), Ficha::Normal, None),
    }
}

/// ¿Aparece `name` como identificador suelto (no `Obj.name`, fuera de comillas)?
pub fn has_ident(src: &str, name: &str) -> bool {
    let b = src.as_bytes();
    let (mut i, mut q) = (0, false);
    while i < b.len() {
        let c = b[i];
        if c == b'"' {
            q = !q;
            i += 1;
        } else if !q && (c.is_ascii_alphabetic() || c == b'_') {
            let st = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            if &src[st..i] == name && (st == 0 || b[st - 1] != b'.') {
                return true;
            }
        } else {
            i += 1;
        }
    }
    false
}

enum Tok {
    Lit(&'static str),
    Arg(usize),
    Args,
}

fn tokens(s: &'static str) -> Vec<Tok> {
    let mut out = vec![];
    let (mut k, mut lit) = (0, 0);
    while k < s.len() {
        let rest = &s[k..];
        let tok = if rest.starts_with("_args_") {
            Some((Tok::Args, 6))
        } else if let Some(d) = rest.strip_prefix('_').map(|r| r.bytes().take_while(u8::is_ascii_digit).count()).filter(|&d| d > 0 && rest[1 + d..].starts_with('_')) {
            Some((Tok::Arg(rest[1..1 + d].parse().unwrap_or(0)), d + 2))
        } else {
            None
        };
        match tok {
            Some((t, len)) => {
                if lit < k {
                    out.push(Tok::Lit(&s[lit..k]));
                }
                out.push(t);
                k += len;
                lit = k;
            }
            None => k += rest.chars().next().map_or(1, char::len_utf8),
        }
    }
    if lit < s.len() {
        out.push(Tok::Lit(&s[lit..]));
    }
    out
}

fn palabras(ui: &mut Ui, s: &str, c: Color32) {
    for w in s.split_whitespace() {
        ui.add(egui::Label::new(RichText::new(w).font(FontId::proportional(15.)).color(c)).selectable(false));
    }
}

/// Dibuja la frase: palabras sueltas (para que se acomoden al ancho) y una ficha por argumento.
fn frase(ui: &mut Ui, m: &Meta, nargs: usize, mut chip: impl FnMut(&mut Ui, usize)) {
    for t in tokens(frase_de(m)) {
        match t {
            Tok::Lit(s) => palabras(ui, s, TEXT),
            Tok::Arg(k) => chip(ui, k),
            Tok::Args => (1..nargs).for_each(|k| chip(ui, k)),
        }
    }
}

/// Tipo y nombre del parámetro `k` (los de `call` después del primero son ingredientes).
fn param(m: &Meta, k: usize) -> (&'static str, P) {
    match m.params.get(k) {
        Some(p) => *p,
        None if m.params.last().is_some_and(|p| p.1 == P::Func) => ("Ingrediente", P::Expr),
        None => ("?", P::Text),
    }
}

// ---------- editores de argumentos ----------

pub struct ArgCx<'a> {
    pub l: &'a Lists,
    pub op: &'a str,
    pub k: usize,
    /// Primer argumento (objeto) para animaciones y propiedades.
    pub obj: &'a str,
    pub cond: bool,
}

pub fn titulo(ui: &mut Ui, t: &str) {
    ui.label(RichText::new(t).font(sb(16.)).color(TEXT));
}

/// Recuadro clicable con dibujo arriba y nombre abajo.
fn tile(ui: &mut Ui, sel: bool, size: Vec2, name: &str, badge: Option<&str>, draw: impl FnOnce(&egui::Painter, Rect)) -> bool {
    let (rect, r) = ui.allocate_exact_size(size, Sense::click());
    let p = ui.painter();
    let hov = r.hovered();
    p.rect_filled(rect, 8, if sel { blend(ACCENT, SURFACE, 0.22) } else if hov { SURFACE_HOVER } else { SURFACE });
    p.rect_stroke(rect, 8, Stroke::new(if sel { 2. } else { 1. }, if sel { ACCENT } else if hov { BORDER_STRONG } else { BORDER }), egui::StrokeKind::Inside);
    let pic = Rect::from_center_size(Pos2::new(rect.center().x, rect.top() + 8. + 22.), Vec2::splat(44.));
    draw(p, pic);
    let g = p.layout(name.to_string(), sb(14.), TEXT, rect.width() - 8.);
    let g = if g.rows.len() > 1 { p.layout_no_wrap(trunc(name, 10), sb(14.), TEXT) } else { g };
    p.galley(Pos2::new(rect.center().x - g.size().x / 2., rect.bottom() - 8. - g.size().y), g, TEXT);
    if let Some(b) = badge {
        let bg = p.layout_no_wrap(b.to_string(), FontId::proportional(11.), ON_CAT);
        let br = Rect::from_min_size(Pos2::new(rect.right() - bg.size().x - 12., rect.top() + 4.), bg.size() + Vec2::new(8., 2.));
        p.rect_filled(br, 6, CAT_OBJ);
        p.galley(br.min + Vec2::new(4., 1.), bg, ON_CAT);
    }
    r.on_hover_text(name).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn trunc(s: &str, n: usize) -> String {
    if s.chars().count() > n { format!("{}...", s.chars().take(n - 1).collect::<String>()) } else { s.into() }
}

pub fn dibujar_thumb(p: &egui::Painter, r: Rect, t: Option<&Thumb>, group: bool) {
    match t {
        _ if group => drop(p.text(r.center(), Align2::CENTER_CENTER, ic::STACK, FontId::proportional(30.), CAT_OBJ)),
        Some(Thumb::Tex(tex)) => {
            let s = tex.size_vec2();
            let k = (r.width() / s.x).min(r.height() / s.y);
            p.image(tex.id(), Rect::from_center_size(r.center(), s * k), Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)), Color32::WHITE);
        }
        Some(Thumb::Color([a, b, c])) => {
            let rr = r.shrink(6.);
            p.rect_filled(rr, 4, Color32::from_rgb(*a, *b, *c));
            p.rect_stroke(rr, 4, Stroke::new(1., BORDER_STRONG), egui::StrokeKind::Outside);
        }
        Some(Thumb::Text([a, b, c])) => drop(p.text(r.center(), Align2::CENTER_CENTER, "T", sb(32.), Color32::from_rgb(*a, *b, *c))),
        _ => drop(p.text(r.center(), Align2::CENTER_CENTER, ic::IMAGE, FontId::proportional(28.), TEXT_FAINT)),
    }
}

pub fn objetos(ui: &mut Ui, v: &mut String, l: &Lists) -> bool {
    let mut ch = false;
    if l.objs.is_empty() {
        ui.label(RichText::new("Esta escena todavía no tiene objetos. Créalos en la pestaña Escena.").color(TEXT_MUTED));
    }
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(6.);
        for o in &l.objs {
            let group = l.groups.contains(o);
            let t = l.thumbs.get(o);
            if tile(ui, v == o, Vec2::new(96., 86.), o, group.then_some("grupo"), |p, r| dibujar_thumb(p, r, t, group)) {
                *v = o.clone();
                ch = true;
            }
        }
    });
    ch
}

fn tecla(ui: &mut Ui, v: &mut String, code: &str, label: &str, w: f32) -> bool {
    let sel = v == code;
    let (rect, r) = ui.allocate_exact_size(Vec2::new(w, 36.), Sense::click());
    let p = ui.painter();
    let hov = r.hovered();
    p.rect_filled(rect.translate(Vec2::new(0., 2.)), 6, BG);
    p.rect_filled(rect, 6, if sel { ACCENT } else if hov { BORDER_STRONG } else { SURFACE_HOVER });
    p.rect_stroke(rect, 6, Stroke::new(1., if sel { ACCENT } else { BORDER_STRONG }), egui::StrokeKind::Inside);
    let font = if label.chars().count() > 2 { FontId::proportional(13.) } else { sb(15.) };
    p.text(rect.center(), Align2::CENTER_CENTER, label, font, if sel { ON_ACCENT } else { TEXT });
    let r = r.on_hover_text(catalog::key_label(code)).on_hover_cursor(egui::CursorIcon::PointingHand);
    if r.clicked() {
        *v = code.into();
    }
    r.clicked()
}

pub fn teclado(ui: &mut Ui, v: &mut String) -> bool {
    let mut ch = false;
    let k = 36.;
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(4.);
            let rows: [(&[&str], f32); 4] = [
                (&["Key1", "Key2", "Key3", "Key4", "Key5", "Key6", "Key7", "Key8", "Key9", "Key0"], 0.),
                (&["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"], 14.),
                (&["A", "S", "D", "F", "G", "H", "J", "K", "L"], 24.),
                (&["Z", "X", "C", "V", "B", "N", "M"], 38.),
            ];
            for (keys, off) in rows {
                ui.horizontal(|ui| {
                    ui.add_space(off);
                    for c in keys {
                        ch |= tecla(ui, v, c, &catalog::key_label(c), k);
                    }
                });
            }
            ui.horizontal(|ui| {
                for (c, l, w) in [("Escape", "Esc", 44.), ("Tab", "Tab", 44.), ("LeftShift", "Shift", 52.), ("LeftControl", "Ctrl", 44.)] {
                    ch |= tecla(ui, v, c, l, w);
                }
                ch |= tecla(ui, v, "Space", "Espacio", 150.);
            });
            ui.horizontal(|ui| {
                ch |= tecla(ui, v, "Enter", "Enter", 70.);
                ch |= tecla(ui, v, "Backspace", "Borrar", 70.);
            });
        });
        ui.add_space(14.);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(4.);
            ui.add_space(k * 2. + 8.);
            ui.horizontal(|ui| {
                ui.add_space(k + 4.);
                ch |= tecla(ui, v, "Up", ic::ARROW_UP, k);
            });
            ui.horizontal(|ui| {
                ch |= tecla(ui, v, "Left", ic::ARROW_LEFT, k);
                ch |= tecla(ui, v, "Down", ic::ARROW_DOWN, k);
                ch |= tecla(ui, v, "Right", ic::ARROW_RIGHT, k);
            });
        });
    });
    if !v.is_empty() {
        ui.label(RichText::new(format!("Elegiste: {}", catalog::key_label(v))).color(TEXT_SOFT));
    }
    ch
}

/// Botón grande de opción (seleccionado = acento).
/// Curvas de un cambio suave: cada opción muestra su forma.
fn curvas(ui: &mut Ui, v: &mut String) -> bool {
    let mut ch = false;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(6.);
        for e in catalog::EASES {
            let sel = v == e;
            let (rect, r) = ui.allocate_exact_size(Vec2::new(132., 104.), Sense::click());
            let p = ui.painter();
            let hov = r.hovered();
            p.rect_filled(rect, 8, if sel { blend(ACCENT, SURFACE, 0.22) } else if hov { SURFACE_HOVER } else { SURFACE });
            p.rect_stroke(rect, 8, Stroke::new(if sel { 2. } else { 1. }, if sel { ACCENT } else if hov { BORDER_STRONG } else { BORDER }), egui::StrokeKind::Inside);
            // La curva: de abajo a la izquierda (empieza) a arriba a la derecha (llega). Puede pasarse un poco.
            let g = Rect::from_min_size(rect.min + Vec2::new(16., 16.), Vec2::new(rect.width() - 32., 40.));
            p.line_segment([g.left_bottom(), g.right_bottom()], Stroke::new(1., BORDER_STRONG));
            p.line_segment([g.left_top(), g.right_top()], Stroke::new(1., BORDER));
            let pts: Vec<Pos2> = (0..=40).map(|k| {
                let t = k as f32 / 40.;
                Pos2::new(g.left() + t * g.width(), g.bottom() - nicostation::runtime::ease(e, t) * g.height())
            }).collect();
            p.add(egui::Shape::line(pts, Stroke::new(2.5, if sel { ACCENT } else { CAT_MOV })));
            let name = catalog::ease_label(e);
            let gl = p.layout(name.to_string(), sb(13.), TEXT, rect.width() - 10.);
            p.galley(Pos2::new(rect.center().x - gl.size().x / 2., rect.bottom() - 6. - gl.size().y), gl, TEXT);
            if r.on_hover_text(format!("{name} ({e})")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                *v = e.to_string();
                ch = true;
            }
        }
    });
    ch
}

fn opcion(ui: &mut Ui, sel: bool, atoms: impl egui::IntoAtoms<'static>, w: f32) -> bool {
    let b = Button::selectable(sel, atoms).min_size(Vec2::new(w, 38.));
    ui.add(b).clicked()
}

fn mouse(ui: &mut Ui, v: &mut String) -> bool {
    let mut ch = false;
    ui.horizontal_wrapped(|ui| {
        for b in catalog::MOUSE {
            if opcion(ui, v == b, (RichText::new(ic::MOUSE).size(18.), RichText::new(catalog::mouse_label(b)).size(15.)), 120.) {
                *v = b.to_string();
                ch = true;
            }
        }
    });
    ch
}

fn ops(ui: &mut Ui, v: &mut String) -> bool {
    let mut ch = false;
    egui::Grid::new("ops").num_columns(2).spacing([6., 6.]).show(ui, |ui| {
        for (i, o) in catalog::OPS.iter().enumerate() {
            let atoms = (RichText::new(catalog::op_words(o)).size(15.), RichText::new(*o).monospace().color(TEXT_MUTED));
            if opcion(ui, v == o, atoms, 210.) {
                *v = o.to_string();
                ch = true;
            }
            if i % 2 == 1 {
                ui.end_row();
            }
        }
    });
    ch
}

fn nombres(ui: &mut Ui, v: &mut String, list: &[String], icon: &'static str, color: Color32, empty_label: &str) -> bool {
    let mut ch = false;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(6.);
        for n in list {
            let shown = if n.is_empty() { empty_label.to_string() } else { n.clone() };
            if opcion(ui, v == n, (RichText::new(icon).size(17.).color(color), RichText::new(shown).size(15.)), 0.) {
                *v = n.clone();
                ch = true;
            }
        }
    });
    ch
}

fn animaciones(ui: &mut Ui, v: &mut String, cx: &ArgCx) -> bool {
    let mut ch = false;
    match cx.l.anims.get(cx.obj) {
        Some(an) if !an.is_empty() => drop(ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(6.);
            for a in an {
                let t = cx.l.anim_tex.get(&(cx.obj.to_string(), a.clone())).cloned().map(Thumb::Tex);
                if tile(ui, v == a, Vec2::new(96., 86.), a, None, |p, r| dibujar_thumb(p, r, t.as_ref(), false)) {
                    *v = a.clone();
                    ch = true;
                }
            }
        })),
        _ => {
            ui.label(RichText::new("Primero elige un objeto con dibujos (sprite). También puedes escribir el nombre:").color(TEXT_MUTED));
            ui.add(TextEdit::singleline(v).desired_width(220.));
        }
    }
    ch
}

fn archivos(ui: &mut Ui, v: &mut String, l: &Lists, audio: bool) -> bool {
    let mut ch = false;
    let exts: &[&str] = if audio { &["wav", "ogg", "mp3"] } else { &["png", "jpg", "jpeg"] };
    let list: Vec<&String> = l.files.iter().filter(|f| f.rsplit('.').next().is_some_and(|e| exts.contains(&e.to_lowercase().as_str()))).collect();
    if list.is_empty() {
        ui.label(RichText::new("No encontré archivos en la carpeta del juego.").color(TEXT_MUTED));
    }
    egui::ScrollArea::vertical().id_salt("files").max_height(220.).show(ui, |ui| {
        for f in list {
            let icon = if audio { ic::FILE_AUDIO } else { ic::IMAGE };
            let r = ui.add(Button::selectable(v == f, (RichText::new(icon).size(17.).color(CAT_AUDIO), RichText::new(f.as_str()).size(15.)))
                .min_size(Vec2::new(ui.available_width(), 32.)));
            if r.clicked() {
                *v = f.clone();
                ch = true;
            }
        }
    });
    if ui.button(it(ic::FOLDER_OPEN, "Buscar archivo...")).clicked() {
        if let Some(f) = rfd::FileDialog::new().set_directory(&l.base).pick_file() {
            *v = super::rel(&l.base, &f);
            ch = true;
        }
    }
    ch
}

fn props(ui: &mut Ui, v: &mut String, cx: &ArgCx) -> bool {
    let mut ch = false;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(6.);
        for p in catalog::PROPS {
            let r = ui.add(Button::selectable(v == p, RichText::new(prop_corta(p)).size(15.)).min_size(Vec2::new(0., 34.)))
                .on_hover_text(mayus(&catalog::prop_label(p)));
            if r.clicked() {
                *v = p.to_string();
                ch = true;
            }
        }
    });
    if let Some(vs) = cx.l.obj_vars.get(cx.obj).filter(|v| !v.is_empty()) {
        ui.label(RichText::new(format!("Cajitas de {}:", cx.obj)).color(TEXT_MUTED));
        ui.horizontal_wrapped(|ui| {
            for n in vs {
                let sel = v == n;
                let st = Pill { stroke: sel.then_some(Color32::WHITE), ..pill_style(Ficha::Cajita, CAT_VAR) };
                if pill(ui, n, &st, Sense::click()).clicked() {
                    *v = n.clone();
                    ch = true;
                }
            }
        });
    }
    ui.horizontal(|ui| {
        ui.label(RichText::new("Otro nombre:").color(TEXT_MUTED));
        ui.add(TextEdit::singleline(v).desired_width(160.));
    });
    ch
}

fn cajitas(ui: &mut Ui, v: &mut String, cx: &ArgCx, new_var: &mut Option<(String, bool)>) -> bool {
    let mut ch = false;
    let global = matches!(cx.op, "set_global" | "add_global");
    let mine = if global { &cx.l.globals } else { &cx.l.vars };
    let mut fila = |ui: &mut Ui, list: &Vec<String>, t: &str| {
        if list.is_empty() {
            return;
        }
        ui.label(RichText::new(t).color(TEXT_MUTED));
        ui.horizontal_wrapped(|ui| {
            for n in list {
                let st = Pill { stroke: (v == n).then_some(Color32::WHITE), ..pill_style(Ficha::Cajita, CAT_VAR) };
                if pill(ui, n, &st, Sense::click()).clicked() {
                    *v = n.clone();
                    ch = true;
                }
            }
        });
    };
    fila(ui, mine, if global { "Cajitas de todo el juego:" } else { "Cajitas de esta escena:" });
    let id = ui.make_persistent_id("nueva_cajita");
    let mut nueva: String = ui.data_mut(|d| d.get_temp(id).unwrap_or_default());
    ui.add_space(4.);
    ui.horizontal(|ui| {
        ui.label(RichText::new(ic::PLUS).color(CAT_VAR));
        let r = ui.add(TextEdit::singleline(&mut nueva).hint_text("Nueva cajita...").desired_width(170.));
        let ok = ident_ok(&nueva) && !mine.contains(&nueva);
        let enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        if ui.add_enabled(ok, Button::new("Crear")).clicked() || (enter && ok) {
            *v = std::mem::take(&mut nueva);
            *new_var = Some((v.clone(), global));
            ch = true;
        }
    });
    if !nueva.is_empty() && !ident_ok(&nueva) {
        let fix = ident_fix(&nueva);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Usa solo letras sin acento, números y _.").color(ROJO_TXT));
            if !fix.is_empty() && ui.button(format!("Usar «{fix}»")).clicked() {
                nueva = fix;
            }
        });
    }
    ui.data_mut(|d| d.insert_temp(id, nueva));
    ch
}

/// Inserta texto en una expresión: reemplaza si está vacía o es 0; si no, lo suma.
fn insertar(v: &mut String, s: &str, unir: &str) {
    if v.trim().is_empty() || v.trim() == "0" {
        *v = s.into();
    } else {
        *v = format!("{v}{unir}{s}");
    }
}

pub fn expr(ui: &mut Ui, v: &mut String, cx: &ArgCx, si_no: bool) -> bool {
    let mut ch = false;
    if si_no {
        ui.horizontal(|ui| {
            for (val, t) in [("1", "Sí"), ("0", "No")] {
                if opcion(ui, v.trim() == val, RichText::new(t).font(sb(16.)), 90.) {
                    *v = val.into();
                    ch = true;
                }
            }
        });
        return ch;
    }
    ui.horizontal(|ui| {
        ui.add(TextEdit::singleline(v).font(FontId::monospace(16.)).desired_width(230.).hint_text("un número, o una cuenta"));
        if let Ok(n) = v.trim().parse::<f64>() {
            let step = if n.abs() >= 100. { 10. } else { 1. };
            for (icon, d) in [(ic::MINUS, -step), (ic::PLUS, step)] {
                let r = ui.add(Button::new(RichText::new(icon).size(18.)).min_size(Vec2::splat(34.))).on_hover_text(format!("{}{step}", if d < 0. { "Restar " } else { "Sumar " }));
                if r.clicked() {
                    *v = Value::Num(n + d).text();
                }
            }
        }
        ui.menu_button(it(ic::WAND, "Insertar"), |ui| {
            menu_spacing(ui);
            ui.menu_button(it(ic::ARCHIVE_BOX, "Una cajita"), |ui| {
                menu_spacing(ui);
                if cx.l.vars.is_empty() && cx.l.globals.is_empty() && cx.l.locals.is_empty() {
                    ui.label(RichText::new("No hay cajitas todavía").color(TEXT_MUTED));
                }
                for n in cx.l.locals.iter().chain(&cx.l.vars).chain(&cx.l.globals) {
                    if ui.button(n).clicked() {
                        insertar(v, n, " + ");
                    }
                }
            });
            ui.menu_button(it(ic::SHAPES, "Un dato de un objeto"), |ui| {
                menu_spacing(ui);
                for o in cx.l.objs.iter().filter(|o| !cx.l.groups.contains(o)) {
                    ui.menu_button(o, |ui| {
                        menu_spacing(ui);
                        for p in catalog::PROPS {
                            if ui.button(format!("{} ({})", prop_corta(p), catalog::prop_label(p))).clicked() {
                                insertar(v, &format!("{o}.{p}"), " + ");
                            }
                        }
                        for p in cx.l.obj_vars.get(o).into_iter().flatten() {
                            if ui.button(format!("{p} (cajita)")).clicked() {
                                insertar(v, &format!("{o}.{p}"), " + ");
                            }
                        }
                    });
                }
            });
            if ui.button(it(ic::DICE, "Un número al azar (de 1 a 10)")).clicked() {
                insertar(v, "random(1, 10)", " + ");
            }
            if ui.button(it(ic::TIMER, "Por segundo (multiplicar por dt)")).clicked() {
                *v = if v.trim().is_empty() { "dt".into() } else { format!("{} * dt", v.trim()) };
            }
        });
    });
    match nicostation::expr::parse(v) {
        Err(e) => drop(ui.label(RichText::new(format!("{} {}", ic::WARNING, mayus(&expr_amable(&e)))).color(ROJO_TXT))),
        Ok(_) if has_ident(v, "dt") => drop(ui.label(RichText::new(format!("{} dt quiere decir «por segundo».", ic::INFO)).color(TEXT_MUTED))),
        Ok(_) => {}
    }
    ch
}

/// Editor amable de un argumento. Devuelve true si se eligió una opción (para cerrar la ventanita).
pub fn editor(ui: &mut Ui, t: P, v: &mut String, cx: &ArgCx, new_var: &mut Option<(String, bool)>) -> bool {
    match t {
        P::Object => objetos(ui, v, cx.l),
        P::Key => teclado(ui, v),
        P::Mouse => mouse(ui, v),
        P::Op => ops(ui, v),
        P::Ease => curvas(ui, v),
        P::Scene => nombres(ui, v, &cx.l.scenes, ic::FILM, CAT_SCENE, ""),
        P::Layer => nombres(ui, v, &cx.l.layers, ic::STACK, TEXT_MUTED, "base"),
        P::Anim => animaciones(ui, v, cx),
        P::File => archivos(ui, v, cx.l, true),
        P::Prop => props(ui, v, cx),
        P::Var => cajitas(ui, v, cx, new_var),
        P::Func => {
            let fs: Vec<String> = cx.l.funcs.iter().filter(|f| f.1 == cx.cond).map(|f| f.0.clone()).collect();
            if fs.is_empty() {
                ui.label(RichText::new("Todavía no inventaste bloques de este tipo. Hazlo en la pestaña Mis bloques.").color(TEXT_MUTED));
            }
            nombres(ui, v, &fs, ic::PUZZLE, CAT_FUNC, "")
        }
        P::Expr => expr(ui, v, cx, es_si_no(cx.op, cx.k)),
        P::Text => {
            ui.add(TextEdit::singleline(v).desired_width(220.).font(FontId::proportional(16.)));
            false
        }
        P::Code => {
            ui.label(RichText::new("Código para programadores (Rhai). Si no lo conoces, mejor usa los bloques.").color(TEXT_MUTED));
            eventos::codigo(ui, v, 460.);
            false
        }
    }
}

/// Opciones discretas: al elegir se cierra la ventanita.
pub fn cierra(t: P) -> bool {
    matches!(t, P::Object | P::Key | P::Mouse | P::Op | P::Ease | P::Scene | P::Layer | P::Anim | P::File | P::Func)
}

// ---------- bloques ----------

pub struct Hoja<'a> {
    pub l: &'a Lists,
    pub probs: &'a [Problema],
    /// Evento a resaltar y mostrar (desde «Hay cosas por arreglar»).
    pub jump: Option<&'a [usize]>,
}

enum BAct {
    Dup,
    Up,
    Down,
    Del,
}

fn menu_item(ui: &mut Ui, icon: &str, t: &str) -> bool {
    ui.add(Button::new(it(icon, t)).min_size(Vec2::new(0., 30.))).clicked()
}

fn dots(ui: &mut Ui, tip: &str, f: impl FnOnce(&mut Ui)) {
    let b = Button::new(RichText::new(ic::DOTS_V).size(18.).color(TEXT_SOFT)).frame_when_inactive(false).min_size(Vec2::splat(32.));
    let (r, _) = egui::containers::menu::MenuButton::from_button(b).ui(ui, |ui| {
        menu_spacing(ui);
        f(ui)
    });
    r.on_hover_text(tip);
}

#[allow(clippy::too_many_arguments)]
fn bloque(ui: &mut Ui, ins: &mut Instr, ev: &[usize], cond: bool, ip: &[usize], h: &Hoja, prob: Option<&str>, out: &mut Salida) -> Option<BAct> {
    let mut act = None;
    let c = cat_op(&ins.op);
    let m = catalog::meta(&ins.op);
    let bg = tint(c.color);
    let r = ui.scope_builder(UiBuilder::new().sense(Sense::click()).layout(Layout::top_down(Align::Min)), |ui| {
        Frame::new().fill(bg).corner_radius(8).inner_margin(Margin { left: 14, right: 2, top: 4, bottom: 4 })
            .stroke(Stroke::new(if prob.is_some() { 1.5 } else { 0. }, DANGER)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_top(|ui| {
                    let w = (ui.available_width() - 46.).max(60.);
                    ui.allocate_ui_with_layout(Vec2::new(w, 0.), Layout::left_to_right(Align::Center).with_main_wrap(true), |ui| {
                        ui.set_min_height(32.);
                        ui.set_min_width(w);
                        ui.spacing_mut().item_spacing = Vec2::new(5., 4.);
                        ui.label(RichText::new(op_icon(&ins.op)).size(18.).color(c.color));
                        if ins.not {
                            pill(ui, "NO", &Pill { fill: CHIP_NO, fg: Color32::WHITE, stroke: None, icon: None }, Sense::hover())
                                .on_hover_text("Al revés: se cumple cuando esto NO pasa");
                        }
                        let Some(m) = m else {
                            ui.label(RichText::new(format!("Bloque desconocido «{}»", ins.op)).color(ROJO_TXT));
                            return;
                        };
                        let a0 = ins.args.first().cloned().unwrap_or_default();
                        let n = ins.args.len();
                        let op = ins.op.clone();
                        let args = &mut ins.args;
                        frase(ui, m, n, |ui, k| {
                            let (pname, t) = param(m, k);
                            if args.len() <= k {
                                args.resize(k + 1, String::new());
                            }
                            let (txt, f, tip) = ficha(&op, k, t, &args[k], h.l);
                            let id = egui::Id::unique(("ficha", ev, cond, ip, k));
                            let r = pill_at(ui, &txt, &pill_style(f, c.color), Sense::click(), Some(id));
                            let r = r.on_hover_text(tip.unwrap_or_else(|| "Clic para cambiar".into()));
                            if t == P::Object && FICHA_AHORA.swap(false, std::sync::atomic::Ordering::Relaxed) {
                                egui::Popup::open_id(ui.ctx(), egui::Popup::default_response_id(&r));
                            }
                            let q = if pname == "Ingrediente" { format!("Ingrediente {k}") } else { pregunta(&op, k, pname, t) };
                            let fr = Frame::popup(ui.style()).inner_margin(14);
                            egui::Popup::from_toggle_button_response(&r).close_behavior(PopupCloseBehavior::CloseOnClickOutside).frame(fr).show(|ui| {
                                ui.set_max_width(if t == P::Key { 640. } else { 460. });
                                titulo(ui, &q);
                                ui.add_space(4.);
                                let cx = ArgCx { l: h.l, op: &op, k, obj: &a0, cond };
                                if editor(ui, t, &mut args[k], &cx, &mut out.new_var) && cierra(t) {
                                    ui.close();
                                }
                                if t == P::Func {
                                    ajustar_func(args, h.l);
                                }
                            });
                        });
                    });
                    dots(ui, "Opciones del bloque", |ui| {
                        if menu_item(ui, ic::PENCIL, "Cambiar este bloque") {
                            out.open = Some(Open { ev: ev.to_vec(), cond, ip: ip.to_vec(), new: false });
                        }
                        if cond && menu_item(ui, ic::X_CIRCLE, if ins.not { "Quitar el NO" } else { "Al revés (NO)" }) {
                            ins.not = !ins.not;
                        }
                        if menu_item(ui, ic::COPY, "Duplicar") {
                            act = Some(BAct::Dup);
                        }
                        if menu_item(ui, ic::CARET_UP, "Subir") {
                            act = Some(BAct::Up);
                        }
                        if menu_item(ui, ic::CARET_DOWN, "Bajar") {
                            act = Some(BAct::Down);
                        }
                        ui.separator();
                        if ui.add(Button::new(danger_it("Borrar bloque")).min_size(Vec2::new(0., 30.))).clicked() {
                            act = Some(BAct::Del);
                        }
                    });
                });
                if cond && matches!(ins.op.as_str(), "or" | "and") {
                    ui.horizontal_top(|ui| {
                        ui.add_space(8.);
                        ui.vertical(|ui| {
                            ui.set_width(ui.available_width() - 10.);
                            bloques(ui, &mut ins.sub, ev, true, ip, h, &[], out);
                            if add_btn(ui, "otra condición", c.color).clicked() {
                                out.open = Some(Open { ev: ev.to_vec(), cond: true, ip: ip.to_vec(), new: true });
                            }
                        });
                    });
                }
            })
    });
    let rect = r.response.rect;
    ui.painter().rect_filled(Rect::from_min_size(rect.min, Vec2::new(5., rect.height())), CornerRadius { nw: 8, sw: 8, ne: 0, se: 0 }, c.color);
    let help = catalog::kid(&ins.op).help;
    let resp = if help.is_empty() { r.response } else { r.response.on_hover_text(help) };
    if resp.clicked() {
        out.open = Some(Open { ev: ev.to_vec(), cond, ip: ip.to_vec(), new: false });
    }
    if let Some(p) = prob {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.;
            ui.label(RichText::new(ic::WARNING).color(ROJO_TXT));
            ui.label(RichText::new(p).color(ROJO_TXT));
        });
    }
    act
}

/// Ajusta los ingredientes de `call` al bloque elegido.
pub fn ajustar_func(args: &mut Vec<String>, l: &Lists) {
    if let Some(f) = l.funcs.iter().find(|f| Some(&f.0) == args.first()) {
        args.resize(1 + f.2.len(), "0".into());
    }
}

#[allow(clippy::too_many_arguments)]
fn bloques(ui: &mut Ui, l: &mut Vec<Instr>, ev: &[usize], cond: bool, ip: &[usize], h: &Hoja, probs: &[&Problema], out: &mut Salida) {
    let mut act = None;
    for (k, ins) in l.iter_mut().enumerate() {
        let ipk: Vec<usize> = ip.iter().copied().chain([k]).collect();
        let prob = if ip.is_empty() { probs.iter().find(|p| p.at == Some((cond, k))).map(|p| p.msg.as_str()) } else { None };
        ui.push_id(k, |ui| {
            if let Some(a) = bloque(ui, ins, ev, cond, &ipk, h, prob, out) {
                act = Some((k, a));
            }
        });
    }
    match act {
        Some((k, BAct::Del)) => {
            l.remove(k);
            out.msg = Some(format!("Bloque borrado · {UNDO_HINT}"));
        }
        Some((k, BAct::Dup)) => {
            let i = l[k].clone();
            l.insert(k + 1, i);
        }
        Some((k, BAct::Up)) if k > 0 => l.swap(k, k - 1),
        Some((k, BAct::Down)) if k + 1 < l.len() => l.swap(k, k + 1),
        _ => {}
    }
}

fn add_btn(ui: &mut Ui, t: &str, c: Color32) -> egui::Response {
    ui.add(Button::new((RichText::new(ic::PLUS).size(16.).color(c), RichText::new(t).size(15.).color(TEXT_SOFT)))
        .fill(Color32::TRANSPARENT).stroke(Stroke::new(1., BORDER_STRONG)).corner_radius(8).min_size(Vec2::new(0., 32.)))
}

// ---------- hoja ----------

enum EAct {
    Below,
    Dup,
    Del,
    Up,
    Down,
    Nota,
    SiNo,
}

/// Color del «si no» (su conector y su encabezado).
const SI_NO: Color32 = CAT_MOV;

const KINDS: [(EventKind, &str); 6] = [
    (EventKind::Standard, "Normal (CUANDO / ENTONCES)"),
    (EventKind::Else, "Si no (cuando lo de arriba no pasó)"),
    (EventKind::Repeat, "Repetir varias veces"),
    (EventKind::ForEach, "Para cada objeto"),
    (EventKind::While, "Mientras se cumpla"),
    (EventKind::Group, "Grupo (una carpeta de eventos)"),
];

/// Hoja de eventos del modo fácil. Devuelve el rectángulo de cada evento (para dibujar conectores).
pub fn hoja(ui: &mut Ui, evs: &mut Vec<Event>, path: &[usize], h: &Hoja, out: &mut Salida) -> Vec<Rect> {
    use std::sync::atomic::Ordering::Relaxed;
    if path.is_empty() && ABRIR_FICHA.load(Relaxed) > 0 {
        ABRIR_FICHA.fetch_sub(1, Relaxed);
        FICHA_AHORA.store(true, Relaxed);
    }
    let mut act = None;
    let mut rects: Vec<Rect> = vec![];
    // Un «si no» queda un poco más separado del de arriba, para que se vea el conector que los une.
    let si_no: Vec<bool> = evs.iter().map(|e| e.kind == EventKind::Else).collect();
    for (i, e) in evs.iter_mut().enumerate() {
        let p: Vec<usize> = path.iter().copied().chain([i]).collect();
        ui.push_id(i, |ui| {
            let (a, r) = evento(ui, e, &p, h, out);
            if si_no[i] && i > 0 {
                conector_si_no(ui, r);
            }
            rects.push(r);
            if let Some(a) = a {
                act = Some((i, a));
            }
        });
        ui.add_space(if si_no.get(i + 1) == Some(&true) { 12. } else { 4. });
    }
    match act {
        Some((i, EAct::Below)) => evs.insert(i + 1, Event::default()),
        Some((i, EAct::SiNo)) => {
            evs.insert(i + 1, Event { kind: EventKind::Else, ..Default::default() });
            out.msg = Some("Nuevo «si no»: pasa solo cuando el evento de arriba no se cumplió".into());
        }
        Some((i, EAct::Nota)) => evs.insert(i, Event { kind: EventKind::Comment, ..Default::default() }),
        Some((i, EAct::Dup)) => {
            let e = evs[i].clone();
            evs.insert(i + 1, e);
        }
        Some((i, EAct::Del)) => {
            let e = evs.remove(i);
            out.msg = Some(format!("{} borrado · {UNDO_HINT}", if e.kind == EventKind::Comment { "Nota" } else { "Evento" }));
        }
        Some((i, EAct::Up)) if i > 0 => evs.swap(i, i - 1),
        Some((i, EAct::Down)) if i + 1 < evs.len() => evs.swap(i, i + 1),
        _ => {}
    }
    rects
}

/// Une un «si no» con el evento de arriba: una pequeña barra con dos puntas en el hueco entre las tarjetas.
fn conector_si_no(ui: &Ui, card: Rect) {
    let x = card.left() + 30.;
    let (y0, y1) = (card.top() - 14., card.top() + 2.);
    let p = ui.painter();
    p.line_segment([Pos2::new(x, y0), Pos2::new(x, y1)], Stroke::new(3., SI_NO));
    p.circle_filled(Pos2::new(x, y0), 3.5, SI_NO);
    p.circle_filled(Pos2::new(x, y1), 3.5, SI_NO);
}

fn menu_evento(ui: &mut Ui, e: &mut Event, act: &mut Option<EAct>) {
    if menu_item(ui, ic::PLUS, "Nuevo evento debajo") {
        *act = Some(EAct::Below);
    }
    if matches!(e.kind, EventKind::Standard | EventKind::Else) && menu_item(ui, ic::BEND_DOWN_RIGHT, "Agregar «si no» debajo") {
        *act = Some(EAct::SiNo);
    }
    if e.kind != EventKind::Comment && menu_item(ui, ic::ELBOW, "Agregar «y además, si...»") {
        e.sub.push(Event::default());
    }
    if menu_item(ui, ic::COPY, "Duplicar") {
        *act = Some(EAct::Dup);
    }
    if menu_item(ui, ic::CARET_UP, "Subir") {
        *act = Some(EAct::Up);
    }
    if menu_item(ui, ic::CARET_DOWN, "Bajar") {
        *act = Some(EAct::Down);
    }
    let (i, t) = if e.disabled { (ic::EYE, "Activar") } else { (ic::EYE_SLASH, "Desactivar (el juego no lo usa)") };
    if menu_item(ui, i, t) {
        e.disabled = !e.disabled;
    }
    ui.separator();
    if menu_item(ui, ic::NOTE, "Poner una nota arriba") {
        *act = Some(EAct::Nota);
    }
    if e.kind == EventKind::Comment {
        if (!e.conds.is_empty() || !e.acts.is_empty() || !e.sub.is_empty()) && menu_item(ui, ic::LIST, "Volver a ser evento") {
            e.kind = EventKind::Standard;
        }
    } else {
        if menu_item(ui, ic::NOTE, "Convertir en nota (se guarda escondido)") {
            if e.text.is_empty() {
                e.text = resumen(e);
            }
            e.kind = EventKind::Comment;
        }
        ui.menu_button(it(ic::REPEAT, "Tipo de evento"), |ui| {
            menu_spacing(ui);
            for (k, n) in KINDS.iter() {
                if ui.radio(e.kind == *k, *n).clicked() {
                    e.kind = k.clone();
                    if *k == EventKind::Group && e.text.is_empty() {
                        e.text = "Grupo".into();
                    }
                }
            }
        });
    }
    ui.separator();
    if ui.add(Button::new(danger_it(if e.kind == EventKind::Comment { "Borrar nota" } else { "Borrar evento" })).min_size(Vec2::new(0., 30.))).clicked() {
        *act = Some(EAct::Del);
    }
}

/// Frase corta de un evento (para notas).
fn resumen(e: &Event) -> String {
    let c: Vec<String> = e.conds.iter().map(catalog::sentence).collect();
    let a: Vec<String> = e.acts.iter().map(catalog::sentence).collect();
    format!("Cuando {}, entonces {}", if c.is_empty() { "siempre".into() } else { c.join(" y ") }, a.join(", "))
}

fn evento(ui: &mut Ui, e: &mut Event, p: &[usize], h: &Hoja, out: &mut Salida) -> (Option<EAct>, Rect) {
    let mut act = None;
    let probs: Vec<&Problema> = h.probs.iter().filter(|x| x.ev == p).collect();
    let jump = h.jump == Some(p);
    let rect = ui.scope(|ui| {
        if e.disabled {
            ui.multiply_opacity(0.5);
        }
        let r = match e.kind {
            EventKind::Comment => nota(ui, e, &mut act),
            EventKind::Group => grupo(ui, e, p, h, out, &mut act, jump),
            _ => tarjeta(ui, e, p, h, &probs, out, &mut act, jump),
        };
        if jump {
            r.scroll_to_me(Some(Align::Center));
            let t = ui.input(|i| i.time);
            let a = (0.5 + 0.5 * (t * 6.).sin()) as f32;
            ui.painter().rect_stroke(r.rect.expand(3.), 12, Stroke::new(2.5, DANGER.gamma_multiply(0.4 + 0.6 * a)), egui::StrokeKind::Outside);
            ui.ctx().request_repaint();
        }
        r.rect
    }).inner;
    // y además, si...
    if e.kind != EventKind::Comment && e.kind != EventKind::Group && !e.sub.is_empty() {
        let x = rect.left() + 18.;
        let r = ui.horizontal_top(|ui| {
            ui.add_space(36.);
            ui.vertical(|ui| {
                ui.add_space(2.);
                ui.label(RichText::new("y además, si...").font(sb(14.)).color(TEXT_MUTED))
                    .on_hover_text("Estos eventos solo se revisan cuando el evento de arriba se cumple.");
                hoja(ui, &mut e.sub, p, h, out)
            }).inner
        }).inner;
        let pa = ui.painter();
        let st = Stroke::new(2., BORDER_STRONG);
        if let Some(last) = r.last() {
            let y1 = last.top() + 22.;
            pa.line_segment([Pos2::new(x, rect.bottom()), Pos2::new(x, y1)], st);
            for c in &r {
                let y = c.top() + 22.;
                pa.line_segment([Pos2::new(x, y), Pos2::new(c.left(), y)], st);
            }
        }
    }
    (act, rect)
}

fn nota(ui: &mut Ui, e: &mut Event, act: &mut Option<EAct>) -> egui::Response {
    let r = Frame::new().fill(blend(WARN, SURFACE, 0.1)).corner_radius(10).inner_margin(Margin { left: 16, right: 6, top: 6, bottom: 6 }).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(RichText::new(ic::NOTE).size(18.).color(WARN)).on_hover_text("Una nota: el juego no la usa, sirve para explicar.");
            let w = ui.available_width() - 44.;
            ui.add(TextEdit::multiline(&mut e.text).text_color(TEXT).font(FontId::proportional(15.)).frame(egui::Frame::NONE)
                .desired_width(w).desired_rows(1).hint_text("Escribe una nota para acordarte de algo..."));
            dots(ui, "Opciones de la nota", |ui| menu_evento(ui, e, act));
        });
        let n = e.conds.len() + e.acts.len();
        if n > 0 {
            ui.label(RichText::new(format!("Guarda {n} bloques escondidos.")).color(TEXT_MUTED));
        }
    });
    let rr = r.response.rect;
    ui.painter().rect_filled(Rect::from_min_size(rr.min, Vec2::new(5., rr.height())), CornerRadius { nw: 10, sw: 10, ne: 0, se: 0 }, WARN);
    r.response
}

fn grupo(ui: &mut Ui, e: &mut Event, p: &[usize], h: &Hoja, out: &mut Salida, act: &mut Option<EAct>, _jump: bool) -> egui::Response {
    let fold_id = ui.make_persistent_id("abierto");
    let mut open = ui.ctx().data_mut(|d| *d.get_temp_mut_or(fold_id, true));
    // Si hay que mostrar un evento de adentro, se abre.
    if h.jump.is_some_and(|j| j.len() > p.len() && j.starts_with(p)) {
        open = true;
    }
    let r = Frame::new().stroke((1., BORDER_STRONG)).fill(PANEL).corner_radius(10).inner_margin(10).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let caret = if open { ic::CARET_DOWN } else { ic::CARET_RIGHT };
            if ui.add(Button::new(RichText::new(caret).size(18.)).frame_when_inactive(false).min_size(Vec2::splat(32.)))
                .on_hover_text(if open { "Cerrar la carpeta" } else { "Abrir la carpeta" }).clicked() {
                open = !open;
            }
            ui.label(RichText::new(ic::FOLDERS).size(18.).color(TEXT_SOFT));
            ui.add(TextEdit::singleline(&mut e.text).hint_text("Nombre del grupo").font(sb(16.)).frame(egui::Frame::NONE).desired_width(ui.available_width() - 130.));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                dots(ui, "Opciones del grupo", |ui| menu_evento(ui, e, act));
                ui.label(RichText::new(format!("{} eventos", e.sub.len())).color(TEXT_MUTED));
            });
        });
        if open {
            ui.add_space(4.);
            hoja(ui, &mut e.sub, p, h, out);
            if add_btn(ui, "Nuevo evento en el grupo", ACCENT).clicked() {
                e.sub.push(Event::default());
            }
        }
    });
    ui.ctx().data_mut(|d| d.insert_temp(fold_id, open));
    r.response
}

#[allow(clippy::too_many_arguments)]
fn tarjeta(ui: &mut Ui, e: &mut Event, p: &[usize], h: &Hoja, probs: &[&Problema], out: &mut Salida, act: &mut Option<EAct>, jump: bool) -> egui::Response {
    let ev_probs: Vec<&&Problema> = probs.iter().filter(|x| x.at.is_none()).collect();
    let bad = !probs.is_empty();
    let stroke = if bad {
        Stroke::new(1.5, DANGER)
    } else if jump {
        Stroke::new(1.5, ACCENT)
    } else if e.kind == EventKind::Else {
        Stroke::new(1.5, blend(SI_NO, SURFACE, 0.6))
    } else {
        Stroke::new(1., BORDER)
    };
    let r = Frame::new().fill(SURFACE).stroke(stroke).corner_radius(10).inner_margin(12).show(ui, |ui| {
        ui.set_width(ui.available_width());
        let wide = ui.available_width() >= 600.;
        // Encabezado de tipos especiales.
        let lbl = |ui: &mut Ui, t: &str| drop(ui.label(caps(t, 14., TEXT)));
        if !matches!(e.kind, EventKind::Standard | EventKind::Else) || e.disabled {
            ui.horizontal_wrapped(|ui| {
                match e.kind {
                    EventKind::Repeat => {
                        ui.label(RichText::new(ic::REPEAT).size(17.).color(CAT_TIME));
                        lbl(ui, "REPETIR");
                        ui.add(TextEdit::singleline(&mut e.text).desired_width(70.).hint_text("3").font(FontId::monospace(15.)));
                        lbl(ui, "VECES");
                    }
                    EventKind::ForEach => {
                        ui.label(RichText::new(ic::REPEAT).size(17.).color(CAT_TIME));
                        lbl(ui, "PARA CADA");
                        let st = pill_style(if e.text.is_empty() { Ficha::Vacia } else { Ficha::Normal }, CAT_TIME);
                        let r = pill(ui, if e.text.is_empty() { "elegir" } else { &e.text }, &st, Sense::click());
                        egui::Popup::from_toggle_button_response(&r).close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                            .frame(Frame::popup(ui.style()).inner_margin(14)).show(|ui| {
                                ui.set_max_width(460.);
                                titulo(ui, "¿Para cada qué objeto?");
                                if objetos(ui, &mut e.text, h.l) {
                                    ui.close();
                                }
                            });
                        ui.label(RichText::new("(uno por uno)").color(TEXT_MUTED));
                    }
                    EventKind::While => {
                        ui.label(RichText::new(ic::REPEAT).size(17.).color(CAT_TIME));
                        lbl(ui, "REPETIR MIENTRAS SE CUMPLA LO DE CUANDO");
                    }
                    _ => {}
                }
                if e.disabled {
                    tag(ui, "Desactivado: el juego no lo usa", TEXT_SOFT);
                }
            });
            ui.add_space(4.);
        }
        for pr in &ev_probs {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(ic::WARNING).color(ROJO_TXT));
                ui.label(RichText::new(&pr.msg).color(ROJO_TXT));
            });
        }
        // Botones del encabezado (se agregan de derecha a izquierda).
        let mut extra = |ui: &mut Ui, e: &mut Event| {
            dots(ui, "Opciones del evento", |ui| menu_evento(ui, e, act));
            let r = ui.add(Button::new((RichText::new(ic::ELBOW).size(16.).color(TEXT_SOFT), RichText::new("y además").size(15.).color(TEXT_SOFT)))
                .frame_when_inactive(false).min_size(Vec2::new(0., 32.)))
                .on_hover_text("Agrega un evento adentro de este: «y además, si...». Solo se revisa cuando este se cumple.");
            if r.clicked() {
                e.sub.push(Event::default());
            }
        };
        // Encabezado de columna: los botones de la derecha reservan su lugar; el título y la pista se recortan si no entran.
        let encabezado = |ui: &mut Ui, e: &mut Event, t: &str, tip: &str, pista: Option<String>, extra: Option<&mut dyn FnMut(&mut Ui, &mut Event)>| {
            ui.horizontal(|ui| {
                ui.set_min_height(32.);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if let Some(x) = extra {
                        x(ui, e);
                    }
                    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                        let c = if t.starts_with("SI NO") { SI_NO } else { TEXT };
                        ui.add(egui::Label::new(caps(t, 14., c)).truncate()).on_hover_text(tip);
                        if let Some(p) = pista {
                            ui.add(egui::Label::new(RichText::new(p).color(TEXT_MUTED)).truncate());
                        }
                    });
                });
            });
        };
        let cuando = |ui: &mut Ui, e: &mut Event, out: &mut Salida, extra: Option<&mut dyn FnMut(&mut Ui, &mut Event)>| {
            let si_no = e.kind == EventKind::Else;
            let t = match (si_no, e.conds.is_empty()) {
                (true, true) => "SI NO",
                (true, false) => "SI NO, CUANDO",
                (false, true) => "SIEMPRE",
                _ if e.kind == EventKind::While => "MIENTRAS",
                _ => "CUANDO",
            };
            let pista = match (si_no, e.conds.is_empty()) {
                (true, true) => Some("si lo de arriba no pasó".to_string()),
                (false, true) => Some(format!("{} todo el tiempo", ic::INFINITY)),
                _ => None,
            };
            let tip = if si_no {
                "SI NO: estos bloques se hacen solo si el evento de arriba no se cumplió (y lo de CUANDO, si hay algo, sí se cumple)."
            } else {
                "CUANDO: lo que tiene que pasar. Si pasa todo esto, se hacen los bloques de ENTONCES."
            };
            encabezado(ui, e, t, tip, pista, extra);
            bloques(ui, &mut e.conds, p, true, &[], h, probs, out);
            if add_btn(ui, "Cuando", ACCENT).on_hover_text("Agregar algo que tiene que pasar").clicked() {
                out.open = Some(Open { ev: p.to_vec(), cond: true, ip: vec![], new: true });
            }
        };
        let entonces = |ui: &mut Ui, e: &mut Event, out: &mut Salida, extra: Option<&mut dyn FnMut(&mut Ui, &mut Event)>| {
            let pista = e.acts.is_empty().then(|| "todavía no hace nada".to_string());
            encabezado(ui, e, "ENTONCES", "ENTONCES: lo que el juego hace cuando pasa lo de CUANDO.", pista, extra);
            bloques(ui, &mut e.acts, p, false, &[], h, probs, out);
            if add_btn(ui, "Entonces", ACCENT).on_hover_text("Agregar algo que el juego hace").clicked() {
                out.open = Some(Open { ev: p.to_vec(), cond: false, ip: vec![], new: true });
            }
        };
        if wide {
            ui.horizontal_top(|ui| {
                let gap = 20.;
                let wl = ((ui.available_width() - gap) * 0.46).floor();
                let wr = ui.available_width() - gap - wl - 2. * ui.spacing().item_spacing.x;
                let left = ui.allocate_ui_with_layout(Vec2::new(wl, 0.), Layout::top_down(Align::Min), |ui| {
                    ui.set_width(wl);
                    cuando(ui, e, out, None);
                });
                let x = left.response.rect.right() + gap / 2. + ui.spacing().item_spacing.x / 2.;
                let top = left.response.rect.top();
                ui.add_space(gap);
                let right = ui.allocate_ui_with_layout(Vec2::new(wr, 0.), Layout::top_down(Align::Min), |ui| {
                    ui.set_width(wr);
                    entonces(ui, e, out, Some(&mut extra));
                });
                let bottom = left.response.rect.bottom().max(right.response.rect.bottom());
                ui.painter().line_segment([Pos2::new(x, top + 4.), Pos2::new(x, bottom - 4.)], Stroke::new(1., BORDER));
                ui.painter().text(Pos2::new(x, top + 16.), Align2::CENTER_CENTER, ic::ARROW_RIGHT, FontId::proportional(16.), TEXT_MUTED);
            });
        } else {
            cuando(ui, e, out, Some(&mut extra));
            ui.add_space(6.);
            entonces(ui, e, out, None);
        }
    });
    r.response
}

/// Estado vacío grande de una hoja. Devuelve (nuevo evento, usar receta).
pub fn vacio(ui: &mut Ui, funcion: bool) -> (bool, bool) {
    let (mut a, mut b) = (false, false);
    ui.add_space(40.);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(if funcion { ic::PUZZLE } else { ic::LIST }).size(44.).color(if funcion { CAT_FUNC } else { ACCENT }));
        ui.add_space(6.);
        if funcion {
            ui.label(RichText::new("Tu bloque todavía está vacío.").font(sb(22.)));
            ui.label(RichText::new("Agrega eventos: lo que pongas aquí se hace cada vez que uses el bloque.").size(16.).color(TEXT_SOFT));
        } else {
            ui.label(RichText::new("Aquí le dices a tu juego qué hacer.").font(sb(22.)));
            ui.add_space(2.);
            ui.label(RichText::new("Cada evento dice CUANDO pasa algo, ENTONCES qué hacer.").size(16.).color(TEXT_SOFT));
            ui.label(RichText::new("Por ejemplo: CUANDO el héroe toca una moneda, ENTONCES la moneda desaparece.").size(15.).color(TEXT_MUTED));
        }
        ui.add_space(16.);
    });
    ui.horizontal(|ui| {
        let w = if funcion { 190. } else { 380. };
        ui.add_space(((ui.available_width() - w) / 2.).max(0.));
        ui.spacing_mut().button_padding = Vec2::new(16., 10.);
        a = primary(ui, (on_accent(ic::PLUS).size(18.), on_accent("Nuevo evento").font(sb(16.)))).clicked();
        if !funcion {
            b = ui.add(Button::new((RichText::new(ic::COOKING_POT).size(18.).color(CAT_FUNC), RichText::new("Usar una receta").font(sb(16.))))).clicked();
        }
    });
    (a, b)
}

/// Aviso «Hay N cosas por arreglar». Devuelve true si se pidió mostrar la primera.
pub fn aviso(ui: &mut Ui, n: usize, otros: &[String]) -> bool {
    let mut ver = false;
    if n == 0 && otros.is_empty() {
        return false;
    }
    Frame::new().fill(blend(DANGER, SURFACE, 0.16)).stroke((1., blend(DANGER, SURFACE, 0.5))).corner_radius(10).inner_margin(Margin::symmetric(12, 6)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(ic::WARNING_CIRCLE).size(20.).color(ROJO_TXT));
            if n > 0 {
                let t = if n == 1 { "Hay 1 cosa por arreglar en esta hoja.".to_string() } else { format!("Hay {n} cosas por arreglar en esta hoja.") };
                ui.label(RichText::new(t).font(sb(15.)).color(TEXT));
                ver = ui.add(Button::new((RichText::new(ic::CROSSHAIR), RichText::new("Mostrar"))).min_size(Vec2::new(0., 30.))).clicked();
            }
            if !otros.is_empty() {
                let t = if n > 0 { format!("Y {} en otras partes del juego.", otros.len()) } else if otros.len() == 1 { "Hay 1 cosa por arreglar en otra parte del juego.".into() } else { format!("Hay {} cosas por arreglar en otras partes del juego.", otros.len()) };
                let tip: Vec<String> = otros.iter().take(12).map(|o| amable(o)).collect();
                ui.label(RichText::new(t).color(TEXT_SOFT)).on_hover_text(tip.join("\n"));
                ui.label(RichText::new("(pasa el mouse para verlas)").color(TEXT_MUTED));
            }
        });
    });
    ver
}

// ---------- selector de bloques ----------

pub struct Selector {
    pub at: Open,
    pub target: Target,
    pub search: String,
    pub cat: usize,
    pub mas: bool,
    /// Paso 2: bloque elegido, completando sus fichas.
    pub ins: Option<Instr>,
    /// Se está cambiando un bloque que ya existía.
    pub editing: bool,
    pub focus: bool,
}

impl Selector {
    pub fn new(at: Open, target: Target, ins: Option<Instr>) -> Self {
        let cat = ins.as_ref().map_or(if at.cond { 1 } else { 2 }, |i| CATS.iter().position(|c| std::ptr::eq(c, cat_op(&i.op))).unwrap_or(0));
        Selector { editing: !at.new, at, target, search: String::new(), cat, mas: false, ins, focus: true }
    }
}

#[derive(PartialEq)]
pub enum SelRes {
    Ok,
    Cancel,
    Del,
}

/// Bloque nuevo con valores por defecto pensados para que la frase ya tenga sentido.
pub fn nuevo(m: &Meta, l: &Lists) -> Instr {
    let mut args: Vec<String> = m.params.iter().map(|p| eventos::default_arg(p.1, l)).collect();
    let solo: Vec<&String> = l.objs.iter().filter(|o| !l.groups.contains(o)).collect();
    for (k, (name, t)) in m.params.iter().enumerate() {
        match t {
            P::Object if k > 0 => {
                if let Some(o) = solo.iter().find(|o| ***o != args[0]) {
                    args[k] = (*o).clone();
                }
            }
            P::Object => args[k] = solo.first().map(|s| s.to_string()).unwrap_or_default(),
            P::Var => {
                let g = matches!(m.op, "set_global" | "add_global");
                args[k] = if g { l.globals.first() } else { l.vars.first() }.cloned().unwrap_or_default();
            }
            P::Anim => args[k] = l.anims.get(&args[0]).and_then(|a| a.first()).cloned().unwrap_or_default(),
            P::File => {
                args[k] = l.files.iter().find(|f| [".wav", ".ogg", ".mp3"].iter().any(|e| f.to_lowercase().ends_with(e))).cloned().unwrap_or_default()
            }
            P::Text => args[k] = "reloj".into(),
            P::Expr => {
                args[k] = match (m.op, *name) {
                    ("every", _) => "1",
                    ("set_text", _) => "\"¡Hola!\"",
                    (_, "Volumen 0-100") => "100",
                    ("camera_zoom", _) | ("flip_x", _) | ("return", _) => "1",
                    ("near", _) => "100",
                    ("add", _) => "5",
                    ("add_var" | "add_global", _) => "1",
                    _ => "0",
                }
                .into()
            }
            _ => {}
        }
    }
    if m.op == "compare" {
        if let Some(v) = l.vars.first() {
            args[0] = v.clone();
        }
    }
    Instr { op: m.op.into(), args, ..Default::default() }
}

fn bloque_func(f: &(String, bool, Vec<String>)) -> Instr {
    let op = if f.1 { "call_cond" } else { "call" };
    Instr { op: op.into(), args: std::iter::once(f.0.clone()).chain(f.2.iter().map(|_| "0".to_string())).collect(), ..Default::default() }
}

/// Vista de un bloque del catálogo con fichas vacías: «[objeto] choca con [otro objeto]».
fn muestra(ui: &mut Ui, m: &Meta, c: Color32, params: Option<&[String]>, fname: Option<&str>) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(5., 4.);
        ui.label(RichText::new(op_icon(m.op)).size(18.).color(c));
        let n = params.map_or(1, |p| p.len() + 1);
        frase(ui, m, n, |ui, k| {
            let txt = match (k, fname, params) {
                (0, Some(f), _) => f.to_string(),
                (k, _, Some(p)) if k > 0 => p.get(k - 1).cloned().unwrap_or_default(),
                _ => {
                    let (name, t) = param(m, k);
                    hueco(name, t)
                }
            };
            let st = if k == 0 && fname.is_some() { pill_style(Ficha::Mio, c) } else { pill_style(Ficha::Hueco, c) };
            pill(ui, &txt, &st, Sense::hover());
        });
    });
}

/// Recuadro grande del catálogo. Devuelve true al hacer clic.
fn ficha_catalogo(ui: &mut Ui, w: f32, c: Color32, help: &str, body: impl FnOnce(&mut Ui)) -> bool {
    let r = ui.scope_builder(UiBuilder::new().sense(Sense::click()).layout(Layout::top_down(Align::Min)), |ui| {
        ui.set_width(w);
        Frame::new().fill(tint(c)).corner_radius(10).inner_margin(Margin { left: 16, right: 10, top: 10, bottom: 10 }).show(ui, |ui| {
            ui.set_width(w - 26.);
            body(ui);
            if !help.is_empty() {
                ui.add_space(2.);
                ui.add(egui::Label::new(RichText::new(help).size(15.).color(TEXT_SOFT)).selectable(false).wrap());
            }
        });
    });
    let rect = r.response.rect;
    let p = ui.painter();
    p.rect_filled(Rect::from_min_size(rect.min, Vec2::new(6., rect.height())), CornerRadius { nw: 10, sw: 10, ne: 0, se: 0 }, c);
    if r.response.hovered() {
        p.rect_stroke(rect, 10, Stroke::new(2., c), egui::StrokeKind::Inside);
    }
    r.response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

pub fn selector(ctx: &egui::Context, s: &mut Selector, l: &Lists, pregunta_fn: bool, new_var: &mut Option<(String, bool)>) -> Option<SelRes> {
    let mut res = None;
    let cond = s.at.cond;
    let scr = ctx.content_rect();
    let w = (scr.width() - 80.).min(960.);
    let hgt = (scr.height() - 220.).clamp(260., 560.);
    let frame = Frame::window(&ctx.global_style()).inner_margin(20).corner_radius(12);
    let r = egui::Modal::new(egui::Id::unique("selector_bloques")).frame(frame).show(ctx, |ui| {
        ui.set_width(w);
        if let Some(ins) = s.ins.as_mut() {
            // ---- paso 2: completar ----
            let m = catalog::meta(&ins.op);
            let c = cat_op(&ins.op).color;
            ui.horizontal(|ui| {
                ui.label(RichText::new(ic::PUZZLE).size(22.).color(c));
                ui.label(RichText::new(if s.editing { "Cambia tu bloque" } else { "Completa tu bloque" }).font(sb(22.)));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ib(ui, ic::X, "Cerrar (Esc)").clicked() {
                        res = Some(SelRes::Cancel);
                    }
                });
            });
            ui.label(RichText::new("Elige las opciones de abajo. Arriba ves cómo va quedando tu bloque.").color(TEXT_SOFT));
            ui.add_space(6.);
            let Some(m) = m else {
                ui.label("Bloque desconocido.");
                return;
            };
            // Vista previa con la frase completa.
            let preview = ins.clone();
            Frame::new().fill(tint(c)).corner_radius(10).inner_margin(Margin { left: 16, right: 10, top: 8, bottom: 8 }).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(5., 4.);
                    ui.label(RichText::new(op_icon(&preview.op)).size(20.).color(c));
                    if preview.not {
                        pill(ui, "NO", &Pill { fill: CHIP_NO, fg: Color32::WHITE, stroke: None, icon: None }, Sense::hover());
                    }
                    frase(ui, m, preview.args.len(), |ui, k| {
                        let (_, t) = param(m, k);
                        let (txt, f, _) = ficha(&preview.op, k, t, preview.args.get(k).map_or("", |s| s.as_str()), l);
                        pill(ui, &txt, &pill_style(f, c), Sense::hover());
                    });
                });
            });
            let help = catalog::kid(&ins.op).help;
            if !help.is_empty() {
                ui.label(RichText::new(help).color(TEXT_SOFT));
            }
            ui.add_space(8.);
            egui::ScrollArea::vertical().max_height(hgt - 40.).auto_shrink([false, true]).show(ui, |ui| {
                if ins.args.len() < m.params.len() {
                    ins.args.resize(m.params.len(), String::new());
                }
                let a0 = ins.args.first().cloned().unwrap_or_default();
                let n = ins.args.len().max(m.params.len());
                for k in 0..n {
                    let (pname, t) = param(m, k);
                    let q = if pname == "Ingrediente" {
                        let f = l.funcs.iter().find(|f| Some(&f.0) == ins.args.first());
                        format!("Ingrediente «{}»", f.and_then(|f| f.2.get(k - 1)).cloned().unwrap_or_default())
                    } else {
                        pregunta(&ins.op, k, pname, t)
                    };
                    ui.push_id(k, |ui| {
                        Frame::new().fill(PANEL).corner_radius(10).inner_margin(12).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            titulo(ui, &format!("{}. {q}", k + 1));
                            ui.add_space(4.);
                            let cx = ArgCx { l, op: &ins.op.clone(), k, obj: &a0, cond };
                            editor(ui, t, &mut ins.args[k], &cx, new_var);
                        });
                    });
                    if t == P::Func {
                        ajustar_func(&mut ins.args, l);
                    }
                    ui.add_space(6.);
                }
                if m.params.is_empty() {
                    ui.label(RichText::new("Este bloque no necesita nada más.").color(TEXT_MUTED));
                }
                if cond {
                    ui.add_space(4.);
                    ui.horizontal(|ui| {
                        let icon = if ins.not { ic::TOGGLE_RIGHT } else { ic::TOGGLE_LEFT };
                        let col = if ins.not { CHIP_NO } else { TEXT_MUTED };
                        if ui.add(Button::new((RichText::new(icon).size(24.).color(col), RichText::new("Al revés (NO)").size(15.))).frame_when_inactive(false).min_size(Vec2::new(0., 34.))).clicked() {
                            ins.not = !ins.not;
                        }
                        ui.label(RichText::new("Se cumple cuando esto NO pasa.").color(TEXT_MUTED));
                    });
                }
            });
            ui.add_space(8.);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.add(Button::new(it(ic::ARROW_LEFT, "Elegir otro bloque")).min_size(Vec2::new(0., 34.))).clicked() {
                    s.ins = None;
                    s.focus = true;
                }
                if s.editing && ui.add(Button::new(danger_it("Borrar bloque")).min_size(Vec2::new(0., 34.))).clicked() {
                    res = Some(SelRes::Del);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().button_padding = Vec2::new(18., 8.);
                    if primary(ui, (on_accent(ic::CHECK).size(18.), on_accent("Listo").font(sb(16.)))).clicked() {
                        res = Some(SelRes::Ok);
                    }
                });
            });
            return;
        }
        // ---- paso 1: elegir ----
        let (t, sub) = if cond {
                ("¿CUÁNDO pasa?", "Elige qué tiene que pasar. Pasa el mouse sobre un bloque para ver qué hace.")
            } else {
                ("¿Qué HACER?", "Elige qué hace el juego. Pasa el mouse sobre un bloque para ver qué hace.")
            };
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ib(ui, ic::X, "Cerrar (Esc)").clicked() {
                    res = Some(SelRes::Cancel);
                }
                let sw = (ui.available_width() - 200.).clamp(140., 240.);
                let r = ui.add(TextEdit::singleline(&mut s.search).hint_text(format!("{}  Buscar un bloque", ic::SEARCH)).desired_width(sw).font(FontId::proportional(16.)));
                if s.focus {
                    r.request_focus();
                    s.focus = false;
                }
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    ui.add(egui::Label::new(RichText::new(t).font(sb(24.))).truncate());
                });
            });
        });
        ui.add(egui::Label::new(RichText::new(sub).color(TEXT_SOFT)).wrap());
        ui.add_space(10.);
        let q = norm(s.search.trim());
        let mine: Vec<&(String, bool, Vec<String>)> = l.funcs.iter().filter(|f| f.1 == cond).collect();
        let fits = |m: &&Meta| m.cond == cond && !matches!(m.op, "call" | "call_cond") && (m.op != "return" || pregunta_fn);
        let matches = |m: &Meta| {
            let k = catalog::kid(m.op);
            [m.label, k.help, frase_de(m), cat(m.group).name].iter().any(|t| norm(t).contains(&q))
        };
        ui.horizontal_top(|ui| {
            // Categorías
            ui.vertical(|ui| {
                ui.set_width(210.);
                ui.spacing_mut().item_spacing.y = 4.;
                for (i, c) in CATS.iter().enumerate() {
                    let n = if i == MIS_BLOQUES { mine.len() + (pregunta_fn && !cond) as usize } else { CATALOG.iter().filter(fits).filter(|m| cat(m.group).name == c.name).count() };
                    if n == 0 && i != MIS_BLOQUES {
                        continue;
                    }
                    let sel = q.is_empty() && s.cat == i;
                    let (fill, fg) = if sel { (c.color, ON_CAT) } else { (Color32::TRANSPARENT, TEXT) };
                    let b = Button::new((RichText::new(c.icon).size(19.).color(if sel { ON_CAT } else { c.color }), RichText::new(c.name).font(sb(15.)).color(fg)))
                        .fill(fill).stroke(Stroke::NONE).corner_radius(8).min_size(Vec2::new(210., 40.));
                    let r = ui.add(b);
                    if !sel {
                        let rr = r.rect;
                        ui.painter().rect_filled(Rect::from_min_size(rr.min + Vec2::new(0., 8.), Vec2::new(4., rr.height() - 16.)), 2, c.color);
                    }
                    if r.clicked() {
                        s.cat = i;
                        s.search.clear();
                    }
                }
                ui.add_space(10.);
                let icon = if s.mas { ic::TOGGLE_RIGHT } else { ic::TOGGLE_LEFT };
                if ui.add(Button::new((RichText::new(icon).size(24.).color(if s.mas { ACCENT } else { TEXT_MUTED }), RichText::new("Ver más bloques").size(15.)))
                    .frame_when_inactive(false).min_size(Vec2::new(0., 34.))).on_hover_text("Muestra también los bloques para cosas más avanzadas").clicked() {
                    s.mas = !s.mas;
                }
            });
            ui.add_space(8.);
            // Bloques
            // Dos columnas de bloques si entran; si no, una.
            let cols = if ui.available_width() >= 560. { 2 } else { 1 };
            let tw = ((ui.available_width() - 12. * cols as f32) / cols as f32).floor();
            egui::ScrollArea::vertical().id_salt("tiles").max_height(hgt).min_scrolled_height(hgt).auto_shrink([false, false]).show(ui, |ui| ui.vertical(|ui| {
                let mut picks: Vec<(&Meta, bool)> = CATALOG.iter().filter(fits)
                    .filter(|m| if q.is_empty() { cat(m.group).name == CATS[s.cat].name } else { matches(m) })
                    .map(|m| (m, catalog::kid(m.op).basic || (m.op == "return" && pregunta_fn)))
                    .collect();
                let hidden = picks.iter().filter(|p| !p.1).count();
                if !s.mas && q.is_empty() {
                    picks.retain(|p| p.1);
                }
                let show_mine = (q.is_empty() && s.cat == MIS_BLOQUES) || (!q.is_empty() && !mine.is_empty());
                let mine_f: Vec<&&(String, bool, Vec<String>)> = mine.iter().filter(|f| q.is_empty() || norm(&f.0).contains(&q)).collect();
                if picks.is_empty() && (!show_mine || mine_f.is_empty()) {
                    ui.add_space(20.);
                    if q.is_empty() && s.cat == MIS_BLOQUES {
                        ui.label(RichText::new("Todavía no inventaste bloques de este tipo.").size(16.));
                        ui.label(RichText::new("Ve a la pestaña «Mis bloques» para crear uno.").color(TEXT_MUTED));
                    } else {
                        ui.label(RichText::new("No encontré bloques con esa palabra.").size(16.).color(TEXT_MUTED));
                    }
                }
                let mut items: Vec<Box<dyn FnOnce(&mut Ui) -> Option<Instr> + '_>> = vec![];
                if show_mine {
                    for f in mine_f {
                        let fm = catalog::meta(if f.1 { "call_cond" } else { "call" }).unwrap();
                        items.push(Box::new(move |ui: &mut Ui| {
                            let help = if f.2.is_empty() { "Un bloque que inventaste.".to_string() } else { format!("Un bloque que inventaste. Ingredientes: {}.", f.2.join(", ")) };
                            ficha_catalogo(ui, tw, CAT_FUNC, &help, |ui| muestra(ui, fm, CAT_FUNC, Some(&f.2), Some(&f.0))).then(|| bloque_func(f))
                        }));
                    }
                }
                for (m, _) in picks {
                    let c = cat(m.group).color;
                    items.push(Box::new(move |ui: &mut Ui| {
                        let help = catalog::kid(m.op).help;
                        ficha_catalogo(ui, tw, c, help, |ui| muestra(ui, m, c, None, None)).then(|| nuevo(m, l))
                    }));
                }
                let mut it_ = items.into_iter();
                loop {
                    let a = it_.next();
                    let Some(a) = a else { break };
                    let b = if cols == 2 { it_.next() } else { None };
                    ui.horizontal_top(|ui| {
                        if let Some(ins) = a(ui) {
                            s.ins = Some(ins);
                        }
                        if let Some(b) = b {
                            if let Some(ins) = b(ui) {
                                s.ins = Some(ins);
                            }
                        }
                    });
                    ui.add_space(4.);
                }
                if hidden > 0 && !s.mas && q.is_empty() {
                    ui.add_space(6.);
                    if ui.button(format!("Ver {hidden} bloques más de esta categoría")).clicked() {
                        s.mas = true;
                    }
                }
            }));
        });
        // Elegido: si no necesita nada, listo.
        if let Some(ins) = &s.ins {
            if catalog::meta(&ins.op).is_some_and(|m| m.params.is_empty()) {
                res = Some(SelRes::Ok);
            }
        }
    });
    if res.is_none() && s.ins.is_some() && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Enter)) && !ctx.egui_wants_keyboard_input() {
        res = Some(SelRes::Ok);
    }
    if res.is_none() && r.should_close() {
        res = Some(SelRes::Cancel);
    }
    res
}

/// Inserta, reemplaza o borra la instrucción elegida en el selector.
pub fn aplicar(evs: &mut [Event], at: &Open, ins: &Instr, del: bool) {
    match eventos::locate(evs, at) {
        Some((l, None)) if !del => l.push(ins.clone()),
        Some((l, Some(k))) if del => drop(l.remove(k)),
        Some((l, Some(k))) => l[k] = ins.clone(),
        _ => {}
    }
}

/// Cajitas que usa una instrucción y que no existen todavía: (nombre, del juego).
pub fn cajitas_faltantes(ins: &Instr, l: &Lists) -> Vec<(String, bool)> {
    let Some(m) = catalog::meta(&ins.op) else { return vec![] };
    m.params.iter().zip(&ins.args)
        .filter(|((_, t), a)| *t == P::Var && ident_ok(a) && !l.vars.contains(a) && !l.globals.contains(a))
        .map(|(_, a)| (a.clone(), matches!(ins.op.as_str(), "set_global" | "add_global")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ubica_problemas() {
        let all = vec![
            "Escena «Nivel» › evento 2.1 › condición 1: argumento 2 (Otro) «Perro»: objeto o grupo desconocido".to_string(),
            "Escena «Nivel» › evento 3 › acción 2 › argumento 3: expresión «1 +»: la expresión termina antes de tiempo".to_string(),
            "Escena «Menu» › evento 1: algo".to_string(),
        ];
        let (h, o) = problemas(&all, "Escena «Nivel»");
        assert_eq!(o.len(), 1);
        assert_eq!(h[0].ev, vec![1, 0]);
        assert_eq!(h[0].at, Some((true, 0)));
        assert!(h[0].msg.contains("«Perro», que no existe"), "{}", h[0].msg);
        assert_eq!(h[1].at, Some((false, 1)));
        assert!(h[1].msg.contains("falta algo al final"), "{}", h[1].msg);
        assert_eq!(ident_fix("comer manzana ñ"), "comer_manzana_n");
        assert!(has_ident("80 * dt", "dt") && !has_ident("Obj.dt", "dt") && !has_ident("\"dt\"", "dt"));
    }
}
