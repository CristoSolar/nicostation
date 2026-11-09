//! Partida guardada: archivo JSON en escritorio, localStorage en el navegador.
//! La usan los dos reproductores (`nicostation` y `nicostation3d`).
use crate::model::Vars;

/// Nombre del juego apto para carpeta o clave: minúsculas, sin acentos, guiones.
pub fn slug(name: &str) -> String {
    let mut s = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        let c = match c {
            'á' | 'à' | 'ä' | 'â' | 'ã' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' | 'õ' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            s.push(c);
        } else if !s.is_empty() && !s.ends_with('-') {
            s.push('-');
        }
    }
    let s = s.trim_end_matches('-');
    if s.is_empty() { "juego".into() } else { s.chars().take(64).collect() }
}

/// Carpeta de datos del usuario, a partir de las variables de entorno.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn data_dir(env: impl Fn(&str) -> Option<String>) -> Option<std::path::PathBuf> {
    let env = |k: &str| env(k).filter(|v| !v.is_empty()).map(std::path::PathBuf::from);
    if cfg!(windows) {
        env("APPDATA")
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|h| h.join("Library/Application Support"))
    } else {
        env("XDG_DATA_HOME").filter(|p| p.is_absolute()).or_else(|| env("HOME").map(|h| h.join(".local/share")))
    }
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn save_file(data_dir: &std::path::Path, game: &str) -> std::path::PathBuf {
    data_dir.join("nicostation").join(slug(game)).join("guardado.json")
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn read(file: &std::path::Path) -> Vars {
    match std::fs::read(file) {
        Ok(b) => serde_json::from_slice(&b).unwrap_or_else(|e| {
            eprintln!("partida guardada dañada ({}): {e}; se empieza de cero", file.display());
            Vars::new()
        }),
        Err(_) => Vars::new(),
    }
}

/// Escribe a un temporal y lo renombra: si se corta la luz, queda la partida anterior entera.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub fn write(file: &std::path::Path, v: &Vars) -> std::io::Result<()> {
    if let Some(d) = file.parent() {
        std::fs::create_dir_all(d)?;
    }
    let tmp = file.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(v)?)?;
    std::fs::rename(&tmp, file)
}

#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
fn native_file(game: &str) -> Option<std::path::PathBuf> {
    data_dir(|k| std::env::var(k).ok()).map(|d| save_file(&d, game))
}

/// Android: carpeta privada de la app, /data/data/<paquete>/files (el paquete sale de /proc/self/cmdline).
#[cfg(target_os = "android")]
fn native_file(game: &str) -> Option<std::path::PathBuf> {
    let cmd = std::fs::read("/proc/self/cmdline").ok()?;
    let pkg = String::from_utf8_lossy(cmd.split(|b| *b == 0).next()?).split(':').next()?.to_string();
    Some(save_file(&std::path::Path::new("/data/data").join(pkg).join("files"), game))
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load(game: &str) -> Vars {
    native_file(game).map(|f| read(&f)).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn store(game: &str, v: &Vars) {
    let Some(f) = native_file(game) else { return eprintln!("no sé dónde guardar la partida (falta HOME)") };
    if let Err(e) = write(&f, v) {
        eprintln!("no pude guardar la partida en {}: {e}", f.display());
    }
}

// En el navegador: funciones del plugin «nicostation_storage» de mq_js_bundle.js.
#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
extern "C" {
    fn nicostation_storage_get(key: *const u8, key_len: u32) -> i32;
    fn nicostation_storage_take(buf: *mut u8);
    fn nicostation_storage_set(key: *const u8, key_len: u32, val: *const u8, val_len: u32);
}

#[cfg(target_arch = "wasm32")]
fn key(game: &str) -> String {
    format!("nicostation:{}", slug(game))
}

#[cfg(target_arch = "wasm32")]
pub fn load(game: &str) -> Vars {
    let k = key(game);
    // El largo en bytes del valor guardado (≤ 0 si no hay nada o falta el plugin), y después se copia.
    let n = unsafe { nicostation_storage_get(k.as_ptr(), k.len() as u32) };
    if n <= 0 {
        return Vars::new();
    }
    let mut buf = vec![0u8; n as usize];
    unsafe { nicostation_storage_take(buf.as_mut_ptr()) };
    serde_json::from_slice(&buf).unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
pub fn store(game: &str, v: &Vars) {
    let (k, s) = (key(game), serde_json::to_string(v).unwrap_or_default());
    unsafe { nicostation_storage_set(k.as_ptr(), k.len() as u32, s.as_ptr(), s.len() as u32) };
}
