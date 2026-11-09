//! Partículas: recetas listas (fuego, humo, chispas, lluvia, nieve, explosión, magia), editor de todos los
//! parámetros con vista previa animada, y el dibujo de una partícula (lo usan el lienzo y la vista previa del juego).
//!
//! La vista previa simula aquí mismo (no hace falta el motor). La dirección `angle` es el centro del cono
//! y `spread` su apertura total (de `angle - spread/2` a `angle + spread/2`).

use super::iconos as ic;
use super::tema::*;
use eframe::egui::{self, Align2, Color32, DragValue, FontId, Pos2, Rect, RichText, Sense, Slider, Stroke, Ui, Vec2};
use nicostation::model::ParticleDef;

// ---------- recetas ----------

pub fn presets() -> Vec<(&'static str, &'static str, ParticleDef)> {
    let base = ParticleDef::default();
    vec![
        ("Fuego", ic::FIRE, ParticleDef {
            color_start: [255, 210, 70], color_end: [255, 40, 0], alpha_start: 255., alpha_end: 0., size_start: 14., size_end: 4.,
            life: [0.4, 0.9], speed: [40., 90.], angle: -90., spread: 25., gravity: [0., -60.], rate: 60., burst: 0, max: 200, additive: true, ..base.clone()
        }),
        ("Humo", ic::CLOUD, ParticleDef {
            color_start: [130, 130, 135], color_end: [70, 70, 75], alpha_start: 170., alpha_end: 0., size_start: 10., size_end: 34.,
            life: [1.2, 2.2], speed: [20., 50.], angle: -90., spread: 30., gravity: [8., -12.], rate: 18., max: 120, additive: false, ..base.clone()
        }),
        ("Chispas", ic::SPARKLE, ParticleDef {
            color_start: [255, 240, 150], color_end: [255, 110, 0], alpha_start: 255., alpha_end: 0., size_start: 4., size_end: 1.,
            life: [0.3, 0.7], speed: [120., 260.], angle: -90., spread: 160., gravity: [0., 400.], rate: 40., max: 200, additive: true, ..base.clone()
        }),
        ("Lluvia", ic::CLOUD_RAIN, ParticleDef {
            color_start: [150, 190, 255], color_end: [120, 160, 230], alpha_start: 210., alpha_end: 150., size_start: 3., size_end: 3.,
            life: [0.9, 1.3], speed: [300., 420.], angle: 95., spread: 10., gravity: [0., 300.], rate: 120., max: 400, additive: false, area: true, ..base.clone()
        }),
        ("Nieve", ic::SNOWFLAKE, ParticleDef {
            color_start: [255, 255, 255], color_end: [220, 235, 255], alpha_start: 255., alpha_end: 180., size_start: 5., size_end: 4.,
            life: [3., 5.], speed: [20., 50.], angle: 90., spread: 50., gravity: [6., 10.], rate: 15., max: 200, additive: false, area: true, ..base.clone()
        }),
        ("Explosión", ic::LIGHTNING, ParticleDef {
            color_start: [255, 230, 120], color_end: [200, 40, 0], alpha_start: 255., alpha_end: 0., size_start: 16., size_end: 2.,
            life: [0.4, 0.9], speed: [150., 350.], angle: 0., spread: 360., gravity: [0., 0.], rate: 0., burst: 80, max: 200, additive: true, auto_destroy: true,
            ..base.clone()
        }),
        ("Magia", ic::WAND, ParticleDef {
            color_start: [190, 120, 255], color_end: [80, 220, 255], alpha_start: 255., alpha_end: 0., size_start: 7., size_end: 1.,
            life: [0.6, 1.4], speed: [20., 60.], angle: -90., spread: 360., gravity: [0., -30.], rate: 40., max: 200, additive: true, ..base
        }),
    ]
}

