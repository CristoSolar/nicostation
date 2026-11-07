//! Revisa un proyecto sin ejecutarlo: referencias rotas, instrucciones mal formadas, expresiones con error y archivos faltantes.

use crate::catalog::{self, P};
use crate::model::*;
use std::collections::BTreeSet;
use std::path::Path;

/// Devuelve un problema por línea, con su ubicación: `Escena «Nivel» › evento 2.1 › condición 1: ...`.
/// `base`: carpeta del proyecto, para comprobar que existen imágenes y sonidos (None = no se comprueba).
pub fn validate(p: &Project, base: Option<&Path>) -> Vec<String> {
    let mut out = vec![];
    if p.scene(&p.first_scene).is_none() {
        out.push(format!("first_scene «{}» no existe", p.first_scene));
    }
    let scenes: BTreeSet<&str> = p.scenes.iter().map(|s| s.name.as_str()).collect();
    if scenes.len() != p.scenes.len() {
        out.push("hay escenas con el mismo nombre".into());
    }
    let file_ok = |f: &str| base.is_none_or(|b| b.join(f).is_file());
    let all_objects: Vec<&ObjectDef> = p.objects.iter().chain(p.scenes.iter().flat_map(|s| s.objects.iter())).collect();
    for o in &all_objects {
        if let Kind::Tilemap { tileset, tile, cols, rows, data, .. } = &o.kind {
            if !file_ok(tileset) {
                out.push(format!("objeto «{}»: no existe la imagen de baldosas «{tileset}»", o.name));
            }
            if *tile <= 0. || *cols == 0 || *rows == 0 {
                out.push(format!("objeto «{}»: el mapa necesita tile > 0 y al menos 1 columna y 1 fila", o.name));
            }
            if data.len() > cols * rows {
                out.push(format!("objeto «{}»: el mapa tiene {} celdas pero cols x rows = {}", o.name, data.len(), cols * rows));
            }
        }
        if let Kind::Sprite { animations, .. } = &o.kind {
            if animations.is_empty() || animations.iter().any(|a| a.frames.is_empty()) {
                out.push(format!("objeto «{}»: cada sprite necesita al menos una animación con una imagen", o.name));
            }
            for f in animations.iter().flat_map(|a| &a.frames) {
                if !file_ok(f) {
                    out.push(format!("objeto «{}»: no existe la imagen «{f}»", o.name));
                }
            }
        }
    }
    for s in &p.scenes {
        let here = format!("Escena «{}»", s.name);
        let objs: Vec<&ObjectDef> = p.objects_of(s).collect();
        let mut seen = BTreeSet::new();
        for o in &objs {
            if !seen.insert(o.name.as_str()) {
                out.push(format!("{here}: objeto repetido «{}»", o.name));
            }
        }
        let layers: Vec<&str> = s.layers.iter().map(|l| l.name.as_str()).collect();
        for (k, i) in s.instances.iter().enumerate() {
            if !seen.contains(i.obj.as_str()) {
                out.push(format!("{here} › instancia {}: objeto desconocido «{}»", k + 1, i.obj));
            }
            if !layers.contains(&i.layer.as_str()) {
                out.push(format!("{here} › instancia {} ({}): capa desconocida «{}»", k + 1, i.obj, i.layer));
            }
        }
        for (g, members) in &s.groups {
            for m in members.iter().filter(|m| !seen.contains(m.as_str())) {
                out.push(format!("{here} › grupo «{g}»: objeto desconocido «{m}»"));
            }
        }
        let ctx = Ctx { p, objs: &objs, groups: s.groups.keys().map(String::as_str).collect(), layers: &layers, file_ok: &file_ok };
        events(&ctx, &s.events, &here, "", &mut out);
    }
    for f in &p.functions {
        let objs: Vec<&ObjectDef> = all_objects.clone();
        let ctx = Ctx { p, objs: &objs, groups: p.scenes.iter().flat_map(|s| s.groups.keys().map(String::as_str)).collect(), layers: &[], file_ok: &file_ok };
        events(&ctx, &f.events, &format!("Función «{}»", f.name), "", &mut out);
    }
    out
}

struct Ctx<'a> {
    p: &'a Project,
    objs: &'a [&'a ObjectDef],
    groups: BTreeSet<&'a str>,
    /// Vacío = no se comprueba (funciones).
    layers: &'a [&'a str],
    file_ok: &'a dyn Fn(&str) -> bool,
}

fn events(c: &Ctx, evs: &[Event], here: &str, prefix: &str, out: &mut Vec<String>) {
    for (k, e) in evs.iter().enumerate() {
        let n = format!("{prefix}{}", k + 1);
        let at = format!("{here} › evento {n}");
        match e.kind {
            EventKind::Repeat => expr(&e.text, &at, out),
            EventKind::ForEach if !c.known_obj(&e.text) => out.push(format!("{at}: «para cada» con objeto desconocido «{}»", e.text)),
            _ => {}
        }
        if e.kind == EventKind::While && e.conds.is_empty() {
            out.push(format!("{at}: «mientras» sin condiciones repetiría para siempre"));
        }
        for (j, i) in e.conds.iter().enumerate() {
            instr(c, i, true, &format!("{at} › condición {}", j + 1), out);
        }
        for (j, i) in e.acts.iter().enumerate() {
            instr(c, i, false, &format!("{at} › acción {}", j + 1), out);
        }
        events(c, &e.sub, here, &format!("{n}."), out);
    }
}

