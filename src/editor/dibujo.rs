//! «Dibujar»: editor de pixel art sencillo (inspirado en Piskel) para sprites y baldosas.
//! Herramientas, paleta, cuadros de animación con piel de cebolla, vista previa animada y
//! su propio deshacer/rehacer mientras la ventana está abierta.

use super::iconos as ic;
use super::tema::*;
use eframe::egui::{self, Align, Align2, Button, Color32, DragValue, FontId, Frame, Key, Layout, Margin, Modifiers, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};
use std::hash::{Hash, Hasher};
use std::path::Path;

pub type Px = [u8; 4];
pub const NADA: Px = [0, 0, 0, 0];
/// Lado máximo del lienzo.
pub const MAX: usize = 256;
pub const TAMANOS: [usize; 5] = [16, 24, 32, 48, 64];

// ---- lógica pura ----

/// Línea de Bresenham entre dos puntos (incluidos).
pub fn linea(a: (i32, i32), b: (i32, i32)) -> Vec<(i32, i32)> {
    let (mut x, mut y) = a;
    let (dx, dy) = ((b.0 - x).abs(), -(b.1 - y).abs());
    let (sx, sy) = (if x < b.0 { 1 } else { -1 }, if y < b.1 { 1 } else { -1 });
    let mut err = dx + dy;
    let mut out = vec![];
    loop {
        out.push((x, y));
        if (x, y) == b {
            return out;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

fn caja(a: (i32, i32), b: (i32, i32)) -> (i32, i32, i32, i32) {
    (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1))
}

/// Rectángulo entre dos esquinas: borde o relleno.
pub fn rect_pts(a: (i32, i32), b: (i32, i32), lleno: bool) -> Vec<(i32, i32)> {
    let (x0, y0, x1, y1) = caja(a, b);
    let mut out = vec![];
    for y in y0..=y1 {
        for x in x0..=x1 {
            if lleno || x == x0 || x == x1 || y == y0 || y == y1 {
                out.push((x, y));
            }
        }
    }
    out
}

/// Elipse inscrita en la caja entre dos esquinas: borde o relleno.
pub fn elipse_pts(a: (i32, i32), b: (i32, i32), lleno: bool) -> Vec<(i32, i32)> {
    let (x0, y0, x1, y1) = caja(a, b);
    let (w, h) = ((x1 - x0 + 1) as f32, (y1 - y0 + 1) as f32);
    let (rx, ry) = (w / 2., h / 2.);
    let ins = |x: i32, y: i32| {
        if x < x0 || x > x1 || y < y0 || y > y1 {
            return false;
        }
        let (u, v) = ((x - x0) as f32 + 0.5 - rx, (y - y0) as f32 + 0.5 - ry);
        (u / rx).powi(2) + (v / ry).powi(2) <= 1.
    };
    let mut out = vec![];
    for y in y0..=y1 {
        for x in x0..=x1 {
            if ins(x, y) && (lleno || !ins(x - 1, y) || !ins(x + 1, y) || !ins(x, y - 1) || !ins(x, y + 1)) {
                out.push((x, y));
            }
        }
    }
    out
}

/// Relleno por inundación (4 vecinos) desde (x, y) con `v`. Devuelve si cambió algo.
pub fn inundar<T: Copy + PartialEq>(cells: &mut [T], w: usize, h: usize, x: usize, y: usize, v: T) -> bool {
    if x >= w || y >= h || cells.len() < w * h {
        return false;
    }
    let old = cells[y * w + x];
    if old == v {
        return false;
    }
    let mut pila = vec![(x, y)];
    while let Some((x, y)) = pila.pop() {
        if cells[y * w + x] != old {
            continue;
        }
        cells[y * w + x] = v;
        if x > 0 {
            pila.push((x - 1, y));
        }
        if x + 1 < w {
            pila.push((x + 1, y));
        }
        if y > 0 {
            pila.push((x, y - 1));
        }
        if y + 1 < h {
            pila.push((x, y + 1));
        }
    }
    true
}

/// Desplaza los pixeles (lo que sale por un borde entra por el otro).
pub fn mover(px: &[Px], w: usize, h: usize, dx: i32, dy: i32) -> Vec<Px> {
    let mut out = vec![NADA; w * h];
    for y in 0..h {
        for x in 0..w {
            let nx = (x as i32 + dx).rem_euclid(w as i32) as usize;
            let ny = (y as i32 + dy).rem_euclid(h as i32) as usize;
            out[ny * w + nx] = px[y * w + x];
        }
    }
    out
}

/// Cambia el tamaño del lienzo sin escalar (anclado arriba a la izquierda).
pub fn recortar(px: &[Px], w: usize, h: usize, nw: usize, nh: usize) -> Vec<Px> {
    let mut out = vec![NADA; nw * nh];
    for y in 0..h.min(nh) {
        for x in 0..w.min(nw) {
            out[y * nw + x] = px.get(y * w + x).copied().unwrap_or(NADA);
        }
    }
    out
}

/// Pixeles que cubre un pincel de lado `n` centrado en (x, y), con espejo horizontal opcional.
pub fn sello(x: i32, y: i32, n: usize, espejo: Option<usize>) -> Vec<(i32, i32)> {
    let o = (n as i32 - 1) / 2;
    let mut out = vec![];
    for j in 0..n as i32 {
        for i in 0..n as i32 {
            let p = (x - o + i, y - o + j);
            out.push(p);
            if let Some(w) = espejo {
                out.push((w as i32 - 1 - p.0, p.1));
            }
        }
    }
    out
}

/// Nombre apto para archivo.
pub fn limpio(s: &str) -> String {
    let t: String = s.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { super::facil::sin_acento(c) } else { '_' }).collect();
    if t.is_empty() { "dibujo".into() } else { t }
}

/// Ruta relativa editable por el dibujo: PNG dentro de la carpeta del proyecto.
fn sobrescribible(p: &str) -> bool {
    let q = Path::new(p);
    p.to_lowercase().ends_with(".png") && q.is_relative() && !q.components().any(|c| matches!(c, std::path::Component::ParentDir))
}

/// Elige rutas para guardar `n` cuadros: reusa las originales editables y si no busca `img/<prefijo>_<k>.png` libres.
pub fn rutas(base: &Path, prefijo: &str, origenes: &[Option<String>]) -> Vec<String> {
    let mut usadas: Vec<String> = vec![];
    let mut out = vec![];
    let mut k = 1;
    for o in origenes {
        match o.as_ref().filter(|p| sobrescribible(p) && !usadas.contains(p)) {
            Some(p) => out.push(p.clone()),
            None => {
                let p = loop {
                    let p = format!("img/{prefijo}_{k}.png");
                    k += 1;
                    let ocupada = usadas.contains(&p) || origenes.iter().flatten().any(|o| *o == p) || base.join(&p).exists();
                    if !ocupada {
                        break p;
                    }
                };
                out.push(p);
            }
        }
        usadas.push(out.last().unwrap().clone());
    }
    out
}

// ---- estado del editor ----

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Herr {
    Lapiz,
    Goma,
    Balde,
    Gotero,
    Linea,
    Rect,
    Circulo,
    Mover,
}

impl Herr {
    pub const TODAS: [Herr; 8] = [Herr::Lapiz, Herr::Goma, Herr::Balde, Herr::Gotero, Herr::Linea, Herr::Rect, Herr::Circulo, Herr::Mover];

    fn icono(self) -> &'static str {
        match self {
            Herr::Lapiz => ic::PENCIL,
            Herr::Goma => ic::ERASER,
            Herr::Balde => ic::PAINT_BUCKET,
            Herr::Gotero => ic::EYEDROPPER,
            Herr::Linea => ic::LINE,
            Herr::Rect => ic::SQUARE,
            Herr::Circulo => ic::CIRCLE,
            Herr::Mover => ic::MOVE_ALL,
        }
    }

    fn nombre(self) -> &'static str {
        match self {
            Herr::Lapiz => "Lápiz",
            Herr::Goma => "Goma",
            Herr::Balde => "Balde",
            Herr::Gotero => "Gotero",
            Herr::Linea => "Línea",
            Herr::Rect => "Rectángulo",
            Herr::Circulo => "Círculo",
            Herr::Mover => "Mover",
        }
    }

    fn tecla(self) -> Key {
        match self {
            Herr::Lapiz => Key::P,
            Herr::Goma => Key::E,
            Herr::Balde => Key::G,
            Herr::Gotero => Key::I,
            Herr::Linea => Key::L,
            Herr::Rect => Key::R,
            Herr::Circulo => Key::C,
            Herr::Mover => Key::M,
        }
    }

    fn ayuda(self) -> &'static str {
        match self {
            Herr::Lapiz => "Lápiz: pinta pixel por pixel",
            Herr::Goma => "Goma: borra (deja transparente)",
            Herr::Balde => "Balde de pintura: rellena toda una zona del mismo color",
            Herr::Gotero => "Gotero: copia un color del dibujo",
            Herr::Linea => "Línea: arrastra de un punto a otro",
            Herr::Rect => "Rectángulo: arrastra de una esquina a la otra",
            Herr::Circulo => "Círculo: arrastra para hacer un círculo o un óvalo",
            Herr::Mover => "Mover: arrastra para correr todo el dibujo",
        }
    }
}