// ---------- aspecto ----------

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Color (sin premultiplicar), opacidad 0..1 y tamaño de una partícula en el punto `t` de su vida (0..1).
pub fn mirada(d: &ParticleDef, t: f32) -> ([u8; 3], f32, f32) {
    let t = t.clamp(0., 1.);
    let c = [0, 1, 2].map(|k| lerp(d.color_start[k] as f32, d.color_end[k] as f32, t).round().clamp(0., 255.) as u8);
    let a = (lerp(d.alpha_start, d.alpha_end, t) / 255.).clamp(0., 1.);
    (c, a, lerp(d.size_start, d.size_end, t).max(0.))
}

/// Color listo para pintar: con `additive`, se suma a lo de abajo (alfa 0 premultiplicado en egui).
pub fn color(c: [u8; 3], a: f32, additive: bool) -> Color32 {
    let m = |v: u8| (v as f32 * a).round() as u8;
    if additive { Color32::from_rgba_premultiplied(m(c[0]), m(c[1]), m(c[2]), 0) } else { Color32::from_rgba_unmultiplied(c[0], c[1], c[2], (a * 255.).round() as u8) }
}

/// Dibuja una partícula centrada en `p` con diámetro `lado` (en pantalla).
pub fn dibujar(painter: &egui::Painter, p: Pos2, lado: f32, col: Color32, tex: Option<&egui::TextureHandle>) {
    if lado < 0.3 {
        return;
    }
    match tex {
        Some(t) => drop(painter.image(t.id(), Rect::from_center_size(p, Vec2::splat(lado)), Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)), col)),
        None => drop(painter.circle_filled(p, lado / 2., col)),
    }
}

// ---------- simulación para la vista previa ----------

#[derive(Clone, Copy)]
struct Part {
    p: Vec2,
    v: Vec2,
    age: f32,
    life: f32,
}

/// Simulación local de un emisor (vista previa en el panel de propiedades y en el lienzo).
pub struct Sim {
    parts: Vec<Part>,
    acc: f32,
    rng: u64,
    /// Segundos desde la última ráfaga (las de una sola vez se repiten para que se vean).
    desde: f32,
    empezo: bool,
}

impl Default for Sim {
    fn default() -> Self {
        Sim { parts: vec![], acc: 0., rng: 0x9e37_79b9_7f4a_7c15, desde: 0., empezo: false }
    }
}

impl Sim {
    fn azar(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 40) as f32 / (1u64 << 24) as f32
    }
    fn entre(&mut self, r: [f32; 2]) -> f32 {
        let t = self.azar();
        lerp(r[0].min(r[1]), r[0].max(r[1]), t)
    }
    fn nacer(&mut self, d: &ParticleDef) {
        if self.parts.len() >= d.max.max(1) {
            return;
        }
        let a = (d.angle + (self.azar() - 0.5) * d.spread).to_radians();
        let s = self.entre(d.speed);
        let life = self.entre(d.life).max(0.05);
        // Vista previa: el «área» es un ancho de 160 px (en la escena es el rectángulo del emisor).
        let p = if d.area { Vec2::new((self.azar() - 0.5) * 160., 0.) } else { Vec2::ZERO };
        self.parts.push(Part { p, v: Vec2::new(a.cos(), a.sin()) * s, age: 0., life });
    }
    pub fn reiniciar(&mut self) {
        self.parts.clear();
        self.acc = 0.;
        self.desde = 0.;
        self.empezo = false;
    }
    pub fn cuantas(&self) -> usize {
        self.parts.len()
    }
    /// Avanza `dt` segundos. Con `repetir`, las ráfagas vuelven a salir cuando se apagan.
    pub fn paso(&mut self, d: &ParticleDef, dt: f32, repetir: bool) {
        let dt = dt.clamp(0., 0.1);
        if !self.empezo {
            self.empezo = true;
            (0..d.burst).for_each(|_| self.nacer(d));
        }
        self.desde += dt;
        if d.rate > 0. {
            self.acc += d.rate * dt;
            while self.acc >= 1. {
                self.acc -= 1.;
                self.nacer(d);
            }
        } else if repetir && self.parts.is_empty() && self.desde > 0.6 {
            self.desde = 0.;
            (0..d.burst.max(1)).for_each(|_| self.nacer(d));
        }
        let g = Vec2::new(d.gravity[0], d.gravity[1]);
        for q in &mut self.parts {
            q.v += g * dt;
            q.p += q.v * dt;
            q.age += dt;
        }
        self.parts.retain(|q| q.age < q.life);
    }
    /// Dibuja las partículas alrededor de `origen` (pantalla) con escala `k`.
    pub fn dibujar(&self, painter: &egui::Painter, origen: Pos2, k: f32, d: &ParticleDef, tex: Option<&egui::TextureHandle>) {
        for q in &self.parts {
            let (c, a, s) = mirada(d, q.age / q.life);
            dibujar(painter, origen + q.p * k, s * k, color(c, a, d.additive), tex);
        }
    }
}

