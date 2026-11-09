//! Explorador de recursos: imágenes, sonidos, fuentes y modelos de la carpeta del juego.
//! Importar (botón o arrastrando archivos a la ventana), renombrar y mover sin romper el juego
//! (se actualizan todas las referencias del proyecto) y borrar avisando si está en uso.

use super::iconos as ic;
use super::tema::*;
use eframe::egui::{self, Align, Align2, AtomExt, Button, Color32, FontId, Frame, Layout, Margin, Pos2, Rect, RichText, Sense, Stroke, TextEdit, Ui, Vec2};
use nicostation::catalog::{self, P};
use nicostation::model::*;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

// ---------- tipos de archivo ----------

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord, Hash)]
pub enum Tipo {
    Imagen,
    Sonido,
    Fuente,
    Modelo,
}

pub const TIPOS: [Tipo; 4] = [Tipo::Imagen, Tipo::Sonido, Tipo::Fuente, Tipo::Modelo];

impl Tipo {
    pub fn de(ruta: &str) -> Option<Tipo> {
        let ext = ruta.rsplit('.').next()?.to_lowercase();
        Some(match ext.as_str() {
            "png" | "jpg" | "jpeg" => Tipo::Imagen,
            "wav" | "ogg" | "mp3" => Tipo::Sonido,
            "ttf" | "otf" => Tipo::Fuente,
            "glb" | "gltf" | "obj" | "hdr" => Tipo::Modelo,
            _ => return None,
        })
    }
    /// Carpeta donde se copian al importar.
    pub fn carpeta(self) -> &'static str {
        match self {
            Tipo::Imagen => "img",
            Tipo::Sonido => "snd",
            Tipo::Fuente => "fuentes",
            Tipo::Modelo => "modelos",
        }
    }
    pub fn nombre(self, facil: bool) -> &'static str {
        match (self, facil) {
            (Tipo::Imagen, _) => "Imágenes",
            (Tipo::Sonido, _) => "Sonidos",
            (Tipo::Fuente, true) => "Letras",
            (Tipo::Fuente, false) => "Fuentes",
            (Tipo::Modelo, true) => "Cosas 3D",
            (Tipo::Modelo, false) => "Modelos 3D",
        }
    }
    pub fn icono(self) -> &'static str {
        match self {
            Tipo::Imagen => ic::IMAGE,
            Tipo::Sonido => ic::FILE_AUDIO,
            Tipo::Fuente => ic::TEXT_AA,
            Tipo::Modelo => ic::CUBE,
        }
    }
    pub fn extensiones(self) -> &'static [&'static str] {
        match self {
            Tipo::Imagen => &["png", "jpg", "jpeg"],
            Tipo::Sonido => &["wav", "ogg", "mp3"],
            Tipo::Fuente => &["ttf", "otf"],
            Tipo::Modelo => &["glb", "gltf", "obj", "hdr"],
        }
    }
}

/// Archivos de recursos dentro de la carpeta del juego (rutas relativas con `/`, ordenadas).
pub fn listar(base: &Path) -> Vec<String> {
    fn walk(base: &Path, dir: &Path, depth: usize, out: &mut Vec<String>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || out.len() >= 2000 {
                continue;
            }
            if p.is_dir() {
                if depth < 5 && !matches!(name.as_str(), "target" | "web" | "node_modules") {
                    walk(base, &p, depth + 1, out);
                }
            } else if let Ok(r) = p.strip_prefix(base) {
                let r = r.to_string_lossy().replace('\\', "/");
                if Tipo::de(&r).is_some() {
                    out.push(r);
                }
            }
        }
    }
    let mut out = vec![];
    walk(base, base, 1, &mut out);
    out.sort();
    out
}

// ---------- referencias en el proyecto ----------