/// Paleta inicial: transparente + 23 colores alegres.
pub const PALETA: [Px; 24] = [
    NADA,
    [0, 0, 0, 255],
    [95, 87, 79, 255],
    [194, 195, 199, 255],
    [255, 255, 255, 255],
    [255, 204, 170, 255],
    [171, 82, 54, 255],
    [92, 58, 33, 255],
    [126, 37, 83, 255],
    [255, 0, 77, 255],
    [255, 119, 168, 255],
    [255, 163, 0, 255],
    [255, 236, 39, 255],
    [255, 246, 170, 255],
    [0, 135, 81, 255],
    [0, 228, 54, 255],
    [168, 240, 112, 255],
    [0, 181, 160, 255],
    [29, 43, 83, 255],
    [41, 173, 255, 255],
    [131, 208, 255, 255],
    [131, 118, 156, 255],
    [142, 68, 173, 255],
    [224, 160, 112, 255],
];

/// Adónde va el dibujo al guardar.
#[derive(Clone, Debug, PartialEq)]
pub enum Destino {
    /// Animación `anim` del objeto sprite `obj`.
    Anim { obj: String, anim: usize },
    /// Imagen de baldosas del mapa `obj`.
    Baldosas { obj: String },
}

#[derive(Clone)]
pub struct Cuadro {
    pub px: Vec<Px>,
    /// Archivo del que se cargó (se sobrescribe al guardar si es un PNG del proyecto).
    pub origen: Option<String>,
}

#[derive(Clone)]
struct Foto {
    w: usize,
    h: usize,
    cuadros: Vec<Cuadro>,
    cur: usize,
}

struct Trazo {
    inicio: (i32, i32),
    ultima: (i32, i32),
    antes: Vec<Px>,
    borrar: bool,
    hecho: bool,
}

pub enum DibRes {
    Guardar,
    Cancelar,
}

pub struct Dibujo {
    pub destino: Destino,
    pub titulo: String,
    pub w: usize,
    pub h: usize,
    pub cuadros: Vec<Cuadro>,
    pub cur: usize,
    pub herr: Herr,
    pub color: Px,
    pub pincel: usize,
    pub espejo: bool,
    pub cebolla: bool,
    pub lleno: bool,
    pub fps: f32,
    pub play: bool,
    /// Rejilla de baldosas (al dibujar un tileset).
    pub rejilla: Option<usize>,
    pub cambiado: bool,
    pub aviso: Option<String>,
    zoom: Option<f32>,
    recientes: Vec<Px>,
    deshacer: Vec<Foto>,
    rehacer: Vec<Foto>,
    trazo: Option<Trazo>,
    confirmar: bool,
    custom: (usize, usize),
    texs: Vec<Option<(u64, egui::TextureHandle)>>,
    damero: Option<egui::TextureHandle>,
    hover: Option<(i32, i32)>,
}

/// Carga imágenes del proyecto. Las que no existen quedan en blanco; si alguna supera 256 px, error.
pub fn cargar(base: &Path, rutas: &[String]) -> Result<(usize, usize, Vec<Cuadro>), String> {
    let mut imgs = vec![];
    for r in rutas {
        let img = image::open(base.join(r)).ok().map(|i| i.to_rgba8());
        if let Some(i) = &img {
            if i.width() as usize > MAX || i.height() as usize > MAX {
                return Err(format!("«{r}» mide {}×{} pixeles: el editor de dibujo acepta hasta {MAX}×{MAX}.", i.width(), i.height()));
            }
        }
        imgs.push((r.clone(), img));
    }
    let (w, h) = imgs.iter().find_map(|(_, i)| i.as_ref().map(|i| (i.width() as usize, i.height() as usize))).unwrap_or((32, 32));
    let cuadros = imgs
        .into_iter()
        .map(|(r, i)| {
            let px = match i {
                Some(i) => {
                    let v: Vec<Px> = i.pixels().map(|p| p.0).collect();
                    recortar(&v, i.width() as usize, i.height() as usize, w, h)
                }
                None => vec![NADA; w * h],
            };
            Cuadro { px, origen: Some(r) }
        })
        .collect();
    Ok((w, h, cuadros))
}