// ---------- editor ----------

/// Rueda para elegir la dirección (y ver la apertura del cono). Devuelve true si cambió.
fn rueda(ui: &mut Ui, angle: &mut f32, spread: f32) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(64.), Sense::click_and_drag());
    let p = ui.painter();
    let c = r.center();
    let rad = r.width() / 2. - 3.;
    p.circle_filled(c, rad, SURFACE);
    p.circle_stroke(c, rad, Stroke::new(1., BORDER_STRONG));
    // Cono.
    let n = 24;
    let a0 = (*angle - spread.min(360.) / 2.).to_radians();
    let a1 = (*angle + spread.min(360.) / 2.).to_radians();
    let mut pts = vec![c];
    for k in 0..=n {
        let a = lerp(a0, a1, k as f32 / n as f32);
        pts.push(c + Vec2::new(a.cos(), a.sin()) * (rad - 2.));
    }
    p.add(egui::Shape::convex_polygon(pts, ACCENT.gamma_multiply(0.22), Stroke::NONE));
    let a = angle.to_radians();
    p.line_segment([c, c + Vec2::new(a.cos(), a.sin()) * rad], Stroke::new(2.5, ACCENT));
    p.circle_filled(c, 3., ACCENT);
    let mut ch = false;
    if let Some(m) = resp.interact_pointer_pos().filter(|_| resp.dragged() || resp.clicked()) {
        let d = m - c;
        let mut g = d.y.atan2(d.x).to_degrees();
        if ui.input(|i| i.modifiers.shift) {
            g = (g / 15.).round() * 15.;
        }
        *angle = g.round();
        ch = true;
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
    }
    resp.on_hover_text("Arrastra para cambiar hacia dónde salen (Shift: de a 15°)");
    ch
}

/// Pedidos del editor de partículas que necesitan al editor principal.
pub enum Pedido {
    ElegirImagen,
}

