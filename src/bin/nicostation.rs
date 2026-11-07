//! Reproductor: `nicostation proyecto.json`. Ventana, imágenes, sonido y entrada con macroquad.
//! También compila a WebAssembly: en el navegador lee `juego.json` junto a la página.
//! Exportar a web: `nicostation --export-web proyecto.json carpeta/`.
//! Exportar a un solo ejecutable: `nicostation --export-desktop proyecto.json salida [--target windows]`.
//! Si el ejecutable trae un juego empaquetado (ver `pack.rs`), lo juega sin leer nada de disco.

use macroquad::audio::{load_sound_from_bytes, play_sound, stop_sound, PlaySoundParams, Sound};
use macroquad::prelude::*;
use nicostation::model::{Kind, Project};
use nicostation::pack::Pack;
use nicostation::runtime::{Cmd, Input, World};
use std::collections::HashMap;

fn project_path() -> String {
    std::env::args().nth(1).unwrap_or("juego.json".into())
}

/// El juego empaquetado dentro de este ejecutable (se lee una sola vez).
fn embedded() -> Option<&'static Pack> {
    static PACK: std::sync::OnceLock<Option<Pack>> = std::sync::OnceLock::new();
    #[cfg(not(target_arch = "wasm32"))]
    return PACK.get_or_init(nicostation::pack::read_embedded).as_ref();
    #[cfg(target_arch = "wasm32")]
    return PACK.get_or_init(|| None).as_ref();
}

fn conf() -> Conf {
    let c = Conf { window_title: "NicoStation".into(), window_width: 800, window_height: 600, window_resizable: true, ..Default::default() };
    // En escritorio la ventana toma el tamaño y nombre del juego; en web los define la página.
    #[cfg(not(target_arch = "wasm32"))]
    let c = {
        let src = match embedded() {
            Some(pk) => Some(pk.project_json().to_vec()),
            None => std::fs::read(project_path()).ok(),
        };
        match src.and_then(|s| serde_json::from_slice::<Project>(&s).ok()) {
            Some(p) => Conf { window_title: p.name, window_width: p.width as i32, window_height: p.height as i32, ..c },
            None => c,
        }
    };
    c
}

/// Partida guardada: archivo JSON en escritorio, localStorage en el navegador.
mod guardado {
    use nicostation::model::Vars;

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

