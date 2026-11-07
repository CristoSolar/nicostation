//! Scripts en Rhai (lenguaje embebido en Rust) para quien quiere programar con código.
//! Acción `script` y condición `script_if`. El código ve el mundo a través de funciones (ver `catalogo`).

use crate::model::Value;
use crate::runtime::{Cmd, Sol, World};
use rhai::{Array, Dynamic, Engine, Scope, AST};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

thread_local! {
    /// Mundo y selección del script que se está ejecutando (solo durante `run`).
    static CTX: Cell<Option<(*mut World, *mut Sol)>> = const { Cell::new(None) };
    static ENGINE: RefCell<Option<Engine>> = const { RefCell::new(None) };
    static CACHE: RefCell<HashMap<String, Result<AST, String>>> = RefCell::new(HashMap::new());
}

/// Acceso al mundo desde una función registrada.
fn with<R>(f: impl FnOnce(&mut World, &mut Sol) -> R) -> R {
    let (w, s) = CTX.with(|c| c.get()).expect("script fuera de contexto");
    // SAFETY: CTX solo tiene valor mientras `run` está en curso; `run` recibió &mut World y &mut Sol
    // y no los usa mientras Rhai ejecuta, así que estos punteros son el único acceso en ese momento.
    unsafe { f(&mut *w, &mut *s) }
}

fn to_dyn(v: Value) -> Dynamic {
    match v {
        Value::Num(n) => Dynamic::from_float(n),
        Value::Str(s) => s.into(),
        Value::Bool(b) => b.into(),
    }
}

fn from_dyn(d: &Dynamic) -> Value {
    if let Some(b) = d.clone().try_cast::<bool>() {
        Value::Bool(b)
    } else if let Ok(f) = d.as_float() {
        Value::Num(f)
    } else if let Ok(i) = d.as_int() {
        Value::Num(i as f64)
    } else {
        Value::Str(d.to_string())
    }
}

fn index(w: &World, id: i64) -> Option<usize> {
    w.insts.iter().position(|i| i.id as i64 == id && !i.dead)
}

