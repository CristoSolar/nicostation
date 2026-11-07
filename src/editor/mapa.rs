//! Mapas de baldosas (tilemaps) en el editor: lógica pura de pintado, dibujo en el lienzo,
//! paleta de baldosas y la imagen de baldosas de ejemplo.

use super::dibujo::{inundar, linea};
use super::iconos as ic;
use super::tema::*;
use eframe::egui::{self, Align2, Color32, FontId, Key, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};
use std::path::Path;

pub const TEXTO_NINOS: &str =
    "Un mapa es una cuadrícula que pintas con baldosas, como un piso de cerámicas. Las baldosas chocan, salvo las que marques como decoración.";

/// Ruta de las baldosas de ejemplo, relativa a la carpeta del proyecto.
pub const EJEMPLO: &str = "img/baldosas.png";
/// Baldosas de ejemplo que no chocan: agua y nube.
pub const EJEMPLO_PASS: [i32; 2] = [5, 6];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Herr {
    Pincel,
    Borrador,
    Rellenar,
    Rect,
    Gotero,
}

impl Herr {
    pub const TODAS: [Herr; 5] = [Herr::Pincel, Herr::Borrador, Herr::Rellenar, Herr::Rect, Herr::Gotero];

    pub fn icono(self) -> &'static str {
        match self {
            Herr::Pincel => ic::PAINT_BRUSH,
            Herr::Borrador => ic::ERASER,
            Herr::Rellenar => ic::PAINT_BUCKET,
            Herr::Rect => ic::RECTANGLE,
            Herr::Gotero => ic::EYEDROPPER,
        }
    }

    pub fn nombre(self) -> &'static str {
        match self {
            Herr::Pincel => "Pincel",
            Herr::Borrador => "Borrador",
            Herr::Rellenar => "Rellenar",
            Herr::Rect => "Rectángulo",
            Herr::Gotero => "Cuentagotas",
        }
    }

    pub fn tecla(self) -> Key {
        match self {
            Herr::Pincel => Key::B,
            Herr::Borrador => Key::E,
            Herr::Rellenar => Key::G,
            Herr::Rect => Key::R,
            Herr::Gotero => Key::I,
        }
    }

    fn ayuda(self) -> &'static str {
        match self {
            Herr::Pincel => "Arrastra para pintar con la baldosa elegida (B). Botón derecho: borra.",
            Herr::Borrador => "Arrastra para borrar baldosas (E).",
            Herr::Rellenar => "Rellena toda la zona del mismo tipo de baldosa (G).",
            Herr::Rect => "Arrastra para llenar un rectángulo (R). Con botón derecho lo borra.",
            Herr::Gotero => "Toca una baldosa del mapa para elegirla (I).",
        }
    }
}

/// Trazo de pintado en curso (desde que se aprieta el botón hasta que se suelta).
#[derive(Clone, Copy, Debug)]
pub struct Trazo {
    pub inicio: (i64, i64),
    pub ultima: (i64, i64),
    pub borrar: bool,
}

pub struct Pintor {
    /// Pintar (true) o mover/seleccionar (false) cuando hay un mapa elegido.
    pub on: bool,
    pub herr: Herr,
    pub baldosa: i32,
    pub trazo: Option<Trazo>,
}

impl Default for Pintor {
    fn default() -> Self {
        Pintor { on: true, herr: Herr::Pincel, baldosa: 0, trazo: None }
    }
}

// ---- lógica pura ----

/// Asegura que `data` tenga `cols * rows` celdas (las nuevas, vacías).
fn ajustar(data: &mut Vec<i32>, cols: usize, rows: usize) {
    if data.len() != cols * rows {
        data.resize(cols * rows, -1);
    }
}

fn dentro(cols: usize, rows: usize, c: i64, r: i64) -> bool {
    c >= 0 && r >= 0 && (c as usize) < cols && (r as usize) < rows
}

