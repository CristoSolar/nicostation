//! Reproductor: `nicostation proyecto.json`. Ventana, imágenes, sonido y entrada con macroquad.
//! También compila a WebAssembly: en el navegador lee `juego.json` junto a la página.
//! Exportar a web: `nicostation --export-web proyecto.json carpeta/`.

use macroquad::audio::{load_sound, play_sound, stop_sound, PlaySoundParams, Sound};
use macroquad::prelude::*;
use nicostation::model::{Kind, Project};
use nicostation::runtime::{Cmd, Input, World};
use std::collections::HashMap;

fn project_path() -> String {
    std::env::args().nth(1).unwrap_or("juego.json".into())
}

fn conf() -> Conf {
    let c = Conf { window_title: "NicoStation".into(), window_width: 800, window_height: 600, window_resizable: true, ..Default::default() };
    // En escritorio la ventana toma el tamaño y nombre del juego; en web los define la página.
    #[cfg(not(target_arch = "wasm32"))]
    let c = match std::fs::read_to_string(project_path()).ok().and_then(|s| serde_json::from_str::<Project>(&s).ok()) {
        Some(p) => Conf { window_title: p.name, window_width: p.width as i32, window_height: p.height as i32, ..c },
        None => c,
    };
    c
}

fn key_name(k: KeyCode) -> String {
    format!("{k:?}")
}

fn main() {
    let conf = conf(); // antes de cambiar de carpeta: lee el proyecto con la ruta original
    #[cfg(not(target_arch = "wasm32"))]
    {
        let a: Vec<String> = std::env::args().collect();
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
    // Ya estamos en la carpeta del proyecto: basta el nombre del archivo.
    let file = std::path::Path::new(&project_path()).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or("juego.json".into());
    let src = load_string(&file).await.unwrap_or_else(|e| panic!("no pude leer {file}: {e}"));
    let mut project: Project = serde_json::from_str(&src).unwrap_or_else(|e| panic!("{file}: {e}"));

    // Imágenes de todas las animaciones; los sprites sin tamaño toman el de su primera imagen.
    let mut tex: HashMap<String, Texture2D> = HashMap::new();
    let defs = project.objects.iter_mut().chain(project.scenes.iter_mut().flat_map(|s| s.objects.iter_mut()));
    for o in defs {
        if let Kind::Tilemap { tileset, .. } = &o.kind {
            if !tex.contains_key(tileset) {
                match load_texture(tileset).await {
                    Ok(t) => {
                        t.set_filter(FilterMode::Nearest);
                        tex.insert(tileset.clone(), t);
                    }
                    Err(e) => eprintln!("imagen {tileset}: {e}"),
                }
            }
        }
        if let Kind::Sprite { w, h, animations } = &mut o.kind {
            for f in animations.iter().flat_map(|a| a.frames.iter()) {
                if !tex.contains_key(f) {
                    match load_texture(f).await {
                        Ok(t) => {
                            t.set_filter(FilterMode::Nearest);
                            tex.insert(f.clone(), t);
                        }
                        Err(e) => eprintln!("imagen {f}: {e}"),
                    }
                }
            }
            if let Some(t) = animations.first().and_then(|a| a.frames.first()).and_then(|f| tex.get(f)) {
                if *w == 0. { *w = t.width() }
                if *h == 0. { *h = t.height() }
            }
        }
    }

    let mut world = World::new(project);
    let mut sounds: HashMap<String, Sound> = HashMap::new();
    let mut music: Option<Sound> = None;

    loop {
        let p = world.project.clone();
        let scale = (screen_width() / p.width).min(screen_height() / p.height);
        let off = vec2((screen_width() - p.width * scale) / 2., (screen_height() - p.height * scale) / 2.);

        let (mx, my) = mouse_position();
        let input = Input {
            down: get_keys_down().into_iter().map(key_name).collect(),
            pressed: get_keys_pressed().into_iter().map(key_name).collect(),
            released: get_keys_released().into_iter().map(key_name).collect(),
            mouse: ((mx - off.x) / scale, (my - off.y) / scale),
            mouse_down: [MouseButton::Left, MouseButton::Right, MouseButton::Middle].map(is_mouse_button_down),
            mouse_pressed: [MouseButton::Left, MouseButton::Right, MouseButton::Middle].map(is_mouse_button_pressed),
        };
        world.step(get_frame_time(), input);

        for cmd in std::mem::take(&mut world.cmds) {
            if let Cmd::PlaySound { file, .. } | Cmd::PlayMusic { file, .. } = &cmd {
                if !sounds.contains_key(file) {
                    match load_sound(file).await {
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
                let alpha = (o.opacity.clamp(0., 255.)) as u8;
                let c = to_screen(o.cx(), o.cy());
                match &def.kind {
                    Kind::Box { color, .. } => draw_rectangle_ex(c.x, c.y, o.w * k, o.h * k, DrawRectangleParams {
                        offset: vec2(0.5, 0.5),
                        rotation: o.angle.to_radians(),
                        color: Color::from_rgba(color[0], color[1], color[2], alpha),
                    }),
                    Kind::Sprite { animations, .. } => {
                        let frame = animations.get(o.anim).and_then(|a| a.frames.get(o.frame)).and_then(|f| tex.get(f));
                        match frame {
                            Some(t) => draw_texture_ex(t, c.x - o.w * k / 2., c.y - o.h * k / 2., Color::from_rgba(255, 255, 255, alpha), DrawTextureParams {
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
                                draw_texture_ex(t, at.x, at.y, Color::from_rgba(255, 255, 255, alpha), DrawTextureParams {
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
                        draw_text(&txt, tl.x, tl.y + size * k * 0.8, size * k, Color::from_rgba(color[0], color[1], color[2], alpha));
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

