//! Reproductor 3D de NicoStation: `nicostation3d proyecto.json [--escena NOMBRE] [--libre]`.
//! El mismo `World` y las mismas hojas de eventos que el reproductor 2D; dibuja con kiss3d (detrás de `render3d`)
//! y suena con kira. Si el ejecutable trae un juego empaquetado (ver `pack.rs`), lo juega sin leer nada de disco.
//!
//! Captura sin ventana (sirve con el monitor apagado):
//! `nicostation3d proyecto.json --captura salida.png [--segundos N] [--teclas "Up:1.0,Left+Up:0.5"]`
//! simula N segundos con esas teclas (tramos uno detrás de otro) y guarda el último cuadro.

mod audio;
mod entrada;
mod render3d;

use nicostation::guardado;
use nicostation::model::{Kind, Project};
use nicostation::pack::Pack;
use nicostation::runtime::{Cmd, World};
use render3d::{Aspecto, Camara, Escena, Evento, Pose};
use std::collections::HashSet;
use std::path::PathBuf;

/// Paso fijo del mundo (60 Hz) y máximo de pasos por cuadro (si la máquina no da abasto, el juego va más lento).
const DT: f32 = 1. / 60.;
const MAX_PASOS: u32 = 5;

/// De dónde salen los archivos del juego.
enum Activos {
    /// Carpeta del proyecto (ya es la carpeta actual).
    Disco,
    /// Paquete pegado al ejecutable. kiss3d carga modelos, texturas y cielos desde rutas, así que lo que se
    /// necesita se copia a una carpeta temporal la primera vez.
    Paquete(Pack, PathBuf),
}

impl Activos {
    fn bytes(&self, rel: &str) -> Option<Vec<u8>> {
        match self {
            Activos::Disco => std::fs::read(rel).ok(),
            Activos::Paquete(pk, _) => pk.files.get(rel).cloned(),
        }
    }

    fn ruta(&self, rel: &str) -> Option<PathBuf> {
        nicostation::export::check_rel(rel).map_err(|e| eprintln!("{e}")).ok()?;
        match self {
            Activos::Disco => Some(PathBuf::from(rel)),
            Activos::Paquete(pk, dir) => {
                let f = dir.join(rel);
                if !f.is_file() {
                    std::fs::create_dir_all(f.parent()?).ok()?;
                    std::fs::write(&f, pk.files.get(rel)?).ok()?;
                }
                Some(f)
            }
        }
    }
}

struct Opciones {
    proyecto: String,
    escena: Option<String>,
    libre: bool,
    captura: Option<PathBuf>,
    segundos: f32,
    teclas: Vec<(Vec<String>, f32)>,
}

fn uso() -> ! {
    eprintln!("uso: nicostation3d proyecto.json [--escena NOMBRE] [--libre]\n     nicostation3d proyecto.json --captura salida.png [--segundos N] [--teclas \"Up:1.0,Left+Up:0.5\"]");
    std::process::exit(2)
}

fn opciones(args: &[String]) -> Result<Opciones, String> {
    let mut o = Opciones { proyecto: "juego.json".into(), escena: None, libre: false, captura: None, segundos: 0., teclas: vec![] };
    let mut it = args.iter();
    let mut posicional = false;
    while let Some(a) = it.next() {
        let mut valor = || it.next().cloned().ok_or_else(|| format!("falta el valor de {a}"));
        match a.as_str() {
            "--escena" => o.escena = Some(valor()?),
            "--libre" => o.libre = true,
            // Absoluta: después el reproductor entra a la carpeta del proyecto.
            "--captura" => o.captura = Some(std::path::absolute(valor()?).map_err(|e| e.to_string())?),
            "--segundos" => o.segundos = valor()?.parse().map_err(|_| "--segundos espera un número".to_string())?,
            "--teclas" => o.teclas = entrada::tramos(&valor()?)?,
            "-h" | "--help" | "--ayuda" => uso(),
            x if x.starts_with("--") => return Err(format!("opción desconocida: {x}")),
            x if !posicional => {
                o.proyecto = x.into();
                posicional = true;
            }
            x => return Err(format!("sobra «{x}»")),
        }
    }
    Ok(o)
}