/// Pone una baldosa; fuera del mapa no hace nada. Devuelve si cambió algo.
pub fn poner(data: &mut Vec<i32>, cols: usize, rows: usize, c: i64, r: i64, v: i32) -> bool {
    if !dentro(cols, rows, c, r) {
        return false;
    }
    ajustar(data, cols, rows);
    let k = r as usize * cols + c as usize;
    let ch = data[k] != v;
    data[k] = v;
    ch
}

/// Pinta una línea de celdas (para que un arrastre rápido no deje huecos).
pub fn poner_linea(data: &mut Vec<i32>, cols: usize, rows: usize, a: (i64, i64), b: (i64, i64), v: i32) -> bool {
    let mut ch = false;
    for (c, r) in linea((a.0 as i32, a.1 as i32), (b.0 as i32, b.1 as i32)) {
        ch |= poner(data, cols, rows, c as i64, r as i64, v);
    }
    ch
}

/// Rellena la zona contigua con la misma baldosa que (c, r).
pub fn rellenar(data: &mut Vec<i32>, cols: usize, rows: usize, c: i64, r: i64, v: i32) -> bool {
    if !dentro(cols, rows, c, r) {
        return false;
    }
    ajustar(data, cols, rows);
    inundar(data, cols, rows, c as usize, r as usize, v)
}

/// Llena el rectángulo entre dos celdas (en cualquier orden), recortado al mapa.
pub fn llenar_rect(data: &mut Vec<i32>, cols: usize, rows: usize, a: (i64, i64), b: (i64, i64), v: i32) -> bool {
    let (c0, c1) = (a.0.min(b.0).max(0), a.0.max(b.0).min(cols as i64 - 1));
    let (r0, r1) = (a.1.min(b.1).max(0), a.1.max(b.1).min(rows as i64 - 1));
    let mut ch = false;
    for r in r0..=r1 {
        for c in c0..=c1 {
            ch |= poner(data, cols, rows, c, r, v);
        }
    }
    ch
}

/// Cambia el tamaño de la grilla conservando cada baldosa en su (columna, fila).
pub fn redimensionar(data: &[i32], oc: usize, or: usize, nc: usize, nr: usize) -> Vec<i32> {
    let mut out = vec![-1; nc * nr];
    for r in 0..or.min(nr) {
        for c in 0..oc.min(nc) {
            out[r * nc + c] = data.get(r * oc + c).copied().unwrap_or(-1);
        }
    }
    out
}

/// Columnas y filas de baldosas en una imagen.
pub fn grilla_tileset(tex: Vec2, tile: f32) -> (usize, usize) {
    let t = tile.max(1.);
    (((tex.x / t).floor() as usize).max(1), ((tex.y / t).floor() as usize).max(1))
}

/// Coordenadas de textura de la baldosa `v`, o None si no está en la imagen.
pub fn uv(v: i32, tile: f32, tex: Vec2) -> Option<Rect> {
    let (tc, tr) = grilla_tileset(tex, tile);
    if v < 0 || v as usize >= tc * tr || tex.x <= 0. || tex.y <= 0. {
        return None;
    }
    let (c, r) = ((v as usize % tc) as f32, (v as usize / tc) as f32);
    Some(Rect::from_min_size(Pos2::new(c * tile / tex.x, r * tile / tex.y), Vec2::new(tile / tex.x, tile / tex.y)))
}

/// Color de reemplazo para la baldosa `v` cuando no hay imagen.
pub fn color_idx(v: i32) -> Color32 {
    const C: [Color32; 8] = [
        Color32::from_rgb(0x59, 0xc0, 0x59),
        Color32::from_rgb(0x9a, 0x63, 0x3c),
        Color32::from_rgb(0x8a, 0x8f, 0x99),
        Color32::from_rgb(0xc0, 0x4a, 0x3a),
        Color32::from_rgb(0xc8, 0x8a, 0x48),
        Color32::from_rgb(0x3d, 0x8b, 0xe0),
        Color32::from_rgb(0xe8, 0xee, 0xf4),
        Color32::from_rgb(0x2f, 0x8a, 0x3c),
    ];
    C[v.rem_euclid(8) as usize]
}

