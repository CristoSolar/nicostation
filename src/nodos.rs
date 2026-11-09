//! Grafos de nodos (como los Blueprints de Unreal): otra forma de ver y guardar la misma lógica.
//! Un grafo se compila a eventos normales (`compile`), así que el motor no cambia; y cualquier hoja
//! de eventos se puede convertir en un grafo legible (`from_events`).
//!
//! Hay dos tipos de cables:
//! - flechas (`flow`): el orden en que pasan las cosas. Cada salida va a un solo nodo; un nodo puede recibir varias.
//! - datos (`wires`): un valor (número, texto, sí/no u objeto) que llega a un parámetro. Se compilan como expresión.
//!
//! Compilación: un camino Evento → Si… → acciones es un evento estándar (condiciones = la del evento + los «Si»;
//! acciones = los nodos de acción). Un «Si» después de acciones, un «Si» con salida «no», una Secuencia o un
//! Repetir abren sub-eventos (y «si no») que conservan la semántica de la hoja. Los eventos salen ordenados por
//! la posición de su nodo de evento: de arriba a abajo y de izquierda a derecha.

use crate::catalog::{self, P};
use crate::expr::{self, Expr};
use crate::model::{Event, EventKind, Instr, Project};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub type NodeId = u32;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Graph {
    pub name: String,
    #[serde(default)]
    pub nodes: Vec<Node>,
    /// Flechas: de la salida `out` de `from` a la entrada de `to`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flow: Vec<Flow>,
    /// Cables de datos: de la salida del nodo de datos `from` al parámetro `arg` de `to`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wires: Vec<Wire>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Node {
    pub id: NodeId,
    #[serde(flatten)]
    pub kind: NodeKind,
    #[serde(default)]
    pub x: f32,
    #[serde(default)]
    pub y: f32,
    /// Valores escritos en el nodo, uno por parámetro (si un parámetro tiene cable, el cable manda).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Condición invertida (eventos y «Si»).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub not: bool,
    /// Sub-condiciones de «O» / «Y».
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sub: Vec<Instr>,
    /// Desactivado: el evento que empieza en este nodo no se ejecuta.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub off: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "k", rename_all = "snake_case")]
pub enum NodeKind {
    /// Punto de entrada. `op` = condición del catálogo; vacío = «Cada cuadro».
    Event { op: String },
    /// «Si…»: condición del catálogo con salidas «sí» (0) y «no» (1).
    Cond { op: String },
    /// Acción del catálogo.
    Act { op: String },
    /// Varias salidas que se recorren en orden.
    Seq { n: u8 },
    /// Repite «cada vez» (0) `args[0]` veces; luego sigue por «al terminar» (1).
    Repeat,
    /// Para cada instancia del objeto `args[0]`.
    ForEach,
    /// Mientras se cumplan los «Si» del principio de «mientras» (0).
    While,
    /// Recuadro con texto (`args[0]`); no hace nada. En la hoja vista como nodos es una nota.
    Comment { w: f32, h: f32 },
    /// Recuadro de grupo (título en `args[0]`): en la hoja vista como nodos, los eventos que quedan
    /// adentro forman un grupo. En un grafo aparte no hace nada.
    Group { w: f32, h: f32 },
    /// Nota dentro de un camino (texto en `args[0]`): en la hoja vista como nodos, una nota entre los sub-eventos.
    /// Va sola en una salida de una Secuencia.
    Note,
    /// Nodo de datos puro (ver `DATA`).
    Data { f: String },
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Flow {
    pub from: NodeId,
    #[serde(default)]
    pub out: u8,
    pub to: NodeId,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Wire {
    pub from: NodeId,
    pub to: NodeId,
    pub arg: u8,
}

/// Tipo de un pin de datos.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinT {
    Num,
    Text,
    Bool,
    Obj,
    /// Cualquier valor (número, texto o sí/no).
    Any,
}

/// ¿Se puede enchufar una salida `src` en una entrada `dst`?
pub fn compatible(src: PinT, dst: PinT) -> bool {
    use PinT::*;
    match (src, dst) {
        (Obj, Obj) => true,
        (Obj, _) | (_, Obj) => false,
        (_, Any) | (Any, _) => true,
        (Bool, Num) | (Num, Bool) => true,
        (a, b) => a == b,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphError {
    pub node: Option<NodeId>,
    pub msg: String,
}

// ---------- tablas ----------

/// Condiciones que sirven como nodo de evento (además de «Cada cuadro»).
pub const TRIGGERS: &[&str] = &[
    "start", "every", "timer", "key_pressed", "key_down", "key_released", "mouse_clicked", "mouse_down", "cursor_on", "collides", "near",
    "alarm", "animation_finished",
];

/// Cómo se escribe un nodo de datos como expresión.
#[derive(Clone, Copy, Debug)]
pub enum Fmt {
    Num,
    Str,
    Bool,
    /// El primer campo tal cual (nombre de cajita u objeto).
    Name,
    /// `Objeto.dato`
    Prop,
    Const(&'static str),
    Bin(&'static str, u8),
    Cmp(&'static str),
    And,
    Or,
    Not,
    Call(&'static str),
    Join,
    Raw,
}

/// Entrada de un nodo de datos: nombre, tipo de pin (None = solo se escribe en el nodo), editor y valor inicial.
pub struct In {
    pub name: &'static str,
    pub pin: Option<PinT>,
    pub p: P,
    pub def: &'static str,
}

pub struct DataDef {
    pub f: &'static str,
    pub group: &'static str,
    pub label: &'static str,
    pub help: &'static str,
    pub ins: &'static [In],
    pub out: PinT,
    pub fmt: Fmt,
}

macro_rules! i {
    ($n:literal, $t:ident, $p:ident, $d:literal) => {
        In { name: $n, pin: Some(PinT::$t), p: P::$p, def: $d }
    };
    ($n:literal, -, $p:ident, $d:literal) => {
        In { name: $n, pin: None, p: P::$p, def: $d }
    };
}

const NUM2: &[In] = &[i!("A", Num, Expr, "0"), i!("B", Num, Expr, "1")];
const NUM1: &[In] = &[i!("Valor", Num, Expr, "0")];
const ANY2: &[In] = &[i!("A", Any, Expr, "0"), i!("B", Any, Expr, "0")];
const BOOL2: &[In] = &[i!("A", Bool, Expr, "true"), i!("B", Bool, Expr, "true")];
const XY4: &[In] = &[i!("X1", Num, Expr, "0"), i!("Y1", Num, Expr, "0"), i!("X2", Num, Expr, "0"), i!("Y2", Num, Expr, "0")];
const TXT1: &[In] = &[i!("Texto", Any, Expr, "\"\"")];

macro_rules! d {
    ($f:literal, $g:literal, $l:literal, $h:literal, $ins:expr, $o:ident, $fmt:expr) => {
        DataDef { f: $f, group: $g, label: $l, help: $h, ins: $ins, out: PinT::$o, fmt: $fmt }
    };
}

pub const DATA: &[DataDef] = &[
    // ---- valores ----
    d!("num", "Valores", "Número", "Un número fijo, como 10 o 2.5.", &[i!("Número", -, Expr, "0")], Num, Fmt::Num),
    d!("text", "Valores", "Texto", "Un texto fijo, como ¡Ganaste!", &[i!("Texto", -, Text, "Hola")], Text, Fmt::Str),
    d!("bool", "Valores", "Sí o no", "Un valor que es sí o no.", &[i!("Valor", -, Text, "1")], Bool, Fmt::Bool),
    d!("obj", "Valores", "Objeto", "Un objeto de la escena, para enchufarlo donde se pide un objeto.", &[i!("Objeto", -, Object, "")], Obj, Fmt::Name),
    d!("var", "Valores", "Cajita", "Lo que hay guardado en una cajita (variable) de la escena.", &[i!("Cajita", -, Var, "")], Any, Fmt::Name),
    d!("global", "Valores", "Cajita del juego", "Lo que hay guardado en una cajita de todo el juego.", &[i!("Cajita", -, Var, "")], Any, Fmt::Name),
    d!("prop", "Valores", "Dato de un objeto", "Un dato de un objeto: su X, su Y, su ángulo o una cajita suya.", &[i!("Objeto", Obj, Object, ""), i!("Dato", -, Prop, "x")], Any, Fmt::Prop),
    d!("count", "Valores", "Cuántos hay", "Cuántas copias de un objeto hay en la escena.", &[i!("Objeto", Obj, Object, "")], Num, Fmt::Call("count")),
    d!("dt", "Valores", "Tiempo de un cuadro (dt)", "Lo que dura un cuadro. Multiplicar por dt quiere decir «por segundo».", &[], Num, Fmt::Const("dt")),
    d!("time", "Valores", "Segundos de juego", "Cuántos segundos pasaron desde que empezó la escena.", &[], Num, Fmt::Const("time")),
    d!("mousex", "Valores", "X del mouse", "Dónde está el mouse, de izquierda a derecha.", &[], Num, Fmt::Const("mousex")),
    d!("mousey", "Valores", "Y del mouse", "Dónde está el mouse, de arriba a abajo.", &[], Num, Fmt::Const("mousey")),
    d!("expr", "Valores", "Cuenta escrita", "Una expresión escrita a mano, como Heroe.x + 10.", &[i!("Expresión", -, Expr, "0")], Any, Fmt::Raw),
    // ---- números ----
    d!("add", "Números", "Sumar (+)", "Suma dos números.", NUM2, Num, Fmt::Bin("+", 5)),
    d!("sub", "Números", "Restar (-)", "Resta el segundo número al primero.", NUM2, Num, Fmt::Bin("-", 5)),
    d!("mul", "Números", "Multiplicar (×)", "Multiplica dos números.", NUM2, Num, Fmt::Bin("*", 6)),
    d!("div", "Números", "Dividir (÷)", "Divide el primer número por el segundo.", NUM2, Num, Fmt::Bin("/", 6)),
    d!("mod", "Números", "Resto (%)", "Lo que sobra al dividir: 7 % 3 es 1.", NUM2, Num, Fmt::Bin("%", 6)),
    d!("min", "Números", "El menor", "El más chico de dos números.", NUM2, Num, Fmt::Call("min")),
    d!("max", "Números", "El mayor", "El más grande de dos números.", NUM2, Num, Fmt::Call("max")),
    d!("abs", "Números", "Sin signo", "El número sin el signo menos: -5 da 5.", NUM1, Num, Fmt::Call("abs")),
    d!("round", "Números", "Redondear", "Redondea al número entero más cercano.", NUM1, Num, Fmt::Call("round")),
    d!("random", "Números", "Al azar", "Un número al azar entre dos números.", &[i!("Desde", Num, Expr, "0"), i!("Hasta", Num, Expr, "10")], Num, Fmt::Call("random")),
    d!("randint", "Números", "Entero al azar", "Un número entero al azar, como tirar un dado.", &[i!("Desde", Num, Expr, "1"), i!("Hasta", Num, Expr, "6")], Num, Fmt::Call("randint")),
    d!("clamp", "Números", "Entre dos límites", "Deja el valor entre un mínimo y un máximo.", &[i!("Valor", Num, Expr, "0"), i!("Mínimo", Num, Expr, "0"), i!("Máximo", Num, Expr, "100")], Num, Fmt::Call("clamp")),
    d!("lerp", "Números", "Mezclar", "Un valor entre dos: con 0 da el primero, con 1 el segundo, con 0.5 la mitad.", &[i!("Desde", Num, Expr, "0"), i!("Hasta", Num, Expr, "100"), i!("Cuánto (0 a 1)", Num, Expr, "0.5")], Num, Fmt::Call("lerp")),
    d!("sin", "Números", "Seno", "Seno de un ángulo en grados (sirve para ir y volver).", NUM1, Num, Fmt::Call("sin")),
    d!("cos", "Números", "Coseno", "Coseno de un ángulo en grados.", NUM1, Num, Fmt::Call("cos")),
    d!("sqrt", "Números", "Raíz cuadrada", "La raíz cuadrada de un número.", NUM1, Num, Fmt::Call("sqrt")),
    d!("distance", "Números", "Distancia", "Distancia entre dos puntos.", XY4, Num, Fmt::Call("distance")),
    d!("angle", "Números", "Ángulo hacia", "Ángulo (en grados) para mirar del primer punto al segundo.", XY4, Num, Fmt::Call("angle")),
    // ---- comparar y lógica ----
    d!("eq", "Comparar", "Es igual (==)", "Sí si los dos valores son iguales.", ANY2, Bool, Fmt::Cmp("==")),
    d!("ne", "Comparar", "Es distinto (!=)", "Sí si los dos valores son distintos.", ANY2, Bool, Fmt::Cmp("!=")),
    d!("lt", "Comparar", "Es menor (<)", "Sí si el primero es más chico.", ANY2, Bool, Fmt::Cmp("<")),
    d!("le", "Comparar", "Menor o igual (<=)", "Sí si el primero es más chico o igual.", ANY2, Bool, Fmt::Cmp("<=")),
    d!("gt", "Comparar", "Es mayor (>)", "Sí si el primero es más grande.", ANY2, Bool, Fmt::Cmp(">")),
    d!("ge", "Comparar", "Mayor o igual (>=)", "Sí si el primero es más grande o igual.", ANY2, Bool, Fmt::Cmp(">=")),
    d!("and", "Comparar", "Y", "Sí solo si los dos son sí.", BOOL2, Bool, Fmt::And),
    d!("or", "Comparar", "O", "Sí si alguno de los dos es sí.", BOOL2, Bool, Fmt::Or),
    d!("not", "Comparar", "No", "Da vuelta: sí pasa a no y no pasa a sí.", &[i!("Valor", Bool, Expr, "true")], Bool, Fmt::Not),
    d!("if", "Comparar", "Elegir según", "Si la condición es sí da el primer valor; si no, el segundo.", &[i!("Condición", Bool, Expr, "true"), i!("Si sí", Any, Expr, "1"), i!("Si no", Any, Expr, "0")], Any, Fmt::Call("if")),
    // ---- texto ----
    d!("join", "Texto", "Unir textos", "Pega dos textos (o un texto y un número): \"Puntos: \" y 5 da Puntos: 5.", &[i!("A", Any, Expr, "\"Puntos: \""), i!("B", Any, Expr, "0")], Text, Fmt::Join),
    d!("len", "Texto", "Largo del texto", "Cuántas letras tiene un texto.", TXT1, Num, Fmt::Call("len")),
    d!("upper", "Texto", "En mayúsculas", "El texto con letras grandes.", TXT1, Text, Fmt::Call("upper")),
    d!("lower", "Texto", "En minúsculas", "El texto con letras chicas.", TXT1, Text, Fmt::Call("lower")),
    d!("str", "Texto", "Número a texto", "Convierte un número en texto.", &[i!("Valor", Any, Expr, "0")], Text, Fmt::Call("str")),
    d!("tonum", "Texto", "Texto a número", "Convierte un texto como \"12\" en el número 12.", TXT1, Num, Fmt::Call("num")),
];

pub fn data_def(f: &str) -> Option<&'static DataDef> {
    DATA.iter().find(|d| d.f == f)
}

/// Un parámetro de un nodo (lo que se escribe en él o le llega por cable).
#[derive(Clone, Debug)]
pub struct ArgSpec {
    pub name: String,
    pub p: P,
    /// Tipo del pin de entrada; None = solo se escribe en el nodo.
    pub pin: Option<PinT>,
}

fn pin_of(p: P) -> Option<PinT> {
    match p {
        P::Expr => Some(PinT::Any),
        P::Object => Some(PinT::Obj),
        _ => None,
    }
}

/// Parámetros de un nodo, en el orden de `args`.
pub fn arg_specs(n: &Node) -> Vec<ArgSpec> {
    let a = |name: &str, p: P, pin: Option<PinT>| ArgSpec { name: name.into(), p, pin };
    match &n.kind {
        NodeKind::Event { op } | NodeKind::Cond { op } | NodeKind::Act { op } => {
            let Some(m) = catalog::meta(op) else { return vec![] };
            let mut v: Vec<ArgSpec> = m.params.iter().map(|&(name, p)| a(name, p, pin_of(p))).collect();
            if m.params.last().is_some_and(|p| p.1 == P::Func) {
                for k in m.params.len()..n.args.len() {
                    v.push(a(&format!("Ingrediente {k}"), P::Expr, Some(PinT::Any)));
                }
            }
            v
        }
        NodeKind::Repeat => vec![a("Veces", P::Expr, Some(PinT::Num))],
        NodeKind::ForEach => vec![a("Objeto", P::Object, Some(PinT::Obj))],
        NodeKind::Data { f } => data_def(f).map_or(vec![], |d| d.ins.iter().map(|i| a(i.name, i.p, i.pin)).collect()),
        NodeKind::Seq { .. } | NodeKind::While | NodeKind::Comment { .. } | NodeKind::Group { .. } | NodeKind::Note => vec![],
    }
}

/// ¿Es un recuadro (nota o grupo)?
pub fn is_frame(n: &Node) -> bool {
    matches!(n.kind, NodeKind::Comment { .. } | NodeKind::Group { .. })
}

/// ¿Tiene entrada de flecha?
pub fn has_exec_in(n: &Node) -> bool {
    matches!(n.kind, NodeKind::Cond { .. } | NodeKind::Act { .. } | NodeKind::Seq { .. } | NodeKind::Repeat | NodeKind::ForEach | NodeKind::While | NodeKind::Note)
}

/// Salidas de flecha, con su etiqueta.
pub fn exec_outs(n: &Node) -> Vec<String> {
    let v = |l: &[&str]| l.iter().map(|s| s.to_string()).collect();
    match &n.kind {
        NodeKind::Event { .. } | NodeKind::Act { .. } => v(&[""]),
        NodeKind::Cond { .. } => v(&["sí", "no"]),
        NodeKind::Seq { n } => (1..=(*n).max(1)).map(|k| k.to_string()).collect(),
        NodeKind::Repeat => v(&["cada vez", "al terminar"]),
        NodeKind::ForEach => v(&["cada uno", "al terminar"]),
        NodeKind::While => v(&["mientras", "al terminar"]),
        NodeKind::Comment { .. } | NodeKind::Group { .. } | NodeKind::Note | NodeKind::Data { .. } => vec![],
    }
}

/// Tipo de la salida de datos (solo los nodos de datos tienen).
pub fn data_out(n: &Node) -> Option<PinT> {
    match &n.kind {
        NodeKind::Data { f } => data_def(f).map(|d| d.out),
        _ => None,
    }
}

/// Título del nodo (con los nombres del catálogo).
pub fn title(n: &Node) -> String {
    let lab = |op: &str| catalog::meta(op).map_or(format!("¿{op}?"), |m| m.label.to_string());
    match &n.kind {
        NodeKind::Event { op } if op.is_empty() => "Cada cuadro".into(),
        NodeKind::Event { op } => lab(op),
        NodeKind::Cond { op } => format!("Si: {}", lab(op)),
        NodeKind::Act { op } => lab(op),
        NodeKind::Seq { .. } => "Secuencia".into(),
        NodeKind::Repeat => "Repetir".into(),
        NodeKind::ForEach => "Para cada objeto".into(),
        NodeKind::While => "Mientras".into(),
        NodeKind::Comment { .. } => "Comentario".into(),
        NodeKind::Group { .. } => "Grupo".into(),
        NodeKind::Note => "Nota".into(),
        NodeKind::Data { f } => data_def(f).map_or(format!("¿{f}?"), |d| d.label.to_string()),
    }
}

// ---------- tamaño (el editor dibuja con estas medidas; el auto-acomodo las usa) ----------

pub const NODE_W: f32 = 250.;
pub const HEAD_H: f32 = 32.;
pub const ROW_H: f32 = 28.;
pub const PAD: f32 = 8.;
/// Alto de una línea de texto de una nota en un camino.
pub const NOTE_LINE: f32 = 18.;

/// Líneas que ocupa el texto de una nota en un camino (entre 1 y 6).
pub fn note_lines(t: &str) -> usize {
    t.split('\n').map(|l| l.chars().count().div_ceil(30).max(1)).sum::<usize>().clamp(1, 6)
}

/// Filas del cuerpo de un nodo, de arriba a abajo.
#[derive(Clone, Debug, PartialEq)]
pub enum Row {
    /// Salida de flecha extra (la 0 va en la cabecera).
    Out(u8),
    /// Parámetro `args[k]`.
    Arg(usize),
    /// Sub-condición de «O» / «Y».
    Sub(usize),
    /// Botón para agregar una sub-condición.
    AddSub,
    /// Botones + / - de una Secuencia.
    SeqButtons,
}

pub fn rows(n: &Node) -> Vec<Row> {
    let outs = exec_outs(n).len();
    let mut v: Vec<Row> = (1..outs).map(|k| Row::Out(k as u8)).collect();
    if matches!(n.kind, NodeKind::Seq { .. }) {
        v.push(Row::SeqButtons);
    }
    v.extend((0..arg_specs(n).len()).map(Row::Arg));
    if matches!(&n.kind, NodeKind::Cond { op } | NodeKind::Event { op } if op == "or" || op == "and") {
        v.extend((0..n.sub.len()).map(Row::Sub));
        v.push(Row::AddSub);
    }
    v
}

/// Ancho y alto del nodo en el lienzo.
pub fn size(n: &Node) -> (f32, f32) {
    if let NodeKind::Comment { w, h } | NodeKind::Group { w, h } = n.kind {
        return (w.max(120.), h.max(60.));
    }
    if let NodeKind::Note = n.kind {
        return (NODE_W, HEAD_H + PAD + NOTE_LINE * note_lines(n.args.first().map_or("", |s| s.as_str())) as f32);
    }
    let r = rows(n).len() as f32;
    (NODE_W, HEAD_H + r * ROW_H + if r > 0. { PAD } else { 0. })
}

// ---------- compilación ----------

/// De qué nodo salió cada parte de los eventos compilados (para ubicar errores de validación).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Origin {
    /// Nodo que abrió el evento (el de evento, el primer «Si», el Repetir...).
    pub ev: NodeId,
    pub conds: Vec<NodeId>,
    pub acts: Vec<NodeId>,
    pub sub: Vec<Origin>,
    /// Otros nodos que forman el evento (el «Cada cuadro» que lo abre, una Secuencia...). Solo en la hoja.
    pub extra: Vec<NodeId>,
}

#[derive(Clone, Debug, Default)]
pub struct Compiled {
    pub events: Vec<Event>,
    /// Paralelo a `events`.
    pub origins: Vec<Origin>,
    pub errors: Vec<GraphError>,
    /// Nodos que quedaron en los eventos (solo en la hoja vista como nodos: los demás son borradores).
    pub used: BTreeSet<NodeId>,
}

struct Out {
    ev: Event,
    org: Origin,
}

impl Out {
    fn new(id: NodeId) -> Self {
        Out { ev: Event::default(), org: Origin { ev: id, ..Default::default() } }
    }
    fn empty(&self) -> bool {
        self.ev.kind == EventKind::Standard && self.ev.conds.is_empty() && self.ev.acts.is_empty() && !self.ev.disabled
    }
}

fn split(v: Vec<Out>) -> (Vec<Event>, Vec<Origin>) {
    v.into_iter().map(|o| (o.ev, o.org)).unzip()
}

/// Resultado de un tramo: eventos y si salieron «sueltos» (de una Secuencia o un Repetir sin evento propio).
type Block = (Vec<Out>, bool);

static NONE: BTreeSet<NodeId> = BTreeSet::new();

struct Cx<'a> {
    g: &'a Graph,
    idx: BTreeMap<NodeId, usize>,
    flow: BTreeMap<(NodeId, u8), NodeId>,
    wires: BTreeMap<(NodeId, u8), NodeId>,
    errs: Vec<GraphError>,
    budget: usize,
    /// Hoja vista como nodos: notas y grupos son eventos, no se desarma nada y lo que está mal se salta.
    sheet: bool,
    /// Nodos que vienen de la hoja: pasan tal cual aunque estén incompletos (como en la hoja).
    lenient: &'a BTreeSet<NodeId>,
    /// Veces que un «Si» que está mal cortó un camino.
    cut: usize,
}

const REQUIRED: &[P] = &[P::Expr, P::Object, P::Key, P::Mouse, P::Op, P::Scene, P::Var, P::Func, P::Prop, P::Ease];

impl<'a> Cx<'a> {
    fn new(g: &'a Graph) -> Self {
        let mut c = Cx { g, idx: BTreeMap::new(), flow: BTreeMap::new(), wires: BTreeMap::new(), errs: vec![], budget: 20_000, sheet: false, lenient: &NONE, cut: 0 };
        for (k, n) in g.nodes.iter().enumerate() {
            if c.idx.insert(n.id, k).is_some() {
                c.err(Some(n.id), "hay dos nodos con el mismo número".into());
            }
        }
        for f in &g.flow {
            let ok = c.node(f.from).is_some_and(|n| (f.out as usize) < exec_outs(n).len()) && c.node(f.to).is_some_and(has_exec_in);
            if !ok {
                c.err(Some(f.from), format!("flecha inválida {}:{} → {}", f.from, f.out, f.to));
            } else if c.flow.insert((f.from, f.out), f.to).is_some() {
                c.err(Some(f.from), "una salida de flecha solo puede ir a un nodo".into());
            }
        }
        for w in &g.wires {
            let ok = c.node(w.from).is_some_and(|n| data_out(n).is_some())
                && c.node(w.to).is_some_and(|n| arg_specs(n).get(w.arg as usize).is_some_and(|a| a.pin.is_some()));
            if !ok {
                c.err(Some(w.to), format!("cable inválido {} → {}:{}", w.from, w.to, w.arg));
            } else if c.wires.insert((w.to, w.arg), w.from).is_some() {
                c.err(Some(w.to), "a un parámetro solo le puede llegar un cable".into());
            }
        }
        c
    }

    fn node(&self, id: NodeId) -> Option<&'a Node> {
        self.idx.get(&id).map(|&k| &self.g.nodes[k])
    }

    fn err(&mut self, node: Option<NodeId>, msg: String) {
        let e = GraphError { node, msg };
        if !self.errs.contains(&e) {
            self.errs.push(e);
        }
    }

    fn next(&self, id: NodeId, out: u8) -> Option<NodeId> {
        self.flow.get(&(id, out)).copied()
    }

    /// ¿El nodo viene de la hoja (y pasa tal cual)?
    fn from_sheet(&self, id: NodeId) -> bool {
        self.sheet && self.lenient.contains(&id)
    }

    // ---- datos ----

    /// Expresión de un nodo de datos: (texto, precedencia, tipo).
    fn data(&mut self, id: NodeId, seen: &mut Vec<NodeId>) -> Option<(String, u8, PinT)> {
        if seen.contains(&id) {
            self.err(Some(id), "los cables de datos forman un círculo".into());
            return None;
        }
        let n = self.node(id)?;
        let NodeKind::Data { f } = &n.kind else {
            self.err(Some(id), "solo los nodos de datos tienen salida de datos".into());
            return None;
        };
        let Some(d) = data_def(f) else {
            self.err(Some(id), format!("nodo de datos desconocido «{f}»"));
            return None;
        };
        seen.push(id);
        let mut vals: Vec<(String, u8, PinT)> = vec![];
        let mut ok = true;
        for (k, inp) in d.ins.iter().enumerate() {
            let v = match inp.pin.and_then(|t| self.wires.get(&(id, k as u8)).map(|&s| (t, s))) {
                Some((t, src)) => match self.data(src, seen) {
                    Some((s, p, st)) if compatible(st, t) => {
                        if t == PinT::Obj {
                            (self.obj_name(src).unwrap_or(s), 8, st)
                        } else {
                            (s, p, st)
                        }
                    }
                    Some(_) => {
                        self.err(Some(id), format!("«{}» no acepta ese tipo de valor", inp.name));
                        ok = false;
                        continue;
                    }
                    None => {
                        ok = false;
                        continue;
                    }
                },
                None => {
                    let raw = n.args.get(k).cloned().unwrap_or_else(|| inp.def.to_string());
                    let t = inp.pin.unwrap_or(PinT::Any);
                    if inp.pin.is_some() && t != PinT::Obj {
                        match expr::parse(&raw) {
                            Ok(e) => (raw.trim().to_string(), prec(&e), t),
                            Err(e) => {
                                self.err(Some(id), format!("«{}»: la expresión «{raw}» tiene un error: {e}", inp.name));
                                ok = false;
                                continue;
                            }
                        }
                    } else {
                        (raw.trim().to_string(), 8, t)
                    }
                }
            };
            vals.push(v);
        }
        seen.pop();
        if !ok {
            return None;
        }
        let paren = |v: &(String, u8, PinT), min: u8| if v.1 < min { format!("({})", v.0) } else { v.0.clone() };
        let first = vals.first().map(|v| v.0.clone()).unwrap_or_default();
        let res = match d.fmt {
            Fmt::Num => match first.parse::<f64>() {
                Ok(_) => (first.clone(), if first.starts_with('-') { 7 } else { 8 }),
                Err(_) => {
                    self.err(Some(id), format!("«{first}» no es un número"));
                    return None;
                }
            },
            Fmt::Str => {
                if first.contains('"') {
                    self.err(Some(id), "un texto no puede tener comillas \" adentro".into());
                    return None;
                }
                (format!("\"{}\"", n.args.first().map(String::as_str).unwrap_or(d.ins[0].def)), 8)
            }
            Fmt::Bool => ((if matches!(first.as_str(), "1" | "true" | "sí" | "si") { "true" } else { "false" }).into(), 8),
            Fmt::Name => {
                if first.is_empty() {
                    self.err(Some(id), format!("falta elegir «{}»", d.ins[0].name));
                    return None;
                }
                (first, 8)
            }
            Fmt::Prop => {
                let (o, p) = (first, vals.get(1).map(|v| v.0.clone()).unwrap_or_default());
                if o.is_empty() || p.is_empty() {
                    self.err(Some(id), "falta elegir el objeto y el dato".into());
                    return None;
                }
                (format!("{o}.{p}"), 8)
            }
            Fmt::Const(s) => (s.to_string(), 8),
            Fmt::Bin(op, p) => (format!("{} {op} {}", paren(&vals[0], p), paren(&vals[1], p + 1)), p),
            Fmt::Cmp(op) => (format!("{} {op} {}", paren(&vals[0], 5), paren(&vals[1], 5)), 4),
            Fmt::And => (format!("{} and {}", paren(&vals[0], 2), paren(&vals[1], 3)), 2),
            Fmt::Or => (format!("{} or {}", paren(&vals[0], 1), paren(&vals[1], 2)), 1),
            Fmt::Not => (format!("not {}", paren(&vals[0], 3)), 3),
            Fmt::Call(f) => (format!("{f}({})", vals.iter().map(|v| v.0.clone()).collect::<Vec<_>>().join(", ")), 8),
            Fmt::Join => {
                let text = vals.iter().any(|v| v.2 == PinT::Text || v.0.starts_with('"'));
                let a = if text { paren(&vals[0], 5) } else { format!("str({})", vals[0].0) };
                (format!("{a} + {}", paren(&vals[1], 6)), 5)
            }
            Fmt::Raw => (first, vals.first().map_or(8, |v| v.1)),
        };
        Some((res.0, res.1, d.out))
    }

    /// Nombre de objeto de un nodo «Objeto» (los parámetros de objeto no son expresiones).
    fn obj_name(&self, id: NodeId) -> Option<String> {
        let n = self.node(id)?;
        matches!(&n.kind, NodeKind::Data { f } if f == "obj").then(|| n.args.first().cloned().unwrap_or_default())
    }

    /// Valor final del parámetro `k` (cable o lo escrito).
    fn arg(&mut self, n: &Node, k: usize, spec: &ArgSpec) -> Option<String> {
        if let (Some(t), Some(&src)) = (spec.pin, self.wires.get(&(n.id, k as u8))) {
            let (s, _, st) = self.data(src, &mut vec![])?;
            if !compatible(st, t) {
                self.err(Some(n.id), format!("«{}» no acepta ese tipo de valor", spec.name));
                return None;
            }
            if t == PinT::Obj {
                return match self.obj_name(src) {
                    Some(o) => Some(o),
                    None => {
                        self.err(Some(n.id), format!("a «{}» hay que enchufarle un nodo Objeto", spec.name));
                        None
                    }
                };
            }
            return Some(s);
        }
        Some(n.args.get(k).cloned().unwrap_or_default())
    }

    fn required(&mut self, n: &Node, spec: &ArgSpec, v: &str) -> bool {
        if v.trim().is_empty() && REQUIRED.contains(&spec.p) {
            self.err(Some(n.id), format!("falta «{}»", spec.name));
            return false;
        }
        true
    }

    fn instr(&mut self, n: &Node, op: &str, cond: bool) -> Option<Instr> {
        if self.from_sheet(n.id) {
            // Tal cual estaba en la hoja (con lo que llegue por cable).
            let mut args = n.args.clone();
            for (k, s) in arg_specs(n).iter().enumerate() {
                if s.pin.is_some() && self.wires.contains_key(&(n.id, k as u8)) {
                    let v = self.arg(n, k, s)?;
                    if args.len() <= k {
                        args.resize(k + 1, String::new());
                    }
                    args[k] = v;
                }
            }
            return Some(Instr { op: op.into(), args, not: n.not, sub: n.sub.clone() });
        }
        let Some(m) = catalog::meta(op) else {
            self.err(Some(n.id), format!("instrucción desconocida «{op}»"));
            return None;
        };
        if m.cond != cond {
            self.err(Some(n.id), if m.cond { format!("«{op}» es una condición: va en un nodo «Si»") } else { format!("«{op}» es una acción: no va en un nodo «Si»") });
            return None;
        }
        let mut args = vec![];
        let mut ok = true;
        for (k, s) in arg_specs(n).iter().enumerate() {
            match self.arg(n, k, s) {
                Some(v) => {
                    ok &= self.required(n, s, &v);
                    args.push(v);
                }
                None => ok = false,
            }
        }
        // Los parámetros opcionales vacíos del final no se escriben (así una hoja vieja vuelve igual).
        while args.len() > catalog::required_args(m) && args.last().is_some_and(|a| a.is_empty()) {
            args.pop();
        }
        ok.then(|| Instr { op: op.into(), args, not: n.not && cond, sub: if matches!(op, "or" | "and") { n.sub.clone() } else { vec![] } })
    }

    /// Agrega una condición al evento; «Y (todas estas)» sin NO se agrega desarmada (salvo en la hoja).
    fn push_cond(&mut self, o: &mut Out, n: &Node, op: &str) -> bool {
        let Some(i) = self.instr(n, op, true) else { return false };
        if i.op == "and" && !i.not && !self.sheet {
            o.org.conds.extend(i.sub.iter().map(|_| n.id));
            o.ev.conds.extend(i.sub);
        } else {
            o.ev.conds.push(i);
            o.org.conds.push(n.id);
        }
        true
    }

    // ---- flechas ----

    /// Eventos del camino que empieza en `start`. `forced`: el primer nodo es un «Si» ya visitado que va como condición.
    fn block(&mut self, start: Option<NodeId>, stack: &mut Vec<NodeId>, forced: bool) -> Block {
        let Some(mut cur) = start else { return (vec![], false) };
        let base = stack.len();
        let mut o = Out::new(cur);
        o.ev.disabled = self.node(cur).is_some_and(|n| n.off && !matches!(n.kind, NodeKind::Repeat | NodeKind::ForEach | NodeKind::While | NodeKind::Note));
        let mut acts = false;
        let mut first = true;
        // En la hoja: un «Si» (o el disparador) que está mal corta el camino ahí.
        let mut cut = false;
        let res = loop {
            self.budget = self.budget.saturating_sub(1);
            if self.budget == 0 {
                self.err(Some(cur), "el grafo es demasiado grande para compilarlo".into());
                break None;
            }
            let pre = forced && first;
            if !pre {
                if stack.contains(&cur) {
                    self.err(Some(cur), "las flechas forman un círculo: este nodo se repetiría para siempre".into());
                    break None;
                }
                stack.push(cur);
            }
            let Some(n) = self.node(cur) else { break None };
            let next = match &n.kind {
                NodeKind::Event { op } => {
                    if !first {
                        self.err(Some(cur), "un nodo de evento no puede ir en medio de un camino".into());
                        break None;
                    }
                    if !op.is_empty() && !self.push_cond(&mut o, n, op) && self.sheet {
                        cut = true;
                        break None;
                    }
                    self.next(cur, 0)
                }
                NodeKind::Cond { op } => {
                    let no = self.next(cur, 1).is_some();
                    if pre || (!acts && !no) {
                        if !self.push_cond(&mut o, n, op) && self.sheet {
                            self.cut += 1;
                            cut = true;
                            break None;
                        }
                        self.next(cur, 0)
                    } else {
                        let b = self.branch(cur, stack);
                        break Some((b, false));
                    }
                }
                NodeKind::Act { op } => {
                    acts = true;
                    if let Some(i) = self.instr(n, op, false) {
                        o.ev.acts.push(i);
                        o.org.acts.push(cur);
                    }
                    self.next(cur, 0)
                }
                NodeKind::Seq { n: k } => {
                    let mut v = vec![];
                    for out in 0..(*k).max(1) {
                        v.extend(self.block(self.next(cur, out), stack, false).0);
                    }
                    if self.sheet {
                        if o.empty() {
                            // Al principio de un camino, la Secuencia es un evento que solo tiene sub-eventos.
                            let (evs, orgs) = split(v);
                            let w = Out { ev: Event { sub: evs, ..Default::default() }, org: Origin { ev: cur, sub: orgs, ..Default::default() } };
                            break Some((vec![w], false));
                        }
                        if !v.is_empty() {
                            o.org.extra.push(cur);
                        }
                    }
                    break Some((v, true));
                }
                NodeKind::Repeat | NodeKind::ForEach | NodeKind::While => {
                    let mut v: Vec<Out> = self.loop_event(n, stack).into_iter().collect();
                    v.extend(self.block(self.next(cur, 1), stack, false).0);
                    break Some((v, true));
                }
                NodeKind::Note if self.sheet => {
                    if !first || !o.empty() {
                        self.err(Some(cur), "una nota va sola, en una salida de una Secuencia".into());
                        break None;
                    }
                    let text = n.args.first().cloned().unwrap_or_default();
                    let note = Out { ev: Event { kind: EventKind::Comment, text, disabled: n.off, ..Default::default() }, org: Origin { ev: cur, ..Default::default() } };
                    break Some((vec![note], false));
                }
                NodeKind::Comment { .. } | NodeKind::Group { .. } | NodeKind::Note | NodeKind::Data { .. } => {
                    self.err(Some(cur), "este nodo no va conectado con flechas".into());
                    break None;
                }
            };
            first = false;
            match next {
                Some(nx) => cur = nx,
                None => break None,
            }
        };
        stack.truncate(base);
        match res {
            Some((mut v, loose)) if o.empty() => {
                if self.sheet {
                    // El nodo que abría este tramo (un «Cada cuadro», por ejemplo) también es parte del primer evento.
                    if let Some(f) = v.first_mut() {
                        f.org.extra.push(o.org.ev);
                        f.org.extra.append(&mut o.org.extra);
                    }
                }
                (v, loose)
            }
            Some((v, _)) => {
                for x in v {
                    o.ev.sub.push(x.ev);
                    o.org.sub.push(x.org);
                }
                (vec![o], false)
            }
            None if cut && o.ev.conds.is_empty() && o.ev.acts.is_empty() => (vec![], false),
            None => (vec![o], false),
        }
    }

    /// «Si» con su camino «sí» y, si tiene, su «si no». El nodo ya está en la pila.
    fn branch(&mut self, c: NodeId, stack: &mut Vec<NodeId>) -> Vec<Out> {
        let mut v = self.block(Some(c), stack, true).0;
        if self.sheet && v.is_empty() {
            // El «Si» está mal: su «si no» tampoco va (quedaría suelto).
            return v;
        }
        if let Some(ns) = self.next(c, 1) {
            let (no, loose) = self.block(Some(ns), stack, false);
            let chain = !loose && no.first().is_some_and(|o| o.ev.kind == EventKind::Standard) && no.iter().skip(1).all(|o| o.ev.kind == EventKind::Else);
            if chain {
                let mut no = no;
                no[0].ev.kind = EventKind::Else;
                v.extend(no);
            } else if !no.is_empty() {
                let (evs, orgs) = split(no);
                v.push(Out { ev: Event { kind: EventKind::Else, sub: evs, ..Default::default() }, org: Origin { ev: ns, sub: orgs, ..Default::default() } });
            }
        }
        v
    }

    /// Repetir / Para cada / Mientras como un evento. El nodo ya está en la pila.
    fn loop_event(&mut self, n: &Node, stack: &mut Vec<NodeId>) -> Option<Out> {
        let (kind, text) = match n.kind {
            NodeKind::Repeat => (EventKind::Repeat, self.arg(n, 0, &arg_specs(n)[0])?),
            NodeKind::ForEach => (EventKind::ForEach, self.arg(n, 0, &arg_specs(n)[0])?),
            _ => (EventKind::While, String::new()),
        };
        if kind != EventKind::While && !self.from_sheet(n.id) && !self.required(n, &arg_specs(n)[0], &text) {
            return None;
        }
        let cut0 = self.cut;
        let (mut body, loose) = self.block(self.next(n.id, 0), stack, false);
        let mut o = if !loose && body.len() == 1 && body[0].ev.kind == EventKind::Standard && !body[0].ev.disabled {
            let mut b = body.pop().unwrap();
            if self.sheet {
                b.org.extra.push(b.org.ev);
            }
            b
        } else {
            let (evs, orgs) = split(body);
            Out { ev: Event { sub: evs, ..Default::default() }, org: Origin { sub: orgs, ..Default::default() } }
        };
        if self.sheet && kind == EventKind::While && self.cut > cut0 && o.ev.conds.is_empty() {
            // Un «Mientras» que perdió sus «Si» repetiría para siempre: mejor que no vaya.
            return None;
        }
        o.ev.kind = kind;
        o.ev.text = text;
        o.ev.disabled = n.off;
        o.org.ev = n.id;
        Some(o)
    }
}

/// Precedencia de una expresión ya leída (8 = átomo).
fn prec(e: &Expr) -> u8 {
    match e {
        Expr::Or(..) => 1,
        Expr::And(..) => 2,
        Expr::Not(_) => 3,
        Expr::Cmp(..) => 4,
        Expr::Bin(b'+' | b'-', ..) => 5,
        Expr::Bin(..) => 6,
        Expr::Neg(_) => 7,
        _ => 8,
    }
}

/// Nodos de evento en orden de ejecución: de arriba a abajo, de izquierda a derecha.
pub fn roots(g: &Graph) -> Vec<&Node> {
    let mut r: Vec<&Node> = g.nodes.iter().filter(|n| matches!(n.kind, NodeKind::Event { .. })).collect();
    r.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)).then(a.id.cmp(&b.id)));
    r
}

/// Compila todo el grafo, con el origen de cada parte. Los eventos de un nodo de evento con errores se omiten.
pub fn compile_mapped(g: &Graph) -> Compiled {
    let mut c = Cx::new(g);
    let mut out = Compiled::default();
    for r in roots(g) {
        let before = c.errs.len();
        let (v, _) = c.block(Some(r.id), &mut vec![], false);
        if c.errs.len() > before {
            continue;
        }
        for mut o in v {
            if o.empty() && o.ev.sub.is_empty() {
                continue;
            }
            if r.off {
                o.ev.disabled = true;
            }
            out.events.push(o.ev);
            out.origins.push(o.org);
        }
    }
    out.errors = c.errs;
    out
}

pub fn compile(g: &Graph) -> Result<Vec<Event>, Vec<GraphError>> {
    let c = compile_mapped(g);
    if c.errors.is_empty() { Ok(c.events) } else { Err(c.errors) }
}

/// Lo que se pudo compilar y los errores (para el motor: un evento roto no frena al resto).
pub fn compile_lossy(g: &Graph) -> (Vec<Event>, Vec<GraphError>) {
    let c = compile_mapped(g);
    (c.events, c.errors)
}

/// Compila la hoja vista como nodos. A diferencia de un grafo aparte:
/// - las notas y los grupos son eventos (un evento es del grupo si su nodo empieza dentro del recuadro);
/// - un nodo de evento siempre da un evento, aunque esté vacío, y nada se desarma ni se reordena;
/// - los nodos de `lenient` (los que vienen de la hoja) pasan tal cual aunque estén incompletos;
/// - un nodo nuevo que está mal no frena al resto: una acción se salta, un «Si» corta el camino ahí,
///   y un nodo de evento nuevo que todavía no hace nada no se agrega. Lo que no llega a los eventos es borrador.
pub fn compile_sheet(g: &Graph, lenient: &BTreeSet<NodeId>) -> Compiled {
    let mut c = Cx::new(g);
    c.sheet = true;
    c.lenient = lenient;
    let items: Vec<&Node> = g.nodes.iter().filter(|n| matches!(n.kind, NodeKind::Event { .. } | NodeKind::Comment { .. } | NodeKind::Group { .. })).collect();
    let area = |n: &Node| size(n).0 * size(n).1;
    let mut kids: BTreeMap<Option<NodeId>, Vec<&Node>> = BTreeMap::new();
    for it in &items {
        let parent = items
            .iter()
            .filter(|gr| matches!(gr.kind, NodeKind::Group { .. }) && gr.id != it.id)
            .filter(|gr| {
                let (w, h) = size(gr);
                it.x >= gr.x && it.x <= gr.x + w && it.y >= gr.y && it.y <= gr.y + h
            })
            .filter(|gr| !matches!(it.kind, NodeKind::Group { .. }) || area(gr) > area(it) || (area(gr) == area(it) && gr.id < it.id))
            .min_by(|a, b| area(a).total_cmp(&area(b)).then(b.id.cmp(&a.id)))
            .map(|gr| gr.id);
        kids.entry(parent).or_default().push(it);
    }
    for v in kids.values_mut() {
        v.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)).then(a.id.cmp(&b.id)));
    }
    fn emit(c: &mut Cx, kids: &BTreeMap<Option<NodeId>, Vec<&Node>>, at: Option<NodeId>, evs: &mut Vec<Event>, orgs: &mut Vec<Origin>) {
        let Some(list) = kids.get(&at) else { return };
        for it in list {
            let text = it.args.first().cloned().unwrap_or_default();
            match &it.kind {
                NodeKind::Comment { .. } => {
                    evs.push(Event { kind: EventKind::Comment, text, disabled: it.off, ..Default::default() });
                    orgs.push(Origin { ev: it.id, ..Default::default() });
                }
                NodeKind::Group { .. } => {
                    let (mut se, mut so) = (vec![], vec![]);
                    emit(c, kids, Some(it.id), &mut se, &mut so);
                    evs.push(Event { kind: EventKind::Group, text, sub: se, disabled: it.off, ..Default::default() });
                    orgs.push(Origin { ev: it.id, sub: so, ..Default::default() });
                }
                _ => {
                    let (v, _) = c.block(Some(it.id), &mut vec![], false);
                    if !c.lenient.contains(&it.id) && v.iter().all(|o| o.ev.acts.is_empty() && o.ev.sub.is_empty()) {
                        continue;
                    }
                    for mut o in v {
                        if it.off {
                            o.ev.disabled = true;
                        }
                        evs.push(o.ev);
                        orgs.push(o.org);
                    }
                }
            }
        }
    }
    let mut out = Compiled::default();
    emit(&mut c, &kids, None, &mut out.events, &mut out.origins);
    fn used(orgs: &[Origin], set: &mut BTreeSet<NodeId>) {
        for o in orgs {
            set.insert(o.ev);
            set.extend(o.conds.iter().chain(&o.acts).chain(&o.extra));
            used(&o.sub, set);
        }
    }
    used(&out.origins, &mut out.used);
    out.errors = c.errs;
    out
}

