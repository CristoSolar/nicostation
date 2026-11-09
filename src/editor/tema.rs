//! Sistema de diseño del editor: colores, radios, tipografía y estilo de egui.

use eframe::egui::{self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Shadow, Stroke, TextStyle, Vec2};
use std::sync::Arc;

pub const BG: Color32 = Color32::from_rgb(0x11, 0x11, 0x13);
pub const PANEL: Color32 = Color32::from_rgb(0x18, 0x18, 0x1b);
pub const SURFACE: Color32 = Color32::from_rgb(0x1f, 0x1f, 0x23);
pub const SURFACE_HOVER: Color32 = Color32::from_rgb(0x27, 0x27, 0x2c);
pub const BORDER: Color32 = Color32::from_rgb(0x2e, 0x2e, 0x34);
pub const BORDER_STRONG: Color32 = Color32::from_rgb(0x3f, 0x3f, 0x46);
pub const TEXT: Color32 = Color32::from_rgb(0xed, 0xed, 0xf0);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0xa1, 0xa1, 0xaa);
pub const TEXT_FAINT: Color32 = Color32::from_rgb(0x71, 0x71, 0x7a);
/// Naranja de la marca NicoStation (variante modo oscuro, ver assets/marca/).
pub const ACCENT: Color32 = Color32::from_rgb(0xff, 0x7a, 0x27);
pub const ON_ACCENT: Color32 = Color32::from_rgb(0x1a, 0x0e, 0x08);
/// Acento al ~16 % (premultiplicado) para filas seleccionadas.
pub const ACCENT_SOFT: Color32 = Color32::from_rgba_premultiplied(41, 20, 11, 41);
pub const DANGER: Color32 = Color32::from_rgb(0xf0, 0x52, 0x52);
pub const OK: Color32 = Color32::from_rgb(0x4a, 0xde, 0x80);
pub const WARN: Color32 = Color32::from_rgb(0xfb, 0xbf, 0x24);
// Hoja de eventos
pub const COND_BG: Color32 = Color32::from_rgb(0x1c, 0x1c, 0x20);
pub const ACT_BG: Color32 = SURFACE;

// Modo fácil: un color por categoría de bloque (como Scratch).
pub const CAT_MOV: Color32 = Color32::from_rgb(0x4c, 0x97, 0xff);
pub const CAT_INPUT: Color32 = Color32::from_rgb(0xff, 0xbf, 0x00);
pub const CAT_OBJ: Color32 = Color32::from_rgb(0xff, 0x8c, 0x1a);
pub const CAT_VAR: Color32 = Color32::from_rgb(0xff, 0x66, 0x19);
pub const CAT_AUDIO: Color32 = Color32::from_rgb(0xcf, 0x63, 0xcf);
pub const CAT_TIME: Color32 = Color32::from_rgb(0xff, 0xab, 0x19);
pub const CAT_SCENE: Color32 = Color32::from_rgb(0x59, 0xc0, 0x59);
pub const CAT_FUNC: Color32 = Color32::from_rgb(0xff, 0x66, 0x80);
/// Texto oscuro para poner encima de un color de categoría lleno (contraste ≥ 5:1 con todos).
pub const ON_CAT: Color32 = Color32::from_rgb(0x16, 0x16, 0x1a);
/// Fichas de argumento dentro de los bloques.
pub const CHIP: Color32 = Color32::from_rgb(0xf4, 0xf4, 0xf6);
pub const CHIP_TEXT: Color32 = Color32::from_rgb(0x18, 0x18, 0x1b);
/// Ficha "NO" (texto blanco, contraste 5.3:1).
pub const CHIP_NO: Color32 = Color32::from_rgb(0xc9, 0x2a, 0x2a);
/// Texto secundario dentro de bloques teñidos (contraste ≥ 7:1).
pub const TEXT_SOFT: Color32 = Color32::from_rgb(0xc8, 0xc8, 0xd0);