// ---- dibujo en el lienzo ----

/// Dibuja las baldosas de un mapa con origen `o` (pantalla) y celdas de tamaño `cs` (pantalla).
pub fn dibujar(p: &egui::Painter, o: Pos2, cs: Vec2, cols: usize, rows: usize, data: &[i32], tile: f32, tex: Option<&egui::TextureHandle>) {
    let clip = p.clip_rect();
    let (mut tm, mut cm) = (tex.map(|t| egui::Mesh::with_texture(t.id())), egui::Mesh::default());
    // Solo las celdas visibles.
    let c0 = (((clip.left() - o.x) / cs.x).floor().max(0.)) as usize;
    let r0 = (((clip.top() - o.y) / cs.y).floor().max(0.)) as usize;
    let c1 = ((((clip.right() - o.x) / cs.x).ceil().max(0.)) as usize).min(cols);
    let r1 = ((((clip.bottom() - o.y) / cs.y).ceil().max(0.)) as usize).min(rows);
    for r in r0..r1 {
        for c in c0..c1 {
            let v = data.get(r * cols + c).copied().unwrap_or(-1);
            if v < 0 {
                continue;
            }
            let rect = Rect::from_min_size(o + Vec2::new(c as f32 * cs.x, r as f32 * cs.y), cs);
            match (tm.as_mut(), tex.and_then(|t| uv(v, tile, t.size_vec2()))) {
                (Some(m), Some(u)) => m.add_rect_with_uv(rect, u, Color32::WHITE),
                _ => cm.add_colored_rect(rect.shrink(cs.x.min(cs.y) * 0.04), color_idx(v)),
            }
        }
    }
    if let Some(m) = tm {
        p.add(m);
    }
    p.add(cm);
}

/// Líneas de la grilla del mapa mientras se pinta.
pub fn rejilla(p: &egui::Painter, o: Pos2, cs: Vec2, cols: usize, rows: usize) {
    if cs.x.min(cs.y) < 4. {
        return;
    }
    let st = Stroke::new(1., Color32::from_white_alpha(28));
    let (w, h) = (cols as f32 * cs.x, rows as f32 * cs.y);
    for c in 0..=cols {
        let x = o.x + c as f32 * cs.x;
        p.line_segment([Pos2::new(x, o.y), Pos2::new(x, o.y + h)], st);
    }
    for r in 0..=rows {
        let y = o.y + r as f32 * cs.y;
        p.line_segment([Pos2::new(o.x, y), Pos2::new(o.x + w, y)], st);
    }
    p.rect_stroke(Rect::from_min_size(o, Vec2::new(w, h)), 0., Stroke::new(1.5, ACCENT.gamma_multiply(0.7)), egui::StrokeKind::Outside);
}

/// Dibuja una baldosa suelta (paleta y vista previa).
pub fn baldosa(p: &egui::Painter, rect: Rect, v: i32, tile: f32, tex: Option<&egui::TextureHandle>, tint: Color32) {
    match tex.and_then(|t| uv(v, tile, t.size_vec2()).map(|u| (t, u))) {
        Some((t, u)) => drop(p.image(t.id(), rect, u, tint)),
        None => drop(p.rect_filled(rect.shrink(2.), 3, color_idx(v).gamma_multiply(tint.a() as f32 / 255.))),
    }
}

// ---- panel de pintado ----

/// Cantidad de baldosas que hay en la imagen (8 de reemplazo si no hay imagen).
pub fn cantidad(tex: Option<&egui::TextureHandle>, tile: f32) -> usize {
    tex.map_or(8, |t| {
        let (c, r) = grilla_tileset(t.size_vec2(), tile);
        c * r
    })
}