/// Cada ruta de archivo que usa el proyecto, con dónde se usa (para avisar y renombrar).
pub fn referencias(p: &Project) -> Vec<(String, String)> {
    let mut v = vec![];
    let mut add = |r: &str, donde: String| {
        if !r.is_empty() {
            v.push((r.to_string(), donde));
        }
    };
    add(&p.settings.icon, "Ícono del juego".into());
    add(&p.settings.loading_image, "Imagen de carga".into());
    let objs = p.objects.iter().map(|o| (o, String::new())).chain(p.scenes.iter().flat_map(|s| s.objects.iter().map(move |o| (o, format!(" (escena «{}»)", s.name)))));
    for (o, en) in objs {
        let n = &o.name;
        match &o.kind {
            Kind::Sprite { animations, .. } => {
                for a in animations {
                    a.frames.iter().for_each(|f| add(f, format!("Sprite «{n}», animación «{}»{en}", a.name)));
                }
            }
            Kind::Tilemap { tileset, .. } => add(tileset, format!("Baldosas del mapa «{n}»{en}")),
            Kind::Background { image, .. } => add(image, format!("Fondo «{n}»{en}")),
            Kind::Text { font, .. } => add(font, format!("Fuente del texto «{n}»{en}")),
            Kind::Particles(d) => add(&d.image, format!("Partículas «{n}»{en}")),
            Kind::Mesh { model, texture, .. } => {
                add(model, format!("Modelo de «{n}»{en}"));
                add(texture, format!("Textura de «{n}»{en}"));
            }
            _ => {}
        }
    }
    for s in &p.scenes {
        if let Some(e) = &s.environment {
            add(&e.sky_image, format!("Cielo de la escena «{}»", s.name));
        }
        archivos_eventos(&s.events, &mut |f| add(f, format!("Eventos de la escena «{}»", s.name)));
        for g in &s.graphs {
            archivos_grafo(g, &mut |f| add(f, format!("Nodos «{}» de la escena «{}»", g.name, s.name)));
        }
    }
    for f in &p.functions {
        archivos_eventos(&f.events, &mut |a| add(a, format!("Bloque «{}»", f.name)));
        if let Some(g) = &f.graph {
            archivos_grafo(g, &mut |a| add(a, format!("Nodos del bloque «{}»", f.name)));
        }
    }
    v
}

fn archivos_instr(l: &[Instr], f: &mut dyn FnMut(&str)) {
    for i in l {
        let ps = catalog::meta(&i.op).map_or(&[][..], |m| m.params);
        for (k, a) in i.args.iter().enumerate() {
            if ps.get(k).is_some_and(|p| p.1 == P::File) {
                f(a);
            }
        }
        archivos_instr(&i.sub, f);
    }
}

fn archivos_eventos(evs: &[Event], f: &mut dyn FnMut(&str)) {
    for e in evs {
        archivos_instr(&e.conds, f);
        archivos_instr(&e.acts, f);
        archivos_eventos(&e.sub, f);
    }
}

fn archivos_grafo(g: &nicostation::nodos::Graph, f: &mut dyn FnMut(&str)) {
    for n in &g.nodes {
        let specs = nicostation::nodos::arg_specs(n);
        for (k, a) in n.args.iter().enumerate() {
            if specs.get(k).is_some_and(|s| s.p == P::File) {
                f(a);
            }
        }
        archivos_instr(&n.sub, f);
    }
}

/// Dónde se usa un archivo (o cualquier archivo dentro de una carpeta, si `ruta` termina en `/`).
pub fn usos(p: &Project, ruta: &str) -> Vec<String> {
    referencias(p).into_iter().filter(|(r, _)| coincide(r, ruta)).map(|(_, d)| d).collect()
}

fn coincide(r: &str, ruta: &str) -> bool {
    if ruta.ends_with('/') { r.starts_with(ruta) } else { r == ruta }
}

/// Nueva ruta de `r` si `viejo` (archivo, o carpeta terminada en `/`) pasa a llamarse `nuevo`.
fn cambiar(r: &str, viejo: &str, nuevo: &str) -> Option<String> {
    if viejo.ends_with('/') {
        r.strip_prefix(viejo).map(|resto| format!("{nuevo}{resto}"))
    } else {
        (r == viejo).then(|| nuevo.to_string())
    }
}