/// Mezcla `c` sobre `base` con opacidad `t` (resultado opaco).
pub fn blend(c: Color32, base: Color32, t: f32) -> Color32 {
    let m = |a: u8, b: u8| (a as f32 * t + b as f32 * (1. - t)).round() as u8;
    Color32::from_rgb(m(c.r(), base.r()), m(c.g(), base.g()), m(c.b(), base.b()))
}

/// Fondo de bloque: color de categoría al 18 % sobre la superficie.
pub fn tint(c: Color32) -> Color32 {
    blend(c, SURFACE, 0.18)
}

pub const R: u8 = 6;
pub const R_WIN: u8 = 8;

pub fn semibold() -> FontFamily {
    FontFamily::Name("semibold".into())
}

pub fn sb(size: f32) -> FontId {
    FontId::new(size, semibold())
}

fn fuentes(ctx: &egui::Context) {
    let mut f = FontDefinitions::default();
    let mut add = |n: &str, b: &'static [u8]| drop(f.font_data.insert(n.into(), Arc::new(FontData::from_static(b))));
    add("geist", include_bytes!("../../assets/fonts/Geist-Regular.ttf"));
    add("geist-sb", include_bytes!("../../assets/fonts/Geist-SemiBold.ttf"));
    add("phosphor", include_bytes!("../../assets/fonts/Phosphor.ttf"));
    // Phosphor justo después de la fuente principal: los íconos de emoji de egui también usan el área privada.
    let prop = f.families.entry(FontFamily::Proportional).or_default();
    prop.splice(0..0, ["geist".to_string(), "phosphor".to_string()]);
    let fallback: Vec<String> = prop[1..].to_vec();
    let mono = f.families.entry(FontFamily::Monospace).or_default();
    mono.insert(1.min(mono.len()), "phosphor".into());
    f.families.insert(semibold(), std::iter::once("geist-sb".to_string()).chain(fallback).collect());
    ctx.set_fonts(f);
}

/// Tamaños de letra: el modo fácil usa texto de cuerpo de 15 px como mínimo.
pub fn tamanos(ctx: &egui::Context, facil: bool) {
    let d = if facil { 1. } else { 0. };
    ctx.all_styles_mut(|s| {
        use TextStyle::*;
        s.text_styles = [
            (Small, FontId::proportional(12. + d)),
            (Body, FontId::proportional(14. + d)),
            (Button, FontId::proportional(14. + d)),
            (Heading, sb(16. + d * 2.)),
            (Monospace, FontId::monospace(13. + d)),
        ]
        .into();
    });
}

/// Texto en mayúsculas con letras separadas (etiquetas CUANDO / ENTONCES).
pub fn caps(t: &str, size: f32, c: Color32) -> egui::WidgetText {
    let mut j = egui::text::LayoutJob::default();
    j.append(t, 0., egui::TextFormat { font_id: sb(size), color: c, extra_letter_spacing: 1.2, ..Default::default() });
    j.into()
}

