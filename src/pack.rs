//! Juego en un solo ejecutable: el reproductor con el proyecto y sus archivos pegados al final.
//!
//! Formato (después de los bytes del reproductor, que no se tocan):
//! `[archivo 1][archivo 2]...[índice JSON][u64 LE: largo del índice]["NICOPACK"]`.
//! Tanto ELF (Linux) como PE (Windows) ignoran los datos que van después del final del programa,
//! así que el ejecutable sigue funcionando igual.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

const MAGIC: &[u8; 8] = b"NICOPACK";
const TRAILER: u64 = 16; // u64 + magia

#[derive(Serialize, Deserialize)]
struct Index {
    /// Dónde empieza el paquete dentro del ejecutable (= tamaño del reproductor).
    start: u64,
    /// Nombre del archivo del proyecto dentro del paquete.
    project: String,
    /// ruta → (posición absoluta, largo)
    files: BTreeMap<String, (u64, u64)>,
}

/// Paquete leído: el proyecto y todos sus archivos en memoria.
pub struct Pack {
    /// Ruta del proyecto dentro de `files` (p. ej. `juego.json`).
    pub project: String,
    pub files: BTreeMap<String, Vec<u8>>,
}

impl Pack {
    pub fn project_json(&self) -> &[u8] {
        self.files.get(&self.project).map(Vec::as_slice).unwrap_or_default()
    }
}

/// Lee el índice si `f` termina en un paquete.
fn read_index(f: &mut std::fs::File) -> Option<Index> {
    let len = f.seek(SeekFrom::End(0)).ok()?;
    if len < TRAILER {
        return None;
    }
    let mut t = [0u8; 16];
    f.seek(SeekFrom::Start(len - TRAILER)).ok()?;
    f.read_exact(&mut t).ok()?;
    if &t[8..] != MAGIC {
        return None;
    }
    let ilen = u64::from_le_bytes(t[..8].try_into().ok()?);
    let ipos = (len - TRAILER).checked_sub(ilen)?;
    let mut buf = vec![0u8; ilen as usize];
    f.seek(SeekFrom::Start(ipos)).ok()?;
    f.read_exact(&mut buf).ok()?;
    let idx: Index = serde_json::from_slice(&buf).ok()?;
    // Cada archivo debe caer entre el inicio del paquete y el índice.
    idx.files.values().all(|&(o, l)| o >= idx.start && o.checked_add(l).is_some_and(|e| e <= ipos)).then_some(idx)
}

/// Lee el paquete pegado al ejecutable `exe`, si lo tiene.
pub fn read_from(exe: &Path) -> Option<Pack> {
    let mut f = std::fs::File::open(exe).ok()?;
    let idx = read_index(&mut f)?;
    let mut files = BTreeMap::new();
    for (path, (off, len)) in idx.files {
        let mut b = vec![0u8; len as usize];
        f.seek(SeekFrom::Start(off)).ok()?;
        f.read_exact(&mut b).ok()?;
        files.insert(path, b);
    }
    files.contains_key(&idx.project).then_some(Pack { project: idx.project, files })
}

/// El paquete del ejecutable que está corriendo (`None` si es el reproductor normal).
pub fn read_embedded() -> Option<Pack> {
    read_from(&std::env::current_exe().ok()?)
}