/// Editor completo: recetas, vista previa animada y parámetros. `dt`: segundos desde el último cuadro.
/// Devuelve (cambió, pedido).
pub fn editor(ui: &mut Ui, d: &mut ParticleDef, sim: &mut Sim, dt: f32, tex: Option<&egui::TextureHandle>, imgs: &[String], facil: bool) -> (bool, Option<Pedido>) {
    let antes = d.clone();
    let mut ped = None;
    // Recetas.
    ui.label(RichText::new(if facil { "Elige una receta y después cámbiala a tu gusto" } else { "Recetas" }).font(sb(13.)).color(TEXT_MUTED));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::splat(4.);
        for (n, icon, pd) in presets() {
            let sel = *d == ParticleDef { image: d.image.clone(), ..pd.clone() };
            let lado = if facil { Vec2::new(62., 56.) } else { Vec2::new(54., 48.) };
            if herr_btn(ui, sel, icon, n, &format!("Usar la receta «{n}»"), lado).clicked() {
                *d = ParticleDef { image: d.image.clone(), ..pd };
                sim.reiniciar();
            }
        }
    });
    ui.add_space(4.);
    // Vista previa.
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 170.), Sense::click());
    let p = ui.painter().with_clip_rect(r);
    p.rect_filled(r, 8, Color32::from_rgb(14, 14, 20));
    p.rect_stroke(r, 8, Stroke::new(1., BORDER), egui::StrokeKind::Inside);
    sim.paso(d, dt, true);
    // Escala para que entre lo que recorren: velocidad × vida (+ la gravedad).
    let radial = d.spread >= 300.;
    let vida = ((d.life[0] + d.life[1]) / 2.).max(0.05);
    let g = Vec2::new(d.gravity[0], d.gravity[1]).length();
    let recorre = ((d.speed[0] + d.speed[1]) / 2. * vida + 0.5 * g * vida * vida + d.size_start.max(d.size_end)).max(1.);
    let k = ((if radial { r.height() * 0.45 } else { r.height() * 0.8 }) / recorre).clamp(0.2, 3.);
    let a = d.angle.to_radians();
    let dir = if radial { Vec2::ZERO } else { Vec2::new(a.cos(), a.sin()) };
    let origen = r.center() - Vec2::new(dir.x * r.width().min(r.height()) * 0.4, dir.y * r.height() * 0.4);
    sim.dibujar(&p, origen, k, d, tex);
    p.circle_stroke(origen, 4., Stroke::new(1., TEXT_FAINT));
    p.text(r.left_bottom() + Vec2::new(8., -6.), Align2::LEFT_BOTTOM, format!("{} partículas · escala {:.0} %", sim.cuantas(), k * 100.), FontId::proportional(11.), TEXT_FAINT);
    p.text(r.right_bottom() + Vec2::new(-8., -6.), Align2::RIGHT_BOTTOM, "clic: empezar de nuevo", FontId::proportional(11.), TEXT_FAINT);
    if resp.clicked() {
        sim.reiniciar();
    }
    ui.ctx().request_repaint();
    ui.add_space(4.);
    // Parámetros.
    // Etiquetas de ancho fijo: así las dos rejillas quedan alineadas.
    let ancho = if facil { 140. } else { 104. };
    let lbl = |ui: &mut Ui, kid: &str, exp: &str| {
        ui.allocate_ui_with_layout(Vec2::new(ancho, 20.), egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_min_width(ancho);
            ui.label(muted(if facil { kid } else { exp }));
        });
    };
    egui::Grid::new("partgrid").num_columns(2).spacing([10., 6.]).min_col_width(96.).show(ui, |ui| {
        lbl(ui, "Imagen", "Imagen");
        ui.horizontal(|ui| {
            let shown = if d.image.is_empty() { "(círculo)".to_string() } else { d.image.rsplit('/').next().unwrap_or(&d.image).to_string() };
            egui::ComboBox::from_id_salt("partimg").selected_text(shown).width(130.).show_ui(ui, |ui| {
                if ui.selectable_label(d.image.is_empty(), "(círculo)").clicked() {
                    d.image.clear();
                }
                for f in imgs {
                    if ui.selectable_label(&d.image == f, f.as_str()).clicked() {
                        d.image = f.clone();
                    }
                }
            });
            if ib(ui, ic::FOLDER_OPEN, "Elegir o importar una imagen").clicked() {
                ped = Some(Pedido::ElegirImagen);
            }
        });
        ui.end_row();
        lbl(ui, "Cuántas por segundo", "Por segundo");
        ui.add(Slider::new(&mut d.rate, 0.0..=300.).max_decimals(0)).on_hover_text("0 = solo salen de golpe al aparecer");
        ui.end_row();
        lbl(ui, "De golpe", "Ráfaga");
        ui.add(Slider::new(&mut d.burst, 0..=500)).on_hover_text("Cuántas salen todas juntas al aparecer el emisor");
        ui.end_row();
        lbl(ui, "Color al nacer", "Color inicial");
        ui.horizontal(|ui| {
            ui.color_edit_button_srgb(&mut d.color_start);
            ui.label(muted(if facil { "y al final" } else { "final" }));
            ui.color_edit_button_srgb(&mut d.color_end);
        });
        ui.end_row();
        lbl(ui, "Tamaño", "Tamaño ini/fin");
        ui.horizontal(|ui| {
            ui.add(DragValue::new(&mut d.size_start).range(0.0..=256.).speed(0.2).suffix(" px"));
            ui.label(muted(ic::ARROW_RIGHT));
            ui.add(DragValue::new(&mut d.size_end).range(0.0..=256.).speed(0.2).suffix(" px"));
        });
        ui.end_row();
        lbl(ui, "Se ve", "Opacidad ini/fin");
        ui.horizontal(|ui| {
            ui.add(DragValue::new(&mut d.alpha_start).range(0.0..=255.).speed(1.));
            ui.label(muted(ic::ARROW_RIGHT));
            ui.add(DragValue::new(&mut d.alpha_end).range(0.0..=255.).speed(1.));
        }).response.on_hover_text("255 = se ve entera, 0 = invisible");
        ui.end_row();
        lbl(ui, "Cuánto viven", "Vida (s)");
        ui.horizontal(|ui| {
            ui.add(DragValue::new(&mut d.life[0]).range(0.05..=30.).speed(0.02).suffix(" s"));
            ui.label(muted("a"));
            ui.add(DragValue::new(&mut d.life[1]).range(0.05..=30.).speed(0.02).suffix(" s"));
        });
        ui.end_row();
        lbl(ui, "Velocidad", "Velocidad");
        ui.horizontal(|ui| {
            ui.add(DragValue::new(&mut d.speed[0]).range(0.0..=3000.).speed(1.));
            ui.label(muted("a"));
            ui.add(DragValue::new(&mut d.speed[1]).range(0.0..=3000.).speed(1.));
        }).response.on_hover_text("Pixeles por segundo (mínimo y máximo)");
        ui.end_row();
    });
    // La dirección va fuera de la rejilla: la rueda es más alta que una fila.
    ui.horizontal(|ui| {
        lbl(ui, "Hacia dónde", "Dirección");
        ui.add_space(2.);
        rueda(ui, &mut d.angle, d.spread);
        ui.vertical(|ui| {
            ui.add(DragValue::new(&mut d.angle).range(-360.0..=360.).suffix("°")).on_hover_text("0 = derecha, -90 = arriba, 90 = abajo");
            ui.horizontal(|ui| {
                ui.label(muted(if facil { "abertura" } else { "apertura" }));
                ui.add(DragValue::new(&mut d.spread).range(0.0..=360.).suffix("°")).on_hover_text("Qué tan abierto es el chorro (360 = para todos lados)");
            });
        });
    });
    egui::Grid::new("partgrid2").num_columns(2).spacing([10., 6.]).min_col_width(96.).show(ui, |ui| {
        lbl(ui, "Gravedad", "Gravedad x/y");
        ui.horizontal(|ui| {
            if !facil {
                ui.add(DragValue::new(&mut d.gravity[0]).range(-3000.0..=3000.).speed(2.));
            }
            ui.add(DragValue::new(&mut d.gravity[1]).range(-3000.0..=3000.).speed(2.)).on_hover_text("Positivo: caen. Negativo: suben (humo, fuego)");
            if facil {
                ui.label(muted(if d.gravity[1] > 0. { "caen" } else if d.gravity[1] < 0. { "suben" } else { "flotan" }));
            }
        });
        ui.end_row();
        lbl(ui, "Brillan", "Aditivo");
        ui.checkbox(&mut d.additive, if facil { "Los colores se suman (fuego, magia)" } else { "Mezcla aditiva" });
        ui.end_row();
        lbl(ui, "Dónde nacen", "Área");
        ui.checkbox(&mut d.area, if facil { "En todo el ancho del objeto (lluvia, nieve)" } else { "En todo el rectángulo del emisor" });
        ui.end_row();
        lbl(ui, "Al terminar", "Auto destruir");
        ui.checkbox(&mut d.auto_destroy, if facil { "Se borra solo (explosiones)" } else { "Borrar el emisor cuando se apaga" });
        ui.end_row();
        if !facil {
            lbl(ui, "", "Máximo");
            ui.add(DragValue::new(&mut d.max).range(1..=5000)).on_hover_text("Partículas vivas como máximo");
            ui.end_row();
        }
    });
    if d.life[0] > d.life[1] {
        d.life.swap(0, 1);
    }
    if d.speed[0] > d.speed[1] {
        d.speed.swap(0, 1);
    }
    let ch = *d != antes;
    if ch && (d.burst != antes.burst || d.max != antes.max) {
        sim.reiniciar();
    }
    (ch, ped)
}