impl Dibujo {
    pub fn new(destino: Destino, titulo: String, w: usize, h: usize, mut cuadros: Vec<Cuadro>, fps: f32) -> Self {
        if cuadros.is_empty() {
            cuadros.push(Cuadro { px: vec![NADA; w * h], origen: None });
        }
        Dibujo {
            destino, titulo, w, h, cuadros, cur: 0, herr: Herr::Lapiz, color: PALETA[1], pincel: 1, espejo: false, cebolla: false, lleno: false,
            fps: fps.clamp(0.5, 60.), play: true, rejilla: None, cambiado: false, aviso: None, zoom: None, recientes: vec![], deshacer: vec![],
            rehacer: vec![], trazo: None, confirmar: false, custom: (w, h), texs: vec![], damero: None, hover: None,
        }
    }

    fn foto(&self) -> Foto {
        Foto { w: self.w, h: self.h, cuadros: self.cuadros.clone(), cur: self.cur }
    }

    fn poner_foto(&mut self, f: Foto) {
        (self.w, self.h, self.cuadros) = (f.w, f.h, f.cuadros);
        self.cur = f.cur.min(self.cuadros.len() - 1);
        self.custom = (self.w, self.h);
    }

    fn guardar_foto(&mut self) {
        self.deshacer.push(self.foto());
        if self.deshacer.len() > 100 {
            self.deshacer.remove(0);
        }
    }

    /// Guarda el estado actual para poder deshacer el próximo cambio.
    pub fn paso(&mut self) {
        self.guardar_foto();
        self.rehacer.clear();
        self.cambiado = true;
    }

    pub fn deshacer(&mut self) {
        if let Some(f) = self.deshacer.pop() {
            self.rehacer.push(self.foto());
            self.poner_foto(f);
            self.cambiado = true;
        }
    }

    pub fn rehacer(&mut self) {
        if let Some(f) = self.rehacer.pop() {
            self.deshacer.push(self.foto());
            self.poner_foto(f);
        }
    }

    fn get(&self, x: i32, y: i32) -> Option<Px> {
        (x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h).then(|| self.cuadros[self.cur].px[y as usize * self.w + x as usize])
    }