/// Nodos a los que se llega con flechas desde algún evento (los demás no hacen nada).
pub fn reachable(g: &Graph) -> BTreeSet<NodeId> {
    let mut seen = BTreeSet::new();
    let mut todo: Vec<NodeId> = roots(g).iter().map(|n| n.id).collect();
    while let Some(id) = todo.pop() {
        if seen.insert(id) {
            todo.extend(g.flow.iter().filter(|f| f.from == id).map(|f| f.to));
        }
    }
    // Los nodos de datos cuentan si alimentan a uno alcanzado.
    loop {
        let more: Vec<NodeId> = g.wires.iter().filter(|w| seen.contains(&w.to) && !seen.contains(&w.from)).map(|w| w.from).collect();
        if more.is_empty() {
            break;
        }
        seen.extend(more);
    }
    seen
}

impl Project {
    /// Copia del proyecto con los grafos compilados y agregados al final de los eventos de su escena (o función).
    /// Es lo que ejecuta el motor.
    pub fn compiled(&self) -> Project {
        let mut p = self.clone();
        for s in &mut p.scenes {
            for g in std::mem::take(&mut s.graphs) {
                s.events.extend(compile_lossy(&g).0);
            }
        }
        for f in &mut p.functions {
            if let Some(g) = f.graph.take() {
                f.events.extend(compile_lossy(&g).0);
            }
        }
        p
    }
}