/// Centro de la escena (promedio de las mallas), para la cámara por defecto.
fn centro(world: &World) -> [f32; 3] {
    let ps: Vec<[f32; 3]> = world.insts.iter()
        .filter(|i| !i.dead && matches!(world.object_def(&i.obj).map(|d| &d.kind), Some(Kind::Mesh { .. })))
        .map(|i| [i.x, i.y, i.z]).collect();
    if ps.is_empty() {
        return [0.; 3];
    }
    let n = ps.len() as f32;
    [0, 1, 2].map(|k| ps.iter().map(|p| p[k]).sum::<f32>() / n)
}

/// Refleja el mundo en la escena 3D: entorno, mallas, luces, cámara y textos (HUD).
fn reflejar(world: &World, escena: &mut Escena, activos: &Activos, centro: [f32; 3]) {
    let archivo = |rel: &str| activos.ruta(rel);
    escena.set_entorno(&world.scene_def().environment.clone().unwrap_or_default(), &archivo);
    let mut vivos = HashSet::new();
    let mut camara = None;
    let p = world.project.clone();
    let (ww, wh) = escena.tamano();
    let k = (ww as f32 / p.width).min(wh as f32 / p.height);
    for (i, o) in world.insts.iter().enumerate() {
        if o.dead {
            continue;
        }
        let Some(def) = world.object_def(&o.obj) else { continue };
        match &def.kind {
            Kind::Mesh { .. } | Kind::Light { .. } => {
                vivos.insert(o.id);
                let pose = Pose { pos: [o.x, o.y, o.z], yaw: o.angle, pitch: o.pitch, roll: o.roll, size: [o.w, o.h, o.d], base: [o.base_w, o.base_h, o.base_d] };
                escena.poner(o.id, &def.kind, &pose, &Aspecto { visible: o.visible, tint: o.tint, opacity: o.opacity }, &archivo);
            }
            Kind::Camera3d { fov, near, far } if camara.is_none() => {
                camara = Some(Camara { pos: [o.x, o.y, o.z], yaw: o.angle, pitch: o.pitch, roll: o.roll, fov: *fov, near: *near, far: *far });
            }
            // HUD: los textos se dibujan encima, en coordenadas de pantalla del juego (ancho × alto del proyecto).
            Kind::Text { size, color, .. } if o.visible => {
                let c = [0, 1, 2].map(|n| (color[n] as u16 * o.tint[n] as u16 / 255) as u8);
                escena.texto(&world.display_text(i), o.x * k, o.y * k, size * k, c, (o.opacity / 255.).clamp(0., 1.));
            }
            _ => {}
        }
    }
    escena.retener(&vivos);
    match camara {
        Some(c) => escena.set_camara(&c),
        None => escena.mirar([0., 6., 12.], centro),
    }
}

/// Cumple los pedidos del mundo. Devuelve true si el juego pidió salir.
fn ordenes(world: &mut World, audio: &mut audio::Audio, activos: &Activos, juego: &str, guardar: bool) -> bool {
    for cmd in std::mem::take(&mut world.cmds) {
        if let Cmd::PlaySound { file, .. } | Cmd::PlayMusic { file, .. } = &cmd {
            if !audio.tiene(file) {
                match activos.bytes(file) {
                    Some(b) => audio.cargar(file, b),
                    None => eprintln!("sonido {file}: no está"),
                }
            }
        }
        match cmd {
            Cmd::PlaySound { file, volume } => audio.sonido(&file, volume),
            Cmd::PlayMusic { file, volume } => audio.musica(&file, volume),
            Cmd::StopMusic => audio.parar_musica(),
            Cmd::StopSound { file } => audio.parar(&file),
            Cmd::Persist(v) if guardar => guardado::store(juego, &v),
            Cmd::Persist(_) => {}
            Cmd::Quit => return true,
        }
    }
    false
}