/// Botones de herramientas de pintado.
pub fn herramientas(ui: &mut Ui, p: &mut Pintor) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(6.);
        for h in Herr::TODAS {
            let tip = format!("{} · tecla {:?}", h.ayuda(), h.tecla());
            if herr_btn(ui, p.herr == h, h.icono(), h.nombre(), &tip, Vec2::new(70., 58.)).clicked() {
                p.herr = h;
            }
        }
    });
}

/// Paleta de baldosas: clic elige; las de decoración llevan un fantasma. Devuelve si cambió `pass`.
pub fn paleta(ui: &mut Ui, tex: Option<&egui::TextureHandle>, tile: f32, pass: &mut Vec<i32>, sel: &mut i32) -> bool {
    let n = cantidad(tex, tile);
    let lado = 52.;
    let checker = |pt: &egui::Painter, r: Rect| {
        pt.rect_filled(r, 4, Color32::from_gray(52));
        let q = r.width() / 4.;
        for i in 0..4 {
            for j in 0..4 {
                if (i + j) % 2 == 0 {
                    pt.rect_filled(Rect::from_min_size(r.min + Vec2::new(i as f32 * q, j as f32 * q), Vec2::splat(q)), 0, Color32::from_gray(70));
                }
            }
        }
    };
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(6.);
        for v in 0..n as i32 {
            let (rect, r) = ui.allocate_exact_size(Vec2::splat(lado), Sense::click());
            let pt = ui.painter();
            let s = *sel == v;
            checker(pt, rect.shrink(3.));
            baldosa(pt, rect.shrink(3.), v, tile, tex, Color32::WHITE);
            let st = if s { Stroke::new(3., ACCENT) } else if r.hovered() { Stroke::new(1.5, BORDER_STRONG) } else { Stroke::new(1., BORDER) };
            pt.rect_stroke(rect, 6, st, egui::StrokeKind::Inside);
            if pass.contains(&v) {
                let b = Rect::from_min_size(rect.right_top() + Vec2::new(-20., 2.), Vec2::splat(18.));
                pt.rect_filled(b, 9, Color32::from_black_alpha(190));
                pt.text(b.center(), Align2::CENTER_CENTER, ic::GHOST, FontId::proportional(13.), TEXT);
            }
            let tip = if pass.contains(&v) { format!("Baldosa {v} · decoración: no choca") } else { format!("Baldosa {v} · choca") };
            if r.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                *sel = v;
            }
        }
    });
    ui.add_space(4.);
    let deco = pass.contains(sel);
    let t = if deco { "Decoración: no choca" } else { "Sólida: choca" };
    let icon = if deco { ic::GHOST } else { ic::WALL };
    let r = ui.add(egui::Button::selectable(deco, (RichText::new(icon).size(18.).color(if deco { ACCENT } else { TEXT_MUTED }), RichText::new(t).font(sb(14.))))
        .min_size(Vec2::new(ui.available_width(), 34.)))
        .on_hover_text("Toca para cambiar si la baldosa elegida choca o es solo decoración (pasto, nubes...)");
    if r.clicked() {
        if deco {
            pass.retain(|x| x != sel);
        } else {
            pass.push(*sel);
            pass.sort_unstable();
        }
        return true;
    }
    false
}

// ---- baldosas de ejemplo ----

/// Ruido determinista 0..255 por coordenada (para dar textura).
fn ruido(x: u32, y: u32, s: u32) -> u32 {
    let mut h = x.wrapping_mul(374761393) ^ y.wrapping_mul(668265263) ^ s.wrapping_mul(2246822519);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    (h ^ (h >> 16)) & 255
}

fn sombra(c: [u8; 3], k: f32) -> [u8; 4] {
    let f = |v: u8| (v as f32 * k).clamp(0., 255.) as u8;
    [f(c[0]), f(c[1]), f(c[2]), 255]
}

