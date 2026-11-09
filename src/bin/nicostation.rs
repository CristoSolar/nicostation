// En Windows, sin ventana de consola detrás del juego (solo en la versión final).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
//! Reproductor: `nicostation proyecto.json`. Ventana, imágenes, sonido y entrada con macroquad.
//! También compila a WebAssembly: en el navegador lee `juego.json` junto a la página.
//! Exportar a web: `nicostation --export-web proyecto.json carpeta/`.
//! Exportar a un solo ejecutable: `nicostation --export-desktop proyecto.json salida [--target windows]`.
//! Exportar a Android: `nicostation --export-android proyecto.json salida.apk` (ver `export.rs`).
//! Foto de un cuadro sin mirar la pantalla: `nicostation proyecto.json --captura foto.png --segundos 2`.
//! Si el ejecutable trae un juego empaquetado (ver `pack.rs`), lo juega sin leer nada de disco.

#[path = "../reproductor/audio.rs"]
mod audio;
#[path = "../reproductor/efectos.rs"]
mod efectos;
#[path = "../reproductor/geometria.rs"]
mod geometria;
#[path = "../reproductor/mando.rs"]
mod mando;

use efectos::{Effect, Materials};
use geometria::Viewport;
use macroquad::prelude::*;
use nicostation::guardado;
use nicostation::model::{parse_project, Kind};
use nicostation::pack::Pack;
use nicostation::runtime::{Cmd, Inst, Input, World};
use std::collections::{HashMap, HashSet};

/// Opciones de la línea de comandos (fuera de las de exportar).
struct Opts {
    project: String,
    /// `--captura foto.png --segundos N`: juega N segundos sin ventana a la vista, guarda el último cuadro y sale.
    capture: Option<(std::path::PathBuf, f32)>,
}

fn opts() -> &'static Opts {
    static O: std::sync::OnceLock<Opts> = std::sync::OnceLock::new();
    O.get_or_init(|| {
        let a: Vec<String> = std::env::args().skip(1).collect();
        let (mut project, mut out, mut secs) = (None, None, 1f32);
        let mut i = 0;
        while i < a.len() {
            match a[i].as_str() {
                "--captura" | "--capture" => {
                    // Antes de entrar a la carpeta del proyecto: la ruta es relativa a donde se llamó.
                    out = a.get(i + 1).map(|f| std::env::current_dir().map(|d| d.join(f)).unwrap_or(f.into()));
                    i += 1;
                }
                "--segundos" | "--seconds" => {
                    secs = a.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(1.);
                    i += 1;
                }
                x if project.is_none() && !x.starts_with("--") => project = Some(x.to_string()),
                _ => {}
            }
            i += 1;
        }
        Opts { project: project.unwrap_or("juego.json".into()), capture: out.map(|o| (o, secs.max(0.))) }
    })
}

fn project_path() -> String {
    opts().project.clone()
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
    // En escritorio la ventana toma el tamaño, nombre e ícono del juego; en web los define la página.
    #[cfg(not(target_arch = "wasm32"))]
    let c = {
        let src = match embedded() {
            Some(pk) => Some(pk.project_json().to_vec()),
            None => std::fs::read(project_path()).ok(),
        };
        match src.and_then(|s| parse_project(&s).ok()) {
            Some(p) => Conf { icon: window_icon(&p).or(c.icon), window_title: p.name.clone(), window_width: p.width as i32, window_height: p.height as i32, ..c },
            None => c,
        }
    };
    c
}

/// Ícono de la ventana desde `settings.icon`, achicado a 16, 32 y 64 px.
#[cfg(not(target_arch = "wasm32"))]
fn window_icon(p: &nicostation::model::Project) -> Option<macroquad::miniquad::conf::Icon> {
    let f = &p.settings.icon;
    if f.is_empty() {
        return None;
    }
    let bytes = match embedded() {
        Some(pk) => pk.files.get(f).cloned(),
        None => std::fs::read(std::path::Path::new(&project_path()).parent().unwrap_or(std::path::Path::new("")).join(f)).ok(),
    };
    let img = match bytes.map(|b| image::load_from_memory(&b)) {
        Some(Ok(i)) => i.to_rgba8(),
        _ => {
            eprintln!("ícono {f}: no pude leerlo");
            return None;
        }
    };
    let filter = if p.settings.pixel_art { image::imageops::FilterType::Nearest } else { image::imageops::FilterType::Lanczos3 };
    let px = |n: u32| image::imageops::resize(&img, n, n, filter).into_raw();
    Some(macroquad::miniquad::conf::Icon { small: px(16).try_into().ok()?, medium: px(32).try_into().ok()?, big: px(64).try_into().ok()? })
}