/// Cambia en todo el proyecto las referencias a `viejo` por `nuevo` (archivo o carpeta terminada en `/`).
/// Devuelve cuántas referencias cambió.
pub fn renombrar_en_proyecto(p: &mut Project, viejo: &str, nuevo: &str) -> usize {
    let mut n = 0;
    let mut f = |s: &mut String| {
        if let Some(x) = cambiar(s, viejo, nuevo) {
            *s = x;
            n += 1;
        }
    };
    f(&mut p.settings.icon);
    f(&mut p.settings.loading_image);
    for o in p.objects.iter_mut().chain(p.scenes.iter_mut().flat_map(|s| s.objects.iter_mut())) {
        match &mut o.kind {
            Kind::Sprite { animations, .. } => animations.iter_mut().flat_map(|a| a.frames.iter_mut()).for_each(&mut f),
            Kind::Tilemap { tileset, .. } => f(tileset),
            Kind::Background { image, .. } => f(image),
            Kind::Text { font, .. } => f(font),
            Kind::Particles(d) => f(&mut d.image),
            Kind::Mesh { model, texture, .. } => {
                f(model);
                f(texture);
            }
            _ => {}
        }
    }
    for s in &mut p.scenes {
        if let Some(e) = &mut s.environment {
            f(&mut e.sky_image);
        }
    }
    // Argumentos de tipo archivo en eventos, funciones y nodos.
    let mut cambios: Vec<(String, String)> = vec![];
    for s in &p.scenes {
        archivos_eventos(&s.events, &mut |a| cambios.extend(cambiar(a, viejo, nuevo).map(|x| (a.to_string(), x))));
        s.graphs.iter().for_each(|g| archivos_grafo(g, &mut |a| cambios.extend(cambiar(a, viejo, nuevo).map(|x| (a.to_string(), x)))));
    }
    for fu in &p.functions {
        archivos_eventos(&fu.events, &mut |a| cambios.extend(cambiar(a, viejo, nuevo).map(|x| (a.to_string(), x))));
        if let Some(g) = &fu.graph {
            archivos_grafo(g, &mut |a| cambios.extend(cambiar(a, viejo, nuevo).map(|x| (a.to_string(), x))));
        }
    }
    n += cambios.len();
    cambios.sort();
    cambios.dedup();
    for (a, b) in cambios {
        for s in &mut p.scenes {
            ren_eventos(&mut s.events, &a, &b);
            s.graphs.iter_mut().chain([&mut s.node_drafts]).for_each(|g| super::nodos::ren_grafo(g, P::File, &a, &b, |x, _, _| x.to_string()));
        }
        for fu in &mut p.functions {
            ren_eventos(&mut fu.events, &a, &b);
            fu.graph.iter_mut().chain([&mut fu.node_drafts]).for_each(|g| super::nodos::ren_grafo(g, P::File, &a, &b, |x, _, _| x.to_string()));
        }
    }
    n
}

fn ren_instr(l: &mut [Instr], a: &str, b: &str) {
    for i in l {
        let ps = catalog::meta(&i.op).map_or(&[][..], |m| m.params);
        for (k, x) in i.args.iter_mut().enumerate() {
            if ps.get(k).is_some_and(|p| p.1 == P::File) && x == a {
                *x = b.into();
            }
        }
        ren_instr(&mut i.sub, a, b);
    }
}

fn ren_eventos(evs: &mut [Event], a: &str, b: &str) {
    for e in evs {
        ren_instr(&mut e.conds, a, b);
        ren_instr(&mut e.acts, a, b);
        ren_eventos(&mut e.sub, a, b);
    }
}

// ---------- archivos ----------

/// Nombre de archivo limpio (sin carpetas ni caracteres raros).
pub fn nombre_limpio(n: &str) -> String {
    let n = n.trim().replace(['/', '\\'], "_");
    n.chars().filter(|c| !matches!(c, ':' | '*' | '?' | '"' | '<' | '>' | '|')).collect::<String>().trim_start_matches('.').to_string()
}

/// Ruta libre: si `rel` ya existe en `base`, agrega _2, _3... antes de la extensión.
pub fn ruta_libre(base: &Path, rel: &str) -> String {
    if !base.join(rel).exists() {
        return rel.to_string();
    }
    let (stem, ext) = match rel.rfind('.') {
        Some(k) if k > rel.rfind('/').map_or(0, |s| s + 1) => (&rel[..k], &rel[k..]),
        _ => (rel, ""),
    };
    (2..).map(|k| format!("{stem}_{k}{ext}")).find(|r| !base.join(r).exists()).unwrap()
}

/// Copia un archivo de afuera a la carpeta que le toca según su tipo. Si ya está dentro del juego, lo deja donde está.
pub fn importar(base: &Path, src: &Path) -> Result<String, String> {
    let name = src.file_name().map(|n| n.to_string_lossy().to_string()).ok_or("archivo sin nombre")?;
    let tipo = Tipo::de(&name).ok_or_else(|| format!("«{name}»: tipo de archivo no reconocido (imágenes png/jpg, sonidos wav/ogg/mp3, fuentes ttf/otf, modelos glb/gltf)"))?;
    if let (Ok(s), Ok(b)) = (src.canonicalize(), base.canonicalize()) {
        if let Ok(r) = s.strip_prefix(&b) {
            return Ok(r.to_string_lossy().replace('\\', "/"));
        }
    }
    let dest = ruta_libre(base, &format!("{}/{}", tipo.carpeta(), nombre_limpio(&name)));
    std::fs::create_dir_all(base.join(tipo.carpeta())).map_err(|e| e.to_string())?;
    std::fs::copy(src, base.join(&dest)).map_err(|e| format!("No se pudo copiar «{name}»: {e}"))?;
    Ok(dest)
}

/// Mueve o renombra un archivo (o carpeta) dentro del juego sin pisar otro.
pub fn mover(base: &Path, viejo: &str, nuevo: &str) -> Result<(), String> {
    let (a, b) = (base.join(viejo.trim_end_matches('/')), base.join(nuevo.trim_end_matches('/')));
    if b.exists() {
        return Err(format!("Ya existe «{}»", nuevo.trim_end_matches('/')));
    }
    if let Some(d) = b.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&a, &b).map_err(|e| e.to_string())
}

