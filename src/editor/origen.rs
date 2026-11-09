//! Punto de origen y máscara de choque de un sprite: se marcan con clics sobre su imagen.
//! El origen es el punto x, y de la instancia y el centro de giro; la máscara es un polígono convexo.

use super::iconos as ic;
use super::tema::*;
use eframe::egui::{self, Align2, Button, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, Ui, Vec2};

/// ¿El polígono es convexo (y no está aplastado)? Con menos de 3 puntos no es un polígono.
pub fn es_convexo(p: &[[f32; 2]]) -> bool {
    let n = p.len();
    if n < 3 {
        return false;
    }
    let mut signo = 0.;
    let mut area = 0.;
    for i in 0..n {
        let (a, b, c) = (p[i], p[(i + 1) % n], p[(i + 2) % n]);
        let cruz = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]);
        area += a[0] * b[1] - b[0] * a[1];
        if cruz.abs() < 1e-6 {
            continue;
        }
        if signo == 0. {
            signo = cruz.signum();
        } else if cruz.signum() != signo {
            return false;
        }
    }
    // Las vueltas tienen que sumar una sola (una estrella de 5 puntas gira siempre igual pero da dos vueltas).
    let mut giro = 0.;
    for i in 0..n {
        let (a, b, c) = (p[i], p[(i + 1) % n], p[(i + 2) % n]);
        let a1 = (b[1] - a[1]).atan2(b[0] - a[0]);
        let a2 = (c[1] - b[1]).atan2(c[0] - b[0]);
        let mut d = a2 - a1;
        while d > std::f32::consts::PI {
            d -= std::f32::consts::TAU;
        }
        while d < -std::f32::consts::PI {
            d += std::f32::consts::TAU;
        }
        giro += d;
    }
    signo != 0. && area.abs() > 1e-3 && (giro.abs() - std::f32::consts::TAU).abs() < 0.1
}