/// De dónde salen los archivos del juego: del paquete pegado al ejecutable o de la carpeta / el servidor.
async fn read_file(path: &str) -> Result<Vec<u8>, String> {
    match embedded() {
        Some(pk) => pk.files.get(path).cloned().ok_or_else(|| "no está en el paquete".into()),
        None => load_file(path).await.map_err(|e| e.to_string()),
    }
}

async fn read_texture(path: &str, filter: FilterMode) -> Result<Texture2D, String> {
    let b = read_file(path).await?;
    let t = Texture2D::from_file_with_format(&b, None);
    t.set_filter(filter);
    Ok(t)
}

/// Pantalla de carga «Cargando… n/m», con la imagen de carga del juego si tiene. Solo dibuja de vez en cuando
/// para no frenar la carga.
async fn loading(n: usize, m: usize, last: &mut f64, image: Option<&Texture2D>) {
    if n != 0 && get_time() - *last < 0.05 {
        return;
    }
    *last = get_time();
    traza(&format!("cargando {n}/{m}: dibujando"));
    let (sw, sh) = (screen_width(), screen_height());
    clear_background(Color::from_rgba(12, 12, 14, 255));
    if let Some(t) = image {
        // La imagen entera, lo más grande posible sin deformar.
        let s = (sw / t.width()).min(sh / t.height());
        let (w, h) = (t.width() * s, t.height() * s);
        draw_texture_ex(t, (sw - w) / 2., (sh - h) / 2., WHITE, DrawTextureParams { dest_size: Some(vec2(w, h)), ..Default::default() });
    }
    let (w, h) = ((sw * 0.5).clamp(160., 480.), 10.);
    let (x, y) = ((sw - w) / 2., if image.is_some() { sh - 40. } else { sh / 2. });
    draw_rectangle(x, y, w, h, Color::new(1., 1., 1., 0.15));
    draw_rectangle(x, y, w * n as f32 / m.max(1) as f32, h, Color::new(1., 1., 1., 0.85));
    if image.is_none() {
        let t = format!("Cargando… {n}/{m}");
        let tw = measure_text(&t, None, 24, 1.).width;
        draw_text(&t, (sw - tw) / 2., y - 16., 24., Color::new(1., 1., 1., 0.85));
    }
    traza("cargando: mostrando cuadro");
    next_frame().await;
    traza("cargando: cuadro mostrado");
}

/// Botones táctiles: (centro, radio, tecla). Cruceta de 4 direcciones o izquierda/derecha, más el botón de acción.
/// Van en coordenadas de la ventana: no dependen del modo de escalado.
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

fn draw_touch_buttons(buttons: &[(Vec2, f32, &str)], held: &HashSet<&str>) {
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

/// Si el juego empieza en una escena 3D, lo juega `nicostation3d` (el binario hermano de este).
/// Devuelve el código de salida si no hay que seguir con el reproductor 2D.
#[cfg(not(target_arch = "wasm32"))]
fn handoff_3d(args: &[String]) -> Option<i32> {
    let p = parse_project(&std::fs::read(project_path()).ok()?).ok()?;
    if !p.scene(&p.first_scene).or(p.scenes.first()).is_some_and(|s| s.dim3) {
        return None;
    }
    let exe = std::env::current_exe().ok()?.with_file_name(format!("nicostation3d{}", std::env::consts::EXE_SUFFIX));
    if !exe.is_file() {
        eprintln!("«{}» es un juego 3D: ábrelo con el reproductor 3D:\n  nicostation3d {}", p.name, project_path());
        return Some(2);
    }
    let mut cmd = std::process::Command::new(&exe);
    cmd.args(&args[1..]);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let e = cmd.exec(); // solo vuelve si falló
        eprintln!("no pude abrir {}: {e}", exe.display());
        Some(2)
    }
    #[cfg(not(unix))]
    match cmd.status() {
        Ok(s) => Some(s.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("no pude abrir {}: {e}", exe.display());
            Some(2)
        }
    }
}

