//! Revisa un proyecto sin ejecutarlo: referencias rotas, instrucciones mal formadas, expresiones con error y archivos faltantes.

use crate::catalog::{self, P};
use crate::nodos;
use crate::model::*;
use std::collections::BTreeSet;
use std::path::Path;

/// Devuelve un problema por línea, con su ubicación: `Escena «Nivel» › evento 2.1 › condición 1: ...`.
/// `base`: carpeta del proyecto, para comprobar que existen imágenes y sonidos (None = no se comprueba).
pub fn validate(p: &Project, base: Option<&Path>) -> Vec<String> {
    let mut out = vec![];
    if p.version > FORMAT_VERSION {
        out.push(format!("el proyecto está hecho con una versión más nueva de NicoStation (formato {}, este programa entiende hasta el {FORMAT_VERSION}): puede que algo no funcione", p.version));
    }
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
        if !o.effect.is_empty() && !catalog::EFFECTS.contains(&o.effect.as_str()) {
            out.push(format!("objeto «{}»: efecto desconocido «{}», usa uno de {:?}", o.name, o.effect, catalog::EFFECTS));
        }
        for b in &o.behaviors {
            match b {
                Behavior::Physics { body, shape, .. } => {
                    if !["dynamic", "static", "kinematic"].contains(&body.as_str()) {
                        out.push(format!("objeto «{}»: física con body «{body}»: usa dynamic, static o kinematic", o.name));
                    }
                    if !["box", "circle"].contains(&shape.as_str()) {
                        out.push(format!("objeto «{}»: física con shape «{shape}»: usa box o circle", o.name));
                    }
                }
                Behavior::Pathfinder { cell, .. } if *cell <= 0. => out.push(format!("objeto «{}»: pathfinder necesita cell > 0", o.name)),
                _ => {}
            }
        }
        if let Kind::Particles(d) = &o.kind {
            if !d.image.is_empty() && !file_ok(&d.image) {
                out.push(format!("objeto «{}»: no existe la imagen de partícula «{}»", o.name, d.image));
            }
            if d.life[0] > d.life[1] || d.speed[0] > d.speed[1] {
                out.push(format!("objeto «{}»: en life y speed va [mínimo, máximo]", o.name));
            }
        }
        if let Kind::Sprite { mask, .. } = &o.kind {
            if !mask.is_empty() && mask.len() < 3 {
                out.push(format!("objeto «{}»: la máscara necesita al menos 3 puntos", o.name));
            }
        }
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
        if let Kind::Mesh { shape, model, texture, .. } = &o.kind {
            if *shape == Shape3::Model && model.is_empty() {
                out.push(format!("objeto «{}»: la forma es «modelo» pero no tiene archivo glTF", o.name));
            }
            for f in [model, texture].into_iter().filter(|f| !f.is_empty()) {
                if !file_ok(f) {
                    out.push(format!("objeto «{}»: no existe el archivo «{f}»", o.name));
                }
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
        let mut names = BTreeSet::new();
        for g in &s.graphs {
            if !names.insert(g.name.as_str()) {
                out.push(format!("{here}: hay dos grafos llamados «{}»", g.name));
            }
            graph_lines(&ctx, g, &here, &mut out);
        }
    }
    for f in &p.functions {
        let objs: Vec<&ObjectDef> = all_objects.clone();
        let ctx = Ctx { p, objs: &objs, groups: p.scenes.iter().flat_map(|s| s.groups.keys().map(String::as_str)).collect(), layers: &[], file_ok: &file_ok };
        let here = format!("Función «{}»", f.name);
        events(&ctx, &f.events, &here, "", &mut out);
        if let Some(g) = &f.graph {
            graph_lines(&ctx, g, &here, &mut out);
        }
    }
    out
}

fn graph_lines(c: &Ctx, g: &nodos::Graph, here: &str, out: &mut Vec<String>) {
    for (id, m) in graph(c, &nodos::compile_mapped(g)) {
        let n = id.and_then(|id| g.nodes.iter().find(|n| n.id == id));
        let at = n.map_or(String::new(), |n| format!(" › nodo {} ({})", n.id, nodos::title(n)));
        out.push(format!("{here} › grafo «{}»{at}: {m}", g.name));
    }
}

/// Problemas de un grafo, cada uno con el nodo donde está: errores de compilación y luego
/// los de validar los eventos que salen de él. `scene` = None para el grafo de una función.
pub fn graph_problems(p: &Project, scene: Option<&Scene>, g: &nodos::Graph, base: Option<&Path>) -> Vec<(Option<nodos::NodeId>, String)> {
    with_ctx(p, scene, base, |c| graph(c, &nodos::compile_mapped(g)))
}

/// Como `graph_problems`, para la hoja vista como nodos (`canon` = nodos que vienen de la hoja, ver `nodos::compile_sheet`):
/// errores de los nodos nuevos y luego los de revisar los eventos de la hoja, cada uno en su nodo.
pub fn sheet_problems(p: &Project, scene: Option<&Scene>, g: &nodos::Graph, canon: &BTreeSet<nodos::NodeId>, base: Option<&Path>) -> Vec<(Option<nodos::NodeId>, String)> {
    with_ctx(p, scene, base, |c| graph(c, &nodos::compile_sheet(g, canon)))
}

fn with_ctx<T>(p: &Project, scene: Option<&Scene>, base: Option<&Path>, f: impl FnOnce(&Ctx) -> T) -> T {
    let file_ok = |f: &str| base.is_none_or(|b| b.join(f).is_file());
    let all: Vec<&ObjectDef> = p.objects.iter().chain(p.scenes.iter().flat_map(|s| s.objects.iter())).collect();
    match scene {
        Some(s) => {
            let objs: Vec<&ObjectDef> = p.objects_of(s).collect();
            let layers: Vec<&str> = s.layers.iter().map(|l| l.name.as_str()).collect();
            f(&Ctx { p, objs: &objs, groups: s.groups.keys().map(String::as_str).collect(), layers: &layers, file_ok: &file_ok })
        }
        None => f(&Ctx { p, objs: &all, groups: p.scenes.iter().flat_map(|s| s.groups.keys().map(String::as_str)).collect(), layers: &[], file_ok: &file_ok }),
    }
}

fn graph(c: &Ctx, comp: &nodos::Compiled) -> Vec<(Option<nodos::NodeId>, String)> {
    let mut res: Vec<(Option<nodos::NodeId>, String)> = comp.errors.iter().map(|e| (e.node, e.msg.clone())).collect();
    graph_events(c, &comp.events, &comp.origins, &mut res);
    res
}

/// Quita la ubicación que agrega `instr` / `expr` (aquí la ubicación es el nodo).
fn sin_lugar(m: &str) -> String {
    let m = m.strip_prefix(": ").or_else(|| m.strip_prefix(" › ")).unwrap_or(m);
    match m.split_once(": expresión") {
        Some((head, rest)) if head.starts_with("argumento ") || head.starts_with("parámetro ") => format!("expresión{rest}"),
        _ => m.to_string(),
    }
}

fn graph_events(c: &Ctx, evs: &[Event], orgs: &[nodos::Origin], res: &mut Vec<(Option<nodos::NodeId>, String)>) {
    for (e, o) in evs.iter().zip(orgs) {
        let mut tmp = vec![];
        match e.kind {
            EventKind::Repeat => expr(&e.text, "", &mut tmp),
            EventKind::ForEach if !c.known_obj(&e.text) => tmp.push(format!(": «para cada» con objeto desconocido «{}»", e.text)),
            EventKind::While if e.conds.is_empty() => tmp.push(": «mientras» sin condiciones repetiría para siempre".into()),
            _ => {}
        }
        res.extend(tmp.drain(..).map(|m| (Some(o.ev), sin_lugar(&m))));
        for (i, id) in e.conds.iter().zip(&o.conds) {
            instr(c, i, true, "", &mut tmp);
            res.extend(tmp.drain(..).map(|m| (Some(*id), sin_lugar(&m))));
        }
        for (i, id) in e.acts.iter().zip(&o.acts) {
            instr(c, i, false, "", &mut tmp);
            res.extend(tmp.drain(..).map(|m| (Some(*id), sin_lugar(&m))));
        }
        graph_events(c, &e.sub, &o.sub, res);
    }
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
    if i.args.len() < catalog::required_args(m) {
        out.push(format!("{at}: «{}» necesita {} argumentos ({}), tiene {}", i.op, catalog::required_args(m),
            m.params.iter().map(|p| p.0).collect::<Vec<_>>().join(", "), i.args.len()));
    }
    for (k, ((name, t), a)) in m.params.iter().zip(&i.args).enumerate() {
        let bad = |what: &str| format!("{at}: argumento {} ({name}) «{a}»: {what}", k + 1);
        match t {
            P::Object if !c.known_obj(a) => out.push(bad("objeto o grupo desconocido")),
            P::Scene if c.p.scene(a).is_none() => out.push(bad("escena desconocida")),
            P::Op if !catalog::OPS.contains(&a.as_str()) => out.push(bad(&format!("operador inválido, usa uno de {:?}", catalog::OPS))),
            P::Key if !catalog::KEYS.contains(&a.as_str()) => out.push(bad("tecla desconocida (ver catálogo)")),
            P::Ease if !catalog::EASES.contains(&a.as_str()) => out.push(bad(&format!("curva desconocida, usa una de {:?}", catalog::EASES))),
            P::Mouse if !catalog::MOUSE.contains(&a.as_str()) => out.push(bad("botón desconocido: Left, Right o Middle")),
            P::Layer if !c.layers.is_empty() && !c.layers.contains(&a.as_str()) => out.push(bad("capa desconocida (\"\" = capa base)")),
            P::File if !(c.file_ok)(a) => out.push(bad("el archivo no existe")),
            P::Text if i.op == "set_effect" && k == 1 && !catalog::EFFECTS.contains(&a.as_str()) => {
                out.push(bad(&format!("efecto desconocido, usa uno de {:?}", catalog::EFFECTS)))
            }
            P::Expr => expr(a, &format!("{at} › argumento {}", k + 1), out),
            P::Code => {
                if let Err(e) = crate::script::check(a) {
                    out.push(bad(&format!("el código tiene un error: {e}")));
                }
            }
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

        // Grafos: errores de compilación y de validación, ubicados en su nodo.
        let mut p: crate::model::Project = serde_json::from_str(src).unwrap();
        let g: crate::nodos::Graph = serde_json::from_str(r#"{"name":"g","nodes":[
            {"id":1,"k":"event","op":"key_pressed","args":["Space"]},
            {"id":2,"k":"act","op":"add","args":["Nadie","x","1 +"]},
            {"id":3,"k":"act","op":"delete","args":[""]}],
            "flow":[{"from":1,"to":2},{"from":2,"to":3}]}"#).unwrap();
        p.scenes[0].graphs.push(g);
        let errs = super::validate(&p, None);
        assert_eq!(errs.len(), 1, "{errs:#?}");
        assert!(errs[0].contains("grafo «g» › nodo 3") && errs[0].contains("falta «Objeto»"), "{errs:#?}");
        p.scenes[0].graphs[0].nodes[2].args[0] = "A".into();
        let errs = super::validate(&p, None);
        assert_eq!(errs.len(), 2, "{errs:#?}");
        assert!(errs[0].contains("nodo 2 (Sumar a propiedad): argumento 1 (Objeto) «Nadie»"), "{errs:#?}");
        let probs = super::graph_problems(&p, Some(&p.scenes[0]), &p.scenes[0].graphs[0], None);
        assert_eq!(probs[1].0, Some(2));
        assert!(probs[1].1.starts_with("expresión «1 +»"), "{probs:?}");
    }

    #[test]
    fn version_nueva_efectos_y_fisica() {
        let src = r#"{"name":"t","width":100,"height":100,"first_scene":"S","version":99,"scenes":[{"name":"S",
            "objects":[{"name":"A","type":"box","w":10,"h":10,"color":[0,0,0],"effect":"brillo",
                "behaviors":[{"type":"physics","body":"flotante"},{"type":"pathfinder"}]},
                {"name":"P","type":"particles","life":[2,1]}],
            "events":[{"conds":[{"op":"key_down","args":["Pad1Start"]}],"acts":[
                {"op":"set_effect","args":["A","grayscale"]},{"op":"set_effect","args":["A","arcoiris","1"]},
                {"op":"apply_impulse","args":["A","0","-300"]},{"op":"move_to","args":["A","10","10"]}]}]}]}"#;
        let p: crate::model::Project = serde_json::from_str(src).unwrap();
        let errs = super::validate(&p, None);
        assert_eq!(errs.len(), 5, "{errs:#?}");
        assert!(errs[0].contains("versión más nueva"), "{errs:#?}");
        assert!(errs[1].contains("efecto desconocido «brillo»"));
        assert!(errs[2].contains("body «flotante»"));
        assert!(errs[3].contains("[mínimo, máximo]"));
        assert!(errs[4].contains("acción 2") && errs[4].contains("efecto desconocido"), "{errs:#?}");
        let mut p = crate::model::parse_project(src.as_bytes()).unwrap();
        assert_eq!(p.version, 99, "migrate no baja la versión de un proyecto más nuevo");
        p.version = 1;
        crate::model::migrate(&mut p);
        assert_eq!(p.version, crate::model::FORMAT_VERSION);
    }
}