/// Envolvente convexa (cadena monótona). Sirve para «arreglar» una máscara que no es convexa.
pub fn envolvente(p: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let mut v = p.to_vec();
    v.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    v.dedup();
    if v.len() < 3 {
        return v;
    }
    let cruz = |o: [f32; 2], a: [f32; 2], b: [f32; 2]| (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
    let mut lo: Vec<[f32; 2]> = vec![];
    for &q in &v {
        while lo.len() >= 2 && cruz(lo[lo.len() - 2], lo[lo.len() - 1], q) <= 0. {
            lo.pop();
        }
        lo.push(q);
    }
    let mut hi: Vec<[f32; 2]> = vec![];
    for &q in v.iter().rev() {
        while hi.len() >= 2 && cruz(hi[hi.len() - 2], hi[hi.len() - 1], q) <= 0. {
            hi.pop();
        }
        hi.push(q);
    }
    lo.pop();
    hi.pop();
    lo.extend(hi);
    lo
}

/// Presets de origen: (nombre para niños, nombre experto, ícono, punto normalizado). `None` = esquina (sin origen).
pub const PRESETS: [(&str, &str, Option<[f32; 2]>); 3] = [
    ("Esquina", "Esquina sup. izq.", None),
    ("Centro", "Centro", Some([0.5, 0.5])),
    ("Pies", "Abajo al medio", Some([0.5, 1.])),
];

#[derive(Clone, Copy, PartialEq, Default)]
pub enum Modo {
    #[default]
    Origen,
    Mascara,
}

/// Estado del editor (qué punto se arrastra).
#[derive(Default)]
pub struct Estado {
    pub modo: Modo,
    arrastra: Option<usize>,
}

/// Editor sobre la imagen: `img` es el tamaño de la imagen en píxeles. Devuelve true si cambió algo.
pub fn editor(ui: &mut Ui, st: &mut Estado, tex: Option<&egui::TextureHandle>, img: Vec2, origin: &mut Option<[f32; 2]>, mask: &mut Vec<[f32; 2]>, facil: bool) -> bool {
    let mut ch = false;
    ui.horizontal(|ui| {
        super::seg(ui, |ui| {
            if super::seg_btn(ui, st.modo == Modo::Origen, (RichText::new(ic::CROSSHAIR), RichText::new(if facil { "Centro" } else { "Origen" })))
                .on_hover_text("El punto que marca su posición (x, y) y alrededor del que gira").clicked() {
                st.modo = Modo::Origen;
            }
            if super::seg_btn(ui, st.modo == Modo::Mascara, (RichText::new(ic::POLYGON), RichText::new(if facil { "Borde de choque" } else { "Máscara" })))
                .on_hover_text("La forma que choca con las demás cosas").clicked() {
                st.modo = Modo::Mascara;
            }
        });
    });
    let ayuda = match (st.modo, facil) {
        (Modo::Origen, true) => "Toca la imagen donde quieres su centro: ahí queda su posición y alrededor de ahí gira.",
        (Modo::Origen, false) => "Clic en la imagen para poner el origen (x, y de la instancia y centro de giro).",
        (Modo::Mascara, true) => "Toca para poner puntas del borde. Arrastra una punta para moverla; clic derecho la borra.",
        (Modo::Mascara, false) => "Clic agrega un vértice; arrastrar mueve; clic derecho borra. Debe ser convexa.",
    };
    ui.label(RichText::new(ayuda).small().color(TEXT_MUTED));
    let w = ui.available_width().min(320.);
    let img = img.max(Vec2::splat(1.));
    let k = ((w - 16.) / img.x).min(220. / img.y).max(0.05);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, img.y * k + 16.), Sense::click_and_drag());
    let pic = Rect::from_center_size(rect.center(), img * k);
    let p = ui.painter().with_clip_rect(rect);
    super::recursos::damero(&p, pic);
    match tex {
        Some(t) => drop(p.image(t.id(), pic, Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)), Color32::WHITE)),
        None => drop(p.text(pic.center(), Align2::CENTER_CENTER, ic::IMAGE, FontId::proportional(28.), TEXT_FAINT)),
    }
    p.rect_stroke(pic, 0, Stroke::new(1., BORDER_STRONG), egui::StrokeKind::Outside);
    let a_pant = |q: [f32; 2]| pic.min + Vec2::new(q[0], q[1]) * k;
    let a_img = |s: Pos2| {
        let v = (s - pic.min) / k;
        [v.x.clamp(0., img.x).round(), v.y.clamp(0., img.y).round()]
    };
    // Máscara (siempre se ve; en modo máscara, con sus puntas).
    let poly: Vec<Pos2> = if mask.is_empty() { vec![pic.left_top(), pic.right_top(), pic.right_bottom(), pic.left_bottom()] } else { mask.iter().map(|q| a_pant(*q)).collect() };
    let convexo = mask.is_empty() || es_convexo(mask);
    let col = if convexo { CAT_SCENE } else { DANGER };
    let activo = st.modo == Modo::Mascara;
    if poly.len() >= 3 {
        p.add(egui::Shape::convex_polygon(poly.clone(), col.gamma_multiply(if activo { 0.22 } else { 0.1 }), Stroke::NONE));
    }
    p.add(egui::Shape::closed_line(poly.clone(), Stroke::new(if activo { 2. } else { 1. }, col.gamma_multiply(if activo { 1. } else { 0.6 }))));
    if activo {
        for (i, q) in mask.iter().enumerate() {
            let s = a_pant(*q);
            p.circle_filled(s, 6., if st.arrastra == Some(i) { ACCENT } else { col });
            p.circle_stroke(s, 6., Stroke::new(1.5, BG));
        }
    }
    // Origen.
    let o = origin.map_or(Pos2::ZERO, |o| a_pant(o));
    let oc = if st.modo == Modo::Origen { ACCENT } else { ACCENT.gamma_multiply(0.6) };
    p.line_segment([o - Vec2::new(10., 0.), o + Vec2::new(10., 0.)], Stroke::new(2., oc));
    p.line_segment([o - Vec2::new(0., 10.), o + Vec2::new(0., 10.)], Stroke::new(2., oc));
    p.circle_stroke(o, 5., Stroke::new(2., oc));
    // Interacción.
    let cerca = |m: &[[f32; 2]], s: Pos2| m.iter().position(|q| (a_pant(*q) - s).length() < 9.);
    match st.modo {
        Modo::Origen => {
            if let Some(s) = resp.interact_pointer_pos().filter(|_| resp.clicked() || resp.dragged()) {
                let n = a_img(s);
                if *origin != Some(n) {
                    *origin = Some(n);
                    ch = true;
                }
            }
        }
        Modo::Mascara => {
            if resp.drag_started() {
                st.arrastra = resp.interact_pointer_pos().and_then(|s| cerca(mask, s));
            }
            if let (Some(i), Some(s)) = (st.arrastra, resp.interact_pointer_pos()) {
                if resp.dragged() && i < mask.len() {
                    mask[i] = a_img(s);
                    ch = true;
                }
            }
            if resp.drag_stopped() {
                st.arrastra = None;
            }
            if resp.clicked() {
                if let Some(s) = resp.interact_pointer_pos() {
                    if cerca(mask, s).is_none() {
                        if mask.is_empty() {
                            // Empieza desde el rectángulo completo para poder recortarlo.
                            mask.extend([[0., 0.], [img.x, 0.], [img.x, img.y], [0., img.y]]);
                        }
                        // Se inserta entre los dos vértices del lado más cercano.
                        let q = a_img(s);
                        let at = lado_cercano(mask, q);
                        mask.insert(at, q);
                        ch = true;
                    }
                }
            }
            if resp.secondary_clicked() {
                if let Some(i) = resp.interact_pointer_pos().and_then(|s| cerca(mask, s)) {
                    mask.remove(i);
                    if mask.len() < 3 {
                        mask.clear();
                    }
                    ch = true;
                }
            }
        }
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
    }
    // Botones.
    ui.horizontal_wrapped(|ui| match st.modo {
        Modo::Origen => {
            for (kid, exp, pt) in PRESETS {
                let n = pt.map(|q| [(q[0] * img.x).round(), (q[1] * img.y).round()]);
                if ui.add(Button::selectable(*origin == n || (pt.is_none() && *origin == Some([0., 0.])), if facil { kid } else { exp })).clicked() && *origin != n {
                    *origin = n;
                    ch = true;
                }
            }
            if let Some(o) = origin {
                ui.label(muted(format!("({:.0}, {:.0}) px", o[0], o[1])));
            }
        }
        Modo::Mascara => {
            if ui.add_enabled(!mask.is_empty(), Button::new(it(ic::SQUARE, "Rectángulo completo"))).on_hover_text("Vuelve a la forma de toda la imagen").clicked() {
                mask.clear();
                ch = true;
            }
            if !convexo && ui.button(it(ic::WAND, "Arreglar forma")).on_hover_text("La deja convexa (sin entradas hacia adentro)").clicked() {
                *mask = envolvente(mask);
                ch = true;
            }
            ui.label(muted(if mask.is_empty() { "Choca con toda la imagen".to_string() } else { format!("{} puntas", mask.len()) }));
        }
    });
    if !convexo {
        ui.label(RichText::new(format!("{} {}", ic::WARNING, if facil { "El borde tiene una parte hundida: el juego no lo puede usar así. Aprieta «Arreglar forma»." } else { "La máscara no es convexa: el motor solo admite polígonos convexos." })).color(WARN));
    }
    ch
}