#[kiss3d::main]
async fn main() {
    let embebido = nicostation::pack::read_embedded();
    let o = if embebido.is_some() {
        // Un juego empaquetado no recibe opciones ni cambia de carpeta: todo está adentro.
        Opciones { proyecto: String::new(), escena: None, libre: false, captura: None, segundos: 0., teclas: vec![] }
    } else {
        let args: Vec<String> = std::env::args().skip(1).collect();
        opciones(&args).unwrap_or_else(|e| {
            eprintln!("{e}");
            uso()
        })
    };
    let (src, activos) = match embebido {
        Some(pk) => {
            let src = pk.project_json().to_vec();
            let dir = std::env::temp_dir().join(format!("nicostation3d-{}", std::process::id()));
            (src, Activos::Paquete(pk, dir))
        }
        None => {
            let path = PathBuf::from(&o.proyecto);
            let src = std::fs::read(&path).unwrap_or_else(|e| {
                eprintln!("no pude leer {}: {e}", path.display());
                std::process::exit(1)
            });
            if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
                std::env::set_current_dir(dir).expect("no pude entrar a la carpeta del proyecto");
            }
            (src, Activos::Disco)
        }
    };
    let mut project: Project = serde_json::from_slice(&src).unwrap_or_else(|e| {
        eprintln!("{}: {e}", o.proyecto);
        std::process::exit(1)
    });
    if let Some(e) = &o.escena {
        if project.scene(e).is_none() {
            eprintln!("no hay una escena «{e}»; hay: {}", project.scenes.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(", "));
            std::process::exit(2);
        }
        project.first_scene = e.clone();
    }
    if !project.scene(&project.first_scene).or(project.scenes.first()).is_some_and(|s| s.dim3) {
        eprintln!("aviso: la escena «{}» es 2D (sin \"dim3\": true); se juega mejor con `nicostation`", project.first_scene);
    }
    let juego = project.name.clone();
    let (ancho, alto) = (project.width.max(64.) as u32, project.height.max(64.) as u32);
    let sonidos: Vec<String> = nicostation::export::assets(&project).into_iter()
        .filter(|f| [".wav", ".ogg", ".mp3"].iter().any(|x| f.to_lowercase().ends_with(x))).collect();

    if let Some(salida) = o.captura.clone() {
        return capturar(project, &activos, &o, &salida, ancho, alto).await;
    }

    let mut escena = Escena::ventana(&juego, ancho, alto).await;
    // Pantalla de carga: se ve mientras se decodifican los sonidos y se cargan modelos y texturas (primer cuadro).
    escena.texto("Cargando…", ancho as f32 / 2. - 70., alto as f32 / 2., 36., [255; 3], 0.85);
    escena.dibujar().await;
    let mut audio = audio::Audio::new(true);
    for f in &sonidos {
        if let Some(b) = activos.bytes(f) {
            audio.cargar(f, b);
        }
    }
    let mut world = World::new(project);
    // La partida guardada tiene que estar antes del primer paso (los eventos «al empezar» la leen).
    world.storage = guardado::load(&juego);
    let mut escena_actual = usize::MAX;
    let mut medio = [0.; 3];
    let mut teclado = entrada::Teclado::default();
    let mut acumulado = 0f32;
    let mut antes = std::time::Instant::now();
    'juego: loop {
        if world.scene != escena_actual {
            escena_actual = world.scene;
            medio = centro(&world);
            if o.libre {
                escena.set_libre([medio[0], medio[1] + 6., medio[2] + 12.], medio);
            }
        }
        for e in escena.eventos() {
            let Evento::Tecla(n, abajo) = e;
            teclado.tecla(n, abajo);
        }
        let ((mx, my), botones) = escena.mouse();
        let (ww, wh) = escena.tamano();
        let p = world.project.clone();
        let mouse = (mx * p.width / ww.max(1) as f32, my * p.height / wh.max(1) as f32);
        let ahora = std::time::Instant::now();
        acumulado += (ahora - antes).as_secs_f32().min(0.25);
        antes = ahora;
        let mut pasos = 0;
        while acumulado >= DT && pasos < MAX_PASOS {
            world.step(DT, teclado.paso(mouse, botones));
            acumulado -= DT;
            pasos += 1;
            if ordenes(&mut world, &mut audio, &activos, &juego, true) {
                break 'juego;
            }
            escena.animar(DT);
        }
        if pasos == MAX_PASOS {
            acumulado = 0.;
        }
        reflejar(&world, &mut escena, &activos, medio);
        if !escena.dibujar().await {
            break;
        }
    }
    if let Activos::Paquete(_, dir) = &activos {
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// `--captura`: simula sin ventana con las teclas programadas y guarda el último cuadro.
async fn capturar(project: Project, activos: &Activos, o: &Opciones, salida: &std::path::Path, ancho: u32, alto: u32) {
    let juego = project.name.clone();
    let total = o.segundos.max(o.teclas.iter().map(|t| t.1).sum::<f32>()).max(0.);
    let mut world = World::new(project);
    world.storage = guardado::load(&juego);
    let medio = centro(&world);
    let mut escena = Escena::fuera_de_pantalla(ancho, alto).await;
    if o.libre {
        escena.set_libre([medio[0], medio[1] + 6., medio[2] + 12.], medio);
    }
    let mut audio = audio::Audio::new(false);
    let mut teclado = entrada::Teclado::default();
    let pasos = (total / DT).round() as u64;
    let t0 = std::time::Instant::now();
    for k in 0..pasos {
        teclado.poner(entrada::teclas_en(&o.teclas, k as f32 * DT));
        world.step(DT, teclado.paso((0., 0.), [false; 3]));
        // Sin guardar la partida: una captura no debe tocar la del jugador.
        if ordenes(&mut world, &mut audio, activos, &juego, false) {
            break;
        }
        escena.animar(DT);
    }
    let sim = t0.elapsed();
    let t1 = std::time::Instant::now();
    // Varios cuadros: el primero sube texturas y mallas; los efectos de pantalla (SSAO, bloom) se asientan.
    for _ in 0..4 {
        reflejar(&world, &mut escena, activos, medio);
        escena.dibujar().await;
    }
    if let Err(e) = escena.guardar_png(salida) {
        eprintln!("no pude guardar {}: {e}", salida.display());
        std::process::exit(1);
    }
    let vars: Vec<String> = world.globals.iter().chain(world.vars.iter()).map(|(k, v)| format!("{k}={}", v.text())).collect();
    println!(
        "{}: {:.2} s simulados ({pasos} pasos, {:.0} ms) + 4 cuadros {ancho}x{alto} ({:.0} ms); {} objetos 3D dibujados; variables: {}",
        salida.display(), total, sim.as_secs_f64() * 1000., t1.elapsed().as_secs_f64() * 1000., escena.cantidad(), vars.join(", ")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn opciones_de_la_linea_de_comandos() {
        let o = opciones(&a(&["mundo/juego.json", "--escena", "Nivel 2", "--libre"])).unwrap();
        assert_eq!((o.proyecto.as_str(), o.escena.as_deref(), o.libre, o.captura.is_none()), ("mundo/juego.json", Some("Nivel 2"), true, true));
        let o = opciones(&a(&["--captura", "x.png", "--segundos", "2.5", "--teclas", "Up:1,Left:0.5"])).unwrap();
        assert_eq!((o.proyecto.as_str(), o.segundos, o.teclas.len()), ("juego.json", 2.5, 2));
        assert_eq!(o.captura, Some(std::env::current_dir().unwrap().join("x.png")));
        assert!(opciones(&a(&["--escena"])).is_err());
        assert!(opciones(&a(&["--nada"])).is_err());
        assert!(opciones(&a(&["a.json", "b.json"])).is_err());
        assert!(opciones(&a(&["--segundos", "dos"])).is_err());
    }

    #[test]
    fn mundo_3d_sin_ventana() {
        // El World corre sin ventana: el reflejo usa sus instancias tal cual (centro = posición, metros).
        let p: Project = serde_json::from_str(r#"{"name":"t","width":800,"height":600,"first_scene":"S","scenes":[{"name":"S","dim3":true,
            "objects":[{"name":"Caja","type":"mesh","shape":"cube","size":[2,1,2]},{"name":"Sol","type":"light","light":"sun"}],
            "instances":[{"obj":"Caja","x":2,"y":0.5,"z":-4},{"obj":"Caja","x":-2,"y":0.5,"z":0},{"obj":"Sol","x":0,"y":10,"z":0,"pitch":-60}]}]}"#).unwrap();
        let w = World::new(p);
        assert_eq!(centro(&w), [0., 0.5, -2.]);
        let caja = &w.insts[0];
        assert_eq!(([caja.w, caja.h, caja.d], [caja.base_w, caja.base_h, caja.base_d]), ([2., 1., 2.], [2., 1., 2.]));
    }
}