pub fn aplicar(ctx: &egui::Context) {
    fuentes(ctx);
    ctx.set_theme(egui::Theme::Dark);
    ctx.all_styles_mut(|s| {
        use TextStyle::*;
        s.text_styles = [
            (Small, FontId::proportional(12.)),
            (Body, FontId::proportional(14.)),
            (Button, FontId::proportional(14.)),
            (Heading, sb(16.)),
            (Monospace, FontId::monospace(13.)),
        ]
        .into();
        let sp = &mut s.spacing;
        sp.item_spacing = Vec2::new(8., 6.);
        sp.button_padding = Vec2::new(10., 5.);
        sp.window_margin = Margin::same(12);
        sp.menu_margin = Margin::same(6);
        sp.indent = 16.;
        sp.interact_size = Vec2::new(24., 26.);
        sp.icon_width = 16.;
        sp.icon_width_inner = 9.;
        sp.icon_spacing = 6.;
        sp.combo_width = 120.;
        sp.text_edit_width = 160.;
        sp.scroll = egui::style::ScrollStyle::thin();
        s.interaction.tooltip_delay = 0.35;

        let v = &mut s.visuals;
        *v = egui::Visuals::dark();
        let cr = CornerRadius::same(R);
        let w = |fill: Color32, stroke: Color32, fg: Color32| egui::style::WidgetVisuals {
            bg_fill: fill, weak_bg_fill: fill, bg_stroke: Stroke::new(1., stroke), corner_radius: cr, fg_stroke: Stroke::new(1., fg), expansion: 0.,
        };
        v.widgets.noninteractive = w(PANEL, BORDER, TEXT);
        v.widgets.inactive = w(SURFACE, BORDER, TEXT);
        v.widgets.hovered = w(SURFACE_HOVER, BORDER_STRONG, TEXT);
        // Activo = presionado o con foco de teclado: anillo de acento.
        v.widgets.active = w(SURFACE_HOVER, ACCENT, TEXT);
        v.widgets.open = w(SURFACE_HOVER, BORDER_STRONG, TEXT);
        v.selection.bg_fill = ACCENT_SOFT;
        v.selection.stroke = Stroke::new(1., ACCENT);
        v.text_cursor.stroke = Stroke::new(2., ACCENT);
        v.hyperlink_color = ACCENT;
        v.faint_bg_color = SURFACE;
        v.extreme_bg_color = BG;
        v.text_edit_bg_color = Some(SURFACE);
        v.code_bg_color = SURFACE;
        v.warn_fg_color = WARN;
        v.error_fg_color = DANGER;
        v.weak_text_color = Some(TEXT_FAINT);
        v.window_corner_radius = CornerRadius::same(R_WIN);
        v.menu_corner_radius = cr;
        v.window_fill = PANEL;
        v.window_stroke = Stroke::new(1., BORDER_STRONG);
        v.window_shadow = Shadow { offset: [0, 8], blur: 24, spread: 0, color: Color32::from_black_alpha(140) };
        v.popup_shadow = Shadow { offset: [0, 4], blur: 12, spread: 0, color: Color32::from_black_alpha(120) };
        v.panel_fill = PANEL;
        v.window_highlight_topmost = false;
        v.striped = false;
        v.indent_has_left_vline = true;
        v.handle_shape = egui::style::HandleShape::Circle;
        v.interact_cursor = Some(egui::CursorIcon::PointingHand);
        v.disabled_alpha = 0.45;
    });
}

// ---- componentes ----

use eframe::egui::{Button, Response, RichText, Ui};

/// Ícono + etiqueta (ícono atenuado).
pub fn it(icon: &str, t: &str) -> (RichText, RichText) {
    (RichText::new(icon).color(TEXT_MUTED), RichText::new(t))
}

/// Acción destructiva: siempre ícono + etiqueta en rojo.
pub fn danger_it(t: &str) -> (RichText, RichText) {
    (RichText::new(super::iconos::TRASH).color(DANGER), RichText::new(t).color(DANGER))
}

/// Botón solo-ícono cuadrado, sin marco hasta pasar el mouse; siempre con etiqueta emergente.
pub fn ib(ui: &mut Ui, icon: &str, tip: &str) -> Response {
    ib_on(ui, true, icon, tip)
}

pub fn ib_on(ui: &mut Ui, enabled: bool, icon: &str, tip: &str) -> Response {
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = Vec2::new(5., 5.);
        ui.add_enabled(enabled, Button::new(RichText::new(icon).size(16.)).frame_when_inactive(false).min_size(Vec2::splat(26.)))
    })
    .inner
    .on_hover_text(tip)
    .on_disabled_hover_text(tip)
}

/// Botón primario relleno de acento (uno por vista).
pub fn primary<'a>(ui: &mut Ui, atoms: impl egui::IntoAtoms<'a>) -> Response {
    let r = ui.add(Button::new(atoms).fill(ACCENT).stroke(Stroke::NONE));
    if r.hovered() && r.enabled() {
        ui.painter().rect_filled(r.rect, R, Color32::from_white_alpha(22));
    }
    if r.has_focus() {
        ui.painter().rect_stroke(r.rect.expand(2.), R + 2, Stroke::new(1.5, ACCENT), egui::StrokeKind::Outside);
    }
    r
}

/// Texto oscuro sobre acento.
pub fn on_accent(t: &str) -> RichText {
    RichText::new(t).color(ON_ACCENT)
}

