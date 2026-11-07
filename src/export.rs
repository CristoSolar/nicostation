//! Exportación web: copia el reproductor WebAssembly, la página y los archivos del juego a una carpeta.
//! El mismo `nicostation.wasm` sirve para cualquier juego: al arrancar lee `juego.json`.

use crate::model::{Event, Instr, Kind, Project};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const INDEX: &str = include_str!("../assets/web/index.html");
const BUNDLE: &str = include_str!("../assets/web/mq_js_bundle.js");

/// Busca el reproductor compilado para web.
pub fn find_wasm() -> Option<PathBuf> {
    let mut c: Vec<PathBuf> = std::env::var_os("NICOSTATION_WASM").map(PathBuf::from).into_iter().collect();
    if let Ok(exe) = std::env::current_exe() {
        let dir = exe.parent().unwrap_or(Path::new("."));
        c.push(dir.join("nicostation.wasm"));
        // target/release/ → target/wasm32-unknown-unknown/release/
        c.push(dir.join("../wasm32-unknown-unknown/release/nicostation.wasm"));
    }
    c.into_iter().find(|p| p.is_file())
}

/// Imágenes y sonidos que usa el proyecto (rutas relativas al archivo del proyecto).
pub fn assets(p: &Project) -> BTreeSet<String> {
    fn instrs(l: &[Instr], out: &mut BTreeSet<String>) {
        for i in l {
            if matches!(i.op.as_str(), "play_sound" | "play_music") {
                if let Some(f) = i.args.first().filter(|f| !f.is_empty()) {
                    out.insert(f.clone());
                }
            }
            instrs(&i.sub, out);
        }
    }
    fn events(l: &[Event], out: &mut BTreeSet<String>) {
        for e in l {
            instrs(&e.conds, out);
            instrs(&e.acts, out);
            events(&e.sub, out);
        }
    }
    let mut out = BTreeSet::new();
    for o in p.objects.iter().chain(p.scenes.iter().flat_map(|s| s.objects.iter())) {
        match &o.kind {
            Kind::Sprite { animations, .. } => out.extend(animations.iter().flat_map(|a| a.frames.iter().cloned())),
            Kind::Tilemap { tileset, .. } => {
                out.insert(tileset.clone());
            }
            _ => {}
        }
    }
    for s in &p.scenes {
        events(&s.events, &mut out);
    }
    for f in &p.functions {
        events(&f.events, &mut out);
    }
    out
}

/// Exporta el proyecto a `out`. Devuelve los archivos escritos.
pub fn export_web(project_file: &Path, out: &Path) -> Result<Vec<String>, String> {
    let src = std::fs::read_to_string(project_file).map_err(|e| format!("no pude leer {}: {e}", project_file.display()))?;
    let p: Project = serde_json::from_str(&src).map_err(|e| format!("{}: {e}", project_file.display()))?;
    let wasm = find_wasm().ok_or(
        "falta nicostation.wasm. Compílalo una vez con:\n  cargo build --release --target wasm32-unknown-unknown --bin nicostation",
    )?;
    let base = project_file.parent().unwrap_or(Path::new("."));
    let mut written = vec![];
    let mut write = |rel: &str, data: &[u8]| -> Result<(), String> {
        let dst = out.join(rel);
        if let Some(d) = dst.parent() {
            std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
        }
        std::fs::write(&dst, data).map_err(|e| format!("{}: {e}", dst.display()))?;
        written.push(rel.to_string());
        Ok(())
    };
    let title = p.name.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    write("index.html", INDEX.replace("{{TITULO}}", &title).as_bytes())?;
    write("mq_js_bundle.js", BUNDLE.as_bytes())?;
    write("nicostation.wasm", &std::fs::read(&wasm).map_err(|e| format!("{}: {e}", wasm.display()))?)?;
    write("juego.json", src.as_bytes())?;
    for a in assets(&p) {
        // Solo rutas dentro de la carpeta del proyecto: nada de ../ ni rutas absolutas.
        if Path::new(&a).is_absolute() || a.split(['/', '\\']).any(|c| c == "..") {
            return Err(format!("la ruta «{a}» sale de la carpeta del proyecto; mueve el archivo dentro de ella"));
        }
        let data = std::fs::read(base.join(&a)).map_err(|e| format!("falta el archivo {a}: {e}"))?;
        write(&a, &data)?;
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    #[test]
    fn assets_del_ejemplo() {
        let src = include_str!("../examples/plataformas/juego.json");
        let p = serde_json::from_str(src).unwrap();
        let a = super::assets(&p);
        assert!(a.contains("img/heroe_quieto.png"));
        assert!(a.contains("snd/moneda.wav")); // dentro de una función
        assert!(a.contains("snd/salto.wav"));
    }
}
