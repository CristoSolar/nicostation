//! Exportación del juego.
//! - Web (`export_web`): copia el reproductor WebAssembly, la página y los archivos del juego a una carpeta.
//!   El mismo `nicostation.wasm` sirve para cualquier juego: al arrancar lee `juego.json`.
//! - Escritorio (`export_desktop`): un solo ejecutable con todo adentro (ver `pack.rs`).

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
    let p = &p.compiled();
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

/// Una ruta de archivo del juego debe quedar dentro de la carpeta del proyecto:
/// nada de `../`, rutas absolutas ni letras de unidad.
pub fn check_rel(a: &str) -> Result<(), String> {
    let bad = Path::new(a).is_absolute() || a.starts_with(['/', '\\']) || a.contains(':') || a.split(['/', '\\']).any(|c| c == "..");
    if bad {
        return Err(format!("la ruta «{a}» sale de la carpeta del proyecto; mueve el archivo dentro de ella"));
    }
    Ok(())
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
        check_rel(&a)?;
        let data = std::fs::read(base.join(&a)).map_err(|e| format!("falta el archivo {a}: {e}"))?;
        write(&a, &data)?;
    }
    Ok(written)
}

/// Plataforma del ejecutable exportado.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopTarget {
    /// La misma plataforma en la que corre NicoStation.
    Current,
    /// Windows x86_64 (`.exe`), con el reproductor compilado para `x86_64-pc-windows-gnu`.
    Windows,
}

impl DesktopTarget {
    /// `"windows"` → Windows; `"linux"`, `"actual"` o vacío → la plataforma actual (si coincide).
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "windows" | "win" | "win64" => Ok(if cfg!(windows) { Self::Current } else { Self::Windows }),
            "" | "actual" | "current" => Ok(Self::Current),
            "linux" if cfg!(target_os = "linux") => Ok(Self::Current),
            "macos" if cfg!(target_os = "macos") => Ok(Self::Current),
            o => Err(format!("plataforma desconocida «{o}»; usa «windows» o deja la actual")),
        }
    }

    fn is_windows(self) -> bool {
        self == Self::Windows || cfg!(windows)
    }
}

const WINDOWS_HELP: &str = "falta el reproductor para Windows (nicostation.exe). Compílalo una vez con:
  rustup target add x86_64-pc-windows-gnu
  sudo pacman -S mingw-w64-gcc        (en Arch/Omarchy; en Debian/Ubuntu: sudo apt install mingw-w64)
  cargo build --release --target x86_64-pc-windows-gnu --bin nicostation
o indica dónde está con la variable NICOSTATION_PLAYER_WINDOWS=/ruta/nicostation.exe";

/// Busca el reproductor que sirve de base para el ejecutable exportado.
/// Plataforma actual: `NICOSTATION_PLAYER`, este mismo programa si es el reproductor, o `nicostation` junto a él
/// (así el editor también puede exportar). Windows: `NICOSTATION_PLAYER_WINDOWS` o
/// `target/x86_64-pc-windows-gnu/release/nicostation.exe` (junto a `target/release/` o desde la carpeta actual).
pub fn find_player(target: DesktopTarget) -> Result<PathBuf, String> {
    let exe = std::env::current_exe().ok();
    let dir = exe.as_deref().and_then(Path::parent).map(Path::to_path_buf).unwrap_or_default();
    let mut c: Vec<PathBuf> = vec![];
    match target {
        DesktopTarget::Windows => {
            c.extend(std::env::var_os("NICOSTATION_PLAYER_WINDOWS").map(PathBuf::from));
            c.push(dir.join("../x86_64-pc-windows-gnu/release/nicostation.exe"));
            c.push(dir.join("nicostation.exe"));
            c.push(PathBuf::from("target/x86_64-pc-windows-gnu/release/nicostation.exe"));
        }
        DesktopTarget::Current => {
            c.extend(std::env::var_os("NICOSTATION_PLAYER").map(PathBuf::from));
            let name = format!("nicostation{}", std::env::consts::EXE_SUFFIX);
            if let Some(e) = exe.filter(|e| e.file_name().is_some_and(|f| f == name.as_str())) {
                c.push(e);
            }
            c.push(dir.join(&name));
        }
    }
    c.into_iter().find(|p| p.is_file()).ok_or_else(|| match target {
        DesktopTarget::Windows => WINDOWS_HELP.to_string(),
        DesktopTarget::Current => "no encuentro el reproductor «nicostation». Compílalo con:\n  cargo build --release --bin nicostation".into(),
    })
}