/// Duración de un WAV en segundos (lee la cabecera RIFF).
pub fn wav_duracion(b: &[u8]) -> Option<f32> {
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return None;
    }
    let (mut i, mut byte_rate) = (12, 0u32);
    while i + 8 <= b.len() {
        let id = &b[i..i + 4];
        let len = u32::from_le_bytes(b[i + 4..i + 8].try_into().ok()?) as usize;
        if id == b"fmt " && i + 16 <= b.len() {
            byte_rate = u32::from_le_bytes(b[i + 16..i + 20].try_into().ok()?);
        } else if id == b"data" {
            return (byte_rate > 0).then(|| len as f32 / byte_rate as f32);
        }
        i += 8 + len + (len & 1);
    }
    None
}

fn tamano_txt(n: u64) -> String {
    if n >= 1 << 20 { format!("{:.1} MB", n as f64 / (1 << 20) as f64) } else if n >= 1024 { format!("{:.0} KB", n as f64 / 1024.) } else { format!("{n} B") }
}

// ---------- panel ----------

#[derive(Default)]
pub struct Explorador {
    pub sel: Option<String>,
    /// Carpeta elegida en el árbol (None = todas; "" = la carpeta del juego).
    pub carpeta: Option<String>,
    pub tipo: Option<Tipo>,
    pub filtro: String,
    /// Nombre en edición del archivo elegido (archivo, texto).
    pub nombre: Option<(String, String)>,
    pub borrar: Option<String>,
    /// Detalles del archivo elegido (ruta, texto) para no leerlo en cada cuadro.
    info: Option<(String, String)>,
}

pub enum Accion {
    Importar,
    Renombrar(String, String),
    Borrar(String),
}

