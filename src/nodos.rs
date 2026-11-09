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
    /// Recuadro con texto (`args[0]`); no hace nada.
    Comment { w: f32, h: f32 },
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
        NodeKind::Seq { .. } | NodeKind::While | NodeKind::Comment { .. } => vec![],
    }
}

/// ¿Tiene entrada de flecha?
pub fn has_exec_in(n: &Node) -> bool {
    matches!(n.kind, NodeKind::Cond { .. } | NodeKind::Act { .. } | NodeKind::Seq { .. } | NodeKind::Repeat | NodeKind::ForEach | NodeKind::While)
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
        NodeKind::Comment { .. } | NodeKind::Data { .. } => vec![],
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
        NodeKind::Data { f } => data_def(f).map_or(format!("¿{f}?"), |d| d.label.to_string()),
    }
}

// ---------- tamaño (el editor dibuja con estas medidas; el auto-acomodo las usa) ----------

pub const NODE_W: f32 = 250.;
pub const HEAD_H: f32 = 32.;
pub const ROW_H: f32 = 28.;
pub const PAD: f32 = 8.;

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
    if let NodeKind::Comment { w, h } = n.kind {
        return (w.max(120.), h.max(60.));
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
}

#[derive(Clone, Debug, Default)]
pub struct Compiled {
    pub events: Vec<Event>,
    /// Paralelo a `events`.
    pub origins: Vec<Origin>,
    pub errors: Vec<GraphError>,
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

struct Cx<'a> {
    g: &'a Graph,
    idx: BTreeMap<NodeId, usize>,
    flow: BTreeMap<(NodeId, u8), NodeId>,
    wires: BTreeMap<(NodeId, u8), NodeId>,
    errs: Vec<GraphError>,
    budget: usize,
}

const REQUIRED: &[P] = &[P::Expr, P::Object, P::Key, P::Mouse, P::Op, P::Scene, P::Var, P::Func, P::Prop, P::Ease];