fn engine() -> Engine {
    let mut e = Engine::new();
    // Que un ciclo infinito no congele el juego.
    e.set_max_operations(2_000_000);
    e.set_max_call_levels(32);
    e.set_max_string_size(100_000);
    e.set_max_array_size(100_000);
    e.on_print(|s| with(|w, _| w.warn(format!("script: {s}"))));
    e.register_fn("log", |s: &str| with(|w, _| w.warn(format!("script: {s}"))));
    e.register_fn("get_var", |n: &str| with(|w, _| to_dyn(w.vars.get(n).or_else(|| w.globals.get(n)).cloned().unwrap_or(Value::Num(0.)))));
    e.register_fn("set_var", |n: &str, v: Dynamic| with(|w, _| { w.vars.insert(n.into(), from_dyn(&v)); }));
    e.register_fn("get_global", |n: &str| with(|w, _| to_dyn(w.globals.get(n).cloned().unwrap_or(Value::Num(0.)))));
    e.register_fn("set_global", |n: &str, v: Dynamic| with(|w, _| { w.globals.insert(n.into(), from_dyn(&v)); }));
    e.register_fn("saved", |k: &str, def: Dynamic| with(|w, _| w.storage.get(k).cloned().map(to_dyn).unwrap_or(def)));
    e.register_fn("save", |k: &str, v: Dynamic| with(|w, _| {
        w.storage.insert(k.into(), from_dyn(&v));
        w.cmds.push(Cmd::Persist(w.storage.clone()));
    }));
    e.register_fn("picked", |obj: &str| with(|w, s| w.pick(obj, s).into_iter().map(|i| Dynamic::from_int(w.insts[i].id as i64)).collect::<Array>()));
    e.register_fn("all", |obj: &str| with(|w, _| w.pick(obj, &Sol::new()).into_iter().map(|i| Dynamic::from_int(w.insts[i].id as i64)).collect::<Array>()));
    e.register_fn("count", |obj: &str| with(|w, _| w.pick(obj, &Sol::new()).len() as i64));
    e.register_fn("prop", |id: i64, p: &str| with(|w, _| index(w, id).map(|i| to_dyn(w.get(i, p))).unwrap_or(Dynamic::UNIT)));
    e.register_fn("set_prop", |id: i64, p: &str, v: Dynamic| with(|w, _| {
        if let Some(i) = index(w, id) {
            w.put(i, p, from_dyn(&v));
        }
    }));
    e.register_fn("object", |id: i64| with(|w, _| index(w, id).map(|i| w.insts[i].obj.clone()).unwrap_or_default()));
    e.register_fn("exists", |id: i64| with(|w, _| index(w, id).is_some()));
    e.register_fn("create", |obj: &str, x: f64, y: f64| with(|w, s| {
        if w.object_def(obj).is_none() {
            w.warn(format!("script: crear objeto desconocido «{obj}»"));
            return -1;
        }
        let k = w.spawn(obj, x as f32, y as f32, "");
        s.entry(obj.into()).or_default().push(k);
        w.insts[k].id as i64
    }));
    e.register_fn("destroy", |id: i64| with(|w, _| {
        if let Some(i) = index(w, id) {
            w.insts[i].dead = true;
        }
    }));
    e.register_fn("key_down", |k: &str| with(|w, _| w.input.down.contains(k)));
    e.register_fn("key_pressed", |k: &str| with(|w, _| w.input.pressed.contains(k)));
    e.register_fn("mouse_x", || with(|w, _| w.mouse_on("").0 as f64));
    e.register_fn("mouse_y", || with(|w, _| w.mouse_on("").1 as f64));
    e.register_fn("random", |a: f64, b: f64| with(|w, _| a + w.random() * (b - a)));
    e.register_fn("dt", || with(|w, _| w.dt as f64));
    e.register_fn("time", || with(|w, _| w.time));
    e.register_fn("play_sound", |f: &str| with(|w, _| w.cmds.push(Cmd::PlaySound { file: f.into(), volume: 100. })));
    e.register_fn("change_scene", |n: &str| with(|w, _| w.request_scene(n)));
    // Matemática en grados, como el resto del motor.
    e.register_fn("dsin", |d: f64| d.to_radians().sin());
    e.register_fn("dcos", |d: f64| d.to_radians().cos());
    e.register_fn("point_angle", |x1: f64, y1: f64, x2: f64, y2: f64| (y2 - y1).atan2(x2 - x1).to_degrees());
    e.register_fn("point_distance", |x1: f64, y1: f64, x2: f64, y2: f64| (x2 - x1).hypot(y2 - y1));
    e
}

/// Revisa la sintaxis sin ejecutar (para el validador y el editor).
pub fn check(code: &str) -> Result<(), String> {
    Engine::new().compile(code).map(|_| ()).map_err(|e| e.to_string())
}

/// Ejecuta un script; devuelve su último valor (para `script_if`).
pub fn run(w: &mut World, sol: &mut Sol, code: &str) -> Option<Value> {
    let ast = CACHE.with(|c| {
        c.borrow_mut().entry(code.to_string()).or_insert_with(|| {
            ENGINE.with(|e| e.borrow_mut().get_or_insert_with(engine).compile(code)).map_err(|e| e.to_string())
        }).clone()
    });
    let ast = match ast {
        Ok(a) => a,
        Err(e) => {
            w.warn(format!("script con error de sintaxis: {e}"));
            return None;
        }
    };
    let prev = CTX.with(|c| c.replace(Some((w as *mut World, sol as *mut Sol))));
    let r = ENGINE.with(|e| e.borrow().as_ref().expect("motor de scripts").eval_ast_with_scope::<Dynamic>(&mut Scope::new(), &ast));
    CTX.with(|c| c.set(prev));
    match r {
        Ok(v) => Some(from_dyn(&v)),
        Err(e) => {
            w.warn(format!("script: {e}"));
            None
        }
    }
}