// ---------- de eventos a grafo ----------

const COL_W: f32 = NODE_W + 60.;
const GAP_Y: f32 = 28.;
const GAP_ROOT: f32 = 56.;
/// Sangría de lo que va dentro de un grupo (en la hoja vista como nodos).
const INDENT: f32 = 24.;

enum Unit<'a> {
    Std(&'a Event, Vec<&'a Event>),
    Loop(&'a Event),
    /// Nota entre sub-eventos (solo en la hoja vista como nodos).
    Note(&'a Event),
}

/// Eventos en unidades: un estándar con sus «si no», o un bucle. Los grupos se desarman; las notas se omiten
/// (salvo con `notes`, en la hoja vista como nodos).
fn units(evs: &[Event], notes: bool) -> Vec<Unit<'_>> {
    let mut out: Vec<Unit> = vec![];
    for e in evs {
        match e.kind {
            EventKind::Comment if notes => out.push(Unit::Note(e)),
            EventKind::Comment => {}
            EventKind::Group => {
                if !e.disabled {
                    out.extend(units(&e.sub, notes));
                }
            }
            EventKind::Standard => out.push(Unit::Std(e, vec![])),
            EventKind::Else => match out.last_mut() {
                Some(Unit::Std(_, els)) => els.push(e),
                _ => out.push(Unit::Std(e, vec![])),
            },
            EventKind::Repeat | EventKind::ForEach | EventKind::While => out.push(Unit::Loop(e)),
        }
    }
    out
}