/// Pestaña «Recursos». `thumbs`: miniaturas ya cargadas de las imágenes; `usos`: dónde se usa cada archivo.
pub fn ui(ui: &mut Ui, ex: &mut Explorador, base: &Path, files: &[String], thumbs: &HashMap<String, egui::TextureHandle>, refs: &[(String, String)], facil: bool, guardado: bool) -> Option<Accion> {
    let mut act = None;
    let usados: HashMap<&str, usize> = refs.iter().fold(HashMap::new(), |mut m, (r, _)| {
        *m.entry(r.as_str()).or_default() += 1;
        m
    });
    // Árbol de carpetas a la izquierda.
    let mut carpetas: BTreeMap<String, usize> = BTreeMap::new();
    for f in files {
        let d = f.rsplit_once('/').map_or("", |x| x.0);
        *carpetas.entry(d.to_string()).or_default() += 1;
    }
    egui::Panel::left("rec_arbol").default_size(220.).size_range(180.0..=360.).frame(Frame::new().fill(PANEL).inner_margin(12)).show(ui, |ui| {
        header(ui, ic::FOLDER_OPEN, if facil { "Mis archivos" } else { "Recursos" }, &format!("{}", files.len()));
        ui.add_space(4.);
        ui.add_enabled_ui(guardado, |ui| {
            ui.spacing_mut().button_padding = Vec2::new(12., 7.);
            if primary(ui, (on_accent(ic::PLUS), on_accent("Importar…").font(sb(14.)))).on_hover_text("Copiar imágenes, sonidos, fuentes o modelos al juego. También puedes arrastrar archivos a la ventana.").clicked() {
                act = Some(Accion::Importar);
            }
        });
        if !guardado {
            ui.label(RichText::new("Guarda el juego primero: los archivos se copian a su carpeta.").color(WARN));
        }
        ui.add_space(8.);
        ui.label(RichText::new("Tipo").font(sb(12.)).color(TEXT_MUTED));
        ui.spacing_mut().item_spacing.y = 2.;
        let todos = Button::selectable(ex.tipo.is_none(), (RichText::new(ic::SQUARES).color(TEXT_MUTED), RichText::new("Todo").atom_grow(true).atom_align(Align2::LEFT_CENTER), RichText::new(files.len().to_string()).small().color(TEXT_MUTED)));
        if ui.add(todos.min_size(Vec2::new(ui.available_width(), 26.))).clicked() {
            ex.tipo = None;
        }
        for t in TIPOS {
            let n = files.iter().filter(|f| Tipo::de(f) == Some(t)).count();
            let b = Button::selectable(ex.tipo == Some(t), (RichText::new(t.icono()).color(TEXT_MUTED), RichText::new(t.nombre(facil)).atom_grow(true).atom_align(Align2::LEFT_CENTER), RichText::new(n.to_string()).small().color(TEXT_MUTED)));
            if ui.add(b.min_size(Vec2::new(ui.available_width(), 26.))).clicked() {
                ex.tipo = Some(t);
            }
        }
        ui.add_space(8.);
        ui.label(RichText::new("Carpetas").font(sb(12.)).color(TEXT_MUTED));
        let b = Button::selectable(ex.carpeta.is_none(), (RichText::new(ic::FOLDER_OPEN).color(TEXT_MUTED), RichText::new("Todas").atom_grow(true).atom_align(Align2::LEFT_CENTER)));
        if ui.add(b.min_size(Vec2::new(ui.available_width(), 26.))).clicked() {
            ex.carpeta = None;
        }
        for (d, n) in &carpetas {
            let nivel = if d.is_empty() { 0 } else { d.matches('/').count() + 1 };
            let nombre = if d.is_empty() { "(carpeta del juego)".to_string() } else { d.rsplit('/').next().unwrap_or(d).to_string() };
            ui.horizontal(|ui| {
                ui.add_space(nivel as f32 * 12.);
                let b = Button::selectable(ex.carpeta.as_ref() == Some(d), (RichText::new(ic::FOLDERS).color(TEXT_MUTED), RichText::new(nombre).atom_grow(true).atom_align(Align2::LEFT_CENTER), RichText::new(n.to_string()).small().color(TEXT_MUTED)));
                if ui.add(b.min_size(Vec2::new(ui.available_width(), 26.))).on_hover_text(if d.is_empty() { "Archivos sueltos junto al juego" } else { d.as_str() }).clicked() {
                    ex.carpeta = Some(d.clone());
                }
            });
        }
    });
    // Detalles del elegido a la derecha.
    let sel = ex.sel.clone().filter(|s| files.contains(s));
    if let Some(f) = &sel {
        egui::Panel::right("rec_detalle").default_size(300.).size_range(240.0..=440.).frame(Frame::new().fill(PANEL).inner_margin(12)).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                if let Some(a) = detalle(ui, ex, base, f, thumbs.get(f), refs, facil) {
                    act = Some(a);
                }
            });
        });
    }
    // Rejilla de archivos.
    Frame::new().inner_margin(12).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(ic::SEARCH).color(TEXT_MUTED));
            ui.add(TextEdit::singleline(&mut ex.filtro).hint_text("Buscar por nombre").desired_width(220.));
            ui.label(muted(if facil { "Toca un archivo para ver dónde se usa, cambiarle el nombre o borrarlo." } else { "Arrastra archivos a la ventana para importarlos." }));
        });
        ui.add_space(6.);
        let filtro = ex.filtro.to_lowercase();
        let lista: Vec<&String> = files
            .iter()
            .filter(|f| ex.tipo.is_none_or(|t| Tipo::de(f) == Some(t)))
            .filter(|f| ex.carpeta.as_ref().is_none_or(|c| f.rsplit_once('/').map_or("", |x| x.0) == c))
            .filter(|f| filtro.is_empty() || f.to_lowercase().contains(&filtro))
            .collect();
        if lista.is_empty() {
            ui.add_space(40.);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(ic::FOLDER_OPEN).size(32.).color(TEXT_FAINT));
                ui.label(RichText::new(if files.is_empty() { "Todavía no hay archivos" } else { "Nada coincide" }).font(sb(16.)));
                ui.label(muted("Importa imágenes, sonidos o fuentes con el botón, o arrástralos aquí."));
            });
        }
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(8.);
                for f in lista {
                    let (rect, r) = ui.allocate_exact_size(Vec2::new(116., 126.), Sense::click());
                    let elegido = ex.sel.as_deref() == Some(f.as_str());
                    let p = ui.painter();
                    p.rect_filled(rect, 8, if elegido { blend(ACCENT, SURFACE, 0.18) } else if r.hovered() { SURFACE_HOVER } else { SURFACE });
                    p.rect_stroke(rect, 8, Stroke::new(if elegido { 2. } else { 1. }, if elegido { ACCENT } else { BORDER }), egui::StrokeKind::Inside);
                    let pic = Rect::from_min_size(rect.min + Vec2::new(10., 8.), Vec2::new(96., 76.));
                    match (Tipo::de(f), thumbs.get(f)) {
                        (Some(Tipo::Imagen), Some(t)) => {
                            damero(p, pic);
                            let s = t.size_vec2();
                            let k = (pic.width() / s.x).min(pic.height() / s.y).min(8.);
                            p.image(t.id(), Rect::from_center_size(pic.center(), s * k), Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)), Color32::WHITE);
                        }
                        (t, _) => drop(p.text(pic.center(), Align2::CENTER_CENTER, t.map_or(ic::QUESTION, Tipo::icono), FontId::proportional(34.), TEXT_SOFT)),
                    }
                    let nombre = f.rsplit('/').next().unwrap_or(f);
                    let g = p.layout(nombre.to_string(), FontId::proportional(12.5), TEXT, rect.width() - 12.);
                    let g = if g.rows.len() > 2 { p.layout_no_wrap(recortar(nombre, 16), FontId::proportional(12.5), TEXT) } else { g };
                    p.galley(Pos2::new(rect.center().x - g.size().x / 2., rect.top() + 90.), g, TEXT);
                    if usados.get(f.as_str()).copied().unwrap_or(0) == 0 {
                        p.text(rect.right_top() + Vec2::new(-8., 8.), Align2::RIGHT_TOP, ic::CIRCLE_DASHED, FontId::proportional(13.), TEXT_FAINT);
                    }
                    let tip = match usados.get(f.as_str()) {
                        Some(n) => format!("{f}\nSe usa en {n} lugar{}", if *n == 1 { "" } else { "es" }),
                        None => format!("{f}\nNo se usa en el juego"),
                    };
                    if r.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        ex.sel = Some(f.clone());
                        ex.nombre = None;
                    }
                }
            });
        });
    });
    act
}