impl Ctx<'_> {
    fn known_obj(&self, n: &str) -> bool {
        self.objs.iter().any(|o| o.name == n) || self.groups.contains(n)
    }
}

fn expr(src: &str, at: &str, out: &mut Vec<String>) {
    if let Err(e) = crate::expr::parse(src) {
        out.push(format!("{at}: expresión «{src}»: {e}"));
    }
}

fn instr(c: &Ctx, i: &Instr, cond: bool, at: &str, out: &mut Vec<String>) {
    let Some(m) = catalog::meta(&i.op) else {
        return out.push(format!("{at}: instrucción desconocida «{}» (ver catálogo)", i.op));
    };
    if m.cond != cond {
        let (es, va) = if m.cond { ("una condición", "en las condiciones") } else { ("una acción", "en las acciones") };
        out.push(format!("{at}: «{}» es {es}; va {va}", i.op));
    }
    if i.args.len() < m.params.len() {
        out.push(format!("{at}: «{}» necesita {} argumentos ({}), tiene {}", i.op, m.params.len(),
            m.params.iter().map(|p| p.0).collect::<Vec<_>>().join(", "), i.args.len()));
    }
    for (k, ((name, t), a)) in m.params.iter().zip(&i.args).enumerate() {
        let bad = |what: &str| format!("{at}: argumento {} ({name}) «{a}»: {what}", k + 1);
        match t {
            P::Object if !c.known_obj(a) => out.push(bad("objeto o grupo desconocido")),
            P::Scene if c.p.scene(a).is_none() => out.push(bad("escena desconocida")),
            P::Op if !catalog::OPS.contains(&a.as_str()) => out.push(bad(&format!("operador inválido, usa uno de {:?}", catalog::OPS))),
            P::Key if !catalog::KEYS.contains(&a.as_str()) => out.push(bad("tecla desconocida (ver catálogo)")),
            P::Mouse if !catalog::MOUSE.contains(&a.as_str()) => out.push(bad("botón desconocido: Left, Right o Middle")),
            P::Layer if !c.layers.is_empty() && !c.layers.contains(&a.as_str()) => out.push(bad("capa desconocida (\"\" = capa base)")),
            P::File if !(c.file_ok)(a) => out.push(bad("el archivo no existe")),
            P::Expr => expr(a, &format!("{at} › argumento {}", k + 1), out),
            P::Anim => {
                let anims = i.args.first().and_then(|o| c.objs.iter().find(|d| &d.name == o)).and_then(|d| match &d.kind {
                    Kind::Sprite { animations, .. } => Some(animations.iter().map(|x| x.name.as_str()).collect::<Vec<_>>()),
                    _ => None,
                });
                if let Some(list) = anims.filter(|l| !l.contains(&a.as_str())) {
                    out.push(bad(&format!("animación desconocida; tiene {list:?}")));
                }
            }
            P::Func => match c.p.functions.iter().find(|f| &f.name == a) {
                None => out.push(bad("función desconocida")),
                Some(f) => {
                    if f.is_condition != cond {
                        out.push(bad(if f.is_condition { "es una función-condición; úsala con call_cond" } else { "es una función-acción; úsala con call" }));
                    }
                    if i.args.len() - 1 != f.params.len() {
                        out.push(bad(&format!("espera {} parámetros {:?}, recibe {}", f.params.len(), f.params, i.args.len() - 1)));
                    }
                    for (j, x) in i.args.iter().skip(1).enumerate() {
                        expr(x, &format!("{at} › parámetro {}", j + 1), out);
                    }
                }
            },
            _ => {}
        }
    }
    for (j, s) in i.sub.iter().enumerate() {
        instr(c, s, true, &format!("{at} › sub-condición {}", j + 1), out);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn proyecto_correcto_y_errores() {
        let src = r#"{"name":"t","width":100,"height":100,"first_scene":"S","scenes":[{"name":"S",
            "objects":[{"name":"A","type":"box","w":10,"h":10,"color":[0,0,0]}],
            "instances":[{"obj":"A","x":0,"y":0}],
            "events":[{"conds":[{"op":"key_down","args":["Left"]}],"acts":[{"op":"add","args":["A","x","-100*dt"]}]}]}]}"#;
        let mut p: crate::model::Project = serde_json::from_str(src).unwrap();
        assert_eq!(super::validate(&p, None), Vec::<String>::new());

        let ev = &mut p.scenes[0].events[0];
        ev.conds[0].args[0] = "Teclaza".into();
        ev.acts[0].args[2] = "1 +".into();
        ev.acts.push(crate::model::Instr::new("change_scene", &["NoExiste"]));
        ev.acts.push(crate::model::Instr::new("start", &[]));
        let errs = super::validate(&p, None);
        assert_eq!(errs.len(), 4, "{errs:#?}");
        assert!(errs[0].contains("tecla desconocida"));
        assert!(errs[1].contains("expresión «1 +»"));
        assert!(errs[2].contains("escena desconocida"));
        assert!(errs[3].contains("es una condición"));
    }
}