    fn set_pts(&mut self, pts: &[(i32, i32)], c: Px) {
        let espejo = self.espejo.then_some(self.w);
        let (w, h, n) = (self.w, self.h, self.pincel);
        let px = &mut self.cuadros[self.cur].px;
        for &(x, y) in pts {
            for (x, y) in sello(x, y, n, espejo) {
                if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                    px[y as usize * w + x as usize] = c;
                }
            }
        }
    }

    pub fn cambiar_tamano(&mut self, nw: usize, nh: usize) {
        let (nw, nh) = (nw.clamp(1, MAX), nh.clamp(1, MAX));
        if (nw, nh) == (self.w, self.h) {
            return;
        }
        self.paso();
        for c in &mut self.cuadros {
            c.px = recortar(&c.px, self.w, self.h, nw, nh);
        }
        (self.w, self.h, self.custom) = (nw, nh, (nw, nh));
        self.zoom = None;
    }

    fn usar_color(&mut self, c: Px) {
        self.recientes.retain(|x| *x != c);
        self.recientes.insert(0, c);
        self.recientes.truncate(8);
    }

    // ---- interacción con el lienzo ----

    fn empezar(&mut self, p: (i32, i32), borrar: bool) {
        let antes = self.cuadros[self.cur].px.clone();
        // El paso de deshacer se confirma al soltar, si el trazo cambió algo.
        self.guardar_foto();
        let c = if borrar || self.herr == Herr::Goma { NADA } else { self.color };
        let mut hecho = false;
        match self.herr {
            Herr::Lapiz | Herr::Goma => self.set_pts(&[p], c),
            Herr::Balde => {
                if let (true, Some(_)) = (p.0 >= 0 && p.1 >= 0, self.get(p.0, p.1)) {
                    let (w, h) = (self.w, self.h);
                    inundar(&mut self.cuadros[self.cur].px, w, h, p.0 as usize, p.1 as usize, c);
                    if self.espejo {
                        let mx = w as i32 - 1 - p.0;
                        inundar(&mut self.cuadros[self.cur].px, w, h, mx as usize, p.1 as usize, c);
                    }
                }
                hecho = true;
            }
            Herr::Gotero => {
                if let Some(c) = self.get(p.0, p.1) {
                    self.color = c;
                    self.herr = Herr::Lapiz;
                }
                hecho = true;
            }
            _ => {}
        }
        self.trazo = Some(Trazo { inicio: p, ultima: p, antes, borrar, hecho });
    }

    fn seguir(&mut self, p: (i32, i32)) {
        let Some(t) = self.trazo.as_ref() else { return };
        if t.hecho || (p == t.ultima && !matches!(self.herr, Herr::Linea | Herr::Rect | Herr::Circulo | Herr::Mover)) {
            return;
        }
        let (inicio, ultima, borrar) = (t.inicio, t.ultima, t.borrar);
        let c = if borrar || self.herr == Herr::Goma { NADA } else { self.color };
        match self.herr {
            Herr::Lapiz | Herr::Goma => self.set_pts(&linea(ultima, p), c),
            Herr::Linea | Herr::Rect | Herr::Circulo => {
                self.cuadros[self.cur].px = self.trazo.as_ref().unwrap().antes.clone();
                let pts = match self.herr {
                    Herr::Linea => linea(inicio, p),
                    Herr::Rect => rect_pts(inicio, p, self.lleno),
                    _ => elipse_pts(inicio, p, self.lleno),
                };
                self.set_pts(&pts, c);
            }
            Herr::Mover => {
                let antes = &self.trazo.as_ref().unwrap().antes;
                self.cuadros[self.cur].px = mover(antes, self.w, self.h, p.0 - inicio.0, p.1 - inicio.1);
            }
            _ => {}
        }
        self.trazo.as_mut().unwrap().ultima = p;
    }

    fn terminar(&mut self) {
        let Some(t) = self.trazo.take() else { return };
        if t.antes == self.cuadros[self.cur].px {
            // Nada cambió: no hace falta un paso de deshacer.
            self.deshacer.pop();
            return;
        }
        self.rehacer.clear();
        self.cambiado = true;
        if !t.borrar && self.herr != Herr::Goma && self.herr != Herr::Mover {
            self.usar_color(self.color);
        }
    }

    // ---- cuadros ----

    pub fn agregar_cuadro(&mut self, copiar: bool) {
        self.paso();
        let px = if copiar { self.cuadros[self.cur].px.clone() } else { vec![NADA; self.w * self.h] };
        self.cuadros.insert(self.cur + 1, Cuadro { px, origen: None });
        self.cur += 1;
    }

    fn borrar_cuadro(&mut self) {
        if self.cuadros.len() > 1 {
            self.paso();
            self.cuadros.remove(self.cur);
            self.cur = self.cur.min(self.cuadros.len() - 1);
        }
    }

    fn mover_cuadro(&mut self, d: i32) {
        let j = self.cur as i32 + d;
        if j >= 0 && (j as usize) < self.cuadros.len() {
            self.paso();
            self.cuadros.swap(self.cur, j as usize);
            self.cur = j as usize;
        }
    }

    /// Escribe los cuadros como PNG y devuelve sus rutas relativas.
    pub fn guardar(&mut self, base: &Path, prefijo: &str) -> Result<Vec<String>, String> {
        let origenes: Vec<Option<String>> = self.cuadros.iter().map(|c| c.origen.clone()).collect();
        let rs = rutas(base, prefijo, &origenes);
        for (c, r) in self.cuadros.iter_mut().zip(&rs) {
            let f = base.join(r);
            if let Some(d) = f.parent() {
                std::fs::create_dir_all(d).map_err(|e| format!("No se pudo crear {}: {e}", d.display()))?;
            }
            let img = image::RgbaImage::from_raw(self.w as u32, self.h as u32, c.px.as_flattened().to_vec()).ok_or("Imagen inválida")?;
            img.save(&f).map_err(|e| format!("No se pudo guardar {r}: {e}"))?;
            c.origen = Some(r.clone());
        }
        self.cambiado = false;
        Ok(rs)
    }

    // ---- texturas ----

    fn tex(&mut self, ctx: &egui::Context, i: usize) -> egui::TextureHandle {
        self.texs.resize(self.cuadros.len(), None);
        let mut hs = std::collections::hash_map::DefaultHasher::new();
        (self.w, self.h).hash(&mut hs);
        self.cuadros[i].px.hash(&mut hs);
        let k = hs.finish();
        if let Some((h, t)) = &self.texs[i] {
            if *h == k {
                return t.clone();
            }
        }
        let img = egui::ColorImage::from_rgba_unmultiplied([self.w, self.h], self.cuadros[i].px.as_flattened());
        let t = match self.texs[i].take() {
            Some((_, mut t)) => {
                t.set(img, egui::TextureOptions::NEAREST);
                t
            }
            None => ctx.load_texture(format!("dibujo{i}"), img, egui::TextureOptions::NEAREST),
        };
        self.texs[i] = Some((k, t.clone()));
        t
    }

    fn damero(&mut self, ctx: &egui::Context) -> egui::TextureHandle {
        self.damero
            .get_or_insert_with(|| {
                let (a, b) = (Color32::from_gray(58), Color32::from_gray(82));
                let img = egui::ColorImage::new([2, 2], vec![a, b, b, a]);
                ctx.load_texture("damero", img, egui::TextureOptions::NEAREST_REPEAT)
            })
            .clone()
    }
}

fn c32(p: Px) -> Color32 {
    Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3])
}

/// Fondo cuadriculado de transparencia: cada cuadro del damero = `k` pixeles del dibujo.
fn pintar_damero(p: &egui::Painter, t: &egui::TextureHandle, r: Rect, w: usize, h: usize, k: f32) {
    p.image(t.id(), r, Rect::from_min_max(Pos2::ZERO, Pos2::new(w as f32 / (2. * k), h as f32 / (2. * k))), Color32::WHITE);
}