/// Dónde empieza el camino de un evento (ver `Builder::chain`).
#[derive(Clone, Copy, PartialEq)]
enum At {
    /// Suelto: en la hoja, si no tiene condiciones ni acciones necesita un nodo propio (una Secuencia).
    Free,
    /// Por la salida «no» de un «Si» (un «si no» sin condiciones).
    Else,
    /// Ya tiene su nodo (el de evento, un «Si» o un bucle), que es el que lleva el «desactivado».
    Anchored,
}

struct Builder<'s> {
    g: Graph,
    next: NodeId,
    /// Hoja vista como nodos: sin perder nada (notas, grupos, eventos vacíos, «desactivado»...).
    sheet: bool,
    /// Corrimiento a la derecha de lo que se agrega (dentro de los grupos).
    x0: f32,
    /// Eventos (por camino, como «3» o «2.1») cuyo disparador va en un «Si» aparte y no en el nodo de evento.
    split: &'s BTreeSet<String>,
    /// Recuadros de grupo y los nodos que tienen adentro.
    groups: Vec<(NodeId, Vec<NodeId>)>,
}

impl Builder<'_> {
    fn add(&mut self, kind: NodeKind, x: f32, y: f32, args: Vec<String>) -> NodeId {
        self.next += 1;
        self.g.nodes.push(Node { id: self.next, kind, x: x + self.x0, y, args, not: false, sub: vec![], off: false });
        self.next
    }

    fn last(&mut self) -> &mut Node {
        self.g.nodes.last_mut().unwrap()
    }

    fn link(&mut self, at: Option<(NodeId, u8)>, to: NodeId) {
        if let Some((from, out)) = at {
            self.g.flow.push(Flow { from, out, to });
        }
    }

    fn bottom(&self, id: NodeId) -> f32 {
        self.g.nodes.iter().find(|n| n.id == id).map_or(0., |n| n.y + size(n).1)
    }

    /// Un «Si» por condición, en fila. Devuelve el primero (el que lleva el «no» de un «si no»), el último y la columna que sigue.
    fn si_chain(&mut self, conds: &[Instr], mut at: Option<(NodeId, u8)>, mut col: f32, y: f32, off: bool, bottom: &mut f32) -> Option<(NodeId, NodeId, f32)> {
        let mut first = None;
        let mut last = 0;
        for c in conds {
            let id = self.add(NodeKind::Cond { op: c.op.clone() }, col * COL_W, y, c.args.clone());
            let n = self.last();
            n.not = c.not;
            n.sub = c.sub.clone();
            if first.is_none() {
                n.off = off;
                first = Some(id);
            }
            self.link(at, id);
            *bottom = bottom.max(self.bottom(id));
            at = Some((id, 0));
            last = id;
            col += 1.;
        }
        first.map(|f| (f, last, col))
    }

    /// Un evento como camino: condiciones («Si»), acciones y sub-eventos. Devuelve la parte de abajo ocupada.
    /// `root` = camino del evento si va con nodo de evento propio (primer nivel o dentro de un grupo).
    #[allow(clippy::too_many_arguments)]
    fn chain(&mut self, e: &Event, skip_conds: bool, mut at: Option<(NodeId, u8)>, mut col: f32, y: f32, root: Option<&str>, kind: At) -> f32 {
        let mut bottom = y;
        let mut conds: &[Instr] = if skip_conds { &[] } else { &e.conds };
        // ¿El próximo nodo que se agregue lleva el «desactivado» del evento?
        let mut mark = kind != At::Anchored;
        let mut made = false;
        if let Some(path) = root {
            let (op, rest) = match conds.first() {
                Some(c) if TRIGGERS.contains(&c.op.as_str()) && !self.split.contains(path) => (Some(c), &conds[1..]),
                _ => (None, conds),
            };
            let id = self.add(NodeKind::Event { op: op.map_or(String::new(), |c| c.op.clone()) }, col * COL_W, y, op.map_or(vec![], |c| c.args.clone()));
            let n = self.last();
            n.not = op.is_some_and(|c| c.not);
            n.sub = op.map_or(vec![], |c| c.sub.clone());
            n.off = e.disabled;
            bottom = bottom.max(self.bottom(id));
            at = Some((id, 0));
            col += 1.;
            conds = rest;
            mark = false;
            made = true;
        }
        if let Some((_, last, c2)) = self.si_chain(conds, at, col, y, mark && e.disabled, &mut bottom) {
            at = Some((last, 0));
            col = c2;
            mark = false;
            made = true;
        }
        for a in &e.acts {
            let id = self.add(NodeKind::Act { op: a.op.clone() }, col * COL_W, y, a.args.clone());
            if mark {
                self.last().off = e.disabled;
                mark = false;
            }
            self.link(at, id);
            bottom = bottom.max(self.bottom(id));
            at = Some((id, 0));
            col += 1.;
            made = true;
        }
        // En la hoja, un evento sin condiciones ni acciones necesita su propio nodo: una Secuencia que solo lleva sus sub-eventos.
        let anchor = self.sheet && !made && kind != At::Anchored;
        // Un nodo de evento seguido de una Secuencia es un evento que solo agrupa sub-eventos.
        let wrapper = self.sheet && root.is_some() && e.conds.is_empty() && e.acts.is_empty();
        let direct = !anchor && !wrapper && !(kind == At::Else && !made);
        bottom.max(self.list(&e.sub, at, col, y, !e.acts.is_empty(), direct, anchor.then_some(e.disabled)))
    }

    /// Sub-eventos colgados de `at`: directo si se puede sin cambiar el sentido, o con una Secuencia.
    /// `anchor`: la Secuencia va siempre (aunque no haya sub-eventos) y es el evento; lleva su «desactivado».
    #[allow(clippy::too_many_arguments)]
    fn list(&mut self, evs: &[Event], at: Option<(NodeId, u8)>, col: f32, y: f32, has_acts: bool, direct: bool, anchor: Option<bool>) -> f32 {
        let us = units(evs, self.sheet);
        if us.is_empty() && anchor.is_none() {
            return y;
        }
        if direct && us.len() == 1 {
            let ok = match &us[0] {
                Unit::Note(_) => false,
                Unit::Loop(_) => true,
                Unit::Std(_, els) if !els.is_empty() => true,
                Unit::Std(e, _) => has_acts && !e.conds.is_empty(),
            };
            if ok {
                return self.unit(&us[0], at, col, y);
            }
        }
        let seq = self.add(NodeKind::Seq { n: us.len().clamp(1, 255) as u8 }, col * COL_W, y, vec![]);
        if let Some(off) = anchor {
            self.last().off = off;
        }
        self.link(at, seq);
        let mut yy = y;
        let mut bottom = self.bottom(seq);
        for (k, u) in us.iter().enumerate().take(255) {
            let b = self.unit(u, Some((seq, k as u8)), col + 1., yy);
            bottom = bottom.max(b);
            yy = b + GAP_Y;
        }
        bottom
    }

    fn unit(&mut self, u: &Unit, at: Option<(NodeId, u8)>, col: f32, y: f32) -> f32 {
        match u {
            Unit::Note(e) => {
                let id = self.add(NodeKind::Note, col * COL_W, y, vec![e.text.clone()]);
                self.last().off = e.disabled;
                self.link(at, id);
                self.bottom(id)
            }
            Unit::Loop(e) => {
                let (kind, args) = match e.kind {
                    EventKind::Repeat => (NodeKind::Repeat, vec![e.text.clone()]),
                    EventKind::ForEach => (NodeKind::ForEach, vec![e.text.clone()]),
                    _ => (NodeKind::While, vec![]),
                };
                let id = self.add(kind, col * COL_W, y, args);
                self.last().off = e.disabled;
                self.link(at, id);
                let b = self.chain(e, false, Some((id, 0)), col + 1., y, None, At::Anchored);
                b.max(self.bottom(id))
            }
            Unit::Std(e, els) if els.is_empty() => self.chain(e, false, at, col, y, None, At::Free),
            Unit::Std(e, els) => {
                let mut b = y;
                let Some((first, last, c2)) = self.si_chain(&e.conds, at, col, y, e.disabled, &mut b) else {
                    // Sin condiciones no hay un «no» del que colgar el «si no» (no se puede mostrar igual).
                    return self.chain(e, false, at, col, y, None, At::Free);
                };
                let b1 = self.chain(e, true, Some((last, 0)), c2, y, None, At::Anchored).max(b);
                let b2 = self.elses(els, (first, 1), col + 1., b1 + GAP_Y);
                b1.max(b2)
            }
        }
    }

    fn elses(&mut self, els: &[&Event], at: (NodeId, u8), col: f32, y: f32) -> f32 {
        let Some((x, rest)) = els.split_first() else { return y };
        let mut b = y;
        let Some((first, last, c2)) = self.si_chain(&x.conds, Some(at), col, y, x.disabled, &mut b) else {
            return self.chain(x, true, Some(at), col, y, None, At::Else);
        };
        let b1 = self.chain(x, true, Some((last, 0)), c2, y, None, At::Anchored).max(b);
        if rest.is_empty() {
            return b1;
        }
        b1.max(self.elses(rest, (first, 1), col + 1., b1 + GAP_Y))
    }

    /// Lista de primer nivel: cada evento (con sus «si no») es un nodo de evento; notas y grupos son recuadros.
    fn top(&mut self, evs: &[Event], prefix: &str, off: bool, y: &mut f32) {
        let mut k = 0;
        while k < evs.len() {
            let e = &evs[k];
            let path = if prefix.is_empty() { k.to_string() } else { format!("{prefix}.{k}") };
            k += 1;
            match e.kind {
                EventKind::Comment => {
                    let lines = e.text.lines().count().max(1) as f32 + e.text.chars().count() as f32 / 70.;
                    let h = (44. + lines * 18.).round();
                    self.add(NodeKind::Comment { w: 2. * COL_W + NODE_W, h }, 0., *y, vec![e.text.clone()]);
                    if self.sheet {
                        self.last().off = e.disabled;
                    }
                    *y += h + GAP_Y;
                }
                EventKind::Group => {
                    let (n0, y0, x0) = (self.g.nodes.len(), *y, self.x0);
                    *y += 48.;
                    if self.sheet {
                        self.x0 += INDENT;
                    }
                    self.top(&e.sub, &path, off || (!self.sheet && e.disabled), y);
                    self.x0 = x0;
                    let right = self.g.nodes[n0..].iter().map(|n| n.x + size(n).0).fold(x0 + NODE_W, f32::max);
                    let h = (*y - y0 - GAP_ROOT + 24.).max(80.);
                    let w = right - (x0 - INDENT) + INDENT;
                    let kind = if self.sheet { NodeKind::Group { w, h } } else { NodeKind::Comment { w, h } };
                    let id = self.add(kind, -INDENT, y0, vec![e.text.clone()]);
                    if self.sheet {
                        self.last().off = e.disabled;
                    }
                    // El recuadro va detrás de sus nodos.
                    let n = self.g.nodes.pop().unwrap();
                    self.g.nodes.insert(n0, n);
                    let inside = self.g.nodes[n0 + 1..].iter().map(|n| n.id).collect();
                    self.groups.push((id, inside));
                    *y = y0 + h + GAP_ROOT;
                }
                _ => {
                    let mut els = vec![];
                    while e.kind == EventKind::Standard && k < evs.len() && evs[k].kind == EventKind::Else {
                        els.push(&evs[k]);
                        k += 1;
                    }
                    let n0 = self.g.nodes.len();
                    let b = match e.kind {
                        EventKind::Standard | EventKind::Else if els.is_empty() => self.chain(e, false, None, 0., *y, Some(&path), At::Free),
                        _ => {
                            let root = self.add(NodeKind::Event { op: String::new() }, 0., *y, vec![]);
                            let u = if matches!(e.kind, EventKind::Standard | EventKind::Else) { Unit::Std(e, els) } else { Unit::Loop(e) };
                            self.unit(&u, Some((root, 0)), 1., *y).max(self.bottom(root))
                        }
                    };
                    if off {
                        if let Some(n) = self.g.nodes.get_mut(n0) {
                            n.off = true;
                        }
                    }
                    *y = b + GAP_ROOT;
                }
            }
        }
    }
}