impl<'a> Cx<'a> {
    fn new(g: &'a Graph) -> Self {
        let mut c = Cx { g, idx: BTreeMap::new(), flow: BTreeMap::new(), wires: BTreeMap::new(), errs: vec![], budget: 20_000 };
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

    /// Agrega una condición al evento; «Y (todas estas)» sin NO se agrega desarmada.
    fn push_cond(&mut self, o: &mut Out, n: &Node, op: &str) {
        if let Some(i) = self.instr(n, op, true) {
            if i.op == "and" && !i.not {
                o.org.conds.extend(i.sub.iter().map(|_| n.id));
                o.ev.conds.extend(i.sub);
            } else {
                o.ev.conds.push(i);
                o.org.conds.push(n.id);
            }
        }
    }

    // ---- flechas ----

    /// Eventos del camino que empieza en `start`. `forced`: el primer nodo es un «Si» ya visitado que va como condición.
    fn block(&mut self, start: Option<NodeId>, stack: &mut Vec<NodeId>, forced: bool) -> Block {
        let Some(mut cur) = start else { return (vec![], false) };
        let base = stack.len();
        let mut o = Out::new(cur);
        o.ev.disabled = self.node(cur).is_some_and(|n| n.off && !matches!(n.kind, NodeKind::Repeat | NodeKind::ForEach | NodeKind::While));
        let mut acts = false;
        let mut first = true;
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
                    if !op.is_empty() {
                        self.push_cond(&mut o, n, op);
                    }
                    self.next(cur, 0)
                }
                NodeKind::Cond { op } => {
                    let no = self.next(cur, 1).is_some();
                    if pre || (!acts && !no) {
                        self.push_cond(&mut o, n, op);
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
                    break Some((v, true));
                }
                NodeKind::Repeat | NodeKind::ForEach | NodeKind::While => {
                    let mut v: Vec<Out> = self.loop_event(n, stack).into_iter().collect();
                    v.extend(self.block(self.next(cur, 1), stack, false).0);
                    break Some((v, true));
                }
                NodeKind::Comment { .. } | NodeKind::Data { .. } => {
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
            Some((v, loose)) if o.empty() => (v, loose),
            Some((v, _)) => {
                for x in v {
                    o.ev.sub.push(x.ev);
                    o.org.sub.push(x.org);
                }
                (vec![o], false)
            }
            None => (vec![o], false),
        }
    }

    /// «Si» con su camino «sí» y, si tiene, su «si no». El nodo ya está en la pila.
    fn branch(&mut self, c: NodeId, stack: &mut Vec<NodeId>) -> Vec<Out> {
        let mut v = self.block(Some(c), stack, true).0;
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
        if kind != EventKind::While && !self.required(n, &arg_specs(n)[0], &text) {
            return None;
        }
        let (mut body, loose) = self.block(self.next(n.id, 0), stack, false);
        let mut o = if !loose && body.len() == 1 && body[0].ev.kind == EventKind::Standard && !body[0].ev.disabled {
            body.pop().unwrap()
        } else {
            let (evs, orgs) = split(body);
            Out { ev: Event { sub: evs, ..Default::default() }, org: Origin { sub: orgs, ..Default::default() } }
        };
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

enum Unit<'a> {
    Std(&'a Event, Vec<&'a Event>),
    Loop(&'a Event),
}

/// Eventos en unidades: un estándar con sus «si no», o un bucle. Los grupos se desarman; las notas se omiten.
fn units(evs: &[Event]) -> Vec<Unit<'_>> {
    let mut out: Vec<Unit> = vec![];
    for e in evs {
        match e.kind {
            EventKind::Comment => {}
            EventKind::Group => {
                if !e.disabled {
                    out.extend(units(&e.sub));
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

struct Builder {
    g: Graph,
    next: NodeId,
}

impl Builder {
    fn add(&mut self, kind: NodeKind, x: f32, y: f32, args: Vec<String>) -> NodeId {
        self.next += 1;
        self.g.nodes.push(Node { id: self.next, kind, x, y, args, not: false, sub: vec![], off: false });
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

    /// «Si» para una lista de condiciones (una sola, o «Y (todas estas)»).
    fn si(&mut self, conds: &[Instr], at: Option<(NodeId, u8)>, col: f32, y: f32) -> NodeId {
        let id = self.add(NodeKind::Cond { op: String::new() }, col * COL_W, y, vec![]);
        let n = self.last();
        match conds {
            [c] => {
                n.kind = NodeKind::Cond { op: c.op.clone() };
                n.args = c.args.clone();
                n.not = c.not;
                n.sub = c.sub.clone();
            }
            _ => {
                n.kind = NodeKind::Cond { op: "and".into() };
                n.sub = conds.to_vec();
            }
        }
        self.link(at, id);
        id
    }

    /// Un evento como camino: condiciones («Si»), acciones y sub-eventos. Devuelve la parte de abajo ocupada.
    #[allow(clippy::too_many_arguments)]
    fn chain(&mut self, e: &Event, skip_conds: bool, mut at: Option<(NodeId, u8)>, mut col: f32, y: f32, root: bool, else_ctx: bool) -> f32 {
        let mut bottom = y;
        let mut conds: &[Instr] = if skip_conds { &[] } else { &e.conds };
        let mut first = true;
        if root {
            let (op, rest) = match conds.first() {
                Some(c) if TRIGGERS.contains(&c.op.as_str()) => (Some(c), &conds[1..]),
                _ => (None, conds),
            };
            let id = self.add(NodeKind::Event { op: op.map_or(String::new(), |c| c.op.clone()) }, col * COL_W, y, op.map_or(vec![], |c| c.args.clone()));
            let n = self.last();
            n.not = op.is_some_and(|c| c.not);
            n.off = e.disabled;
            bottom = bottom.max(self.bottom(id));
            at = Some((id, 0));
            col += 1.;
            conds = rest;
            first = false;
        }
        for c in conds {
            let id = self.si(std::slice::from_ref(c), at, col, y);
            if first {
                self.last().off = e.disabled;
                first = false;
            }
            bottom = bottom.max(self.bottom(id));
            at = Some((id, 0));
            col += 1.;
        }
        for a in &e.acts {
            let id = self.add(NodeKind::Act { op: a.op.clone() }, col * COL_W, y, a.args.clone());
            if first {
                self.last().off = e.disabled;
                first = false;
            }
            self.link(at, id);
            bottom = bottom.max(self.bottom(id));
            at = Some((id, 0));
            col += 1.;
        }
        let direct = !(else_ctx && first);
        bottom.max(self.list(&e.sub, at, col, y, !e.acts.is_empty(), direct))
    }

    /// Sub-eventos colgados de `at`: directo si se puede sin cambiar el sentido, o con una Secuencia.
    fn list(&mut self, evs: &[Event], at: Option<(NodeId, u8)>, col: f32, y: f32, has_acts: bool, direct: bool) -> f32 {
        let us = units(evs);
        if us.is_empty() {
            return y;
        }
        if direct && us.len() == 1 {
            let ok = match &us[0] {
                Unit::Loop(_) => true,
                Unit::Std(_, els) if !els.is_empty() => true,
                Unit::Std(e, _) => has_acts && !e.conds.is_empty(),
            };
            if ok {
                return self.unit(&us[0], at, col, y);
            }
        }
        let seq = self.add(NodeKind::Seq { n: us.len().min(255) as u8 }, col * COL_W, y, vec![]);
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
            Unit::Loop(e) => {
                let (kind, args) = match e.kind {
                    EventKind::Repeat => (NodeKind::Repeat, vec![e.text.clone()]),
                    EventKind::ForEach => (NodeKind::ForEach, vec![e.text.clone()]),
                    _ => (NodeKind::While, vec![]),
                };
                let id = self.add(kind, col * COL_W, y, args);
                self.last().off = e.disabled;
                self.link(at, id);
                let b = self.chain(e, false, Some((id, 0)), col + 1., y, false, false);
                b.max(self.bottom(id))
            }
            Unit::Std(e, els) if els.is_empty() => self.chain(e, false, at, col, y, false, false),
            Unit::Std(e, els) => {
                let si = self.si(&e.conds, at, col, y);
                self.last().off = e.disabled;
                let b1 = self.chain(e, true, Some((si, 0)), col + 1., y, false, false).max(self.bottom(si));
                let b2 = self.elses(els, (si, 1), col + 1., b1 + GAP_Y);
                b1.max(b2)
            }
        }
    }

    fn elses(&mut self, els: &[&Event], at: (NodeId, u8), col: f32, y: f32) -> f32 {
        let Some((x, rest)) = els.split_first() else { return y };
        if x.conds.is_empty() {
            return self.chain(x, true, Some(at), col, y, false, true);
        }
        let si = self.si(&x.conds, Some(at), col, y);
        self.last().off = x.disabled;
        let b1 = self.chain(x, true, Some((si, 0)), col + 1., y, false, false).max(self.bottom(si));
        if rest.is_empty() {
            return b1;
        }
        b1.max(self.elses(rest, (si, 1), col + 1., b1 + GAP_Y))
    }

    /// Lista de primer nivel: cada evento (con sus «si no») es un nodo de evento; notas y grupos son recuadros.
    fn top(&mut self, evs: &[Event], off: bool, y: &mut f32) {
        let mut k = 0;
        while k < evs.len() {
            let e = &evs[k];
            k += 1;
            match e.kind {
                EventKind::Comment => {
                    let lines = e.text.lines().count().max(1) as f32 + e.text.chars().count() as f32 / 70.;
                    let h = (44. + lines * 18.).round();
                    self.add(NodeKind::Comment { w: 2. * COL_W + NODE_W, h }, 0., *y, vec![e.text.clone()]);
                    *y += h + GAP_Y;
                }
                EventKind::Group => {
                    let (n0, y0) = (self.g.nodes.len(), *y);
                    *y += 48.;
                    self.top(&e.sub, off || e.disabled, y);
                    let right = self.g.nodes[n0..].iter().map(|n| n.x + size(n).0).fold(NODE_W, f32::max);
                    let h = (*y - y0 - GAP_ROOT + 24.).max(80.);
                    self.add(NodeKind::Comment { w: right + 48., h }, -24., y0, vec![e.text.clone()]);
                    // El recuadro va detrás de sus nodos.
                    let n = self.g.nodes.pop().unwrap();
                    self.g.nodes.insert(n0, n);
                    *y += GAP_Y;
                }
                _ => {
                    let mut els = vec![];
                    while e.kind == EventKind::Standard && k < evs.len() && evs[k].kind == EventKind::Else {
                        els.push(&evs[k]);
                        k += 1;
                    }
                    let n0 = self.g.nodes.len();
                    let b = match e.kind {
                        EventKind::Standard | EventKind::Else if els.is_empty() => self.chain(e, false, None, 0., *y, true, false),
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
    let mut b = Builder { g: Graph { name: name.into(), ..Default::default() }, next: 0 };
    let mut y = 0.;
    b.top(evs, false, &mut y);
    b.g
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
                assert_eq!(r.len(), units(&s.events).len(), "{t}/{}", s.name);
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
}