fn muestra(ui: &mut Ui, c: Px, sel: bool, lado: f32, tip: &str) -> bool {
    let (rect, r) = ui.allocate_exact_size(Vec2::splat(lado), Sense::click());
    let p = ui.painter();
    if c[3] < 255 {
        let q = rect.width() / 2.;
        p.rect_filled(rect, 4, Color32::from_gray(82));
        p.rect_filled(Rect::from_min_size(rect.min, Vec2::splat(q)), 0, Color32::from_gray(58));
        p.rect_filled(Rect::from_min_size(rect.min + Vec2::splat(q), Vec2::splat(q)), 0, Color32::from_gray(58));
    }
    p.rect_filled(rect, 4, c32(c));
    if c[3] == 0 {
        p.line_segment([rect.left_bottom() + Vec2::new(3., -3.), rect.right_top() + Vec2::new(-3., 3.)], Stroke::new(2., DANGER));
    }
    let st = if sel { Stroke::new(3., ACCENT) } else if r.hovered() { Stroke::new(1.5, TEXT_MUTED) } else { Stroke::new(1., BORDER_STRONG) };
    p.rect_stroke(rect, 4, st, egui::StrokeKind::Inside);
    r.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn toggle(ui: &mut Ui, on: &mut bool, icon: &str, label: &str, tip: &str) {
    if herr_btn(ui, *on, icon, label, tip, Vec2::new(68., 56.)).clicked() {
        *on = !*on;
    }
}

/// Ventana del editor de dibujo. Devuelve Some al guardar o al cerrar.
pub fn ventana(ctx: &egui::Context, d: &mut Dibujo, kid: bool) -> Option<DibRes> {
    let mut res = None;
    // Atajos (sin texto con foco).
    if !ctx.egui_wants_keyboard_input() && d.trazo.is_none() {
        let (cmd, cs) = (Modifiers::COMMAND, Modifiers::COMMAND | Modifiers::SHIFT);
        let (redo, undo, herr) = ctx.input_mut(|i| {
            let redo = i.consume_key(cs, Key::Z) || i.consume_key(cmd, Key::Y);
            let undo = i.consume_key(cmd, Key::Z);
            let herr = Herr::TODAS.into_iter().find(|h| i.consume_key(Modifiers::NONE, h.tecla()));
            (redo, undo, herr)
        });
        if redo {
            d.rehacer();
        }
        if undo {
            d.deshacer();
        }
        if let Some(h) = herr {
            d.herr = h;
        }
    }
    let scr = ctx.content_rect();
    let w = (scr.width() - 60.).clamp(700., 1240.);
    let alto = (scr.height() - 330.).clamp(240., 620.);
    let frame = Frame::window(&ctx.global_style()).inner_margin(18).corner_radius(12);
    let now = ctx.input(|i| i.time);
    let r = egui::Modal::new(egui::Id::unique("dibujar")).frame(frame).show(ctx, |ui| {
        ui.set_width(w);
        ui.horizontal(|ui| {
            ui.label(RichText::new(ic::PALETTE).size(22.).color(ACCENT));
            ui.label(RichText::new("Dibujar").font(sb(22.)));
            ui.label(muted(&d.titulo));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ib(ui, ic::X, "Cerrar (Esc)").clicked() {
                    res = Some(DibRes::Cancelar);
                }
                ui.add_space(8.);
                if ib_on(ui, !d.rehacer.is_empty(), ic::REDO, "Rehacer (Ctrl+Shift+Z)").clicked() {
                    d.rehacer();
                }
                if ib_on(ui, !d.deshacer.is_empty(), ic::UNDO, "Deshacer (Ctrl+Z)").clicked() {
                    d.deshacer();
                }
            });
        });
        if kid {
            ui.label(RichText::new("Elige una herramienta y un color, y dibuja en la cuadrícula: cada cuadrito es un pixel.").color(TEXT_SOFT));
        }
        ui.add_space(6.);
        ui.horizontal_top(|ui| {
            // ---- herramientas ----
            ui.vertical(|ui| {
                ui.set_width(146.);
                ui.label(RichText::new("Herramientas").font(sb(13.)).color(TEXT_MUTED));
                egui::Grid::new("dib_herr").spacing([6., 6.]).show(ui, |ui| {
                    for (k, h) in Herr::TODAS.into_iter().enumerate() {
                        let tip = format!("{} (tecla {:?})", h.ayuda(), h.tecla());
                        if herr_btn(ui, d.herr == h, h.icono(), h.nombre(), &tip, Vec2::new(68., 56.)).clicked() {
                            d.herr = h;
                        }
                        if k % 2 == 1 {
                            ui.end_row();
                        }
                    }
                });
                ui.add_space(4.);
                ui.label(RichText::new("Grosor").font(sb(13.)).color(TEXT_MUTED));
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.;
                    for n in 1..=3 {
                        let b = Button::selectable(d.pincel == n, RichText::new(format!("{n}")).font(sb(15.))).min_size(Vec2::new(44., 30.));
                        if ui.add(b).on_hover_text(format!("Pincel de {n}×{n} pixeles")).clicked() {
                            d.pincel = n;
                        }
                    }
                });
                ui.add_space(4.);
                egui::Grid::new("dib_tog").spacing([6., 6.]).show(ui, |ui| {
                    toggle(ui, &mut d.lleno, ic::SQUARE_HALF, "Relleno", "Rectángulos y círculos rellenos (si no, solo el borde)");
                    toggle(ui, &mut d.espejo, ic::FLIP_H, "Espejo", "Espejo: lo que dibujas de un lado aparece del otro");
                    ui.end_row();
                    if d.cuadros.len() > 1 || matches!(d.destino, Destino::Anim { .. }) {
                        toggle(ui, &mut d.cebolla, ic::GHOST, "Cebolla", "Piel de cebolla: muestra el cuadro anterior clarito, para animar");
                    }
                });
            });
            // ---- lienzo ----
            let cw = ui.available_width() - 222.;
            ui.vertical(|ui| {
                ui.set_width(cw);
                lienzo_barra(ui, d);
                lienzo(ui, d, Vec2::new(cw, alto));
            });
            // ---- colores y vista previa ----
            ui.vertical(|ui| {
                ui.set_width(210.);
                colores(ui, d);
                ui.add_space(8.);
                vista_previa(ui, d, now);
            });
        });
        if matches!(d.destino, Destino::Anim { .. }) {
            ui.add_space(6.);
            tira(ui, d);
        }
        ui.add_space(8.);
        if let Some(a) = &d.aviso {
            ui.horizontal(|ui| {
                ui.label(RichText::new(ic::WARNING).color(WARN));
                ui.label(RichText::new(a).color(TEXT));
            });
        }
        ui.horizontal(|ui| {
            if d.confirmar {
                ui.label(RichText::new(ic::WARNING).size(18.).color(WARN));
                ui.label(RichText::new("¿Cerrar sin guardar? Se pierde lo que dibujaste.").font(sb(15.)));
                if ui.button(danger_it("Sí, descartar")).clicked() {
                    res = Some(DibRes::Cancelar);
                    d.cambiado = false;
                }
                if ui.button("No, seguir dibujando").clicked() {
                    d.confirmar = false;
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().button_padding = Vec2::new(14., 8.);
                if primary(ui, (on_accent(ic::FLOPPY).size(17.), on_accent("Guardar dibujo").font(sb(15.)))).on_hover_text("Guarda los PNG en la carpeta img/ del proyecto").clicked() {
                    res = Some(DibRes::Guardar);
                }
                if !d.confirmar && ui.button(RichText::new("Cancelar").font(sb(15.))).clicked() {
                    res = Some(DibRes::Cancelar);
                }
            });
        });
    });
    if res.is_none() && r.should_close() {
        res = Some(DibRes::Cancelar);
    }
    if matches!(res, Some(DibRes::Cancelar)) && d.cambiado {
        d.confirmar = true;
        return None;
    }
    res
}

