//! Plantillas: juegos completos para empezar (carpeta `plantillas/` del repositorio).

use std::path::{Path, PathBuf};

pub struct Plantilla {
    pub carpeta: String,
    pub nombre: String,
    pub dir: PathBuf,
    /// Captura del juego (puede no existir).
    pub miniatura: PathBuf,
}

/// Carpeta de plantillas: `NICOSTATION_PLANTILLAS`, junto al ejecutable, o la del repositorio (target/release/../../plantillas).
pub fn dir() -> Option<PathBuf> {
    let mut c: Vec<PathBuf> = std::env::var_os("NICOSTATION_PLANTILLAS").map(PathBuf::from).into_iter().collect();
    if let Ok(exe) = std::env::current_exe() {
        let d = exe.parent().unwrap_or(Path::new("."));
        c.push(d.join("plantillas"));
        c.push(d.join("../../plantillas"));
    }
    c.into_iter().find(|p| p.is_dir())
}

pub fn lista() -> Vec<Plantilla> {
    let Some(d) = dir() else { return vec![] };
    let mut out: Vec<Plantilla> = std::fs::read_dir(&d).into_iter().flatten().flatten()
        .filter(|e| e.path().join("juego.json").is_file())
        .filter_map(|e| {
            let dir = e.path();
            let p: crate::model::Project = serde_json::from_str(&std::fs::read_to_string(dir.join("juego.json")).ok()?).ok()?;
            Some(Plantilla { carpeta: e.file_name().to_string_lossy().into_owned(), nombre: p.name, miniatura: dir.join("miniatura.png"), dir })
        })
        .collect();
    out.sort_by(|a, b| a.carpeta.cmp(&b.carpeta));
    out
}

/// Copia la plantilla a `destino` (carpeta nueva o vacía). Devuelve la ruta del juego.json creado.
pub fn crear(carpeta: &str, destino: &Path) -> Result<PathBuf, String> {
    let t = lista().into_iter().find(|t| t.carpeta == carpeta).ok_or(format!("no existe la plantilla «{carpeta}»"))?;
    if destino.join("juego.json").exists() {
        return Err(format!("{} ya tiene un juego.json", destino.display()));
    }
    copy_dir(&t.dir, destino)?;
    // Recordar la plantilla de origen (para las misiones), aunque luego cambie el nombre.
    let json = destino.join("juego.json");
    let mut p: crate::model::Project = serde_json::from_str(&std::fs::read_to_string(&json).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    p.plantilla = carpeta.into();
    std::fs::write(&json, serde_json::to_string_pretty(&p).map_err(|e| e.to_string())? + "\n").map_err(|e| e.to_string())?;
    Ok(json)
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| format!("{}: {e}", to.display()))?;
    for e in std::fs::read_dir(from).map_err(|e| e.to_string())?.flatten() {
        let (src, dst) = (e.path(), to.join(e.file_name()));
        if src.is_dir() {
            copy_dir(&src, &dst)?;
        } else if src.extension().is_none_or(|x| x != "py") {
            std::fs::copy(&src, &dst).map_err(|e| format!("{}: {e}", dst.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn crear_desde_plantilla() {
        std::env::set_var("NICOSTATION_PLANTILLAS", concat!(env!("CARGO_MANIFEST_DIR"), "/plantillas"));
        let l = super::lista();
        assert_eq!(l.iter().map(|t| t.carpeta.as_str()).collect::<Vec<_>>(), ["atrapa", "naves", "plataformas", "recolector"]);
        let dst = std::env::temp_dir().join(format!("nicostation-plantilla-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dst);
        let json = super::crear("naves", &dst).unwrap();
        let p: crate::model::Project = serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
        assert_eq!(crate::validate::validate(&p, Some(&dst)), Vec::<String>::new());
        assert!(!dst.join("generar.py").exists());
        assert!(super::crear("naves", &dst).is_err()); // no pisa un juego existente
        std::fs::remove_dir_all(&dst).unwrap();
    }
}