/// Imagen 4×2 con baldosas: pasto con tierra, tierra, piedra, ladrillo, plataforma de madera, agua, nube, hojas.
pub fn baldosas_ejemplo(t: u32) -> image::RgbaImage {
    let mut img = image::RgbaImage::new(t * 4, t * 2);
    let tierra = [150, 98, 56];
    for k in 0..8u32 {
        let (ox, oy) = ((k % 4) * t, (k / 4) * t);
        for y in 0..t {
            for x in 0..t {
                let n = ruido(x, y, k) as f32 / 255.;
                let (fx, fy) = (x as f32 / t as f32, y as f32 / t as f32);
                let px: [u8; 4] = match k {
                    0 => {
                        let borde = 0.22 + 0.06 * ((fx * 18.).sin() + 1.);
                        if fy < borde {
                            sombra([88, 190, 72], if fy < 0.06 { 1.2 } else { 0.9 + n * 0.2 })
                        } else if fy < borde + 0.05 {
                            sombra([60, 140, 50], 1.)
                        } else {
                            sombra(tierra, if n > 0.88 { 0.7 } else { 0.92 + n * 0.12 })
                        }
                    }
                    1 => sombra(tierra, if n > 0.88 { 0.7 } else if n < 0.06 { 1.2 } else { 0.92 + n * 0.12 }),
                    2 => {
                        let (bx, by) = ((x + if (y / (t / 2)) % 2 == 1 { t / 4 } else { 0 }) % (t / 2), y % (t / 2));
                        if bx == 0 || by == 0 {
                            sombra([90, 94, 104], 1.)
                        } else if bx == 1 || by == 1 {
                            sombra([170, 176, 186], 1.)
                        } else {
                            sombra([132, 138, 150], 0.92 + n * 0.14)
                        }
                    }
                    3 => {
                        let fila = y / (t / 4);
                        let bx = (x + if fila % 2 == 1 { t / 4 } else { 0 }) % (t / 2);
                        if y % (t / 4) == 0 || bx == 0 {
                            sombra([200, 190, 176], 0.9)
                        } else {
                            sombra([178, 66, 52], 0.9 + n * 0.15)
                        }
                    }
                    4 => {
                        let alto = t * 3 / 8;
                        if y >= alto {
                            [0, 0, 0, 0]
                        } else if y == 0 || y == alto - 1 || x % (t / 2) == 0 {
                            sombra([110, 66, 32], 1.)
                        } else if (x % (t / 2) == 3 || x % (t / 2) == t / 2 - 3) && y == alto / 2 {
                            [60, 60, 66, 255]
                        } else {
                            sombra([206, 146, 80], 0.92 + n * 0.1 + if y == 1 { 0.15 } else { 0. })
                        }
                    }
                    5 => {
                        let ola = ((fx * 12.56 + fy * 3.).sin() * 0.5 + 0.5) > 0.86 && (y % (t / 4)) < 2;
                        let mut c = if ola { sombra([150, 210, 255], 1.) } else { sombra([52, 132, 220], 0.9 + fy * 0.1) };
                        c[3] = 210;
                        c
                    }
                    6 => {
                        let d = |cx: f32, cy: f32, r: f32| ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt() < r;
                        let dentro = d(0.32, 0.58, 0.22) || d(0.55, 0.45, 0.27) || d(0.75, 0.62, 0.18) || (fy > 0.58 && fy < 0.8 && fx > 0.18 && fx < 0.88);
                        if !dentro {
                            [0, 0, 0, 0]
                        } else if fy > 0.7 {
                            [214, 226, 240, 255]
                        } else {
                            [250, 252, 255, 255]
                        }
                    }
                    _ => {
                        let hoja = ruido(x / 3, y / 3, 99) > 150;
                        sombra(if hoja { [72, 168, 64] } else { [40, 116, 48] }, 0.9 + n * 0.2)
                    }
                };
                img.put_pixel(ox + x, oy + y, image::Rgba(px));
            }
        }
    }
    img
}