/// `--export-desktop proyecto.json salida [--target windows]`, `--export-web proyecto.json carpeta/`,
/// `--export-android proyecto.json salida.apk`. Devuelve el código de salida si era una exportación.
#[cfg(not(target_arch = "wasm32"))]
fn export_cli(a: &[String]) -> Option<i32> {
    let fail = |e: String| {
        eprintln!("{e}");
        1
    };
    match a.get(1).map(String::as_str) {
        Some("--export-desktop") => {
            let mut target = nicostation::export::DesktopTarget::Current;
            let mut rest = vec![];
            let mut it = a[2..].iter();
            while let Some(x) = it.next() {
                if x == "--target" {
                    match it.next().map(|t| nicostation::export::DesktopTarget::parse(t)) {
                        Some(Ok(t)) => target = t,
                        Some(Err(e)) => {
                            eprintln!("{e}");
                            return Some(2);
                        }
                        None => rest.push(x),
                    }
                } else {
                    rest.push(x);
                }
            }
            let [p, out] = rest[..] else {
                eprintln!("uso: nicostation --export-desktop proyecto.json salida [--target windows]");
                return Some(2);
            };
            Some(match nicostation::export::export_desktop(p.as_ref(), out.as_ref(), target) {
                Ok((f, files)) => {
                    println!("Exportado a {} ({} archivos adentro). Es un solo ejecutable: cópialo y listo.", f.display(), files.len());
                    0
                }
                Err(e) => fail(e),
            })
        }
        Some("--export-web") => {
            let (Some(p), Some(out)) = (a.get(2), a.get(3)) else {
                eprintln!("uso: nicostation --export-web proyecto.json carpeta/");
                return Some(2);
            };
            Some(match nicostation::export::export_web(p.as_ref(), out.as_ref()) {
                Ok(f) => {
                    println!("Exportado a {out} ({} archivos). Pruébalo con:\n  python3 -m http.server -d {out} 8000", f.len());
                    0
                }
                Err(e) => fail(e),
            })
        }
        Some("--export-android") => {
            let (Some(p), Some(out)) = (a.get(2), a.get(3)) else {
                eprintln!("uso: nicostation --export-android proyecto.json salida.apk");
                return Some(2);
            };
            Some(match nicostation::export::export_android(p.as_ref(), out.as_ref()) {
                Ok((f, files)) => {
                    println!("Exportado a {} ({} archivos adentro). Instálalo con:\n  adb install -r {}", f.display(), files.len(), f.display());
                    0
                }
                Err(e) => fail(e),
            })
        }
        _ => None,
    }
}

/// Android: cpal (sonido con kira) pide la JavaVM y la Activity a `ndk-context`; miniquad las tiene.
#[cfg(target_os = "android")]
fn android_context() {
    use macroquad::miniquad::native::android::{attach_jni_env, ACTIVITY};
    // Una sola vez por proceso: ndk-context entra en pánico si se inicializa dos veces (Activity recreada).
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| unsafe {
        let env = attach_jni_env();
        let mut vm = std::ptr::null_mut();
        ((**env).GetJavaVM.unwrap())(env, &mut vm);
        ndk_context::initialize_android_context(vm as _, ACTIVITY as _);
    });
}

/// Registro de arranque opcional: `NICOSTATION_LOG=archivo` anota cada etapa (para diagnosticar equipos raros).
fn traza(msg: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(path) = std::env::var_os("NICOSTATION_LOG") {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(f, "{msg}");
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = msg;
}

/// Si el juego falla, que se entienda por qué: en Windows (sin consola) un cuadro de diálogo,
/// y en todos lados el mensaje en el registro de arranque.
fn avisar_fallos() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = info.payload().downcast_ref::<&str>().map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned()).unwrap_or_else(|| "error desconocido".into());
        traza(&format!("FALLO: {msg} ({})", info.location().map(|l| l.to_string()).unwrap_or_default()));
        #[cfg(windows)]
        {
            let pista = if msg.contains("WGL") || msg.contains("OpenGL") || msg.contains("GL_") {
                "\n\nEste computador no tiene OpenGL 2.1 o más nuevo. Actualiza el controlador de la tarjeta de video."
            } else {
                ""
            };
            mensaje_windows("No se pudo abrir el juego", &format!("{msg}{pista}"));
        }
        prev(info);
    }));
}

#[cfg(windows)]
fn mensaje_windows(titulo: &str, texto: &str) {
    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(hwnd: *mut core::ffi::c_void, text: *const u16, caption: *const u16, kind: u32) -> i32;
    }
    let w = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let (t, c) = (w(texto), w(titulo));
    // SAFETY: cadenas UTF-16 terminadas en 0 que viven durante la llamada; ventana dueña nula.
    unsafe {
        MessageBoxW(std::ptr::null_mut(), t.as_ptr(), c.as_ptr(), 0x10); // MB_ICONERROR
    }
}

fn main() {
    avisar_fallos();
    traza("inicio");
    #[cfg(target_os = "android")]
    android_context();
    let conf = conf(); // antes de cambiar de carpeta: lee el proyecto con la ruta original
    // Un juego empaquetado no recibe opciones ni cambia de carpeta: todo está adentro.
    #[cfg(not(target_arch = "wasm32"))]
    if embedded().is_none() {
        let a: Vec<String> = std::env::args().collect();
        if let Some(code) = export_cli(&a) {
            std::process::exit(code);
        }
        if let Some(code) = handoff_3d(&a) {
            std::process::exit(code);
        }
        let path = std::path::PathBuf::from(project_path());
        if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::env::set_current_dir(dir).expect("no pude entrar a la carpeta del proyecto");
        }
    }
    traza("abriendo ventana");
    macroquad::Window::from_config(conf, game());
}