fn lienzo_barra(ui: &mut Ui, d: &mut Dibujo) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.;
        ui.label(muted("Tamaño"));
        for n in TAMANOS {
            let sel = d.w == n && d.h == n;
            if ui.add(Button::selectable(sel, format!("{n}")).min_size(Vec2::new(34., 26.))).on_hover_text(format!("{n}×{n} pixeles")).clicked() {
                d.cambiar_tamano(n, n);
            }
        }
        ui.add_space(6.);
        ui.add(DragValue::new(&mut d.custom.0).range(1..=MAX)).on_hover_text("Ancho");
        ui.label(muted("×"));
        ui.add(DragValue::new(&mut d.custom.1).range(1..=MAX)).on_hover_text("Alto");
        if d.custom != (d.w, d.h) && ui.button("Cambiar").clicked() {
            d.cambiar_tamano(d.custom.0, d.custom.1);
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let z = d.zoom.unwrap_or(0.);
            if ib(ui, ic::FIT, "Ajustar").clicked() {
                d.zoom = None;
            }
            if ib(ui, ic::ZOOM_IN, "Acercar (rueda del mouse)").clicked() {
                d.zoom = Some((z.max(1.) * 1.5).min(64.));
            }
            if ib(ui, ic::ZOOM_OUT, "Alejar").clicked() {
                d.zoom = Some((z.max(1.) / 1.5).max(1.));
            }
            let c = d.hover.map_or("x -  y -".into(), |(x, y)| format!("x {x}  y {y}"));
            ui.label(RichText::new(c).monospace().color(TEXT_MUTED)).on_hover_text("Pixel bajo el mouse");
        });
    });
}

fn lienzo(ui: &mut Ui, d: &mut Dibujo, size: Vec2) {
    let ctx = ui.ctx().clone();
    let (resp, painter) = ui.allocate_painter(size, Sense::click_and_drag());
    let r = resp.rect;
    painter.rect_filled(r, 8, BG);
    let (w, h) = (d.w as f32, d.h as f32);
    let auto = ((r.width() - 16.) / w).min((r.height() - 16.) / h).max(1.);
    let z = d.zoom.unwrap_or(auto.floor().max(1.));
    if resp.hovered() {
        let dy = ui.input(|i| i.smooth_scroll_delta.y);
        if dy != 0. {
            d.zoom = Some((z * (dy * 0.004).exp()).clamp(1., 64.));
        }
    }
    let img = Rect::from_center_size(r.center(), Vec2::new(w, h) * z);
    let painter = painter.with_clip_rect(r.shrink(1.));
    let dam = d.damero(&ctx);
    pintar_damero(&painter, &dam, img, d.w, d.h, if d.w.max(d.h) > 64 { (d.w.max(d.h) / 32) as f32 } else { 1. });
    let full = Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.));
    if d.cebolla && d.cur > 0 {
        let t = d.tex(&ctx, d.cur - 1);
        painter.image(t.id(), img, full, Color32::from_white_alpha(90));
    }
    let t = d.tex(&ctx, d.cur);
    painter.image(t.id(), img, full, Color32::WHITE);
    if z >= 6. {
        let st = Stroke::new(1., Color32::from_white_alpha(18));
        for x in 1..d.w {
            let sx = img.left() + x as f32 * z;
            painter.line_segment([Pos2::new(sx, img.top()), Pos2::new(sx, img.bottom())], st);
        }
        for y in 1..d.h {
            let sy = img.top() + y as f32 * z;
            painter.line_segment([Pos2::new(img.left(), sy), Pos2::new(img.right(), sy)], st);
        }
    }
    if let Some(g) = d.rejilla.filter(|g| *g > 0) {
        let st = Stroke::new(1.5, ACCENT.gamma_multiply(0.75));
        for x in (g..d.w).step_by(g) {
            let sx = img.left() + x as f32 * z;
            painter.line_segment([Pos2::new(sx, img.top()), Pos2::new(sx, img.bottom())], st);
        }
        for y in (g..d.h).step_by(g) {
            let sy = img.top() + y as f32 * z;
            painter.line_segment([Pos2::new(img.left(), sy), Pos2::new(img.right(), sy)], st);
        }
    }
    if d.espejo {
        painter.line_segment([Pos2::new(img.center().x, img.top() - 6.), Pos2::new(img.center().x, img.bottom() + 6.)], Stroke::new(1.5, CAT_MOV));
    }
    painter.rect_stroke(img, 0., Stroke::new(1., BORDER_STRONG), egui::StrokeKind::Outside);

    // Pixel bajo el mouse.
    let pos = ui.input(|i| i.pointer.latest_pos());
    let pix = |p: Pos2| (((p.x - img.left()) / z).floor() as i32, ((p.y - img.top()) / z).floor() as i32);
    d.hover = pos.filter(|p| resp.hovered() || resp.is_pointer_button_down_on() && r.contains(*p)).map(pix).filter(|&(x, y)| x >= 0 && y >= 0 && x < d.w as i32 && y < d.h as i32);
    if let Some((x, y)) = d.hover {
        let n = if matches!(d.herr, Herr::Balde | Herr::Gotero | Herr::Mover) { 1 } else { d.pincel };
        for (px, py) in sello(x, y, n, d.espejo.then_some(d.w)) {
            let pr = Rect::from_min_size(img.min + Vec2::new(px as f32, py as f32) * z, Vec2::splat(z));
            if d.herr == Herr::Lapiz && d.trazo.is_none() {
                painter.rect_filled(pr, 0., c32(d.color).gamma_multiply(0.6));
            }
            painter.rect_stroke(pr, 0., Stroke::new(1., Color32::WHITE), egui::StrokeKind::Inside);
        }
        let cur = match d.herr {
            Herr::Mover => egui::CursorIcon::Move,
            _ => egui::CursorIcon::Crosshair,
        };
        ctx.set_cursor_icon(cur);
    }

    // Trazos: del apretón al soltar (fuera del dibujo también, para líneas y figuras).
    let down = resp.is_pointer_button_down_on();
    let (prim, sec) = ui.input(|i| (i.pointer.primary_down(), i.pointer.secondary_down()));
    match (down && (prim || sec), d.trazo.is_some(), pos) {
        (true, false, Some(p)) if d.hover.is_some() => d.empezar(pix(p), sec && !prim),
        (true, true, Some(p)) => d.seguir(pix(p)),
        (false, true, _) => d.terminar(),
        _ => {}
    }
}