/// Grafo legible a partir de una hoja de eventos, acomodado en columnas (un evento por fila).
/// Al compilarlo da los mismos eventos (sin notas ni grupos, que quedan como recuadros).
pub fn from_events(name: &str, evs: &[Event]) -> Graph {
    let mut b = Builder { g: Graph { name: name.into(), ..Default::default() }, next: 0, sheet: false, x0: 0., split: &NONE_S, groups: vec![] };
    let mut y = 0.;
    b.top(evs, "", false, &mut y);
    b.g
}

static NONE_S: BTreeSet<String> = BTreeSet::new();

/// La hoja como nodos, sin perder nada: `compile_sheet` de esto da exactamente los mismos eventos
/// (si la hoja tiene algo que los nodos no pueden mostrar, no; ver `sheet_load`).
fn sheet_graph(evs: &[Event], split: &BTreeSet<String>) -> (Graph, Vec<(NodeId, Vec<NodeId>)>) {
    let mut b = Builder { g: Graph::default(), next: 0, sheet: true, x0: 0., split, groups: vec![] };
    let mut y = 0.;
    b.top(evs, "", false, &mut y);
    (b.g, b.groups)
}

// ---------- la hoja vista como nodos ----------
//
// La hoja (`events`) es la verdad; los nodos son otra forma de verla y editarla. Al editar en nodos se compila
// con `sheet_save`: lo que está bien va a los eventos, dónde quedó cada nodo va a `Layout` (por una clave que sale
// del lugar del evento: «3.1a2» = evento 3, sub-evento 1, acción 2) y lo que todavía no llega a los eventos
// (nodos sin conectar, incompletos, de datos) queda como borrador. `sheet_load` hace el camino de vuelta.

/// Dónde quedó un nodo de la hoja vista como nodos.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct NodePos {
    pub id: NodeId,
    pub x: f32,
    pub y: f32,
    /// Qué nodo era (tipo e instrucción), para reconocerlo si la hoja cambió.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub k: String,
    /// Huella de su contenido.
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub c: u32,
    /// Ancho y alto de una nota o un grupo.
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub w: f32,
    #[serde(default, skip_serializing_if = "is_zero_f32")]
    pub h: f32,
    /// Salidas de una Secuencia.
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub n: u8,
}

fn is_zero_u32(v: &u32) -> bool {
    *v == 0
}
fn is_zero_u8(v: &u8) -> bool {
    *v == 0
}
fn is_zero_f32(v: &f32) -> bool {
    *v == 0.
}

/// Lugar de cada nodo de la hoja, por clave.
pub type Layout = BTreeMap<String, NodePos>;

impl Graph {
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty() && self.flow.is_empty() && self.wires.is_empty()
    }
}

fn sig(n: &Node) -> String {
    match &n.kind {
        NodeKind::Event { op } => format!("e:{op}"),
        NodeKind::Cond { op } => format!("c:{op}"),
        NodeKind::Act { op } => format!("a:{op}"),
        NodeKind::Seq { .. } => "seq".into(),
        NodeKind::Repeat => "rep".into(),
        NodeKind::ForEach => "each".into(),
        NodeKind::While => "while".into(),
        NodeKind::Comment { .. } => "note".into(),
        NodeKind::Group { .. } => "group".into(),
        NodeKind::Note => "nota".into(),
        NodeKind::Data { f } => format!("d:{f}"),
    }
}

/// Huella (FNV-1a) del contenido de un nodo: tipo, valores, NO y sub-condiciones.
fn content(n: &Node) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    let mut eat = |s: &str| {
        for b in s.bytes().chain([0x1f]) {
            h ^= b as u32;
            h = h.wrapping_mul(0x0100_0193);
        }
    };
    eat(&sig(n));
    for a in &n.args {
        eat(a);
    }
    eat(if n.not { "no" } else { "" });
    eat(&serde_json::to_string(&n.sub).unwrap_or_default());
    h.max(1)
}

fn pos_of(n: &Node) -> NodePos {
    let (w, h) = match n.kind {
        NodeKind::Comment { w, h } | NodeKind::Group { w, h } => (w, h),
        _ => (0., 0.),
    };
    let seq = if let NodeKind::Seq { n } = n.kind { n } else { 0 };
    NodePos { id: n.id, x: n.x, y: n.y, k: sig(n), c: content(n), w, h, n: seq }
}

/// Clave de cada nodo de los eventos compilados: camino del evento + papel («e» el que lo abre,
/// «c0» condición, «a0» acción, «x0» otro). Si un nodo aparece dos veces, vale la primera.
pub fn node_keys(evs: &[Event], orgs: &[Origin]) -> BTreeMap<NodeId, String> {
    fn walk(evs: &[Event], orgs: &[Origin], prefix: &str, out: &mut BTreeMap<NodeId, String>) {
        for (k, (e, o)) in evs.iter().zip(orgs).enumerate() {
            let p = if prefix.is_empty() { k.to_string() } else { format!("{prefix}.{k}") };
            out.entry(o.ev).or_insert(format!("{p}e"));
            for (i, id) in o.conds.iter().enumerate() {
                out.entry(*id).or_insert(format!("{p}c{i}"));
            }
            for (i, id) in o.acts.iter().enumerate() {
                out.entry(*id).or_insert(format!("{p}a{i}"));
            }
            for (i, id) in o.extra.iter().enumerate() {
                out.entry(*id).or_insert(format!("{p}x{i}"));
            }
            walk(&e.sub, &o.sub, &p, out);
        }
    }
    let mut out = BTreeMap::new();
    walk(evs, orgs, "", &mut out);
    out
}

/// Camino de una clave como números («3.1a2» → [3, 1]) y su papel («a2»).
fn key_parts(k: &str) -> (Vec<i64>, &str) {
    let cut = k.find(|c: char| c.is_ascii_alphabetic()).unwrap_or(k.len());
    (k[..cut].split('.').filter_map(|s| s.parse().ok()).collect(), &k[cut..])
}

fn key_dist(a: &str, b: &str) -> i64 {
    let ((pa, ra), (pb, rb)) = (key_parts(a), key_parts(b));
    let common: i64 = pa.iter().zip(&pb).map(|(x, y)| (x - y).abs()).sum();
    common + 100 * (pa.len() as i64 - pb.len() as i64).abs() + if ra == rb { 0 } else { 1000 }
}

/// Resultado de editar la hoja en nodos (lo que se guarda en la escena o la función).
#[derive(Clone, Debug, Default)]
pub struct SheetOut {
    pub events: Vec<Event>,
    pub layout: Layout,
    pub drafts: Graph,
    /// Errores de los nodos nuevos (los que quedan como borrador porque están mal).
    pub errors: Vec<GraphError>,
}

/// La hoja vista como nodos, lista para editar.
#[derive(Clone, Debug, Default)]
pub struct SheetIn {
    pub graph: Graph,
    /// Nodos que vienen de la hoja (los demás son borradores).
    pub canon: BTreeSet<NodeId>,
    /// false: la hoja tiene algo que los nodos todavía no pueden mostrar igual (solo se puede mirar).
    pub exact: bool,
}

fn same_events(a: &[Event], b: &[Event]) -> bool {
    serde_json::to_value(a).ok() == serde_json::to_value(b).ok()
}

/// De nodos a hoja: eventos, lugar de cada nodo y borradores. `canon` = nodos que vienen de la hoja (ver `compile_sheet`).
pub fn sheet_save(g: &Graph, canon: &BTreeSet<NodeId>) -> SheetOut {
    let c = compile_sheet(g, canon);
    let keys = node_keys(&c.events, &c.origins);
    let layout: Layout = g.nodes.iter().filter_map(|n| keys.get(&n.id).map(|k| (k.clone(), pos_of(n)))).collect();
    let draft: BTreeSet<NodeId> = g.nodes.iter().filter(|n| !c.used.contains(&n.id)).map(|n| n.id).collect();
    let drafts = Graph {
        name: String::new(),
        nodes: g.nodes.iter().filter(|n| draft.contains(&n.id)).cloned().collect(),
        flow: g.flow.iter().filter(|f| draft.contains(&f.from) || draft.contains(&f.to)).copied().collect(),
        wires: g.wires.iter().filter(|w| draft.contains(&w.from) || draft.contains(&w.to)).copied().collect(),
    };
    SheetOut { events: c.events, layout, drafts, errors: c.errors }
}

/// De hoja a nodos: los eventos como nodos, cada uno donde había quedado (si se lo reconoce) y los borradores encima.
/// Siempre da un grafo que compila exactamente a `evs`; si con los lugares guardados no se puede (porque la hoja
/// cambió de orden, por ejemplo), prueba de a poco con menos cosas guardadas.
pub fn sheet_load(evs: &[Event], layout: &Layout, drafts: &Graph) -> SheetIn {
    // Eventos cuyo disparador el usuario había puesto en un «Si» aparte.
    let split: BTreeSet<String> = layout
        .iter()
        .filter(|(k, p)| k.ends_with('e') && p.k == "e:")
        .map(|(k, _)| k[..k.len() - 1].to_string())
        .filter(|p| layout.get(&format!("{p}c0")).is_some_and(|c| c.k.starts_with("c:")))
        .collect();
    let (base, groups) = sheet_graph(evs, &split);
    let all: BTreeSet<NodeId> = base.nodes.iter().map(|n| n.id).collect();
    let comp = compile_sheet(&base, &all);
    if !same_events(&comp.events, evs) {
        return SheetIn { graph: base, canon: all, exact: false };
    }
    let keys = node_keys(&comp.events, &comp.origins);
    let tries = [(true, true, true), (true, true, false), (false, true, true), (false, true, false), (false, false, false)];
    let mut last = SheetIn::default();
    for (roam, saved, links) in tries {
        let (graph, canon) = place(&base, &keys, &groups, layout, drafts, roam, saved, links);
        let ok = same_events(&compile_sheet(&graph, &canon).events, evs);
        last = SheetIn { graph, canon, exact: true };
        if ok {
            break;
        }
    }
    last
}

fn vote(votes: &mut BTreeMap<String, BTreeMap<String, usize>>, new: &str, old: &str) {
    let path = |k: &str| k[..k.find(|c: char| c.is_ascii_alphabetic()).unwrap_or(k.len())].to_string();
    *votes.entry(path(new)).or_default().entry(path(old)).or_default() += 1;
}

/// Qué tiene la hoja que los nodos todavía no pueden mostrar igual (para avisar).
pub fn unshowable(evs: &[Event]) -> String {
    unshowable_known(evs).unwrap_or("algo que los nodos todavía no saben dibujar").to_string()
}

/// Lo que se sabe que los nodos todavía no pueden mostrar igual, si la hoja lo tiene.
pub fn unshowable_known(evs: &[Event]) -> Option<&'static str> {
    fn walk(evs: &[Event], top: bool) -> Option<&'static str> {
        for (k, e) in evs.iter().enumerate() {
            if e.kind == EventKind::Else {
                let ok = k > 0 && matches!(evs[k - 1].kind, EventKind::Standard | EventKind::Else) && !evs[k - 1].conds.is_empty();
                if !ok {
                    return Some("un «si no» que no está justo debajo de un evento con condiciones");
                }
            }
            if !top && e.kind == EventKind::Group {
                return Some("un grupo adentro de un evento");
            }
            if let Some(r) = walk(&e.sub, top && e.kind == EventKind::Group) {
                return Some(r);
            }
        }
        None
    }
    walk(evs, true)
}