/// Imágenes, fuentes y shaders cargados.
struct Gfx {
    tex: HashMap<String, Texture2D>,
    fonts: HashMap<String, Font>,
    mats: Materials,
}

/// Líneas de un texto ya partidas: (línea, ancho), ancho del bloque y alto de cada línea, en px del juego.
fn text_block(gfx: &Gfx, font: &str, size: f32, wrap: f32, txt: &str) -> (Vec<(String, f32)>, f32, f32) {
    let font = gfx.fonts.get(font);
    let fs = size.round().clamp(1., 2000.) as u16;
    let measure = |s: &str| measure_text(s, font, fs, 1.).width * size / fs as f32;
    let lines: Vec<(String, f32)> = nicostation::runtime::wrap_lines(txt, size, wrap, measure).into_iter().map(|l| {
        let w = measure(&l);
        (l, w)
    }).collect();
    let widest = lines.iter().map(|l| l.1).fold(0., f32::max);
    (lines, if wrap > 0. { wrap } else { widest }, size * 1.25)
}

async fn game() {
    traza("ventana lista");
    let capture = opts().capture.clone();
    let (file, src) = match embedded() {
        Some(pk) => (pk.project.clone(), pk.project_json().to_vec()),
        None => {
            // Ya estamos en la carpeta del proyecto: basta el nombre del archivo.
            let file = std::path::Path::new(&project_path()).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or("juego.json".into());
            let src = load_file(&file).await.unwrap_or_else(|e| panic!("no pude leer {file}: {e}"));
            (file, src)
        }
    };
    let mut project = parse_project(&src).unwrap_or_else(|e| panic!("{file}: {e}"));
    let settings = project.settings.clone();
    let filter = if settings.pixel_art { FilterMode::Nearest } else { FilterMode::Linear };

    // Imágenes, fuentes y sonidos, con pantalla de carga.
    let mut images: Vec<String> = vec![];
    let mut fonts: Vec<String> = vec![];
    for o in project.objects.iter().chain(project.scenes.iter().flat_map(|s| s.objects.iter())) {
        match &o.kind {
            Kind::Tilemap { tileset, .. } => images.push(tileset.clone()),
            Kind::Sprite { animations, .. } => images.extend(animations.iter().flat_map(|a| a.frames.iter().cloned())),
            Kind::Background { image, .. } => images.push(image.clone()),
            Kind::Particles(def) if !def.image.is_empty() => images.push(def.image.clone()),
            Kind::Text { font, .. } if !font.is_empty() => fonts.push(font.clone()),
            _ => {}
        }
    }
    images.sort();
    images.dedup();
    fonts.sort();
    fonts.dedup();
    let skip: HashSet<&String> = images.iter().chain(&fonts).chain([&settings.icon, &settings.loading_image]).collect();
    let sounds: Vec<String> = nicostation::export::assets(&project).into_iter().filter(|a| !skip.contains(a)).collect();
    let loading_img = if settings.loading_image.is_empty() { None } else { read_texture(&settings.loading_image, filter).await.map_err(|e| eprintln!("imagen de carga: {e}")).ok() };
    let total = images.len() + fonts.len() + sounds.len();
    let mut last = 0.;
    let mut gfx = Gfx { tex: HashMap::new(), fonts: HashMap::new(), mats: Materials::new() };
    for (n, f) in images.iter().enumerate() {
        loading(n, total, &mut last, loading_img.as_ref()).await;
        match read_texture(f, filter).await {
            Ok(t) => {
                gfx.tex.insert(f.clone(), t);
            }
            Err(e) => eprintln!("imagen {f}: {e}"),
        }
    }
    for (n, f) in fonts.iter().enumerate() {
        loading(images.len() + n, total, &mut last, loading_img.as_ref()).await;
        match read_file(f).await.and_then(|b| load_ttf_font_from_bytes(&b).map_err(|e| e.to_string())) {
            Ok(mut font) => {
                font.set_filter(filter);
                gfx.fonts.insert(f.clone(), font);
            }
            Err(e) => eprintln!("fuente {f}: {e}"),
        }
    }
    traza("iniciando audio");
    let mut snd = audio::Platform::new(capture.is_none());
    traza("audio listo");
    for (n, f) in sounds.iter().enumerate() {
        loading(images.len() + fonts.len() + n, total, &mut last, loading_img.as_ref()).await;
        match read_file(f).await {
            Ok(b) => snd.load(f, b),
            Err(e) => eprintln!("sonido {f}: {e}"),
        }
    }
    // En el navegador los sonidos se decodifican aparte: se espera un poco a que estén.
    let t0 = get_time();
    while !sounds.iter().all(|f| snd.loaded(f)) && get_time() - t0 < 5. {
        loading(total, total, &mut last, loading_img.as_ref()).await;
    }
    // Los sprites sin tamaño toman el de su primera imagen.
    for o in project.objects.iter_mut().chain(project.scenes.iter_mut().flat_map(|s| s.objects.iter_mut())) {
        if let Kind::Sprite { w, h, animations, .. } = &mut o.kind {
            if let Some(t) = animations.first().and_then(|a| a.frames.first()).and_then(|f| gfx.tex.get(f)) {
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
    let mut touch_prev: HashSet<&str> = Default::default();
    let game_name = project.name.clone();
    let mut world = World::new(project);
    // La partida guardada tiene que estar antes del primer paso (los eventos «al empezar» la leen).
    world.storage = guardado::load(&game_name);
    let mut mixer = audio::Mixer::default();
    traza("iniciando mandos");
    let mut pads = mando::Platform::new();
    traza("mandos listos; empieza el juego");
    let mut pad_prev: HashSet<String> = HashSet::new();
    // Reloj del juego (para los fondos que se desplazan y las ondas).
    let mut clock = 0f32;
    #[cfg(not(target_arch = "wasm32"))]
    let mut last_frame = get_time();

    if let Some((out, secs)) = capture {
        // Juega `secs` segundos de una (a 60 cuadros por segundo, sin entrada), dibuja el último cuadro y lo guarda.
        let dt = 1. / 60.;
        for _ in 0..(secs * 60.).round() as usize {
            world.step(dt, Input::default());
            clock += dt;
            if world.cmds.drain(..).any(|c| c == Cmd::Quit) {
                break;
            }
            text_sizes(&mut world, &gfx);
        }
        let (pw, ph) = (world.project.width, world.project.height);
        let rt = render_target(pw as u32, ph as u32);
        rt.texture.set_filter(FilterMode::Nearest);
        let mut cam = Camera2D::from_display_rect(Rect::new(0., 0., pw, ph));
        cam.render_target = Some(rt.clone());
        set_camera(&cam);
        draw_world(&world, &gfx, geometria::viewport("fit", pw, ph, pw, ph), pw, ph, clock);
        set_default_camera();
        next_frame().await;
        let img = rt.texture.get_texture_data();
        img.export_png(&out.to_string_lossy());
        println!("captura guardada en {} ({}x{}, {secs} s)", out.display(), img.width, img.height);
        return;
    }

    loop {
        let (pw, ph) = (world.project.width, world.project.height);
        let (sw, sh) = (screen_width(), screen_height());
        let vp = geometria::viewport(&settings.scale_mode, sw, sh, pw, ph);

        let (mx, my) = mouse_position();
        // Toques: los botones en pantalla aparecen la primera vez que se toca (celular o tablet).
        let ts = touches();
        touch_seen |= !ts.is_empty();
        let buttons = if touch_seen { touch_buttons(sw, sh, four_way) } else { vec![] };
        let held: HashSet<&str> = buttons.iter()
            .filter(|(c, r, _)| ts.iter().any(|t| !matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled) && t.position.distance(*c) < r * 1.2))
            .map(|b| b.2).collect();
        let mut down: HashSet<String> = get_keys_down().into_iter().map(key_name).collect();
        let mut pressed: HashSet<String> = get_keys_pressed().into_iter().map(key_name).collect();
        let mut released: HashSet<String> = get_keys_released().into_iter().map(key_name).collect();
        down.extend(held.iter().map(|k| k.to_string()));
        pressed.extend(held.difference(&touch_prev).map(|k| k.to_string()));
        released.extend(touch_prev.difference(&held).map(|k| k.to_string()));
        touch_prev = held.clone();
        // Mandos: sus botones son teclas más («Pad1A»…), apretadas/soltadas según el cuadro anterior.
        let (pad_down, axes) = mando::pad_inputs(&pads.read());
        pressed.extend(pad_down.difference(&pad_prev).cloned());
        released.extend(pad_prev.difference(&pad_down).cloned());
        down.extend(pad_down.iter().cloned());
        pad_prev = pad_down;
        let input = Input {
            down,
            pressed,
            released,
            mouse: ((mx - vp.off.0) / vp.scale.0, (my - vp.off.1) / vp.scale.1),
            mouse_down: [MouseButton::Left, MouseButton::Right, MouseButton::Middle].map(is_mouse_button_down),
            mouse_pressed: [MouseButton::Left, MouseButton::Right, MouseButton::Middle].map(is_mouse_button_pressed),
            axes,
            ..Default::default()
        };
        let dt = get_frame_time();
        world.step(dt, input);
        clock += dt;

        for cmd in std::mem::take(&mut world.cmds) {
            if let Cmd::PlaySound { file, .. } | Cmd::PlayMusic { file, .. } = &cmd {
                if !audio::Backend::has(&snd, file) {
                    match read_file(file).await {
                        Ok(b) => snd.load(file, b),
                        Err(e) => eprintln!("sonido {file}: {e}"),
                    }
                }
            }
            if mixer.command(&mut snd, &cmd) {
                continue;
            }
            match cmd {
                Cmd::Persist(v) => guardado::store(&game_name, &v),
                Cmd::Quit => return,
                _ => {}
            }
        }
        mixer.update(&mut snd, dt);

        draw_world(&world, &gfx, vp, sw, sh, clock);
        draw_touch_buttons(&buttons, &held);
        text_sizes(&mut world, &gfx);
        // Límite de cuadros por segundo (en web manda el navegador).
        #[cfg(not(target_arch = "wasm32"))]
        {
            if settings.max_fps > 0 {
                let wait = last_frame + 1. / settings.max_fps as f64 - get_time();
                if wait > 0. {
                    std::thread::sleep(std::time::Duration::from_secs_f64(wait));
                }
            }
            last_frame = get_time();
        }
        next_frame().await
    }
}

/// El tamaño real de los textos (bloque con sus líneas), para colisiones y «cursor sobre».
fn text_sizes(world: &mut World, gfx: &Gfx) {
    for i in 0..world.insts.len() {
        if let Some(Kind::Text { size, font, wrap, .. }) = world.object_def(&world.insts[i].obj).map(|d| d.kind.clone()) {
            let (lines, w, lh) = text_block(gfx, &font, size, wrap, &world.display_text(i));
            world.insts[i].w = w;
            world.insts[i].h = lh * lines.len().max(1) as f32;
        }
    }
}

/// Cómo pasar del mundo a la ventana en una capa.
struct LayerView {
    vp: Viewport,
    cam: (f32, f32),
    zoom: f32,
    pw: f32,
    ph: f32,
}

impl LayerView {
    fn to_screen(&self, x: f32, y: f32) -> Vec2 {
        vec2(
            self.vp.off.0 + ((x - self.cam.0) * self.zoom + self.pw / 2.) * self.vp.scale.0,
            self.vp.off.1 + ((y - self.cam.1) * self.zoom + self.ph / 2.) * self.vp.scale.1,
        )
    }
    /// Píxeles de ventana por píxel del mundo.
    fn k(&self) -> Vec2 {
        vec2(self.zoom * self.vp.scale.0, self.zoom * self.vp.scale.1)
    }
    /// Lo que ve la cámara de esta capa, en el mundo (x, y, w, h).
    fn view(&self) -> [f32; 4] {
        let (w, h) = (self.pw / self.zoom, self.ph / self.zoom);
        [self.cam.0 - w / 2., self.cam.1 - h / 2., w, h]
    }
}

fn draw_world(world: &World, gfx: &Gfx, vp: Viewport, sw: f32, sh: f32, clock: f32) {
    let (pw, ph) = (world.project.width, world.project.height);
    let sc = world.scene_def();
    let [r, g, b] = sc.background;
    clear_background(BLACK);
    draw_rectangle(vp.off.0, vp.off.1, pw * vp.scale.0, ph * vp.scale.1, Color::from_rgba(r, g, b, 255));

    for layer in sc.layers.iter().filter(|l| l.visible) {
        let lv = if layer.fixed {
            LayerView { vp, cam: (pw / 2., ph / 2.), zoom: 1., pw, ph }
        } else {
            LayerView { vp, cam: geometria::parallax_camera(world.camera, layer.parallax, pw, ph), zoom: world.zoom, pw, ph }
        };
        let mut order: Vec<usize> = (0..world.insts.len()).filter(|&i| world.insts[i].layer == layer.name).collect();
        order.sort_by(|&a, &b| world.insts[a].z.total_cmp(&world.insts[b].z));
        for i in order {
            draw_inst(world, gfx, i, &lv, clock);
        }
        draw_particles(world, gfx, &layer.name, &lv);
    }
    // Franjas negras encima de lo que se salga del área de juego.
    let (ox, oy) = (vp.off.0.max(0.), vp.off.1.max(0.));
    if ox > 0. {
        draw_rectangle(0., 0., ox, sh, BLACK);
        draw_rectangle(sw - ox, 0., ox + 1., sh, BLACK);
    }
    if oy > 0. {
        draw_rectangle(0., 0., sw, oy, BLACK);
        draw_rectangle(0., sh - oy, sw, oy + 1., BLACK);
    }
}

fn draw_inst(world: &World, gfx: &Gfx, i: usize, lv: &LayerView, clock: f32) {
    let o = &world.insts[i];
    let Some(def) = world.object_def(&o.obj) else { return };
    if !o.visible {
        return;
    }
    let alpha = o.opacity.clamp(0., 255.) as u8;
    let [tr, tg, tb] = o.tint;
    // El color de la instancia multiplica al del dibujo.
    let mul = |c: [u8; 3]| Color::from_rgba((c[0] as u16 * tr as u16 / 255) as u8, (c[1] as u16 * tg as u16 / 255) as u8, (c[2] as u16 * tb as u16 / 255) as u8, alpha);
    let effect = Effect::parse(&o.effect, o.effect_amount);
    let k = lv.k();
    match &def.kind {
        Kind::Box { color, .. } => {
            let c = lv.to_screen(o.cx(), o.cy());
            let mut col = mul(*color);
            if let Some(e) = effect {
                col = e.apply_color(col, o.effect_amount);
            }
            let params = DrawRectangleParams { offset: vec2(0.5, 0.5), rotation: o.angle.to_radians(), color: col };
            draw_rectangle_ex(c.x, c.y, o.w * k.x, o.h * k.y, params.clone());
            if effect == Some(Effect::Outline) {
                let line = Color::new(1., 1., 1., o.effect_amount.clamp(0., 1.) * alpha as f32 / 255.);
                draw_rectangle_lines_ex(c.x, c.y, o.w * k.x + 4., o.h * k.y + 4., 2., DrawRectangleParams { color: line, ..params });
            }
        }
        Kind::Sprite { animations, .. } => {
            let frame = animations.get(o.anim).and_then(|a| a.frames.get(o.frame)).and_then(|f| gfx.tex.get(f));
            match frame {
                Some(t) => draw_sprite(gfx, t, o, mul([255, 255, 255]), effect, lv, clock),
                None => {
                    let (x, y, w, h, ..) = o.draw_rect();
                    let tl = lv.to_screen(x, y);
                    draw_rectangle(tl.x, tl.y, w * k.x, h * k.y, MAGENTA)
                }
            }
        }
        Kind::Background { image, scroll, .. } => {
            let Some(t) = gfx.tex.get(image) else { return };
            let on = effect.filter(|e| !e.needs_margin()).is_some_and(|e| gfx.mats.use_effect(e, o.effect_amount, clock, Vec2::ZERO));
            let tiles = geometria::tiles([o.x, o.y, o.w, o.h], (t.width(), t.height()), (scroll[0] * clock, scroll[1] * clock), lv.view());
            for (d, s) in tiles {
                let at = lv.to_screen(d[0], d[1]);
                draw_texture_ex(t, at.x, at.y, mul([255, 255, 255]), DrawTextureParams {
                    // +0.5 px para que no se vean rayas entre baldosas al escalar
                    dest_size: Some(vec2(d[2] * k.x + 0.5, d[3] * k.y + 0.5)),
                    source: Some(Rect::new(s[0], s[1], s[2], s[3])),
                    ..Default::default()
                });
            }
            if on {
                gl_use_default_material();
            }
        }
        Kind::Tilemap { tileset, tile, cols, .. } => {
            let Some(t) = gfx.tex.get(tileset) else { return };
            let on = effect.filter(|e| !e.needs_margin()).is_some_and(|e| gfx.mats.use_effect(e, o.effect_amount, clock, Vec2::ZERO));
            let per_row = (t.width() / tile).floor().max(1.) as i32;
            // Solo las celdas que se ven en pantalla.
            let [vx, vy, vw, vh] = lv.view();
            let (c0, c1) = (((vx - o.x) / tile).floor().max(0.) as usize, ((vx + vw - o.x) / tile).ceil().max(0.) as usize);
            let (r0, r1) = (((vy - o.y) / tile).floor().max(0.) as usize, ((vy + vh - o.y) / tile).ceil().max(0.) as usize);
            let rows = o.tiles.len() / cols.max(&1);
            for r in r0..r1.min(rows) {
                for c in c0..c1.min(*cols) {
                    let n = o.tiles[r * cols + c];
                    if n < 0 {
                        continue;
                    }
                    let at = lv.to_screen(o.x + c as f32 * tile, o.y + r as f32 * tile);
                    draw_texture_ex(t, at.x, at.y, mul([255, 255, 255]), DrawTextureParams {
                        dest_size: Some(vec2(tile * k.x + 0.5, tile * k.y + 0.5)),
                        source: Some(Rect::new((n % per_row) as f32 * tile, (n / per_row) as f32 * tile, *tile, *tile)),
                        ..Default::default()
                    });
                }
            }
            if on {
                gl_use_default_material();
            }
        }
        Kind::Text { size, color, font, align, wrap, .. } => {
            let (lines, block_w, lh) = text_block(gfx, font, *size, *wrap, &world.display_text(i));
            let fs = (size * k.y).round().clamp(1., 2000.) as u16;
            let mut col = mul(*color);
            if let Some(e) = effect {
                col = e.apply_color(col, o.effect_amount);
            }
            for (n, (line, w)) in lines.iter().enumerate() {
                let at = lv.to_screen(o.x + geometria::align_x(align, *w, block_w), o.y + n as f32 * lh + size * 0.8);
                draw_text_ex(line, at.x, at.y, TextParams { font: gfx.fonts.get(font), font_size: fs, font_scale: 1., font_scale_aspect: k.x / k.y, color: col, ..Default::default() });
            }
        }
        // Los emisores no se ven (se ven sus partículas); los objetos 3D los dibuja el reproductor 3D.
        _ => {}
    }
}

/// Un sprite con su origen (el rectángulo y el centro de giro los da el motor), giro, volteo y efecto.
fn draw_sprite(gfx: &Gfx, t: &Texture2D, o: &Inst, color: Color, effect: Option<Effect>, lv: &LayerView, clock: f32) {
    let k = lv.k();
    let (tw, th) = (t.width().max(1.), t.height().max(1.));
    let (x, y, w, h, px, py) = o.draw_rect();
    let (fx, fy) = (w / tw, h / th); // píxeles del mundo por píxel de la imagen
    // Silueta y ondas se salen del rectángulo: se dibuja con un margen (uv fuera de 0..1 es transparente).
    let mut texel = Vec2::ZERO;
    let (mx, my) = match effect {
        Some(Effect::Outline) => {
            let px = (2. / (fx * k.x).max(1e-3)).max(1.); // ~2 px de pantalla, al menos un píxel de la imagen
            let py = (2. / (fy * k.y).max(1e-3)).max(1.);
            texel = vec2(px / tw, py / th);
            (px + 1., py + 1.)
        }
        Some(Effect::Wave) => ((tw * 0.05).ceil() + 1., (th * 0.03).ceil() + 1.),
        _ => (0., 0.),
    };
    let on = effect.is_some_and(|e| gfx.mats.use_effect(e, o.effect_amount, clock, texel));
    let (mx, my) = if on { (mx, my) } else { (0., 0.) };
    let at = lv.to_screen(x - mx * fx, y - my * fy);
    draw_texture_ex(t, at.x, at.y, color, DrawTextureParams {
        dest_size: Some(vec2((w + 2. * mx * fx) * k.x, (h + 2. * my * fy) * k.y)),
        source: if mx > 0. || my > 0. { Some(Rect::new(-mx, -my, tw + 2. * mx, th + 2. * my)) } else { None },
        rotation: o.angle.to_radians(),
        flip_x: o.flip_x,
        pivot: Some(lv.to_screen(px, py)),
        ..Default::default()
    });
    if on {
        gl_use_default_material();
    }
}

/// Partículas de una capa (encima de sus instancias). Las aditivas se dibujan juntas al final.
fn draw_particles(world: &World, gfx: &Gfx, layer: &str, lv: &LayerView) {
    let k = lv.k();
    let mut additive = vec![];
    let draw = |p: &nicostation::runtime::Particle, def: &nicostation::model::ParticleDef| {
        let (c, a, s) = nicostation::particulas::particle_look(def, p);
        if a <= 0. || s <= 0. {
            return;
        }
        let col = Color::from_rgba(c[0], c[1], c[2], a as u8);
        let at = lv.to_screen(p.x, p.y);
        match gfx.tex.get(&def.image) {
            Some(t) => draw_texture_ex(t, at.x - s * k.x / 2., at.y - s * k.y / 2., col, DrawTextureParams { dest_size: Some(vec2(s * k.x, s * k.y)), ..Default::default() }),
            None => draw_ellipse(at.x, at.y, s * k.x / 2., s * k.y / 2., 0., col),
        }
    };
    for p in world.particles.iter().filter(|p| p.layer == layer) {
        let Some(Kind::Particles(def)) = world.object_def(&p.obj).map(|d| &d.kind) else { continue };
        if def.additive {
            additive.push((p, def));
        } else {
            draw(p, def);
        }
    }
    if !additive.is_empty() {
        let on = gfx.mats.use_additive();
        for (p, def) in additive {
            draw(p, def);
        }
        if on {
            gl_use_default_material();
        }
    }
}

#[cfg(test)]
mod tests {
    use nicostation::guardado;

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

    #[test]
    fn raton_con_cada_escalado() {
        // El mouse en la ventana vuelve a píxeles del juego con cualquier escalado.
        for mode in ["fit", "stretch", "integer"] {
            let vp = super::geometria::viewport(mode, 1000., 700., 320., 180.);
            let to_win = |x: f32, y: f32| (vp.off.0 + x * vp.scale.0, vp.off.1 + y * vp.scale.1);
            let (wx, wy) = to_win(100., 50.);
            let back = ((wx - vp.off.0) / vp.scale.0, (wy - vp.off.1) / vp.scale.1);
            assert!((back.0 - 100.).abs() < 1e-3 && (back.1 - 50.).abs() < 1e-3, "{mode}");
        }
    }
}