/// Dónde insertar un punto nuevo: después del vértice que empieza el lado más cercano.
fn lado_cercano(m: &[[f32; 2]], q: [f32; 2]) -> usize {
    let n = m.len();
    let d = |a: [f32; 2], b: [f32; 2]| {
        let (ab, aq) = (Vec2::new(b[0] - a[0], b[1] - a[1]), Vec2::new(q[0] - a[0], q[1] - a[1]));
        let t = (aq.dot(ab) / ab.length_sq().max(1e-6)).clamp(0., 1.);
        (aq - ab * t).length()
    };
    (0..n).min_by(|&i, &j| d(m[i], m[(i + 1) % n]).total_cmp(&d(m[j], m[(j + 1) % n]))).map_or(n, |i| i + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convexidad() {
        assert!(es_convexo(&[[0., 0.], [10., 0.], [10., 10.], [0., 10.]]));
        // En el otro sentido también.
        assert!(es_convexo(&[[0., 10.], [10., 10.], [10., 0.], [0., 0.]]));
        // Con una entrada (forma de flecha) no.
        assert!(!es_convexo(&[[0., 0.], [10., 0.], [5., 3.], [10., 10.], [0., 10.]]));
        // Estrella: siempre dobla igual pero da dos vueltas.
        let estrella: Vec<[f32; 2]> = (0..5).map(|k| {
            let a = (k * 2) as f32 * std::f32::consts::TAU / 5.;
            [a.cos() * 10., a.sin() * 10.]
        }).collect();
        assert!(!es_convexo(&estrella));
        // Pocos puntos o puntos alineados.
        assert!(!es_convexo(&[[0., 0.], [1., 1.]]));
        assert!(!es_convexo(&[[0., 0.], [5., 5.], [10., 10.]]));
        // Un punto en medio de un lado no rompe la convexidad.
        assert!(es_convexo(&[[0., 0.], [5., 0.], [10., 0.], [10., 10.], [0., 10.]]));
    }

    #[test]
    fn envolvente_arregla() {
        let h = envolvente(&[[0., 0.], [10., 0.], [5., 3.], [10., 10.], [0., 10.]]);
        assert_eq!(h.len(), 4);
        assert!(es_convexo(&h));
        assert!(!h.contains(&[5., 3.]));
    }

    #[test]
    fn insertar_en_el_lado_cercano() {
        let m = [[0., 0.], [10., 0.], [10., 10.], [0., 10.]];
        assert_eq!(lado_cercano(&m, [5., -1.]), 1);
        assert_eq!(lado_cercano(&m, [11., 5.]), 2);
        assert_eq!(lado_cercano(&m, [-1., 5.]), 4);
    }
}