fn recortar(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { format!("{}…", s.chars().take(n - 1).collect::<String>()) }
}

/// Fondo a cuadros (para ver la transparencia).
pub fn damero(p: &egui::Painter, r: Rect) {
    p.rect_filled(r, 4, Color32::from_gray(52));
    let t = 8.;
    let (cols, rows) = ((r.width() / t).ceil() as i32, (r.height() / t).ceil() as i32);
    for y in 0..rows {
        for x in 0..cols {
            if (x + y) % 2 == 0 {
                let c = Rect::from_min_size(r.min + Vec2::new(x as f32 * t, y as f32 * t), Vec2::splat(t)).intersect(r);
                p.rect_filled(c, 0, Color32::from_gray(64));
            }
        }
    }
}

fn header(ui: &mut Ui, icon: &str, t: &str, detail: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(icon).size(16.).color(ACCENT));
        ui.label(RichText::new(t).font(sb(16.)));
        ui.label(muted(detail));
    });
}

fn detalle(ui: &mut Ui, ex: &mut Explorador, base: &Path, f: &str, tex: Option<&egui::TextureHandle>, refs: &[(String, String)], facil: bool) -> Option<Accion> {
    let mut act = None;
    let tipo = Tipo::de(f);
    header(ui, tipo.map_or(ic::QUESTION, Tipo::icono), f.rsplit('/').next().unwrap_or(f), "");
    ui.label(muted(f));
    ui.add_space(6.);
    if let Some(t) = tex {
        let w = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 180.), Sense::hover());
        damero(ui.painter(), rect);
        let s = t.size_vec2();
        let k = (rect.width() / s.x).min(rect.height() / s.y).min(16.);
        ui.painter().image(t.id(), Rect::from_center_size(rect.center(), s * k), Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)), Color32::WHITE);
    }
    if ex.info.as_ref().is_none_or(|i| i.0 != f) {
        let path = base.join(f);
        let meta = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let mut t = tamano_txt(meta);
        match tipo {
            Some(Tipo::Imagen) => {
                if let Some(x) = tex {
                    t = format!("{} × {} px · {t}", x.size()[0], x.size()[1]);
                }
            }
            Some(Tipo::Sonido) => {
                if let Some(d) = std::fs::read(&path).ok().and_then(|b| wav_duracion(&b)) {
                    t = format!("{d:.2} s · {t}");
                }
            }
            _ => {}
        }
        ex.info = Some((f.to_string(), t));
    }
    if let Some((_, t)) = &ex.info {
        ui.label(t.as_str());
    }
    if tipo == Some(Tipo::Sonido) {
        ui.label(RichText::new(format!("{} El editor no reproduce sonido: pruébalo con Probar (F5).", ic::SPEAKER_SLASH)).small().color(TEXT_MUTED));
    }
    ui.add_space(8.);
    // Renombrar y mover.
    ui.label(RichText::new(if facil { "Nombre" } else { "Nombre y carpeta" }).font(sb(13.)).color(TEXT_MUTED));
    let (dir, nombre) = f.rsplit_once('/').map_or(("", f), |x| x);
    let buf = match &mut ex.nombre {
        Some((k, b)) if k == f => b,
        _ => {
            ex.nombre = Some((f.to_string(), if facil { nombre.to_string() } else { f.to_string() }));
            &mut ex.nombre.as_mut().unwrap().1
        }
    };
    let r = ui.add(TextEdit::singleline(buf).desired_width(f32::INFINITY));
    let nuevo = if facil { if dir.is_empty() { nombre_limpio(buf) } else { format!("{dir}/{}", nombre_limpio(buf)) } } else { buf.trim().trim_start_matches('/').replace('\\', "/") };
    let ext_ok = Tipo::de(&nuevo) == tipo;
    let cambia = nuevo != f && !nuevo.is_empty();
    let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    ui.horizontal(|ui| {
        if ui.add_enabled(cambia && ext_ok, Button::new(it(ic::PENCIL, "Renombrar"))).on_hover_text("Cambia el nombre del archivo y todos los lugares del juego que lo usan").clicked() || (enter && cambia && ext_ok) {
            act = Some(Accion::Renombrar(f.to_string(), nuevo.clone()));
        }
        if !facil {
            let actual = dir.to_string();
            let destinos: Vec<String> = ["img", "snd", "fuentes", "modelos"].iter().map(|s| s.to_string()).filter(|d| *d != actual).collect();
            ui.menu_button(it(ic::FOLDERS, "Mover a"), |ui| {
                menu_spacing(ui);
                for d in destinos {
                    if ui.button(format!("{d}/")).clicked() {
                        act = Some(Accion::Renombrar(f.to_string(), format!("{d}/{nombre}")));
                        ui.close();
                    }
                }
            });
        }
    });
    if cambia && !ext_ok {
        ui.label(RichText::new("Mantén la misma extensión (.png, .wav...).").color(WARN));
    }
    ui.add_space(8.);
    let donde: Vec<&String> = refs.iter().filter(|(r, _)| r == f).map(|(_, d)| d).collect();
    ui.label(RichText::new(if donde.is_empty() { "No se usa".to_string() } else { format!("Se usa en {} lugar{}", donde.len(), if donde.len() == 1 { "" } else { "es" }) }).font(sb(13.)).color(TEXT_MUTED));
    for d in donde.iter().take(30) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(ic::CARET_RIGHT).color(TEXT_FAINT));
            ui.label(d.as_str());
        });
    }
    ui.add_space(10.);
    if ex.borrar.as_deref() == Some(f) {
        Frame::new().fill(blend(DANGER, SURFACE, 0.14)).stroke((1., DANGER)).corner_radius(8).inner_margin(Margin::same(10)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            if donde.is_empty() {
                ui.label("¿Borrar este archivo del disco? No se puede deshacer.");
            } else {
                ui.label(RichText::new(format!("{} En uso: {} lugar{} se quedarán sin este archivo.", ic::WARNING, donde.len(), if donde.len() == 1 { "" } else { "es" })).color(WARN));
            }
            ui.horizontal(|ui| {
                if ui.button(danger_it("Borrar de verdad")).clicked() {
                    act = Some(Accion::Borrar(f.to_string()));
                    ex.borrar = None;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button("Cancelar").clicked() {
                        ex.borrar = None;
                    }
                });
            });
        });
    } else if ui.button(danger_it("Borrar archivo…")).clicked() {
        ex.borrar = Some(f.to_string());
    }
    act
}