/// Acomoda los nodos de `base` con los lugares guardados y le pone los borradores encima.
/// `roam`: reconocer un nodo por su contenido aunque haya cambiado de lugar en la hoja; `saved`: usar los lugares
/// guardados; `links`: conectar los borradores con los nodos de la hoja.
#[allow(clippy::too_many_arguments)]
fn place(base: &Graph, keys: &BTreeMap<NodeId, String>, groups: &[(NodeId, Vec<NodeId>)], layout: &Layout, drafts: &Graph, roam: bool, saved: bool, links: bool) -> (Graph, BTreeSet<NodeId>) {
    let draft_ids: BTreeSet<NodeId> = drafts.nodes.iter().map(|n| n.id).collect();
    let mut taken: BTreeSet<&str> = BTreeSet::new();
    let mut ids_used: BTreeSet<NodeId> = BTreeSet::new();
    let mut found: BTreeMap<NodeId, &NodePos> = BTreeMap::new();
    // Camino nuevo de un evento → camino que tenía (votos de los nodos reconocidos por su contenido).
    let mut votes: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut take = |n: &Node, k: &str, p: &'_ NodePos, taken: &mut BTreeSet<&str>, found: &mut BTreeMap<NodeId, &NodePos>| -> bool {
        if found.contains_key(&n.id) || taken.contains(k) || ids_used.contains(&p.id) || draft_ids.contains(&p.id) || p.id == 0 {
            return false;
        }
        ids_used.insert(p.id);
        true
    };
    if saved {
        // 1) misma clave y mismo contenido.
        for n in &base.nodes {
            if let Some((k, p)) = keys.get(&n.id).and_then(|k| layout.get_key_value(k)) {
                if p.k == sig(n) && p.c == content(n) && take(n, k, p, &mut taken, &mut found) {
                    taken.insert(k);
                    found.insert(n.id, p);
                    vote(&mut votes, &keys[&n.id], k);
                }
            }
        }
        // 2) mismo contenido en otro lugar de la hoja (el más cercano).
        if roam {
            for n in &base.nodes {
                let Some(kn) = keys.get(&n.id) else { continue };
                if found.contains_key(&n.id) {
                    continue;
                }
                let (s, c) = (sig(n), content(n));
                let best = layout.iter().filter(|(k, p)| !taken.contains(k.as_str()) && p.k == s && p.c == c).min_by_key(|(k, _)| key_dist(k, kn));
                if let Some((k, p)) = best {
                    if take(n, k, p, &mut taken, &mut found) {
                        taken.insert(k);
                        found.insert(n.id, p);
                        vote(&mut votes, kn, k);
                    }
                }
            }
        }
        // 3) el mismo papel en el mismo evento (que quizás se movió en la hoja), o la misma clave; mismo tipo de nodo.
        let moved = |path: &str| -> String {
            let mut p = path;
            let mut tail = String::new();
            loop {
                if let Some(best) = votes.get(p).and_then(|v| v.iter().max_by_key(|(k, c)| (**c, std::cmp::Reverse(k.len()))).map(|(k, _)| k.clone())) {
                    return format!("{best}{tail}");
                }
                match p.rfind('.') {
                    Some(i) => {
                        tail = format!("{}{tail}", &p[i..]);
                        p = &p[..i];
                    }
                    None => return path.to_string(),
                }
            }
        };
        for n in &base.nodes {
            let Some(kn) = keys.get(&n.id) else { continue };
            if found.contains_key(&n.id) {
                continue;
            }
            let (path, role) = kn.split_at(kn.find(|c: char| c.is_ascii_alphabetic()).unwrap_or(kn.len()));
            for cand in [format!("{}{role}", moved(path)), kn.clone()] {
                if let Some((k, p)) = layout.get_key_value(&cand) {
                    let kind = |s: &str| s.split(':').next().unwrap_or("").to_string();
                    if kind(&p.k) == kind(&sig(n)) && take(n, k, p, &mut taken, &mut found) {
                        taken.insert(k);
                        found.insert(n.id, p);
                        break;
                    }
                }
            }
        }
    }
    // Números: los guardados; los nuevos, después del más alto.
    let mut fresh = layout.values().map(|p| p.id).chain(draft_ids.iter().copied()).max().unwrap_or(0);
    let ids: BTreeMap<NodeId, NodeId> = base
        .nodes
        .iter()
        .map(|n| {
            let id = found.get(&n.id).map(|p| p.id).unwrap_or_else(|| {
                fresh += 1;
                fresh
            });
            (n.id, id)
        })
        .collect();
    // Lugares: los guardados; los nuevos, al lado del nodo que los precede (como quedaban en la hoja recién armada).
    let pred: BTreeMap<NodeId, NodeId> = base.flow.iter().rev().map(|f| (f.to, f.from)).collect();
    let mut pos: BTreeMap<NodeId, (f32, f32)> = BTreeMap::new();
    let mut prev_item: Option<&Node> = None;
    let mut nodes: Vec<Node> = Vec::with_capacity(base.nodes.len());
    for n in &base.nodes {
        let mut m = n.clone();
        let item = matches!(n.kind, NodeKind::Event { .. } | NodeKind::Comment { .. } | NodeKind::Group { .. });
        let at = match found.get(&n.id) {
            Some(p) => {
                match &mut m.kind {
                    NodeKind::Comment { w, h } | NodeKind::Group { w, h } if p.w > 0. && p.h > 0. => (*w, *h) = (p.w, p.h),
                    NodeKind::Seq { n } => *n = (*n).max(p.n),
                    _ => {}
                }
                (p.x, p.y)
            }
            None => {
                let anchor = pred.get(&n.id).and_then(|p| base.nodes.iter().find(|x| x.id == *p)).or(if item { prev_item } else { None });
                match anchor.and_then(|a| pos.get(&a.id).map(|q| (a, *q))) {
                    Some((a, (ax, ay))) => (ax + n.x - a.x, ay + n.y - a.y),
                    None => (n.x, n.y),
                }
            }
        };
        pos.insert(n.id, at);
        (m.x, m.y) = at;
        m.id = ids[&n.id];
        if item {
            prev_item = Some(n);
        }
        nodes.push(m);
    }
    // Los grupos encierran lo que tienen adentro (de los de adentro hacia afuera).
    let at: BTreeMap<NodeId, usize> = base.nodes.iter().enumerate().map(|(k, n)| (n.id, k)).collect();
    for (gid, inside) in groups {
        let Some(&gi) = at.get(gid) else { continue };
        let inner = inside.iter().filter_map(|id| at.get(id)).map(|&k| &nodes[k]).map(|n| (n.x, n.y, n.x + size(n).0, n.y + size(n).1)).reduce(|p, q| (p.0.min(q.0), p.1.min(q.1), p.2.max(q.2), p.3.max(q.3)));
        let Some((x0, y0, x1, y1)) = inner else { continue };
        let want = (x0 - INDENT, y0 - 48., x1 + INDENT, y1 + 24.);
        let fr = &mut nodes[gi];
        let (w, h) = size(fr);
        let (fx0, fy0, fx1, fy1) = if found.contains_key(gid) { (fr.x.min(want.0), fr.y.min(want.1), (fr.x + w).max(want.2), (fr.y + h).max(want.3)) } else { want };
        fr.x = fx0;
        fr.y = fy0;
        if let NodeKind::Group { w, h } = &mut fr.kind {
            (*w, *h) = (fx1 - fx0, fy1 - fy0);
        }
    }
    let canon: BTreeSet<NodeId> = ids.values().copied().collect();
    let mut g = Graph { name: String::new(), nodes, flow: base.flow.iter().map(|f| Flow { from: ids[&f.from], out: f.out, to: ids[&f.to] }).collect(), wires: vec![] };
    // Borradores encima: sus flechas mandan sobre las de la hoja (un nodo nuevo metido en medio de un camino).
    g.nodes.extend(drafts.nodes.iter().cloned());
    let has = |g: &Graph, id: NodeId| g.nodes.iter().any(|n| n.id == id);
    for f in &drafts.flow {
        let ok = g.nodes.iter().any(|n| n.id == f.from && (f.out as usize) < exec_outs(n).len()) && g.nodes.iter().any(|n| n.id == f.to && has_exec_in(n));
        if !ok || (!links && (canon.contains(&f.from) || canon.contains(&f.to))) {
            continue;
        }
        g.flow.retain(|x| !(x.from == f.from && x.out == f.out));
        g.flow.push(*f);
    }
    for w in &drafts.wires {
        if !has(&g, w.from) || !has(&g, w.to) || (!links && canon.contains(&w.to)) {
            continue;
        }
        g.wires.retain(|x| !(x.to == w.to && x.arg == w.arg));
        g.wires.push(*w);
    }
    // Un cable a un nodo de la hoja vale solo si da lo mismo que dice la hoja (si no, la hoja cambió: manda la hoja).
    let bad: Vec<Wire> = {
        let mut cx = Cx::new(&g);
        g.wires
            .iter()
            .filter(|w| canon.contains(&w.to))
            .filter(|w| {
                let Some(n) = cx.node(w.to) else { return true };
                let Some(spec) = arg_specs(n).get(w.arg as usize).cloned() else { return true };
                cx.arg(n, w.arg as usize, &spec).as_deref() != Some(n.args.get(w.arg as usize).map_or("", |s| s.as_str()))
            })
            .copied()
            .collect()
    };
    g.wires.retain(|w| !bad.contains(w));
    (g, canon)
}

// ---------- texto ----------

fn node_line(n: &Node) -> String {
    let args = arg_specs(n);
    let shown: Vec<String> = args.iter().enumerate().map(|(k, a)| format!("{}={}", a.name, n.args.get(k).map(String::as_str).unwrap_or(""))).collect();
    let mut s = format!("#{} {}", n.id, title(n));
    if n.not {
        s.push_str(" (NO)");
    }
    if n.off {
        s.push_str(" [desactivado]");
    }
    if !shown.is_empty() {
        s.push_str(&format!(" [{}]", shown.join(", ")));
    }
    if !n.sub.is_empty() {
        s.push_str(&format!(" {{{}}}", n.sub.iter().map(catalog::sentence).collect::<Vec<_>>().join("; ")));
    }
    if let NodeKind::Comment { .. } = n.kind {
        s = format!("#{} Comentario: {}", n.id, n.args.first().map(String::as_str).unwrap_or(""));
    }
    s
}