fn colores(ui: &mut Ui, d: &mut Dibujo) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::new(44., 44.), Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect, 8, Color32::from_gray(70));
        p.rect_filled(rect, 8, c32(d.color));
        p.rect_stroke(rect, 8, Stroke::new(2., BORDER_STRONG), egui::StrokeKind::Inside);
        ui.vertical(|ui| {
            ui.label(RichText::new("Color").font(sb(14.)));
            let mut c = c32(d.color);
            let r = egui::color_picker::color_edit_button_srgba(ui, &mut c, egui::color_picker::Alpha::OnlyBlend).on_hover_text("Todos los colores: elige uno nuevo");
            if r.changed() {
                let u = c.to_srgba_unmultiplied();
                d.color = u;
            }
        });
    });
    ui.add_space(4.);
    egui::Grid::new("paleta").spacing([4., 4.]).show(ui, |ui| {
        for (k, c) in PALETA.iter().enumerate() {
            let tip = if c[3] == 0 { "Transparente (como la goma)".to_string() } else { format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]) };
            if muestra(ui, *c, d.color == *c, 30., &tip) {
                d.color = *c;
            }
            if k % 6 == 5 {
                ui.end_row();
            }
        }
    });
    if !d.recientes.is_empty() {
        ui.add_space(4.);
        ui.label(RichText::new("Usados hace poco").small().color(TEXT_MUTED));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(4.);
            for c in d.recientes.clone() {
                if muestra(ui, c, d.color == c, 22., "Usado hace poco") {
                    d.color = c;
                }
            }
        });
    }
}

fn vista_previa(ui: &mut Ui, d: &mut Dibujo, now: f64) {
    let n = d.cuadros.len();
    ui.label(RichText::new("Vista previa").font(sb(13.)).color(TEXT_MUTED));
    let k = if d.play && n > 1 { ((now * d.fps as f64) as usize) % n } else { d.cur };
    let lado = 110.;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(210., lado), Sense::hover());
    let s = (lado / d.w.max(d.h) as f32).min(8.).max(0.25);
    let img = Rect::from_center_size(rect.center(), Vec2::new(d.w as f32, d.h as f32) * s);
    let ctx = ui.ctx().clone();
    let dam = d.damero(&ctx);
    ui.painter().rect_filled(rect, 8, BG);
    pintar_damero(ui.painter(), &dam, img, d.w, d.h, (d.w.max(d.h) / 8).max(1) as f32);
    let t = d.tex(&ctx, k);
    ui.painter().image(t.id(), img, Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)), Color32::WHITE);
    if n > 1 {
        ui.horizontal(|ui| {
            let (icon, tip) = if d.play { (ic::PAUSE, "Pausar") } else { (ic::PLAY, "Reproducir") };
            if ui.add(Button::new(RichText::new(icon).size(16.)).min_size(Vec2::new(34., 28.))).on_hover_text(tip).clicked() {
                d.play = !d.play;
            }
            ui.add(DragValue::new(&mut d.fps).range(0.5..=60.).speed(0.2).suffix(" cuadros/s")).on_hover_text("Velocidad de la animación");
        });
        if d.play {
            ui.ctx().request_repaint();
        }
    }
}

fn tira(ui: &mut Ui, d: &mut Dibujo) {
    let ctx = ui.ctx().clone();
    Frame::new().fill(BG).corner_radius(8).inner_margin(Margin::same(8)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(RichText::new(ic::FILM_STRIP).size(18.).color(TEXT_MUTED)).on_hover_text("Cuadros de la animación");
            let bw = 330.;
            egui::ScrollArea::horizontal().max_width(ui.available_width() - bw).id_salt("tira").show(ui, |ui| {
                ui.horizontal(|ui| {
                    for i in 0..d.cuadros.len() {
                        let (rect, r) = ui.allocate_exact_size(Vec2::new(64., 64.), Sense::click());
                        let s = (56. / d.w.max(d.h) as f32).max(0.1);
                        let img = Rect::from_center_size(rect.center() - Vec2::new(0., 4.), Vec2::new(d.w as f32, d.h as f32) * s);
                        let dam = d.damero(&ctx);
                        let t = d.tex(&ctx, i);
                        let p = ui.painter();
                        p.rect_filled(rect, 6, SURFACE);
                        pintar_damero(p, &dam, img, d.w, d.h, (d.w.max(d.h) / 8).max(1) as f32);
                        p.image(t.id(), img, Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)), Color32::WHITE);
                        let sel = i == d.cur;
                        p.rect_stroke(rect, 6, if sel { Stroke::new(3., ACCENT) } else { Stroke::new(1., BORDER_STRONG) }, egui::StrokeKind::Inside);
                        let nr = Rect::from_min_size(rect.left_bottom() + Vec2::new(3., -17.), Vec2::new(18., 14.));
                        p.rect_filled(nr, 4, if sel { ACCENT } else { Color32::from_black_alpha(170) });
                        p.text(nr.center(), Align2::CENTER_CENTER, format!("{}", i + 1), FontId::proportional(11.), if sel { ON_ACCENT } else { TEXT });
                        if r.on_hover_text(format!("Cuadro {}", i + 1)).clicked() {
                            d.cur = i;
                        }
                    }
                });
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 4.;
                let n = d.cuadros.len();
                if ib_on(ui, d.cur + 1 < n, ic::ARROW_RIGHT, "Mover el cuadro a la derecha").clicked() {
                    d.mover_cuadro(1);
                }
                if ib_on(ui, d.cur > 0, ic::ARROW_LEFT, "Mover el cuadro a la izquierda").clicked() {
                    d.mover_cuadro(-1);
                }
                if ib_on(ui, n > 1, ic::TRASH, "Borrar este cuadro").clicked() {
                    d.borrar_cuadro();
                }
                if ui.button(it(ic::COPY, "Duplicar")).on_hover_text("Copia este cuadro para cambiarlo un poquito").clicked() {
                    d.agregar_cuadro(true);
                }
                if ui.button(it(ic::PLUS, "Cuadro")).on_hover_text("Agrega un cuadro vacío después de este").clicked() {
                    d.agregar_cuadro(false);
                }
            });
        });
    });
}