/// Etiqueta pequeña con borde.
pub fn tag(ui: &mut Ui, t: &str, c: Color32) {
    egui::Frame::new().stroke((1., BORDER_STRONG)).corner_radius(4).inner_margin(Margin::symmetric(6, 1)).show(ui, |ui| {
        ui.label(RichText::new(t).small().color(c));
    });
}

pub fn muted(t: impl Into<String>) -> RichText {
    RichText::new(t).color(TEXT_MUTED)
}

/// Encabezado de sección: título en negrita y cantidad atenuada.
pub fn section(t: &str, n: Option<usize>) -> egui::WidgetText {
    let mut j = egui::text::LayoutJob::default();
    j.append(t, 0., egui::TextFormat::simple(sb(14.), TEXT));
    if let Some(n) = n {
        j.append(&n.to_string(), 8., egui::TextFormat::simple(FontId::proportional(13.), TEXT_MUTED));
    }
    j.into()
}

/// Los menús de egui vienen muy apretados; los alineamos al ritmo 4/8.
pub fn menu_spacing(ui: &mut Ui) {
    ui.spacing_mut().button_padding = Vec2::new(8., 4.);
    ui.set_min_width(200.);
}

/// Botón grande de herramienta: ícono arriba y nombre abajo; resaltado con el acento si está elegido.
pub fn herr_btn(ui: &mut Ui, sel: bool, icon: &str, label: &str, tip: &str, size: Vec2) -> Response {
    let (rect, r) = ui.allocate_exact_size(size, egui::Sense::click());
    let p = ui.painter();
    let hov = r.hovered();
    p.rect_filled(rect, 8, if sel { blend(ACCENT, SURFACE, 0.24) } else if hov { SURFACE_HOVER } else { SURFACE });
    p.rect_stroke(rect, 8, Stroke::new(if sel { 2. } else { 1. }, if sel { ACCENT } else if hov { BORDER_STRONG } else { BORDER }), egui::StrokeKind::Inside);
    let c = rect.center();
    p.text(egui::Pos2::new(c.x, rect.top() + size.y * 0.36), egui::Align2::CENTER_CENTER, icon, FontId::proportional(22.), if sel { ACCENT } else { TEXT });
    let g = p.layout_no_wrap(label.to_string(), FontId::proportional(12.5), if sel { TEXT } else { TEXT_SOFT });
    let g = if g.size().x > size.x - 6. { p.layout(label.to_string(), FontId::proportional(11.), TEXT_SOFT, size.x - 4.) } else { g };
    p.galley(egui::Pos2::new(c.x - g.size().x / 2., rect.bottom() - 5. - g.size().y), g, TEXT);
    r.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Ícono y logo de NicoStation como texturas (se cargan una vez y quedan en la memoria de egui).
pub fn marca(ctx: &egui::Context) -> (egui::TextureHandle, egui::TextureHandle) {
    let id = egui::Id::unique("nicostation-marca");
    if let Some(t) = ctx.data(|d| d.get_temp::<(egui::TextureHandle, egui::TextureHandle)>(id)) {
        return t;
    }
    let cargar = |nombre: &str, bytes: &[u8]| {
        let img = image::load_from_memory(bytes).map(|i| i.to_rgba8()).unwrap_or_else(|_| image::RgbaImage::new(1, 1));
        let (w, h) = img.dimensions();
        let ci = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &img.into_raw());
        ctx.load_texture(nombre, ci, egui::TextureOptions::LINEAR)
    };
    let t = (cargar("marca-icono", include_bytes!("../../assets/icono.png")), cargar("marca-logo", include_bytes!("../../assets/logo_oscuro.png")));
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    t
}

/// Imagen de la marca a una altura dada (en puntos), respetando su proporción.
pub fn imagen_marca(ui: &mut egui::Ui, t: &egui::TextureHandle, alto: f32) -> egui::Response {
    let s = t.size_vec2();
    ui.add(egui::Image::new((t.id(), egui::vec2(alto * s.x / s.y, alto))))
}