/// Texto del grafo: nodos con sus parámetros, flechas y cables (para la herramienta MCP).
pub fn dump(g: &Graph) -> String {
    let mut out = format!("Grafo «{}»: {} nodos, {} flechas, {} cables\n", g.name, g.nodes.len(), g.flow.len(), g.wires.len());
    for n in &g.nodes {
        out.push_str(&node_line(n));
        let outs = exec_outs(n);
        let fl: Vec<String> = g.flow.iter().filter(|f| f.from == n.id).map(|f| {
            let l = outs.get(f.out as usize).cloned().unwrap_or_default();
            if l.is_empty() { format!("→ #{}", f.to) } else { format!("{l} → #{}", f.to) }
        }).collect();
        if !fl.is_empty() {
            out.push_str(&format!("  {}", fl.join(", ")));
        }
        for w in g.wires.iter().filter(|w| w.from == n.id) {
            let to = g.nodes.iter().find(|m| m.id == w.to).and_then(|m| arg_specs(m).get(w.arg as usize).map(|a| a.name.clone())).unwrap_or_default();
            out.push_str(&format!("  ⇒ #{}.{to}", w.to));
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Project;

    fn node(id: NodeId, kind: NodeKind, y: f32, args: &[&str]) -> Node {
        Node { id, kind, x: 0., y, args: args.iter().map(|s| s.to_string()).collect(), not: false, sub: vec![], off: false }
    }

    fn act(op: &str) -> NodeKind {
        NodeKind::Act { op: op.into() }
    }

    fn json(evs: &[Event]) -> serde_json::Value {
        serde_json::to_value(evs).unwrap()
    }

    /// Forma normal para comparar hojas: sin notas, grupos desarmados, «Y» sin NO desarmado,
    /// eventos vacíos que solo agrupan reemplazados por su contenido.
    fn norm(evs: &[Event]) -> Vec<Event> {
        fn flat(evs: &[Event], out: &mut Vec<Event>) {
            for e in evs {
                match e.kind {
                    EventKind::Comment => {}
                    EventKind::Group if e.disabled => {}
                    EventKind::Group => flat(&e.sub, out),
                    _ => out.push(e.clone()),
                }
            }
        }
        let mut v = vec![];
        flat(evs, &mut v);
        for k in 0..v.len() {
            if v[k].kind == EventKind::Else && (k == 0 || !matches!(v[k - 1].kind, EventKind::Standard | EventKind::Else)) {
                v[k].kind = EventKind::Standard;
            }
            let conds = std::mem::take(&mut v[k].conds);
            for c in conds {
                if c.op == "and" && !c.not {
                    v[k].conds.extend(c.sub);
                } else {
                    v[k].conds.push(c);
                }
            }
            v[k].sub = norm(&v[k].sub);
        }
        let mut out: Vec<Event> = vec![];
        for (k, e) in v.iter().enumerate() {
            let wrapper = e.kind == EventKind::Standard && e.conds.is_empty() && e.acts.is_empty() && !e.disabled;
            let next_else = v.get(k + 1).is_some_and(|n| n.kind == EventKind::Else);
            if wrapper && !next_else {
                out.extend(e.sub.iter().cloned());
            } else {
                out.push(e.clone());
            }
        }
        out
    }

    #[test]
    fn camino_simple_y_secuencia() {
        // Tecla → Si en el suelo → saltar → sonido;  Cada cuadro → Secuencia(mover, Si x > 700 → reiniciar)
        let mut g = Graph { name: "g".into(), ..Default::default() };
        g.nodes = vec![
            node(1, NodeKind::Event { op: "key_pressed".into() }, 0., &["Space"]),
            node(2, NodeKind::Cond { op: "on_floor".into() }, 0., &["Heroe"]),
            node(3, act("jump"), 0., &["Heroe"]),
            node(4, act("play_sound"), 0., &["snd/salto.wav", "100"]),
            node(5, NodeKind::Event { op: String::new() }, 100., &[]),
            node(6, NodeKind::Seq { n: 2 }, 100., &[]),
            node(7, act("add"), 100., &["Heroe", "x", ""]),
            node(8, NodeKind::Cond { op: "compare_prop".into() }, 100., &["Heroe", "y", ">", "700"]),
            node(9, act("restart_scene"), 100., &[]),
            node(10, NodeKind::Data { f: "mul".into() }, 100., &["80", "1"]),
            node(11, NodeKind::Data { f: "dt".into() }, 100., &[]),
        ];
        let f = |from, out, to| Flow { from, out, to };
        g.flow = vec![f(1, 0, 2), f(2, 0, 3), f(3, 0, 4), f(5, 0, 6), f(6, 0, 7), f(6, 1, 8), f(8, 0, 9)];
        g.wires = vec![Wire { from: 10, to: 7, arg: 2 }, Wire { from: 11, to: 10, arg: 1 }];
        let evs = compile(&g).unwrap();
        let want: Vec<Event> = serde_json::from_str(r#"[
            {"conds":[{"op":"key_pressed","args":["Space"]},{"op":"on_floor","args":["Heroe"]}],
             "acts":[{"op":"jump","args":["Heroe"]},{"op":"play_sound","args":["snd/salto.wav","100"]}]},
            {"acts":[{"op":"add","args":["Heroe","x","80 * dt"]}]},
            {"conds":[{"op":"compare_prop","args":["Heroe","y",">","700"]}],"acts":[{"op":"restart_scene"}]}
        ]"#).unwrap();
        assert_eq!(json(&evs), json(&want));
    }

    #[test]
    fn si_no_y_si_despues_de_acciones() {
        // Clic → poner texto → Si puntos >= 10 (sí → ganar, no → Si vidas = 0 (sí → perder))
        let mut g = Graph::default();
        g.nodes = vec![
            node(1, NodeKind::Event { op: "mouse_clicked".into() }, 0., &["Left"]),
            node(2, act("set_text"), 0., &["Cartel", "\"hola\""]),
            node(3, NodeKind::Cond { op: "compare".into() }, 0., &["puntos", ">=", "10"]),
            node(4, act("change_scene"), 0., &["Fin"]),
            node(5, NodeKind::Cond { op: "compare".into() }, 0., &["vidas", "=", "0"]),
            node(6, act("restart_scene"), 0., &[]),
        ];
        g.flow = vec![Flow { from: 1, out: 0, to: 2 }, Flow { from: 2, out: 0, to: 3 }, Flow { from: 3, out: 0, to: 4 }, Flow { from: 3, out: 1, to: 5 }, Flow { from: 5, out: 0, to: 6 }];
        let evs = compile(&g).unwrap();
        let want: Vec<Event> = serde_json::from_str(r#"[
            {"conds":[{"op":"mouse_clicked","args":["Left"]}],"acts":[{"op":"set_text","args":["Cartel","\"hola\""]}],"sub":[
                {"conds":[{"op":"compare","args":["puntos",">=","10"]}],"acts":[{"op":"change_scene","args":["Fin"]}]},
                {"kind":"else","conds":[{"op":"compare","args":["vidas","=","0"]}],"acts":[{"op":"restart_scene"}]}
            ]}
        ]"#).unwrap();
        assert_eq!(json(&evs), json(&want));
    }

    #[test]
    fn repetir_y_expresiones_con_parentesis() {
        let mut g = Graph::default();
        g.nodes = vec![
            node(1, NodeKind::Event { op: "start".into() }, 0., &[]),
            node(2, NodeKind::Repeat, 0., &["5"]),
            node(3, act("create"), 0., &["Roca", "", "0", ""]),
            node(4, act("set_var"), 0., &["listo", "1"]),
            // (a + b) * 2  y  a - (b - c)
            node(5, NodeKind::Data { f: "add".into() }, 0., &["10", "20"]),
            node(6, NodeKind::Data { f: "mul".into() }, 0., &["0", "2"]),
            node(7, NodeKind::Data { f: "sub".into() }, 0., &["Roca.x", "3 - 1"]),
            node(8, NodeKind::Data { f: "join".into() }, 0., &["\"P: \"", "puntos"]),
        ];
        g.flow = vec![Flow { from: 1, out: 0, to: 2 }, Flow { from: 2, out: 0, to: 3 }, Flow { from: 2, out: 1, to: 4 }];
        g.wires = vec![Wire { from: 5, to: 6, arg: 0 }, Wire { from: 6, to: 3, arg: 1 }, Wire { from: 7, to: 3, arg: 2 }];
        let evs = compile(&g).unwrap();
        let want: Vec<Event> = serde_json::from_str(r#"[
            {"conds":[{"op":"start"}],"sub":[
                {"kind":"repeat","text":"5","acts":[{"op":"create","args":["Roca","(10 + 20) * 2","Roca.x - (3 - 1)",""]}]},
                {"acts":[{"op":"set_var","args":["listo","1"]}]}
            ]}
        ]"#).unwrap();
        assert_eq!(json(&evs), json(&want));
        let mut c = Cx::new(&g);
        assert_eq!(c.data(8, &mut vec![]).unwrap().0, "\"P: \" + puntos");
    }

    #[test]
    fn errores_con_nodo() {
        let mut g = Graph::default();
        g.nodes = vec![
            node(1, NodeKind::Event { op: "every".into() }, 0., &[""]),
            node(2, act("delete"), 0., &[""]),
            node(3, act("jump"), 0., &["A"]),
            node(4, NodeKind::Event { op: String::new() }, 50., &[]),
            node(5, act("restart_scene"), 50., &[]),
            node(6, NodeKind::Event { op: "start".into() }, 90., &[]),
            node(7, act("quit"), 90., &[]),
        ];
        // 2 → 3 → 2: círculo. «every» y «delete» sin completar.
        g.flow = vec![Flow { from: 1, out: 0, to: 2 }, Flow { from: 2, out: 0, to: 3 }, Flow { from: 3, out: 0, to: 2 }, Flow { from: 4, out: 0, to: 5 }, Flow { from: 6, out: 0, to: 7 }];
        let errs = compile(&g).unwrap_err();
        assert!(errs.iter().any(|e| e.node == Some(1) && e.msg.contains("Segundos")), "{errs:?}");
        assert!(errs.iter().any(|e| e.node == Some(2) && e.msg.contains("Objeto")), "{errs:?}");
        assert!(errs.iter().any(|e| e.node == Some(2) && e.msg.contains("círculo")), "{errs:?}");
        // Los otros eventos se compilan igual.
        let (evs, _) = compile_lossy(&g);
        assert_eq!(evs.len(), 2);
        // Tipos: un número no entra donde va un objeto, y un «Si» con una acción es un error.
        g.nodes.push(node(8, NodeKind::Data { f: "num".into() }, 0., &["3"]));
        g.wires.push(Wire { from: 8, to: 3, arg: 0 });
        g.nodes.push(node(9, NodeKind::Cond { op: "jump".into() }, 90., &["A"]));
        g.flow.push(Flow { from: 7, out: 0, to: 9 });
        let errs = compile(&g).unwrap_err();
        assert!(errs.iter().any(|e| e.node == Some(3) && e.msg.contains("tipo")), "{errs:?}");
        assert!(errs.iter().any(|e| e.node == Some(9) && e.msg.contains("acción")), "{errs:?}");
        let mut g2 = Graph::default();
        g2.nodes = vec![node(1, NodeKind::Data { f: "add".into() }, 0., &["1", "2"]), node(2, NodeKind::Data { f: "abs".into() }, 0., &["0"])];
        g2.wires = vec![Wire { from: 1, to: 2, arg: 0 }, Wire { from: 2, to: 1, arg: 0 }];
        assert!(Cx::new(&g2).data(1, &mut vec![]).is_none());
    }

    #[test]
    fn ida_y_vuelta_plantillas() {
        for t in ["plataformas", "recolector", "naves", "atrapa"] {
            let src = std::fs::read_to_string(format!("{}/plantillas/{t}/juego.json", env!("CARGO_MANIFEST_DIR"))).unwrap();
            let p: Project = serde_json::from_str(&src).unwrap();
            for s in &p.scenes {
                let g = from_events(&s.name, &s.events);
                let back = compile(&g).unwrap_or_else(|e| panic!("{t}/{}: {e:?}", s.name));
                assert_eq!(json(&norm(&back)), json(&norm(&s.events)), "{t}/{}", s.name);
                // El grafo se guarda y se vuelve a leer igual.
                let again: Graph = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
                assert_eq!(again, g);
                // Ninguna fila se pisa con la siguiente.
                let r = roots(&g);
                assert_eq!(r.len(), units(&s.events, false).len(), "{t}/{}", s.name);
            }
        }
    }

    #[test]
    fn ida_y_vuelta_si_no_grupos_y_bucles() {
        let evs: Vec<Event> = serde_json::from_str(r#"[
            {"kind":"comment","text":"nota"},
            {"conds":[{"op":"key_down","args":["Left"]},{"op":"on_floor","args":["H"]}],"acts":[{"op":"add","args":["H","x","-5"]}]},
            {"kind":"else","conds":[{"op":"key_down","args":["Right"]}],"acts":[{"op":"add","args":["H","x","5"]}]},
            {"kind":"else","acts":[{"op":"set","args":["H","speed","0"]}]},
            {"kind":"group","text":"Enemigos","sub":[
                {"kind":"for_each","text":"E","conds":[{"op":"near","args":["E","H","100"]}],"acts":[{"op":"look_at","args":["E","H.x","H.y"]}]},
                {"conds":[{"op":"collides","args":["H","E"]}],"sub":[
                    {"conds":[{"op":"falling","args":["H"]}],"acts":[{"op":"delete","args":["E"]}]},
                    {"kind":"else","acts":[{"op":"restart_scene"}]}
                ]}
            ]},
            {"conds":[{"op":"start"}],"acts":[{"op":"set_var","args":["n","0"]}],"sub":[
                {"kind":"while","conds":[{"op":"compare","args":["n","<","3"]}],"acts":[{"op":"add_var","args":["n","1"]}]},
                {"acts":[{"op":"create","args":["E","n * 10","0",""]}]}
            ]},
            {"disabled":true,"conds":[{"op":"every","args":["2"]}],"acts":[{"op":"quit"}]},
            {"conds":[{"op":"or","sub":[{"op":"key_pressed","args":["Enter"]},{"op":"mouse_clicked","args":["Left"]}]}],"acts":[{"op":"change_scene","args":["S"]}]},
            {"conds":[{"op":"compare","args":["a","=","1"]}],"acts":[{"op":"quit"}],"sub":[{"conds":[{"op":"compare","args":["b","=","1"]}],"acts":[{"op":"quit"}]}]},
            {"kind":"repeat","text":"3","acts":[{"op":"create","args":["E","0","0",""]}]}
        ]"#).unwrap();
        let g = from_events("t", &evs);
        let back = compile(&g).unwrap();
        assert_eq!(json(&norm(&back)), json(&norm(&evs)));
        // La nota y el grupo quedan como recuadros.
        assert_eq!(g.nodes.iter().filter(|n| matches!(n.kind, NodeKind::Comment { .. })).count(), 2);
        // Las filas no se pisan: cada nodo de evento está más abajo que todo lo anterior.
        let r = roots(&g);
        for w in r.windows(2) {
            assert!(w[1].y > w[0].y);
        }
    }

    #[test]
    fn proyecto_compilado_y_formato_igual() {
        let src = std::fs::read_to_string(format!("{}/plantillas/naves/juego.json", env!("CARGO_MANIFEST_DIR"))).unwrap();
        let p: Project = serde_json::from_str(&src).unwrap();
        // Sin grafos el JSON queda igual (no aparecen campos nuevos).
        let a = serde_json::to_value(&p).unwrap();
        assert!(!serde_json::to_string(&p).unwrap().contains("graph"));
        let mut q = p.clone();
        let evs = std::mem::take(&mut q.scenes[0].events);
        q.scenes[0].graphs.push(from_events("todo", &evs));
        let c = q.compiled();
        assert!(c.scenes[0].graphs.is_empty());
        assert_eq!(json(&norm(&c.scenes[0].events)), json(&norm(&evs)));
        assert_eq!(serde_json::to_value(&p.compiled()).unwrap(), a);
        // El juego con el grafo corre igual que con la hoja.
        let mut w1 = crate::runtime::World::new(p.clone());
        let mut w2 = crate::runtime::World::new(q);
        for _ in 0..90 {
            let inp = crate::runtime::Input::default();
            w1.step(1. / 60., inp.clone());
            w2.step(1. / 60., inp);
        }
        let pos = |w: &crate::runtime::World| w.insts.iter().map(|i| (i.obj.clone(), i.x.round() as i64, i.y.round() as i64)).collect::<Vec<_>>();
        assert_eq!(pos(&w1), pos(&w2));
    }

    // ---------- la hoja vista como nodos ----------

    /// Todas las hojas de las plantillas y los ejemplos (escenas y bloques), con su nombre.
    fn hojas() -> Vec<(String, Vec<Event>)> {
        fn walk(d: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for e in std::fs::read_dir(d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().is_some_and(|x| x == "json") {
                    out.push(p);
                }
            }
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = vec![];
        for d in ["plantillas", "examples"] {
            walk(&root.join(d), &mut files);
        }
        files.sort();
        let mut v = vec![];
        for f in files {
            let Ok(p) = serde_json::from_str::<Project>(&std::fs::read_to_string(&f).unwrap()) else { continue };
            let name = f.strip_prefix(root).unwrap().display().to_string();
            for s in &p.scenes {
                v.push((format!("{name} › {}", s.name), s.events.clone()));
            }
            for fu in &p.functions {
                v.push((format!("{name} › bloque {}", fu.name), fu.events.clone()));
            }
        }
        assert!(v.len() >= 6, "{v:?}");
        v
    }

    fn texto(evs: &[Event]) -> String {
        serde_json::to_string(evs).unwrap()
    }

    /// Hoja con todo lo que tiene que sobrevivir la ida y vuelta.
    fn hoja_completa() -> Vec<Event> {
        serde_json::from_str(r#"[
            {"kind":"comment","text":"nota de arriba"},
            {},
            {"disabled":true},
            {"conds":[{"op":"key_down","args":["Left"]},{"op":"on_floor","args":["H"]}],"acts":[{"op":"add","args":["H","x","-5"]}]},
            {"kind":"else","conds":[{"op":"key_down","args":["Right"]}],"acts":[{"op":"add","args":["H","x","5"]}]},
            {"kind":"else","disabled":true,"acts":[{"op":"set","args":["H","speed","0"]}]},
            {"kind":"group","text":"Enemigos","sub":[
                {"kind":"comment","text":"nota en el grupo"},
                {"kind":"for_each","text":"E","conds":[{"op":"near","args":["E","H","100"]}],"acts":[{"op":"look_at","args":["E","H.x","H.y"]}]},
                {"conds":[{"op":"collides","args":["H","E"]}],"sub":[
                    {"conds":[{"op":"falling","args":["H"]}],"acts":[{"op":"delete","args":["E"]}]},
                    {"kind":"else","acts":[{"op":"restart_scene"}]}
                ]},
                {"kind":"group","text":"Adentro","disabled":true,"sub":[
                    {"conds":[{"op":"every","args":["1"]}],"acts":[{"op":"wait","args":["0.5"]},{"op":"call","args":["Disparar","3"]}]}
                ]},
                {"kind":"group","text":""}
            ]},
            {"conds":[{"op":"start"}],"acts":[{"op":"set_var","args":["n","0"]}],"sub":[
                {"kind":"while","conds":[{"op":"compare","args":["n","<","3"]}],"acts":[{"op":"add_var","args":["n","1"]}]},
                {"acts":[{"op":"create","args":["E","n * 10","0",""]}]},
                {},
                {"disabled":true,"sub":[{"acts":[{"op":"quit"}]}]},
                {"sub":[{"kind":"repeat","text":"2","acts":[{"op":"quit"}]}]}
            ]},
            {"disabled":true,"conds":[{"op":"every","args":["2"]}],"acts":[{"op":"quit"}]},
            {"conds":[{"op":"or","sub":[{"op":"key_pressed","args":["Enter"]},{"op":"mouse_clicked","args":["Left"]}]}],"acts":[{"op":"change_scene","args":["S"]}]},
            {"conds":[{"op":"and","sub":[{"op":"key_down","args":["A"]},{"op":"key_down","args":["B"]}]}],"acts":[{"op":"quit"}]},
            {"conds":[{"op":"compare","args":["a","=","1"],"not":true}],"acts":[{"op":"quit"}],"sub":[{"conds":[{"op":"compare","args":["b","=","1"]}],"acts":[{"op":"quit"}]}]},
            {"conds":[{"op":"key_pressed","args":["Space"],"not":true}],"acts":[{"op":"set","args":["H","x",""]}]},
            {"kind":"repeat","text":"3","acts":[{"op":"create","args":["E","0","0",""]}]},
            {"kind":"repeat","text":"4","disabled":true,"conds":[{"op":"compare","args":["a","=","1"]}],"acts":[{"op":"quit"}]},
            {"kind":"repeat","text":"","sub":[{"acts":[{"op":"quit"}]}]},
            {"kind":"while"},
            {"sub":[{"kind":"for_each","text":"E","acts":[{"op":"delete","args":["E"]}]}]},
            {"disabled":true,"sub":[{"acts":[{"op":"quit"}]},{"acts":[{"op":"quit"}]}]},
            {"conds":[{"op":"compare","args":["a","=","1"]}],"acts":[{"op":"quit"}],"sub":[{"kind":"comment","text":"nota adentro"},{"acts":[{"op":"quit"}]}]},
            {"kind":"repeat","text":"2","sub":[{"kind":"comment","text":"sola en el bucle","disabled":true}]},
            {"conds":[{"op":"compare","args":["a","=","2"]}],"sub":[{"kind":"comment","text":"solo una nota"}]},
            {"conds":[{"op":"compare","args":["a","=","1"]}]},
            {"kind":"else","sub":[{"acts":[{"op":"quit"}]}]},
            {"conds":[{"op":"inventada","args":["x"]}],"acts":[{"op":"tampoco"},{"op":"delete"}]},
            {"conds":[{"op":"compare","args":["c","=","1"]},{"op":"compare","args":["d","=","2"]}]},
            {"kind":"else"},
            {"kind":"comment","text":"nota de abajo","disabled":true}
        ]"#).unwrap()
    }

    #[test]
    fn hoja_ida_y_vuelta_exacta() {
        let mut casos = hojas();
        casos.push(("hoja completa".into(), hoja_completa()));
        for (name, evs) in casos {
            // Los ejemplos los edita el usuario: si les pone algo que se sabe que los nodos no muestran, no cuentan.
            if name.starts_with("examples") {
                if let Some(r) = unshowable_known(&evs) {
                    eprintln!("{name}: se salta ({r})");
                    continue;
                }
            }
            let h = sheet_load(&evs, &Layout::new(), &Graph::default());
            assert!(h.exact, "{name}: la hoja no se pudo mostrar igual");
            let c = compile_sheet(&h.graph, &h.canon);
            assert!(c.errors.is_empty(), "{name}: {:?}", c.errors);
            assert_eq!(texto(&c.events), texto(&evs), "{name}");
            // Todos los nodos son de la hoja: no hay borradores.
            assert_eq!(c.used.len(), h.graph.nodes.len(), "{name}");
            // Guardar sin cambiar nada da la misma hoja y el mismo grafo (mismos números y lugares).
            let s = sheet_save(&h.graph, &h.canon);
            assert_eq!(texto(&s.events), texto(&evs), "{name}");
            assert!(s.drafts.is_empty(), "{name}");
            let h2 = sheet_load(&s.events, &s.layout, &s.drafts);
            for (a, b) in h.graph.nodes.iter().zip(&h2.graph.nodes) {
                assert_eq!(a, b, "{name}");
            }
            assert_eq!(h2.graph, h.graph, "{name}");
            assert_eq!(h2.canon, h.canon, "{name}");
            // Las notas y los grupos son nodos.
            let frames = |evs: &[Event]| -> usize {
                fn f(evs: &[Event]) -> usize {
                    evs.iter().map(|e| matches!(e.kind, EventKind::Comment | EventKind::Group) as usize + f(&e.sub)).sum()
                }
                f(evs)
            };
            assert_eq!(h.graph.nodes.iter().filter(|n| is_frame(n) || n.kind == NodeKind::Note).count(), frames(&evs), "{name}");
        }
    }

    #[test]
    fn hoja_que_no_se_puede_mostrar_es_solo_para_mirar() {
        // Un «si no» suelto (sin un evento arriba) todavía no tiene forma de nodo.
        let evs: Vec<Event> = serde_json::from_str(r#"[{"conds":[{"op":"start"}],"sub":[{"kind":"else"},{"acts":[{"op":"quit"}]}]}]"#).unwrap();
        assert!(!sheet_load(&evs, &Layout::new(), &Graph::default()).exact);
        assert!(unshowable(&evs).contains("si no"));
    }

    fn naves() -> Vec<Event> {
        let src = std::fs::read_to_string(format!("{}/plantillas/naves/juego.json", env!("CARGO_MANIFEST_DIR"))).unwrap();
        serde_json::from_str::<Project>(&src).unwrap().scenes[0].events.clone()
    }

    fn find<'a>(g: &'a Graph, op: &str, arg: &str) -> &'a Node {
        g.nodes.iter().find(|n| matches!(&n.kind, NodeKind::Act { op: o } | NodeKind::Cond { op: o } | NodeKind::Event { op: o } if o == op) && n.args.iter().any(|a| a == arg)).unwrap()
    }

    #[test]
    fn editar_en_nodos_cambia_la_hoja() {
        let evs = naves();
        let h = sheet_load(&evs, &Layout::new(), &Graph::default());
        let mut g = h.graph.clone();
        let id = find(&g, "add", "-320 * dt").id;
        g.nodes.iter_mut().find(|n| n.id == id).unwrap().args[2] = "-500 * dt".into();
        let s = sheet_save(&g, &h.canon);
        let mut want = evs.clone();
        want[2].acts[0].args[2] = "-500 * dt".into();
        assert_eq!(texto(&s.events), texto(&want));
        // Mover un nodo de evento más abajo que el siguiente cambia el orden en la hoja.
        let r2 = find(&g, "key_down", "Left").id;
        let r3 = find(&g, "key_down", "Right");
        let below = r3.y + 10.;
        let (dx, dy) = {
            let n = g.nodes.iter().find(|n| n.id == r2).unwrap();
            (0., below - n.y)
        };
        // Se mueve todo su camino (como al arrastrarlo con lo que tiene detrás).
        let chain: Vec<NodeId> = std::iter::successors(Some(r2), |id| g.flow.iter().find(|f| f.from == *id).map(|f| f.to)).collect();
        for n in g.nodes.iter_mut().filter(|n| chain.contains(&n.id)) {
            n.x += dx;
            n.y += dy;
        }
        let s2 = sheet_save(&g, &h.canon);
        assert_eq!(texto(&s2.events[2..4]), texto(&[want[3].clone(), want[2].clone()]));
        // Y al volver, cada nodo sigue donde quedó.
        let h2 = sheet_load(&s2.events, &s2.layout, &s2.drafts);
        assert!(h2.exact);
        for n in &g.nodes {
            let m = h2.graph.nodes.iter().find(|m| m.id == n.id).unwrap_or_else(|| panic!("falta el nodo {}", n.id));
            assert_eq!((m.x, m.y), (n.x, n.y));
        }
    }

    #[test]
    fn editar_la_hoja_respeta_los_lugares() {
        let evs = naves();
        let h = sheet_load(&evs, &Layout::new(), &Graph::default());
        // El usuario acomodó todo a su gusto: cada nodo corrido distinto.
        let mut g = h.graph.clone();
        for (k, n) in g.nodes.iter_mut().enumerate() {
            n.x += 40. + (k % 3) as f32 * 8.;
            n.y += 16.;
        }
        let s = sheet_save(&g, &h.canon);
        assert_eq!(texto(&s.events), texto(&evs));
        // Ahora cambia la hoja: un evento nuevo arriba, un valor cambiado y un evento borrado.
        let mut evs2 = s.events.clone();
        let changed = find(&g, "add", "320 * dt").id;
        evs2[3].acts[0].args[2] = "400 * dt".into();
        let gone: Vec<Event> = evs2.drain(4..5).collect();
        evs2.insert(1, serde_json::from_str(r#"{"conds":[{"op":"key_pressed","args":["P"]}],"acts":[{"op":"pause_toggle"}]}"#).unwrap());
        let h2 = sheet_load(&evs2, &s.layout, &s.drafts);
        assert!(h2.exact);
        assert_eq!(texto(&compile_sheet(&h2.graph, &h2.canon).events), texto(&evs2));
        // Los nodos que no cambiaron siguen en su lugar y con su número; el del valor cambiado también.
        let gone_g = sheet_graph(&gone, &BTreeSet::new()).0;
        let mut same = 0;
        for n in &g.nodes {
            if gone_g.nodes.iter().any(|m| sig(m) == sig(n) && m.args == n.args) && n.args.iter().any(|a| a.contains("Laser")) {
                continue;
            }
            if let Some(m) = h2.graph.nodes.iter().find(|m| m.id == n.id) {
                assert_eq!((m.x, m.y), (n.x, n.y), "nodo {} ({})", n.id, title(n));
                same += 1;
            }
        }
        assert!(same + 8 >= g.nodes.len(), "{same} de {}", g.nodes.len());
        assert!(h2.graph.nodes.iter().any(|m| m.id == changed));
        // El evento nuevo queda entre los que lo rodean (de arriba a abajo).
        let r = roots(&h2.graph);
        assert!(r.windows(2).all(|w| w[0].y <= w[1].y));
    }

    #[test]
    fn borradores_no_cambian_la_hoja_y_no_se_pierden() {
        let evs = naves();
        let h = sheet_load(&evs, &Layout::new(), &Graph::default());
        let mut g = h.graph.clone();
        let a = find(&g, "create", "Laser").id;
        let b = g.flow.iter().find(|f| f.from == a).unwrap().to;
        let next = g.nodes.iter().map(|n| n.id).max().unwrap() + 1;
        // Una acción nueva metida en medio del camino, sin su objeto; un nodo suelto; y un nodo de datos enchufado a la nueva.
        g.nodes.push(Node { id: next, kind: act("set"), x: 900., y: 900., args: vec!["".into(), "angle".into(), "0".into()], not: false, sub: vec![], off: false });
        g.nodes.push(Node { id: next + 1, kind: act("jump"), x: 1300., y: 1300., args: vec!["Nave".into()], not: false, sub: vec![], off: false });
        g.nodes.push(Node { id: next + 2, kind: NodeKind::Data { f: "mul".into() }, x: 600., y: 1000., args: vec!["2".into(), "45".into()], not: false, sub: vec![], off: false });
        g.flow.retain(|f| f.from != a);
        g.flow.push(Flow { from: a, out: 0, to: next });
        g.flow.push(Flow { from: next, out: 0, to: b });
        g.wires.push(Wire { from: next + 2, to: next, arg: 2 });
        let s = sheet_save(&g, &h.canon);
        assert_eq!(texto(&s.events), texto(&evs), "los borradores no cambian la hoja");
        assert!(s.errors.iter().any(|e| e.node == Some(next) && e.msg.contains("Objeto")), "{:?}", s.errors);
        assert_eq!(s.drafts.nodes.len(), 3);
        // Al volver (otra sesión), los borradores siguen ahí, conectados, con sus números y lugares.
        let h2 = sheet_load(&s.events, &s.layout, &s.drafts);
        assert!(h2.exact);
        for id in [next, next + 1, next + 2] {
            let (n, m) = (g.nodes.iter().find(|n| n.id == id).unwrap(), h2.graph.nodes.iter().find(|n| n.id == id).unwrap());
            assert_eq!(n, m);
            assert!(!h2.canon.contains(&id));
        }
        assert!(h2.graph.flow.contains(&Flow { from: a, out: 0, to: next }) && h2.graph.flow.contains(&Flow { from: next, out: 0, to: b }));
        assert!(h2.graph.wires.contains(&Wire { from: next + 2, to: next, arg: 2 }));
        // Al completarla, pasa a la hoja en su lugar (con el valor que le llega por el cable).
        let mut g2 = h2.graph.clone();
        g2.nodes.iter_mut().find(|n| n.id == next).unwrap().args[0] = "Laser".into();
        let s2 = sheet_save(&g2, &h2.canon);
        let ev = &s2.events[4];
        let ops: Vec<&str> = ev.acts.iter().map(|i| i.op.as_str()).collect();
        assert_eq!(ops[..2], ["create", "set"]);
        assert_eq!(ev.acts[1].args, ["Laser", "angle", "2 * 45"]);
        assert_eq!(s2.drafts.nodes.len(), 2, "quedan el nodo suelto y el de datos");
        let h3 = sheet_load(&s2.events, &s2.layout, &s2.drafts);
        assert!(h3.canon.contains(&next) && h3.graph.wires.contains(&Wire { from: next + 2, to: next, arg: 2 }));
        // Si la hoja cambia ese valor, manda la hoja: el cable se suelta pero el nodo de datos queda.
        let mut evs4 = s2.events.clone();
        evs4[4].acts[1].args[2] = "10".into();
        let h4 = sheet_load(&evs4, &s2.layout, &s2.drafts);
        assert_eq!(texto(&compile_sheet(&h4.graph, &h4.canon).events), texto(&evs4));
        assert!(h4.graph.nodes.iter().any(|n| n.id == next + 2) && h4.graph.wires.is_empty());
    }

    #[test]
    fn deshacer_vuelve_a_los_mismos_nodos() {
        let evs = naves();
        let h = sheet_load(&evs, &Layout::new(), &Graph::default());
        let s0 = sheet_save(&h.graph, &h.canon);
        let mut g = h.graph.clone();
        g.nodes[3].x += 100.;
        let id = find(&g, "add", "-320 * dt").id;
        g.nodes.iter_mut().find(|n| n.id == id).unwrap().args[2] = "-1".into();
        let s1 = sheet_save(&g, &h.canon);
        assert_ne!(texto(&s1.events), texto(&s0.events));
        // Deshacer = volver a lo guardado antes: los nodos son los de antes (números y lugares).
        let back = sheet_load(&s0.events, &s0.layout, &s0.drafts);
        assert_eq!(back.graph, h.graph);
        let again = sheet_load(&s1.events, &s1.layout, &s1.drafts);
        assert_eq!(again.graph, g);
    }

    #[test]
    fn nodos_nuevos_incompletos_no_ensucian_la_hoja() {
        let mut g = Graph::default();
        // Un «Cuando» nuevo sin nada conectado y un «Cada segundo» sin los segundos: no son eventos todavía.
        g.nodes = vec![node(1, NodeKind::Event { op: "key_pressed".into() }, 0., &["Space"]), node(2, NodeKind::Event { op: "every".into() }, 100., &[""])];
        g.nodes.push(node(3, act("quit"), 100., &[]));
        g.flow = vec![Flow { from: 2, out: 0, to: 3 }];
        let s = sheet_save(&g, &BTreeSet::new());
        assert!(s.events.is_empty(), "{:?}", s.events);
        assert_eq!(s.drafts.nodes.len(), 3);
        // Un «Si» nuevo incompleto en medio de un evento de la hoja corta el camino ahí (no deja la acción sin su condición).
        let evs: Vec<Event> = serde_json::from_str(r#"[{"conds":[{"op":"key_pressed","args":["Space"]}],"acts":[{"op":"quit"}]}]"#).unwrap();
        let h = sheet_load(&evs, &Layout::new(), &Graph::default());
        let mut g = h.graph.clone();
        let (r, q) = (g.nodes[0].id, g.nodes[1].id);
        g.nodes.push(node(50, NodeKind::Cond { op: "collides".into() }, 0., &["", ""]));
        g.flow = vec![Flow { from: r, out: 0, to: 50 }, Flow { from: 50, out: 0, to: q }];
        let s = sheet_save(&g, &h.canon);
        assert_eq!(texto(&s.events), r#"[{"conds":[{"op":"key_pressed","args":["Space"]}]}]"#);
        let h2 = sheet_load(&s.events, &s.layout, &s.drafts);
        assert!(h2.exact && h2.graph.nodes.len() == 3 && h2.graph.flow.len() == 2, "{:?}", h2.graph);
    }

    #[test]
    fn el_motor_no_usa_los_borradores() {
        let src = std::fs::read_to_string(format!("{}/plantillas/naves/juego.json", env!("CARGO_MANIFEST_DIR"))).unwrap();
        let p: Project = serde_json::from_str(&src).unwrap();
        let mut q = p.clone();
        let h = sheet_load(&q.scenes[0].events, &Layout::new(), &Graph::default());
        let mut g = h.graph.clone();
        g.nodes.push(node(999, NodeKind::Event { op: "start".into() }, -500., &[]));
        g.nodes.push(node(1000, act("quit"), -500., &[]));
        g.nodes.push(node(1001, act("delete"), -400., &[""]));
        g.flow.push(Flow { from: 999, out: 0, to: 1001 });
        let s = sheet_save(&g, &h.canon);
        (q.scenes[0].node_layout, q.scenes[0].node_drafts) = (s.layout, s.drafts);
        assert!(!q.scenes[0].node_drafts.is_empty());
        let a = serde_json::to_value(&p.compiled().scenes[0].events).unwrap();
        let b = serde_json::to_value(&q.compiled().scenes[0].events).unwrap();
        assert_eq!(a, b);
        assert_eq!(crate::validate::validate(&p, None), crate::validate::validate(&q, None));
        // Y el proyecto se guarda y se vuelve a leer igual.
        let back: Project = serde_json::from_str(&serde_json::to_string(&q).unwrap()).unwrap();
        assert_eq!(back.scenes[0].node_layout, q.scenes[0].node_layout);
        assert_eq!(back.scenes[0].node_drafts, q.scenes[0].node_drafts);
    }
}