/// Escribe las baldosas de ejemplo en la carpeta del proyecto (si no existen ya) y devuelve su ruta relativa.
pub fn crear_ejemplo(base: &Path) -> Result<String, String> {
    let f = base.join(EJEMPLO);
    if !f.exists() {
        if let Some(d) = f.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        baldosas_ejemplo(32).save(&f).map_err(|e| e.to_string())?;
    }
    Ok(EJEMPLO.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapa(s: &[&str]) -> (Vec<i32>, usize, usize) {
        let rows = s.len();
        let cols = s[0].len();
        let d = s.iter().flat_map(|l| l.chars().map(|c| if c == '.' { -1 } else { c.to_digit(10).unwrap() as i32 })).collect();
        (d, cols, rows)
    }

    #[test]
    fn rellenar_zona_contigua() {
        let (mut d, c, r) = mapa(&["..1.", "..1.", "111.", "...."]);
        assert!(rellenar(&mut d, c, r, 0, 0, 2));
        assert_eq!(d, mapa(&["221.", "221.", "111.", "...."]).0);
        // Rellenar con la misma baldosa no cambia nada; fuera del mapa tampoco.
        assert!(!rellenar(&mut d, c, r, 0, 0, 2));
        assert!(!rellenar(&mut d, c, r, 9, 0, 3));
    }

    #[test]
    fn rellenar_datos_cortos() {
        // `data` vacío (valor por defecto del JSON) = todo vacío.
        let mut d = vec![];
        assert!(rellenar(&mut d, 3, 2, 1, 1, 4));
        assert_eq!(d, vec![4; 6]);
    }

    #[test]
    fn rectangulo_recortado() {
        let (mut d, c, r) = mapa(&["....", "....", "...."]);
        assert!(llenar_rect(&mut d, c, r, (2, 2), (5, -3), 7));
        assert_eq!(d, mapa(&["..77", "..77", "..77"]).0);
        assert!(llenar_rect(&mut d, c, r, (0, 0), (0, 0), -1) == false);
    }

    #[test]
    fn poner_fuera_no_hace_nada() {
        let (mut d, c, r) = mapa(&["..", ".."]);
        assert!(!poner(&mut d, c, r, -1, 0, 1));
        assert!(!poner(&mut d, c, r, 0, 2, 1));
        assert!(poner(&mut d, c, r, 1, 1, 1));
        assert_eq!(d, mapa(&["..", ".1"]).0);
    }

    #[test]
    fn linea_sin_huecos() {
        let mut d = vec![-1; 25];
        poner_linea(&mut d, 5, 5, (0, 0), (4, 2), 1);
        assert_eq!(d.iter().filter(|v| **v == 1).count(), 5);
    }

    #[test]
    fn redimensionar_conserva_celdas() {
        let (d, c, r) = mapa(&["12", "34"]);
        let g = redimensionar(&d, c, r, 3, 3);
        assert_eq!(g, mapa(&["12.", "34.", "..."]).0);
        let p = redimensionar(&g, 3, 3, 1, 2);
        assert_eq!(p, vec![1, 3]);
        // Datos más cortos que la grilla: lo que falta queda vacío.
        assert_eq!(redimensionar(&[5], 2, 2, 2, 1), vec![5, -1]);
    }

    #[test]
    fn coordenadas_de_textura() {
        let t = Vec2::new(128., 64.);
        assert_eq!(grilla_tileset(t, 32.), (4, 2));
        let u = uv(5, 32., t).unwrap();
        assert_eq!(u.min, Pos2::new(0.25, 0.5));
        assert!(uv(8, 32., t).is_none());
        assert!(uv(-1, 32., t).is_none());
    }

    #[test]
    fn ejemplo_tiene_ocho_baldosas() {
        let img = baldosas_ejemplo(32);
        assert_eq!((img.width(), img.height()), (128, 64));
        // La plataforma de madera es transparente abajo; la nube, en la esquina.
        assert_eq!(img.get_pixel(0, 63)[3], 0);
        assert_eq!(img.get_pixel(64 + 1, 32 + 1)[3], 0);
        assert_eq!(img.get_pixel(5, 30)[3], 255);
    }
}