/// Ícono pequeño de emisor para el lienzo (cuando no hay vista previa).
pub fn icono_emisor(painter: &egui::Painter, c: Pos2, z: f32) {
    painter.circle_stroke(c, 10. * z.clamp(0.5, 2.), Stroke::new(1., TEXT_FAINT));
    painter.text(c, Align2::CENTER_CENTER, ic::SPARKLE, FontId::proportional((16. * z).clamp(8., 32.)), TEXT_SOFT);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recetas_completas() {
        let p = presets();
        let nombres: Vec<&str> = p.iter().map(|x| x.0).collect();
        assert_eq!(nombres, ["Fuego", "Humo", "Chispas", "Lluvia", "Nieve", "Explosión", "Magia"]);
        // Todas se ven y viven algo; la explosión es de una vez.
        for (n, _, d) in &p {
            assert!(d.life[0] > 0. && d.life[0] <= d.life[1], "{n}");
            assert!(d.rate > 0. || d.burst > 0, "{n}");
        }
        assert!(p[5].2.auto_destroy && p[5].2.rate == 0.);
    }

    #[test]
    fn mirada_interpola() {
        let d = ParticleDef { color_start: [0, 0, 0], color_end: [200, 100, 50], alpha_start: 255., alpha_end: 0., size_start: 10., size_end: 2., ..Default::default() };
        assert_eq!(mirada(&d, 0.), ([0, 0, 0], 1., 10.));
        assert_eq!(mirada(&d, 0.5), ([100, 50, 25], 0.5, 6.));
        assert_eq!(mirada(&d, 2.), ([200, 100, 50], 0., 2.));
        assert_eq!(color([200, 100, 0], 0.5, true), Color32::from_rgba_premultiplied(100, 50, 0, 0));
    }

    #[test]
    fn simulacion() {
        let d = ParticleDef { rate: 60., burst: 10, max: 15, life: [1., 1.], speed: [100., 100.], angle: 0., spread: 0., gravity: [0., 0.], ..Default::default() };
        let mut s = Sim::default();
        s.paso(&d, 0., false);
        assert_eq!(s.cuantas(), 10);
        for _ in 0..30 {
            s.paso(&d, 1. / 60., false);
        }
        // Tope de `max`.
        assert_eq!(s.cuantas(), 15);
        // Van a la derecha a 100 px/s.
        assert!(s.parts.iter().all(|q| q.v == Vec2::new(100., 0.) && q.p.x > 0. && q.p.y.abs() < 1e-3));
        // Sin ritmo: se apaga solo y, con «repetir», vuelve a salir.
        let e = ParticleDef { rate: 0., burst: 5, life: [0.1, 0.1], ..d };
        let mut s = Sim::default();
        s.paso(&e, 0.05, false);
        assert_eq!(s.cuantas(), 5);
        for _ in 0..20 {
            s.paso(&e, 0.05, false);
        }
        assert_eq!(s.cuantas(), 0);
        for _ in 0..20 {
            s.paso(&e, 0.05, true);
        }
        assert!(s.desde < 1.);
    }
}