    #[cfg(not(target_arch = "wasm32"))]
    fn native_file(game: &str) -> Option<std::path::PathBuf> {
        data_dir(|k| std::env::var(k).ok()).map(|d| save_file(&d, game))
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
}

/// De dónde salen los archivos del juego: del paquete pegado al ejecutable o de la carpeta / el servidor.
async fn read_file(path: &str) -> Result<Vec<u8>, String> {
    match embedded() {
        Some(pk) => pk.files.get(path).cloned().ok_or_else(|| "no está en el paquete".into()),
        None => load_file(path).await.map_err(|e| e.to_string()),
    }
}

async fn read_texture(path: &str) -> Result<Texture2D, String> {
    let b = read_file(path).await?;
    let t = Texture2D::from_file_with_format(&b, None);
    t.set_filter(FilterMode::Nearest);
    Ok(t)
}

async fn read_sound(path: &str) -> Result<Sound, String> {
    let b = read_file(path).await?;
    load_sound_from_bytes(&b).await.map_err(|e| e.to_string())
}

/// Pantalla de carga «Cargando… n/m». Solo dibuja de vez en cuando para no frenar la carga.
async fn loading(n: usize, m: usize, last: &mut f64) {
    if n != 0 && get_time() - *last < 0.05 {
        return;
    }
    *last = get_time();
    let (sw, sh) = (screen_width(), screen_height());
    clear_background(Color::from_rgba(12, 12, 14, 255));
    let (w, h) = ((sw * 0.5).clamp(160., 480.), 10.);
    let (x, y) = ((sw - w) / 2., sh / 2.);
    draw_rectangle(x, y, w, h, Color::new(1., 1., 1., 0.15));
    draw_rectangle(x, y, w * n as f32 / m.max(1) as f32, h, Color::new(1., 1., 1., 0.85));
    let t = format!("Cargando… {n}/{m}");
    let tw = measure_text(&t, None, 24, 1.).width;
    draw_text(&t, (sw - tw) / 2., y - 16., 24., Color::new(1., 1., 1., 0.85));
    next_frame().await
}

/// Botones táctiles: (centro, radio, tecla). Cruceta de 4 direcciones o izquierda/derecha, más el botón de acción.
fn touch_buttons(sw: f32, sh: f32, four_way: bool) -> Vec<(Vec2, f32, &'static str)> {
    let r = (sw.min(sh) * 0.08).clamp(32., 64.);
    let m = r * 1.5;
    let action = (vec2(sw - m, sh - m), r * 1.25, "Space");
    if four_way {
        let d = r * 2.1;
        let c = vec2(d + r + 12., sh - d - r - 12.);
        vec![(c - vec2(d, 0.), r, "Left"), (c + vec2(d, 0.), r, "Right"), (c - vec2(0., d), r, "Up"), (c + vec2(0., d), r, "Down"), action]
    } else {
        vec![(vec2(m, sh - m), r, "Left"), (vec2(m + r * 2.6, sh - m), r, "Right"), action]
    }
}

fn draw_touch_buttons(buttons: &[(Vec2, f32, &str)], held: &std::collections::HashSet<&str>) {
    for (c, r, k) in buttons {
        let on = held.contains(k);
        draw_circle(c.x, c.y, *r, Color::new(1., 1., 1., if on { 0.35 } else { 0.18 }));
        draw_circle_lines(c.x, c.y, *r, 2., Color::new(1., 1., 1., 0.5));
        let a = r * 0.45;
        let (p1, p2, p3) = match *k {
            "Left" => (vec2(-a, 0.), vec2(a * 0.6, -a), vec2(a * 0.6, a)),
            "Right" => (vec2(a, 0.), vec2(-a * 0.6, -a), vec2(-a * 0.6, a)),
            "Up" => (vec2(0., -a), vec2(-a, a * 0.6), vec2(a, a * 0.6)),
            "Down" => (vec2(0., a), vec2(-a, -a * 0.6), vec2(a, -a * 0.6)),
            _ => {
                draw_circle(c.x, c.y, r * 0.35, Color::new(1., 1., 1., 0.6));
                continue;
            }
        };
        draw_triangle(*c + p1, *c + p2, *c + p3, Color::new(1., 1., 1., 0.7));
    }
}

fn key_name(k: KeyCode) -> String {
    format!("{k:?}")
}

fn main() {
    let conf = conf(); // antes de cambiar de carpeta: lee el proyecto con la ruta original
    // Un juego empaquetado no recibe opciones ni cambia de carpeta: todo está adentro.
    #[cfg(not(target_arch = "wasm32"))]
    if embedded().is_none() {
        let a: Vec<String> = std::env::args().collect();
        if a.get(1).map(String::as_str) == Some("--export-desktop") {
            let mut target = nicostation::export::DesktopTarget::Current;
            let mut rest = vec![];
            let mut it = a[2..].iter();
            while let Some(x) = it.next() {
                if x == "--target" {
                    match it.next().map(|t| nicostation::export::DesktopTarget::parse(t)) {
                        Some(Ok(t)) => target = t,
                        Some(Err(e)) => {
                            eprintln!("{e}");
                            std::process::exit(2);
                        }
                        None => rest.push(x),
                    }
                } else {
                    rest.push(x);
                }
            }
            let [p, out] = rest[..] else {
                eprintln!("uso: nicostation --export-desktop proyecto.json salida [--target windows]");
                std::process::exit(2);
            };
            match nicostation::export::export_desktop(p.as_ref(), out.as_ref(), target) {
                Ok((f, files)) => println!("Exportado a {} ({} archivos adentro). Es un solo ejecutable: cópialo y listo.", f.display(), files.len()),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
            return;
        }
        if a.get(1).map(String::as_str) == Some("--export-web") {
            let (Some(p), Some(out)) = (a.get(2), a.get(3)) else {
                eprintln!("uso: nicostation --export-web proyecto.json carpeta/");
                std::process::exit(2);
            };
            match nicostation::export::export_web(p.as_ref(), out.as_ref()) {
                Ok(f) => println!("Exportado a {out} ({} archivos). Pruébalo con:\n  python3 -m http.server -d {out} 8000", f.len()),
                Err(e) => {
                    eprintln!("{e}");
                    std::process::exit(1);
                }
            }
            return;
        }
        let path = std::path::PathBuf::from(project_path());
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::env::set_current_dir(dir).expect("no pude entrar a la carpeta del proyecto");
        }
    }
    macroquad::Window::from_config(conf, game());
}

async fn game() {
    let (file, src) = match embedded() {
        Some(pk) => (pk.project.clone(), pk.project_json().to_vec()),
        None => {
            // Ya estamos en la carpeta del proyecto: basta el nombre del archivo.
            let file = std::path::Path::new(&project_path()).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or("juego.json".into());
            let src = load_file(&file).await.unwrap_or_else(|e| panic!("no pude leer {file}: {e}"));
            (file, src)
        }
    };
    let mut project: Project = serde_json::from_slice(&src).unwrap_or_else(|e| panic!("{file}: {e}"));

    // Imágenes de objetos y sonidos de los eventos, con pantalla de carga.
    let mut images: Vec<String> = vec![];
    for o in project.objects.iter().chain(project.scenes.iter().flat_map(|s| s.objects.iter())) {
        match &o.kind {
            Kind::Tilemap { tileset, .. } => images.push(tileset.clone()),
            Kind::Sprite { animations, .. } => images.extend(animations.iter().flat_map(|a| a.frames.iter().cloned())),
            _ => {}
        }
    }
    images.sort();
    images.dedup();
    let audio: Vec<String> = nicostation::export::assets(&project).into_iter().filter(|a| images.binary_search(a).is_err()).collect();
    let total = images.len() + audio.len();
    let mut last = 0.;
    let mut tex: HashMap<String, Texture2D> = HashMap::new();
    for (n, f) in images.iter().enumerate() {
        loading(n, total, &mut last).await;
        match read_texture(f).await {
            Ok(t) => {
                tex.insert(f.clone(), t);
            }
            Err(e) => eprintln!("imagen {f}: {e}"),
        }
    }
    let mut sounds: HashMap<String, Sound> = HashMap::new();
    for (n, f) in audio.iter().enumerate() {
        loading(images.len() + n, total, &mut last).await;
        match read_sound(f).await {
            Ok(s) => {
                sounds.insert(f.clone(), s);
            }
            Err(e) => eprintln!("sonido {f}: {e}"),
        }
    }
    // Los sprites sin tamaño toman el de su primera imagen.
    for o in project.objects.iter_mut().chain(project.scenes.iter_mut().flat_map(|s| s.objects.iter_mut())) {
        if let Kind::Sprite { w, h, animations } = &mut o.kind {
            if let Some(t) = animations.first().and_then(|a| a.frames.first()).and_then(|f| tex.get(f)) {
                if *w == 0. { *w = t.width() }
                if *h == 0. { *h = t.height() }
            }
        }
    }

    // Cruceta de 4 direcciones si el juego se mueve en vista de arriba.
    let four_way = project.objects.iter().chain(project.scenes.iter().flat_map(|s| s.objects.iter()))
        .any(|o| o.behaviors.iter().any(|b| matches!(b, nicostation::model::Behavior::TopDown { .. })));
    // NICOSTATION_TACTIL=1 muestra los botones desde el inicio (para probar sin pantalla táctil).
    let mut touch_seen = cfg!(not(target_arch = "wasm32")) && std::env::var_os("NICOSTATION_TACTIL").is_some();
    let mut touch_prev: std::collections::HashSet<&str> = Default::default();
    let game_name = project.name.clone();
    let mut world = World::new(project);
    // La partida guardada tiene que estar antes del primer paso (los eventos «al empezar» la leen).
    world.storage = guardado::load(&game_name);
    let mut music: Option<Sound> = None;

    loop {
        let p = world.project.clone();
        let scale = (screen_width() / p.width).min(screen_height() / p.height);
        let off = vec2((screen_width() - p.width * scale) / 2., (screen_height() - p.height * scale) / 2.);

        let (mx, my) = mouse_position();
        // Toques: los botones en pantalla aparecen la primera vez que se toca (celular o tablet).
        let ts = touches();
        touch_seen |= !ts.is_empty();
        let buttons = if touch_seen { touch_buttons(screen_width(), screen_height(), four_way) } else { vec![] };
        let held: std::collections::HashSet<&str> = buttons.iter()
            .filter(|(c, r, _)| ts.iter().any(|t| !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled) && t.position.distance(*c) < r * 1.2))
            .map(|b| b.2).collect();
        let mut down: std::collections::HashSet<String> = get_keys_down().into_iter().map(key_name).collect();
        let mut pressed: std::collections::HashSet<String> = get_keys_pressed().into_iter().map(key_name).collect();
        let mut released: std::collections::HashSet<String> = get_keys_released().into_iter().map(key_name).collect();
        down.extend(held.iter().map(|k| k.to_string()));
        pressed.extend(held.difference(&touch_prev).map(|k| k.to_string()));
        released.extend(touch_prev.difference(&held).map(|k| k.to_string()));
        touch_prev = held.clone();
        let input = Input {
            down: down.into_iter().collect(),
            pressed: pressed.into_iter().collect(),
            released: released.into_iter().collect(),
            mouse: ((mx - off.x) / scale, (my - off.y) / scale),
            mouse_down: [MouseButton::Left, MouseButton::Right, MouseButton::Middle].map(is_mouse_button_down),
            mouse_pressed: [MouseButton::Left, MouseButton::Right, MouseButton::Middle].map(is_mouse_button_pressed),
        };
        world.step(get_frame_time(), input);

        for cmd in std::mem::take(&mut world.cmds) {
            if let Cmd::PlaySound { file, .. } | Cmd::PlayMusic { file, .. } = &cmd {
                if !sounds.contains_key(file) {
                    match read_sound(file).await {
                        Ok(snd) => {
                            sounds.insert(file.clone(), snd);
                        }
                        Err(e) => {
                            eprintln!("sonido {file}: {e}");
                            continue;
                        }
                    }
                }
            }
            match cmd {
                Cmd::PlaySound { file, volume } => play_sound(&sounds[&file], PlaySoundParams { looped: false, volume: volume / 100. }),
                Cmd::PlayMusic { file, volume } => {
                    if let Some(m) = music.take() {
                        stop_sound(&m);
                    }
                    let s = sounds[&file].clone();
                    play_sound(&s, PlaySoundParams { looped: true, volume: volume / 100. });
                    music = Some(s);
                }
                Cmd::StopMusic => {
                    if let Some(m) = music.take() {
                        stop_sound(&m);
                    }
                }
                Cmd::StopSound { file } => {
                    if let Some(snd) = sounds.get(&file) {
                        stop_sound(snd);
                    }
                }
                Cmd::Persist(v) => guardado::store(&game_name, &v),
                Cmd::Quit => return,
            }
        }

        let sc = world.scene_def().clone();
        let [r, g, b] = sc.background;
        clear_background(BLACK);
        draw_rectangle(off.x, off.y, p.width * scale, p.height * scale, Color::from_rgba(r, g, b, 255));

        for layer in sc.layers.iter().filter(|l| l.visible) {
            let (z, cam) = if layer.fixed { (1., (p.width / 2., p.height / 2.)) } else { (world.zoom, world.camera) };
            let to_screen = |x: f32, y: f32| off + (vec2(x - cam.0, y - cam.1) * z + vec2(p.width, p.height) / 2.) * scale;
            let k = z * scale;
            let mut order: Vec<usize> = (0..world.insts.len()).filter(|&i| world.insts[i].layer == layer.name).collect();
            order.sort_by(|&a, &b| world.insts[a].z.total_cmp(&world.insts[b].z));
            for i in order {
                let o = &world.insts[i];
                let Some(def) = world.object_def(&o.obj) else { continue };
                if !o.visible {
                    continue;
                }
                let alpha = (o.opacity.clamp(0., 255.)) as u8;
                let [tr, tg, tb] = o.tint;
                // El color de la instancia multiplica al del dibujo.
                let mul = |c: [u8; 3]| Color::from_rgba((c[0] as u16 * tr as u16 / 255) as u8, (c[1] as u16 * tg as u16 / 255) as u8, (c[2] as u16 * tb as u16 / 255) as u8, alpha);
                let c = to_screen(o.cx(), o.cy());
                match &def.kind {
                    Kind::Box { color, .. } => draw_rectangle_ex(c.x, c.y, o.w * k, o.h * k, DrawRectangleParams {
                        offset: vec2(0.5, 0.5),
                        rotation: o.angle.to_radians(),
                        color: mul(*color),
                    }),
                    Kind::Sprite { animations, .. } => {
                        let frame = animations.get(o.anim).and_then(|a| a.frames.get(o.frame)).and_then(|f| tex.get(f));
                        match frame {
                            Some(t) => draw_texture_ex(t, c.x - o.w * k / 2., c.y - o.h * k / 2., mul([255, 255, 255]), DrawTextureParams {
                                dest_size: Some(vec2(o.w * k, o.h * k)),
                                rotation: o.angle.to_radians(),
                                flip_x: o.flip_x,
                                ..Default::default()
                            }),
                            None => draw_rectangle(c.x - o.w * k / 2., c.y - o.h * k / 2., o.w * k, o.h * k, MAGENTA),
                        }
                    }
                    Kind::Tilemap { tileset, tile, cols, .. } => {
                        let Some(t) = tex.get(tileset) else { continue };
                        let per_row = (t.width() / tile).floor().max(1.) as i32;
                        // Solo las celdas que se ven en pantalla.
                        let (vw, vh) = (p.width / z, p.height / z);
                        let (c0, c1) = (((cam.0 - vw / 2. - o.x) / tile).floor().max(0.) as usize, ((cam.0 + vw / 2. - o.x) / tile).ceil().max(0.) as usize);
                        let (r0, r1) = (((cam.1 - vh / 2. - o.y) / tile).floor().max(0.) as usize, ((cam.1 + vh / 2. - o.y) / tile).ceil().max(0.) as usize);
                        let rows = o.tiles.len() / cols.max(&1);
                        for r in r0..r1.min(rows) {
                            for c in c0..c1.min(*cols) {
                                let n = o.tiles[r * cols + c];
                                if n < 0 {
                                    continue;
                                }
                                let at = to_screen(o.x + c as f32 * tile, o.y + r as f32 * tile);
                                draw_texture_ex(t, at.x, at.y, mul([255, 255, 255]), DrawTextureParams {
                                    // +0.5 px para que no se vean rayas entre baldosas al escalar
                                    dest_size: Some(vec2(tile * k + 0.5, tile * k + 0.5)),
                                    source: Some(Rect::new((n % per_row) as f32 * tile, (n / per_row) as f32 * tile, *tile, *tile)),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                    Kind::Text { size, color, .. } => {
                        let txt = world.display_text(i);
                        let tl = to_screen(o.x, o.y);
                        draw_text(&txt, tl.x, tl.y + size * k * 0.8, size * k, mul(*color));
                    }
                }
            }
        }
        // Franjas negras encima de lo que se salga del área de juego.
        let (sw, sh) = (screen_width(), screen_height());
        draw_rectangle(0., 0., off.x, sh, BLACK);
        draw_rectangle(sw - off.x, 0., off.x, sh, BLACK);
        draw_rectangle(0., 0., sw, off.y, BLACK);
        draw_rectangle(0., sh - off.y, sw, off.y, BLACK);
        draw_touch_buttons(&buttons, &held);
        // El tamaño real del texto, para colisiones y "cursor sobre".
        for i in 0..world.insts.len() {
            if let Some(Kind::Text { size, .. }) = world.object_def(&world.insts[i].obj).map(|d| d.kind.clone()) {
                let w = measure_text(&world.display_text(i), None, size as u16, 1.).width;
                world.insts[i].w = w;
            }
        }
        next_frame().await
    }
}


#[cfg(test)]
mod tests {
    use super::guardado;

    #[test]
    fn slug_del_juego() {
        assert_eq!(guardado::slug("Naves del Espacio"), "naves-del-espacio");
        assert_eq!(guardado::slug("  ¡Pingüino Ñandú! 2 "), "pinguino-nandu-2");
        assert_eq!(guardado::slug("../../etc"), "etc");
        assert_eq!(guardado::slug("???"), "juego");
        assert_eq!(guardado::slug(""), "juego");
    }

    #[test]
    fn carpeta_de_datos() {
        let env = |vars: &'static [(&'static str, &'static str)]| move |k: &str| vars.iter().find(|v| v.0 == k).map(|v| v.1.to_string());
        let p = std::path::PathBuf::from;
        if cfg!(windows) {
            assert_eq!(guardado::data_dir(env(&[("APPDATA", "C:\\U\\AppData\\Roaming")])), Some(p("C:\\U\\AppData\\Roaming")));
        } else if cfg!(target_os = "linux") {
            assert_eq!(guardado::data_dir(env(&[("XDG_DATA_HOME", "/x/data"), ("HOME", "/h")])), Some(p("/x/data")));
            assert_eq!(guardado::data_dir(env(&[("XDG_DATA_HOME", ""), ("HOME", "/h")])), Some(p("/h/.local/share")));
            assert_eq!(guardado::data_dir(env(&[("XDG_DATA_HOME", "relativo"), ("HOME", "/h")])), Some(p("/h/.local/share")));
            assert_eq!(guardado::data_dir(env(&[])), None);
            assert_eq!(guardado::save_file(&p("/x/data"), "Mi Juego"), p("/x/data/nicostation/mi-juego/guardado.json"));
        }
    }

    #[test]
    fn guardar_y_leer() {
        use nicostation::model::Value;
        let d = std::env::temp_dir().join(format!("nicoguardado-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let f = guardado::save_file(&d, "Prueba");
        assert!(guardado::read(&f).is_empty(), "sin partida: vacío");
        let mut v = nicostation::model::Vars::new();
        v.insert("record".into(), Value::Num(12.));
        v.insert("nombre".into(), Value::Str("Ana".into()));
        guardado::write(&f, &v).unwrap();
        assert_eq!(guardado::read(&f), v);
        v.insert("record".into(), Value::Num(13.));
        guardado::write(&f, &v).unwrap();
        assert_eq!(guardado::read(&f), v);
        assert!(!f.with_extension("json.tmp").exists(), "no queda el temporal");
        std::fs::write(&f, "{dañado").unwrap();
        assert!(guardado::read(&f).is_empty());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn botones_tactiles_dentro_de_pantalla() {
        for (w, h) in [(800., 600.), (390., 844.), (1920., 1080.)] {
            for four in [false, true] {
                let b = super::touch_buttons(w, h, four);
                assert_eq!(b.len(), if four { 5 } else { 3 });
                for (c, r, _) in &b {
                    assert!(c.x - r >= 0. && c.x + r <= w && c.y - r >= 0. && c.y + r <= h, "{w}x{h}: botón fuera");
                }
                // Que no se pisen entre sí.
                for (i, a) in b.iter().enumerate() {
                    for c in &b[i + 1..] {
                        assert!(a.0.distance(c.0) >= a.1 + c.1, "{w}x{h}: botones superpuestos");
                    }
                }
            }
        }
    }
}