/// Exporta el juego como **un solo ejecutable** (sin json, imágenes ni sonidos sueltos).
///
/// - `project_file`: el `juego.json`; sus imágenes y sonidos se buscan relativos a él (ver [`assets`]).
/// - `out`: archivo de salida. Para Windows se agrega `.exe` si falta.
/// - `target`: [`DesktopTarget::Current`] usa el reproductor de esta plataforma; [`DesktopTarget::Windows`] necesita
///   el reproductor compilado para Windows (si falta, el error explica cómo compilarlo).
///
/// Devuelve la ruta final y la lista de archivos empaquetados. Los errores vienen en castellano, listos para mostrar.
pub fn export_desktop(project_file: &Path, out: &Path, target: DesktopTarget) -> Result<(PathBuf, Vec<String>), String> {
    let player = find_player(target)?;
    let out = if target.is_windows() && !out.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe")) {
        let mut s = out.as_os_str().to_owned();
        s.push(".exe");
        PathBuf::from(s)
    } else {
        out.to_path_buf()
    };
    if target == DesktopTarget::Windows {
        let mut head = [0u8; 2];
        let ok = std::fs::File::open(&player).and_then(|mut f| std::io::Read::read_exact(&mut f, &mut head)).is_ok() && &head == b"MZ";
        if !ok {
            return Err(format!("{} no es un ejecutable de Windows", player.display()));
        }
    }
    if std::fs::canonicalize(&out).ok().is_some_and(|o| std::fs::canonicalize(&player).ok() == Some(o)) {
        return Err("el archivo de salida no puede ser el mismo reproductor".into());
    }
    let files = crate::pack::pack(&player, project_file, &out)?;
    Ok((out, files))
}

#[cfg(test)]
mod tests {
    #[test]
    fn rutas_seguras() {
        for ok in ["img/a.png", "snd/x y.wav", "a..b.png"] {
            assert!(super::check_rel(ok).is_ok(), "{ok}");
        }
        for bad in ["../a.png", "/a.png", "\\a.png", "C:\\a.png", "img/../../a.png", "img\\..\\a.png"] {
            assert!(super::check_rel(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn exportar_para_windows() {
        let d = std::env::temp_dir().join(format!("nicoexport-win-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("img")).unwrap();
        std::fs::write(d.join("img/a.png"), b"PNG").unwrap();
        let proj = d.join("juego.json");
        let p = serde_json::json!({ "name": "W", "width": 10, "height": 10, "first_scene": "A", "scenes": [],
            "objects": [{ "name": "O", "type": "sprite", "animations": [{ "name": "x", "frames": ["img/a.png"] }] }] });
        std::fs::write(&proj, p.to_string()).unwrap();
        std::fs::write(d.join("falso.exe"), b"MZ\x90\0 reproductor de Windows falso").unwrap();
        std::fs::write(d.join("no_pe"), b"\x7fELF").unwrap();
        // Un solo test toca la variable de entorno, para no pisarse con otros en paralelo.
        std::env::set_var("NICOSTATION_PLAYER_WINDOWS", d.join("no_pe"));
        assert!(super::export_desktop(&proj, &d.join("juego"), super::DesktopTarget::Windows).unwrap_err().contains("no es un ejecutable de Windows"));
        std::env::set_var("NICOSTATION_PLAYER_WINDOWS", d.join("falso.exe"));
        let (out, files) = super::export_desktop(&proj, &d.join("juego"), super::DesktopTarget::Windows).unwrap();
        std::env::remove_var("NICOSTATION_PLAYER_WINDOWS");
        assert_eq!(out, d.join("juego.exe"));
        assert_eq!(files, ["juego.json", "img/a.png"]);
        let pk = crate::pack::read_from(&out).unwrap();
        assert_eq!(pk.files["img/a.png"], b"PNG");
        assert!(std::fs::read(&out).unwrap().starts_with(b"MZ\x90\0"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn plataformas() {
        use super::DesktopTarget::*;
        assert_eq!(super::DesktopTarget::parse("").unwrap(), Current);
        assert_eq!(super::DesktopTarget::parse("Windows").unwrap(), if cfg!(windows) { Current } else { Windows });
        assert!(super::DesktopTarget::parse("amiga").is_err());
    }

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
