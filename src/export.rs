//! Exportación del juego.
//! - Web (`export_web`): copia el reproductor WebAssembly, la página y los archivos del juego a una carpeta.
//!   El mismo `nicostation.wasm` sirve para cualquier juego: al arrancar lee `juego.json`.
//! - Escritorio (`export_desktop`): un solo ejecutable con todo adentro (ver `pack.rs`).
//! - Android (`export_android`): el APK plantilla del reproductor con el juego en `assets/`, alineado y firmado.

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
            Kind::Mesh { model, texture, .. } => out.extend([model, texture].into_iter().filter(|f| !f.is_empty()).cloned()),
            Kind::Background { image, .. } => {
                out.insert(image.clone());
            }
            Kind::Particles(def) => out.extend([def.image.clone()].into_iter().filter(|f| !f.is_empty())),
            Kind::Text { font, .. } => out.extend([font.clone()].into_iter().filter(|f| !f.is_empty())),
            _ => {}
        }
    }
    out.extend([&p.settings.icon, &p.settings.loading_image].into_iter().filter(|f| !f.is_empty()).cloned());
    for s in &p.scenes {
        events(&s.events, &mut out);
        if let Some(img) = s.environment.as_ref().map(|e| &e.sky_image).filter(|i| !i.is_empty()) {
            out.insert(img.clone());
        }
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
    let p = crate::model::parse_project(src.as_bytes()).map_err(|e| format!("{}: {e}", project_file.display()))?;
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
    // Ícono de la pestaña: el del juego (ajustes) si tiene, o el de NicoStation.
    let favicon = Some(&p.settings.icon).filter(|i| !i.is_empty()).and_then(|i| std::fs::read(base.join(i)).ok());
    write("icono.png", favicon.as_deref().unwrap_or(include_bytes!("../assets/web/favicon.png")))?;
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

#[cfg(not(target_arch = "wasm32"))]
const ANDROID_HELP: &str = "falta el APK plantilla del reproductor para Android. Se compila una vez con docker (ver README, «Exportar a Android»):
  docker build -t nicostation-android assets/android
  docker run --rm -v $PWD:/root/src -v nico-cargo-registry:/usr/local/cargo/registry -w /root/src nicostation-android \
    bash -c 'cargo quad-apk build --release --bin nicostation; chown -R $(id -u):$(id -g) target'
o indica dónde está con NICOSTATION_ANDROID_TEMPLATE=/ruta/nicostation.apk";

#[cfg(not(target_arch = "wasm32"))]
/// Busca el APK plantilla (el reproductor compilado para Android, sin juego adentro):
/// `NICOSTATION_ANDROID_TEMPLATE`, `target/android-artifacts/release/apk/nicostation.apk` (junto a `target/release/`
/// o desde la carpeta actual) o `nicostation.apk` junto a este programa.
pub fn find_android_template() -> Result<PathBuf, String> {
    let dir = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)).unwrap_or_default();
    let mut c: Vec<PathBuf> = std::env::var_os("NICOSTATION_ANDROID_TEMPLATE").map(PathBuf::from).into_iter().collect();
    c.push(dir.join("../android-artifacts/release/apk/nicostation.apk"));
    c.push(PathBuf::from("target/android-artifacts/release/apk/nicostation.apk"));
    c.push(dir.join("nicostation.apk"));
    c.into_iter().find(|p| p.is_file()).ok_or_else(|| ANDROID_HELP.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
/// Copia el APK plantilla sin su firma y le agrega el juego en `assets/` (ahí lo busca el reproductor en Android).
/// El resultado no está alineado ni firmado.
pub fn apk_with_game(template: &Path, files: &[(String, Vec<u8>)], out: &Path) -> Result<(), String> {
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    let err = |e: zip::result::ZipError| format!("{}: {e}", template.display());
    let mut src = zip::ZipArchive::new(std::fs::File::open(template).map_err(|e| format!("{}: {e}", template.display()))?).map_err(err)?;
    let mut dst = zip::ZipWriter::new(std::fs::File::create(out).map_err(|e| format!("{}: {e}", out.display()))?);
    for i in 0..src.len() {
        let f = src.by_index_raw(i).map_err(err)?;
        // La firma vieja no sirve (cambia el contenido) y los assets del juego van nuevos.
        if f.name().starts_with("META-INF/") || f.name().starts_with("assets/") {
            continue;
        }
        dst.raw_copy_file(f).map_err(err)?;
    }
    for (name, data) in files {
        dst.start_file(format!("assets/{name}"), SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated)).map_err(err)?;
        dst.write_all(data).map_err(|e| e.to_string())?;
    }
    dst.finish().map_err(err)?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
/// Corre una herramienta de Android (`zipalign`, `apksigner`, `keytool`): instalada (PATH o `$ANDROID_HOME/build-tools/*`)
/// o, si no, dentro de docker con la imagen `NICOSTATION_ANDROID_IMAGE` (por defecto `notfl3/cargo-apk`, que las trae).
/// `dir` se monta como /w en docker; las rutas de `args` deben ser relativas a `dir`.
fn android_tool(tool: &str, args: &[&str], dir: &Path) -> Result<(), String> {
    let mut local: Vec<PathBuf> = vec![];
    if let Some(home) = std::env::var_os("ANDROID_HOME").or_else(|| std::env::var_os("ANDROID_SDK_ROOT")) {
        let mut bt: Vec<PathBuf> = std::fs::read_dir(Path::new(&home).join("build-tools")).into_iter().flatten().flatten().map(|e| e.path()).collect();
        bt.sort();
        local.extend(bt.into_iter().rev().map(|d| d.join(tool)));
    }
    let in_path = std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(tool).is_file()));
    let exe = local.into_iter().find(|p| p.is_file()).or(in_path.then(|| PathBuf::from(tool)));
    let output = match exe {
        Some(exe) => std::process::Command::new(exe).args(args).current_dir(dir).output(),
        None => {
            let image = std::env::var("NICOSTATION_ANDROID_IMAGE").unwrap_or("notfl3/cargo-apk".into());
            let mut cmd = std::process::Command::new("docker");
            cmd.args(["run", "--rm", "-v", &format!("{}:/w", dir.display()), "-w", "/w", "-e", "HOME=/w"]);
            #[cfg(unix)]
            {
                // Que los archivos que crea docker sean del usuario, no de root.
                let id = |f: &str| std::process::Command::new("id").arg(f).output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
                if let (Some(u), Some(g)) = (id("-u"), id("-g")) {
                    cmd.args(["--user", &format!("{u}:{g}")]);
                }
            }
            cmd.arg(&image).arg(tool).args(args).output()
        }
    };
    let o = output.map_err(|e| format!("no pude correr {tool} (ni instalado ni con docker): {e}"))?;
    if !o.status.success() {
        return Err(format!("{tool} falló:\n{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)));
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
/// Exporta el juego a Android: un `.apk` firmado con una clave de depuración, listo para `adb install`.
///
/// Toma el APK plantilla (ver [`find_android_template`]), le pone `juego.json` y sus archivos en `assets/`, lo alinea con
/// `zipalign` y lo firma con `apksigner` (instalados o vía docker). La clave de depuración se crea una vez en
/// `<carpeta de datos>/nicostation/android-debug.keystore` (o `NICOSTATION_ANDROID_KEYSTORE`, contraseña `android`).
/// Todos los juegos salen con el mismo id (`com.nicostation.nicostation`): instalar uno reemplaza al anterior.
pub fn export_android(project_file: &Path, out: &Path) -> Result<(PathBuf, Vec<String>), String> {
    let template = find_android_template()?;
    let src = std::fs::read(project_file).map_err(|e| format!("no pude leer {}: {e}", project_file.display()))?;
    let p = crate::model::parse_project(&src).map_err(|e| format!("{}: {e}", project_file.display()))?;
    let base = project_file.parent().unwrap_or(Path::new("."));
    // En Android el reproductor arranca sin argumentos y busca `juego.json`.
    let mut files = vec![("juego.json".to_string(), src)];
    for a in assets(&p) {
        check_rel(&a)?;
        if a == "juego.json" {
            return Err("un archivo del juego no puede llamarse juego.json".into());
        }
        files.push((a.replace('\\', "/"), std::fs::read(base.join(&a)).map_err(|e| format!("falta el archivo {a}: {e}"))?));
    }
    let out = if out.extension().is_some_and(|e| e.eq_ignore_ascii_case("apk")) { out.to_path_buf() } else { out.with_extension("apk") };

    let work = std::env::temp_dir().join(format!("nicostation-apk-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).map_err(|e| format!("{}: {e}", work.display()))?;
    let result = (|| {
        apk_with_game(&template, &files, &work.join("sin-firma.apk"))?;
        android_tool("zipalign", &["-p", "-f", "4", "sin-firma.apk", "alineado.apk"], &work)?;
        let ks = match std::env::var_os("NICOSTATION_ANDROID_KEYSTORE") {
            Some(k) => PathBuf::from(k),
            None => crate::guardado::data_dir(|k| std::env::var(k).ok()).unwrap_or_else(std::env::temp_dir).join("nicostation/android-debug.keystore"),
        };
        if !ks.is_file() {
            android_tool("keytool", &["-genkeypair", "-keystore", "debug.keystore", "-storepass", "android", "-keypass", "android", "-alias", "androiddebugkey",
                "-keyalg", "RSA", "-keysize", "2048", "-validity", "10000", "-dname", "CN=Android Debug,O=Android,C=US"], &work)?;
            if let Some(d) = ks.parent() {
                std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
            }
            std::fs::copy(work.join("debug.keystore"), &ks).map_err(|e| format!("{}: {e}", ks.display()))?;
        } else {
            std::fs::copy(&ks, work.join("debug.keystore")).map_err(|e| format!("{}: {e}", ks.display()))?;
        }
        android_tool("apksigner", &["sign", "--ks", "debug.keystore", "--ks-pass", "pass:android", "--key-pass", "pass:android", "--out", "firmado.apk", "alineado.apk"], &work)?;
        if let Some(d) = out.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
        }
        std::fs::copy(work.join("firmado.apk"), &out).map_err(|e| format!("{}: {e}", out.display()))?;
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&work);
    result.map(|()| (out, files.into_iter().map(|f| f.0).collect()))
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

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn apk_con_el_juego() {
        use std::io::{Read, Write};
        let d = std::env::temp_dir().join(format!("nicoexport-apk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        // Plantilla falsa: manifiesto, una biblioteca guardada sin comprimir, una firma vieja y un asset viejo.
        let mut z = zip::ZipWriter::new(std::fs::File::create(d.join("t.apk")).unwrap());
        let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (n, b) in [("AndroidManifest.xml", &b"MANIFEST"[..]), ("lib/arm64-v8a/libnicostation.so", b"ELF"), ("META-INF/CERT.RSA", b"firma"), ("assets/viejo.txt", b"x")] {
            z.start_file(n, stored).unwrap();
            z.write_all(b).unwrap();
        }
        z.finish().unwrap();
        let files = vec![("juego.json".to_string(), b"{}".to_vec()), ("img/a.png".to_string(), b"PNG".to_vec())];
        super::apk_with_game(&d.join("t.apk"), &files, &d.join("o.apk")).unwrap();
        let mut a = zip::ZipArchive::new(std::fs::File::open(d.join("o.apk")).unwrap()).unwrap();
        let mut names: Vec<String> = a.file_names().map(String::from).collect();
        names.sort();
        assert_eq!(names, ["AndroidManifest.xml", "assets/img/a.png", "assets/juego.json", "lib/arm64-v8a/libnicostation.so"]);
        assert_eq!(a.by_name("lib/arm64-v8a/libnicostation.so").unwrap().compression(), zip::CompressionMethod::Stored, "se copia tal cual");
        let mut s = String::new();
        a.by_name("assets/juego.json").unwrap().read_to_string(&mut s).unwrap();
        assert_eq!(s, "{}");
        let _ = std::fs::remove_dir_all(&d);
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
