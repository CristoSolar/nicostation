//! Formato del proyecto NicoStation (JSON). Lo leen el reproductor y el editor, y el editor lo escribe.
//! Modelo inspirado en GDevelop (MIT): escenas, capas, objetos, comportamientos, eventos.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type Vars = BTreeMap<String, Value>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum Value {
    Bool(bool),
    Num(f64),
    Str(String),
}

impl Value {
    pub fn num(&self) -> f64 {
        match self {
            Value::Num(n) => *n,
            Value::Bool(b) => *b as i32 as f64,
            Value::Str(s) => s.trim().parse().unwrap_or(0.),
        }
    }
    pub fn text(&self) -> String {
        match self {
            Value::Num(n) if n.fract() == 0. && n.abs() < 1e15 => format!("{}", *n as i64),
            Value::Num(n) => format!("{n}"),
            Value::Bool(b) => b.to_string(),
            Value::Str(s) => s.clone(),
        }
    }
    pub fn truthy(&self) -> bool {
        match self {
            Value::Num(n) => *n != 0.,
            Value::Bool(b) => *b,
            Value::Str(s) => !matches!(s.as_str(), "" | "0" | "false"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Project {
    pub name: String,
    pub width: f32,
    pub height: f32,
    pub first_scene: String,
    #[serde(default)]
    pub globals: Vars,
    /// Objetos globales: disponibles en todas las escenas.
    #[serde(default)]
    pub objects: Vec<ObjectDef>,
    pub scenes: Vec<Scene>,
    #[serde(default)]
    pub functions: Vec<Function>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Scene {
    pub name: String,
    #[serde(default)]
    pub background: [u8; 3],
    #[serde(default = "default_layers")]
    pub layers: Vec<Layer>,
    #[serde(default)]
    pub objects: Vec<ObjectDef>,
    #[serde(default)]
    pub groups: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub instances: Vec<Instance>,
    #[serde(default)]
    pub vars: Vars,
    #[serde(default)]
    pub events: Vec<Event>,
}

fn default_layers() -> Vec<Layer> {
    vec![Layer { name: String::new(), visible: true, fixed: false }]
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Layer {
    /// "" = capa base.
    pub name: String,
    #[serde(default = "yes")]
    pub visible: bool,
    /// No sigue a la cámara (útil para HUD).
    #[serde(default)]
    pub fixed: bool,
}

fn yes() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Instance {
    pub obj: String,
    pub x: f32,
    pub y: f32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub layer: String,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub angle: f32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub z: f32,
    /// Tamaño propio; si no, el del objeto.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub w: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h: Option<f32>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vars: Vars,
}

fn is_zero(v: &f32) -> bool {
    *v == 0.
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ObjectDef {
    pub name: String,
    #[serde(flatten)]
    pub kind: Kind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub behaviors: Vec<Behavior>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub vars: Vars,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Kind {
    /// Imagen con animaciones. w/h = 0 → tamaño de la primera imagen.
    Sprite {
        #[serde(default)]
        w: f32,
        #[serde(default)]
        h: f32,
        animations: Vec<Animation>,
    },
    Text {
        text: String,
        #[serde(default = "text_size")]
        size: f32,
        #[serde(default = "white")]
        color: [u8; 3],
    },
    /// Rectángulo de color (prototipos sin imágenes).
    Box { w: f32, h: f32, color: [u8; 3] },
}

fn text_size() -> f32 {
    24.
}
fn white() -> [u8; 3] {
    [255, 255, 255]
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Animation {
    pub name: String,
    /// Rutas de imagen relativas al archivo del proyecto.
    pub frames: Vec<String>,
    #[serde(default = "fps")]
    pub fps: f32,
    #[serde(default = "yes")]
    pub looping: bool,
}

fn fps() -> f32 {
    10.
}

/// Comportamientos con los valores por defecto de GDevelop.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Behavior {
    /// Personaje de plataformas (PlatformerObject).
    Platformer {
        #[serde(default = "d250")]
        max_speed: f32,
        #[serde(default = "d1500")]
        acceleration: f32,
        #[serde(default = "d1500")]
        deceleration: f32,
        #[serde(default = "d1000")]
        gravity: f32,
        #[serde(default = "d700")]
        max_falling_speed: f32,
        #[serde(default = "d600")]
        jump_speed: f32,
        #[serde(default = "d0_2")]
        jump_sustain_time: f32,
        #[serde(default = "d150")]
        ladder_speed: f32,
        /// Pendiente máxima que puede subir, en grados.
        #[serde(default = "d60")]
        slope_max_angle: f32,
        /// Se agarra de los bordes de plataformas marcadas como `grabbable`.
        #[serde(default)]
        can_grab_platforms: bool,
        /// Flecha abajo atraviesa las plataformas `jumpthru`.
        #[serde(default = "yes")]
        can_go_down_from_jumpthru: bool,
    },
    /// Suelo o pared para el Platformer. Rotada sirve de pendiente.
    Platform {
        /// Se atraviesa desde abajo.
        #[serde(default)]
        jumpthru: bool,
        /// Escalera: se sube y baja con las flechas.
        #[serde(default)]
        ladder: bool,
        /// El personaje puede agarrarse de su borde.
        #[serde(default)]
        grabbable: bool,
    },
    /// Movimiento de vista superior con flechas (TopDownMovement).
    TopDown {
        #[serde(default = "d200")]
        max_speed: f32,
        #[serde(default = "d400")]
        acceleration: f32,
        #[serde(default = "d800")]
        deceleration: f32,
        #[serde(default = "yes")]
        allow_diagonals: bool,
        #[serde(default)]
        rotate: bool,
    },
    /// Avanza en la dirección de su ángulo.
    Bullet { speed: f32 },
    Rotate { speed: f32 },
    DestroyOutside,
    BoundToLayout,
    ScreenWrap,
    Draggable,
}

fn d60() -> f32 { 60. }
fn d150() -> f32 { 150. }
fn d200() -> f32 { 200. }
fn d250() -> f32 { 250. }
fn d400() -> f32 { 400. }
fn d600() -> f32 { 600. }
fn d700() -> f32 { 700. }
fn d800() -> f32 { 800. }
fn d1000() -> f32 { 1000. }
fn d1500() -> f32 { 1500. }
fn d0_2() -> f32 { 0.2 }

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    #[default]
    Standard,
    /// `text` = cantidad (expresión).
    Repeat,
    /// `text` = objeto; cada iteración elige una sola instancia.
    ForEach,
    /// Repite mientras `conds` se cumplan (máx. 100000 vueltas).
    While,
    /// Solo agrupa sub-eventos. `text` = título.
    Group,
    Comment,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Event {
    #[serde(default, skip_serializing_if = "is_default")]
    pub kind: EventKind,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conds: Vec<Instr>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acts: Vec<Instr>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sub: Vec<Event>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub disabled: bool,
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

/// Condición o acción: `op` del catálogo (ver `catalog.rs`) + argumentos como texto.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Instr {
    pub op: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Condición invertida (por instancia, como en GDevelop).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub not: bool,
    /// Sub-condiciones de `or` / `and`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sub: Vec<Instr>,
}

impl Instr {
    pub fn new(op: &str, args: &[&str]) -> Self {
        Instr { op: op.into(), args: args.iter().map(|s| s.to_string()).collect(), ..Default::default() }
    }
}

/// Acción o condición propia hecha con eventos. Los parámetros llegan como variables locales.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Function {
    pub name: String,
    /// true = se usa como condición; devuelve con la acción `return`.
    #[serde(default)]
    pub is_condition: bool,
    #[serde(default)]
    pub params: Vec<String>,
    pub events: Vec<Event>,
}

impl Project {
    pub fn scene(&self, name: &str) -> Option<&Scene> {
        self.scenes.iter().find(|s| s.name == name)
    }
    /// Objetos visibles desde una escena: los suyos y los globales.
    pub fn objects_of<'a>(&'a self, scene: &'a Scene) -> impl Iterator<Item = &'a ObjectDef> {
        scene.objects.iter().chain(self.objects.iter())
    }
}