/// Imágenes que conviene precargar para las miniaturas (las de la vista actual, hasta un tope).
pub fn a_precargar(ex: &Explorador, files: &[String]) -> Vec<String> {
    let mut v: Vec<String> = files.iter().filter(|f| Tipo::de(f) == Some(Tipo::Imagen))
        .filter(|f| ex.carpeta.as_ref().is_none_or(|c| f.rsplit_once('/').map_or("", |x| x.0) == c))
        .take(240).cloned().collect();
    v.extend(ex.sel.clone().filter(|s| Tipo::de(s) == Some(Tipo::Imagen)));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proyecto() -> Project {
        serde_json::from_str(r#"{"name":"t","width":100,"height":100,"first_scene":"E",
            "settings":{"icon":"img/icono.png"},
            "objects":[{"name":"G","type":"text","text":"hola","font":"fuentes/linda.ttf"}],
            "scenes":[{"name":"E",
              "objects":[
                {"name":"S","type":"sprite","animations":[{"name":"a","frames":["img/heroe1.png","img/heroe2.png"]}]},
                {"name":"F","type":"background","image":"img/cielo.png","w":100,"h":100},
                {"name":"P","type":"particles","image":"img/chispa.png"},
                {"name":"M","type":"tilemap","tileset":"img/baldosas.png","tile":32,"cols":1,"rows":1}],
              "events":[{"conds":[{"op":"start"}],"acts":[{"op":"play_sound","args":["snd/salto.wav","100"]},{"op":"play_music","args":["snd/musica.ogg","50"]}]}]}],
            "functions":[{"name":"f","events":[{"acts":[{"op":"play_sound","args":["snd/salto.wav"]}]}]}]}"#).unwrap()
    }

    #[test]
    fn junta_todas_las_referencias() {
        let p = proyecto();
        let r: Vec<String> = referencias(&p).into_iter().map(|x| x.0).collect();
        for f in ["img/icono.png", "fuentes/linda.ttf", "img/heroe1.png", "img/heroe2.png", "img/cielo.png", "img/chispa.png", "img/baldosas.png", "snd/salto.wav", "snd/musica.ogg"] {
            assert!(r.contains(&f.to_string()), "falta {f}");
        }
        assert_eq!(usos(&p, "snd/salto.wav").len(), 2);
        assert_eq!(usos(&p, "img/").len(), 6);
        assert!(usos(&p, "img/otra.png").is_empty());
    }

    #[test]
    fn renombrar_actualiza_todo() {
        let mut p = proyecto();
        assert_eq!(renombrar_en_proyecto(&mut p, "snd/salto.wav", "snd/brinco.wav"), 2);
        assert_eq!(p.scenes[0].events[0].acts[0].args[0], "snd/brinco.wav");
        assert_eq!(p.functions[0].events[0].acts[0].args[0], "snd/brinco.wav");
        // El volumen (expresión) no se toca aunque se parezca.
        assert_eq!(p.scenes[0].events[0].acts[0].args[1], "100");
        // Carpeta entera: img/ pasa a imagenes/.
        let n = renombrar_en_proyecto(&mut p, "img/", "imagenes/");
        assert_eq!(n, 6);
        assert_eq!(p.settings.icon, "imagenes/icono.png");
        match &p.scenes[0].objects[0].kind {
            Kind::Sprite { animations, .. } => assert_eq!(animations[0].frames, vec!["imagenes/heroe1.png", "imagenes/heroe2.png"]),
            _ => panic!(),
        }
        match &p.scenes[0].objects[2].kind {
            Kind::Particles(d) => assert_eq!(d.image, "imagenes/chispa.png"),
            _ => panic!(),
        }
        assert_eq!(renombrar_en_proyecto(&mut p, "fuentes/linda.ttf", "fuentes/bonita.ttf"), 1);
        assert!(matches!(&p.objects[0].kind, Kind::Text { font, .. } if font == "fuentes/bonita.ttf"));
        assert_eq!(renombrar_en_proyecto(&mut p, "nada.png", "otra.png"), 0);
    }

    #[test]
    fn tipos_y_nombres() {
        assert_eq!(Tipo::de("a/B.PNG"), Some(Tipo::Imagen));
        assert_eq!(Tipo::de("x.otf"), Some(Tipo::Fuente));
        assert_eq!(Tipo::de("x.glb").map(Tipo::carpeta), Some("modelos"));
        assert_eq!(Tipo::de("x.txt"), None);
        assert_eq!(nombre_limpio(" ../mi:foto?.png"), "_mifoto.png");
        let d = std::env::temp_dir().join(format!("ns_rec_{}", std::process::id()));
        std::fs::create_dir_all(d.join("img")).unwrap();
        std::fs::write(d.join("img/a.png"), b"x").unwrap();
        assert_eq!(ruta_libre(&d, "img/a.png"), "img/a_2.png");
        assert_eq!(ruta_libre(&d, "img/b.png"), "img/b.png");
        // Importar copia a la carpeta del tipo sin pisar.
        let src = std::env::temp_dir().join(format!("ns_src_{}.png", std::process::id()));
        std::fs::write(&src, b"y").unwrap();
        let r = importar(&d, &src).unwrap();
        assert!(r.starts_with("img/ns_src_"));
        assert!(d.join(&r).exists());
        // Un archivo que ya está en el juego se usa donde está.
        assert_eq!(importar(&d, &d.join("img/a.png")).unwrap(), "img/a.png");
        mover(&d, "img/a.png", "otros/a.png").unwrap();
        assert!(d.join("otros/a.png").exists() && mover(&d, "otros/a.png", &r).is_err());
        std::fs::remove_dir_all(&d).ok();
        std::fs::remove_file(&src).ok();
    }

    #[test]
    fn duracion_de_wav() {
        // 8000 Hz, mono, 8 bits: byte_rate 8000; 4000 bytes de datos = 0,5 s.
        let mut b = b"RIFF\0\0\0\0WAVEfmt ".to_vec();
        b.extend(16u32.to_le_bytes());
        b.extend([1, 0, 1, 0]);
        b.extend(8000u32.to_le_bytes());
        b.extend(8000u32.to_le_bytes());
        b.extend([1, 0, 8, 0]);
        b.extend(b"data");
        b.extend(4000u32.to_le_bytes());
        assert_eq!(wav_duracion(&b), Some(0.5));
        assert_eq!(wav_duracion(b"hola"), None);
    }
}