/// Escribe en `out_exe` el reproductor `player_exe` con el proyecto y sus archivos pegados al final.
/// Si el reproductor ya traía un paquete, se descarta. Devuelve las rutas empaquetadas.
pub fn pack(player_exe: &Path, project_file: &Path, out_exe: &Path) -> Result<Vec<String>, String> {
    let src = std::fs::read(project_file).map_err(|e| format!("no pude leer {}: {e}", project_file.display()))?;
    let p = crate::model::parse_project(&src).map_err(|e| format!("{}: {e}", project_file.display()))?;
    let base = project_file.parent().unwrap_or(Path::new("."));
    let name = project_file.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or("juego.json".into());

    let mut entries: Vec<(String, Vec<u8>)> = vec![(name.clone(), src)];
    for a in crate::export::assets(&p) {
        crate::export::check_rel(&a)?;
        if a == name {
            return Err(format!("el archivo «{a}» tiene el mismo nombre que el proyecto"));
        }
        let data = std::fs::read(base.join(&a)).map_err(|e| format!("falta el archivo {a}: {e}"))?;
        entries.push((a, data));
    }

    let mut player = std::fs::File::open(player_exe).map_err(|e| format!("reproductor {}: {e}", player_exe.display()))?;
    let start = match read_index(&mut player) {
        Some(i) => i.start,
        None => player.seek(SeekFrom::End(0)).map_err(|e| e.to_string())?,
    };
    player.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;

    let err = |e: std::io::Error| format!("{}: {e}", out_exe.display());
    if let Some(d) = out_exe.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    // Se escribe a un temporal y se renombra: nunca queda un ejecutable a medias.
    let tmp = out_exe.with_extension("nicopack-tmp");
    let mut write = || -> Result<(), String> {
        let mut out = std::io::BufWriter::new(std::fs::File::create(&tmp).map_err(err)?);
        std::io::copy(&mut (&mut player).take(start), &mut out).map_err(err)?;
        let mut idx = Index { start, project: name.clone(), files: BTreeMap::new() };
        let mut pos = start;
        for (path, data) in &entries {
            out.write_all(data).map_err(err)?;
            idx.files.insert(path.clone(), (pos, data.len() as u64));
            pos += data.len() as u64;
        }
        let json = serde_json::to_vec(&idx).map_err(|e| e.to_string())?;
        out.write_all(&json).map_err(err)?;
        out.write_all(&(json.len() as u64).to_le_bytes()).map_err(err)?;
        out.write_all(MAGIC).map_err(err)?;
        out.into_inner().map_err(|e| e.to_string())?.sync_all().map_err(err)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755)).map_err(err)?;
        }
        std::fs::rename(&tmp, out_exe).map_err(err)
    };
    write().inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })?;
    Ok(entries.into_iter().map(|e| e.0).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("nicopack-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn project(dir: &Path, frame: &str) -> std::path::PathBuf {
        let p = serde_json::json!({
            "name": "Prueba", "width": 320, "height": 240, "first_scene": "A", "scenes": [],
            "objects": [{ "name": "Heroe", "type": "sprite", "animations": [{ "name": "x", "frames": [frame] }] }]
        });
        let f = dir.join("juego.json");
        std::fs::write(&f, p.to_string()).unwrap();
        f
    }

    #[test]
    fn empaquetar_y_leer() {
        let d = tmpdir("ida-vuelta");
        let player: Vec<u8> = b"MZ reproductor falso".iter().cycle().take(5000).copied().collect();
        std::fs::write(d.join("player"), &player).unwrap();
        std::fs::create_dir_all(d.join("img")).unwrap();
        std::fs::write(d.join("img/a.png"), b"PNGDATA").unwrap();
        let proj = project(&d, "img/a.png");
        assert!(read_from(&d.join("player")).is_none());

        let files = pack(&d.join("player"), &proj, &d.join("out/juego")).unwrap();
        assert_eq!(files, ["juego.json", "img/a.png"]);
        let out = std::fs::read(d.join("out/juego")).unwrap();
        assert_eq!(&out[..player.len()], &player[..], "el reproductor queda intacto");
        let pk = read_from(&d.join("out/juego")).unwrap();
        assert_eq!(pk.project, "juego.json");
        assert_eq!(pk.files["img/a.png"], b"PNGDATA");
        assert_eq!(pk.project_json(), std::fs::read(&proj).unwrap());

        // Reempaquetar desde un juego ya empaquetado descarta el paquete viejo.
        std::fs::write(d.join("img/a.png"), b"OTRA").unwrap();
        pack(&d.join("out/juego"), &proj, &d.join("out/juego2")).unwrap();
        let out2 = std::fs::read(d.join("out/juego2")).unwrap();
        assert_eq!(&out2[..player.len()], &player[..]);
        assert_eq!(read_from(&d.join("out/juego2")).unwrap().files["img/a.png"], b"OTRA");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn rutas_fuera_del_proyecto() {
        let d = tmpdir("rutas");
        std::fs::write(d.join("player"), b"MZ").unwrap();
        for bad in ["../secreto.png", "/etc/passwd", "img/../../x.png", "C:\\x.png", "img\\..\\..\\x.png"] {
            let proj = project(&d, bad);
            let e = pack(&d.join("player"), &proj, &d.join("out")).unwrap_err();
            assert!(e.contains("sale de la carpeta"), "{bad}: {e}");
            assert!(!d.join("out").exists());
        }
        // Falta un archivo: error claro y no queda nada escrito.
        let proj = project(&d, "img/no_existe.png");
        assert!(pack(&d.join("player"), &proj, &d.join("out")).unwrap_err().contains("falta el archivo"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn indice_corrupto_se_ignora() {
        let d = tmpdir("corrupto");
        let mut b = b"MZ basura".to_vec();
        b.extend_from_slice(&999999u64.to_le_bytes());
        b.extend_from_slice(MAGIC);
        std::fs::write(d.join("x"), &b).unwrap();
        assert!(read_from(&d.join("x")).is_none());
        let _ = std::fs::remove_dir_all(&d);
    }
}