/// Dibujo de prueba (opción `--abrir dibujo`): un monstruito verde que salta, en 3 cuadros de 16×16.
pub fn demo(d: &mut Dibujo) {
    let (verde, oscuro, blanco, negro, rosa) = ([0, 228, 54, 255], [0, 135, 81, 255], [255, 255, 255, 255], [0, 0, 0, 255], [255, 119, 168, 255]);
    d.w = 16;
    d.h = 16;
    d.custom = (16, 16);
    d.cuadros.clear();
    d.pincel = 1;
    d.espejo = false;
    // (caja del cuerpo, alto de los ojos)
    for (k, (a, b, oy)) in [((2, 7), (13, 15), 10), ((1, 9), (14, 15), 11), ((3, 2), (12, 12), 5)].into_iter().enumerate() {
        d.cuadros.push(Cuadro { px: vec![NADA; 256], origen: None });
        d.cur = k;
        d.set_pts(&elipse_pts(a, b, true), verde);
        d.set_pts(&elipse_pts(a, b, false), oscuro);
        d.set_pts(&[(5, oy), (10, oy)], blanco);
        d.set_pts(&[(5, oy + 1), (10, oy + 1)], negro);
        d.set_pts(&[(4, oy + 2), (11, oy + 2)], rosa);
    }
    d.cur = 0;
    d.deshacer.clear();
    d.rehacer.clear();
    d.cambiado = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linea_recta_y_diagonal() {
        assert_eq!(linea((0, 0), (3, 0)), vec![(0, 0), (1, 0), (2, 0), (3, 0)]);
        assert_eq!(linea((2, 2), (0, 0)), vec![(2, 2), (1, 1), (0, 0)]);
        let l = linea((0, 0), (5, 2));
        assert_eq!(l.len(), 6);
        assert_eq!((l[0], l[5]), ((0, 0), (5, 2)));
        // Sin saltos: cada punto toca al anterior.
        assert!(l.windows(2).all(|w| (w[0].0 - w[1].0).abs() <= 1 && (w[0].1 - w[1].1).abs() <= 1));
        assert_eq!(linea((4, 4), (4, 4)), vec![(4, 4)]);
    }

    #[test]
    fn rectangulo_borde_y_relleno() {
        assert_eq!(rect_pts((0, 0), (2, 2), true).len(), 9);
        let b = rect_pts((2, 2), (0, 0), false);
        assert_eq!(b.len(), 8);
        assert!(!b.contains(&(1, 1)));
    }

    #[test]
    fn elipse_simetrica() {
        let e = elipse_pts((0, 0), (6, 6), true);
        assert!(e.contains(&(3, 3)) && e.contains(&(3, 0)) && e.contains(&(0, 3)));
        assert!(!e.contains(&(0, 0)));
        let b = elipse_pts((0, 0), (6, 6), false);
        assert!(!b.contains(&(3, 3)) && b.contains(&(6, 3)));
        assert_eq!(elipse_pts((1, 1), (1, 1), false), vec![(1, 1)]);
    }

    #[test]
    fn balde_contiguo() {
        let a = [1, 0, 0, 0];
        let b = [2, 0, 0, 255];
        // Columna de b en x=1 separa dos zonas de a.
        let mut px = vec![a, b, a, a, b, a, a, b, a];
        assert!(inundar(&mut px, 3, 3, 0, 0, NADA));
        assert_eq!(px, vec![NADA, b, a, NADA, b, a, NADA, b, a]);
        assert!(!inundar(&mut px, 3, 3, 0, 0, NADA));
    }

    #[test]
    fn mover_da_la_vuelta() {
        let (a, b) = ([1, 1, 1, 255], NADA);
        let px = vec![a, b, b, b];
        assert_eq!(mover(&px, 2, 2, 1, 1), vec![b, b, b, a]);
        assert_eq!(mover(&px, 2, 2, -1, 0), vec![b, a, b, b]);
    }

    #[test]
    fn recortar_ancla_arriba_izquierda() {
        let a = [9, 9, 9, 255];
        let px = vec![a; 4];
        let g = recortar(&px, 2, 2, 3, 1);
        assert_eq!(g, vec![a, a, NADA]);
    }

    #[test]
    fn sello_y_espejo() {
        assert_eq!(sello(5, 5, 1, None), vec![(5, 5)]);
        assert_eq!(sello(0, 0, 3, None).len(), 9);
        assert_eq!(sello(1, 0, 1, Some(8)), vec![(1, 0), (6, 0)]);
    }

    #[test]
    fn deshacer_trazo() {
        let mut d = Dibujo::new(Destino::Baldosas { obj: "m".into() }, String::new(), 4, 4, vec![], 10.);
        d.empezar((0, 0), false);
        d.seguir((3, 0));
        d.terminar();
        assert_eq!(d.cuadros[0].px[..4].iter().filter(|p| p[3] == 255).count(), 4);
        d.deshacer();
        assert!(d.cuadros[0].px.iter().all(|p| *p == NADA));
        d.rehacer();
        assert_eq!(d.cuadros[0].px[3], PALETA[1]);
        // Un clic que no cambia nada no deja paso de deshacer.
        let n = d.deshacer.len();
        d.herr = Herr::Gotero;
        d.empezar((0, 0), false);
        d.terminar();
        assert_eq!(d.deshacer.len(), n);
    }

    #[test]
    fn rutas_reusan_y_no_pisan() {
        let base = std::env::temp_dir().join(format!("nico_rutas_{}", std::process::id()));
        std::fs::create_dir_all(base.join("img")).unwrap();
        std::fs::write(base.join("img/heroe_quieto_1.png"), b"x").unwrap();
        let o = vec![Some("img/a.png".to_string()), None, Some("img/a.png".to_string()), Some("../fuera.png".to_string()), Some("img/b.jpg".to_string())];
        let r = rutas(&base, "heroe_quieto", &o);
        assert_eq!(r, vec!["img/a.png", "img/heroe_quieto_2.png", "img/heroe_quieto_3.png", "img/heroe_quieto_4.png", "img/heroe_quieto_5.png"]);
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn guardar_y_cargar() {
        let base = std::env::temp_dir().join(format!("nico_guardar_{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        let mut d = Dibujo::new(Destino::Anim { obj: "o".into(), anim: 0 }, String::new(), 4, 4, vec![], 10.);
        demo(&mut d);
        let rs = d.guardar(&base, "o_a").unwrap();
        assert_eq!(rs, vec!["img/o_a_1.png", "img/o_a_2.png", "img/o_a_3.png"]);
        let (w, h, c) = cargar(&base, &rs).unwrap();
        assert_eq!((w, h, c.len()), (16, 16, 3));
        assert!(c.iter().zip(&d.cuadros).all(|(a, b)| a.px == b.px));
        // Guardar de nuevo sobrescribe los mismos archivos.
        assert_eq!(d.guardar(&base, "o_a").unwrap(), rs);
        // Una imagen grande se rechaza con un mensaje.
        image::RgbaImage::new(300, 10).save(base.join("grande.png")).unwrap();
        assert!(cargar(&base, &["grande.png".into()]).err().unwrap().contains("300×10"));
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn nombre_limpio() {
        assert_eq!(limpio("Héroe rojo"), "Heroe_rojo");
    }
}
