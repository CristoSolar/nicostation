//! Pantalla de bienvenida («¿Qué juego quieres hacer hoy?»): plantillas, empezar de cero,
//! abrir un juego y juegos recientes. También crea el juego nuevo a partir de una plantilla.

use super::iconos as ic;
use super::tema::*;
use eframe::egui::{self, Align, Align2, Color32, FontId, Key, Layout, Pos2, Rect, RichText, Sense, Stroke, TextEdit, Ui, Vec2};
use nicostation::plantillas;
use std::path::{Path, PathBuf};

const CARD_W: f32 = 240.;
const PIC_H: f32 = 168.;
const CARD_H: f32 = 280.;
const GAP: f32 = 16.;
const MAX_RECIENTES: usize = 6;

struct Tarjeta {
    carpeta: String,
    nombre: String,
    miniatura: PathBuf,
    /// Textura cargada (None = sin cargar todavía; Some(None) = no hay imagen).
    tex: Option<Option<egui::TextureHandle>>,
}

/// Ventanita «¿Cómo se llama tu juego?».
pub struct Dialogo {
    carpeta: String,
    pub nombre: String,
    pub dir: String,
    /// La carpeta sigue al nombre hasta que el niño la cambie a mano.
    dir_auto: bool,
    error: Option<String>,
    foco: bool,
}

pub struct Inicio {
    tarjetas: Vec<Tarjeta>,
    /// Juegos recientes: ruta y nombre del juego.
    recientes: Vec<(PathBuf, String)>,
    pub dialogo: Option<Dialogo>,
    /// Hay un juego abierto detrás (Archivo > Nuevo): se puede volver a él.
    pub puede_volver: bool,
}

pub enum Accion {
    /// Juego creado desde una plantilla: ruta de su juego.json.
    Creado(PathBuf),
    Vacio,
    Abrir,
    Reciente(PathBuf),
    Volver,
}

/// Frase corta para cada plantilla (para niños).
pub fn descripcion(carpeta: &str) -> Option<&'static str> {
    Some(match carpeta {
        "plataformas" => "Salta, junta monedas y llega a la bandera",
        "recolector" => "Un gato que come peces y escapa de un perro",
        "naves" => "Dispara a los asteroides antes de que te choquen",
        "atrapa" => "Atrapa la fruta con una canasta",
        _ => return None,
    })
}

/// Nombre de carpeta sin espacios ni acentos: «Aventura en plataformas» → «aventura-en-plataformas».
pub fn carpeta_de(nombre: &str) -> String {
    let mut s = String::new();
    for c in nombre.trim().chars().flat_map(char::to_lowercase) {
        let c = match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        };
        if c != '-' || !(s.is_empty() || s.ends_with('-')) {
            s.push(c);
        }
    }
    let s = s.trim_end_matches('-').to_string();
    if s.is_empty() { "mi-juego".into() } else { s }
}

/// Carpeta donde se guardan los juegos nuevos: `NICOSTATION_JUEGOS` o `~/Juegos`.
pub fn dir_juegos() -> PathBuf {
    if let Some(d) = std::env::var_os("NICOSTATION_JUEGOS") {
        return PathBuf::from(d);
    }
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".")).join("Juegos")
}

/// Carpeta propuesta para un juego nuevo: `~/Juegos/<nombre>`, o `<nombre>-2`, `-3`… si ya está ocupada.
pub fn carpeta_libre(nombre: &str) -> PathBuf {
    let (base, slug) = (dir_juegos(), carpeta_de(nombre));
    (1..100)
        .map(|n| base.join(if n == 1 { slug.clone() } else { format!("{slug}-{n}") }))
        .find(|d| problema_carpeta(d).is_none())
        .unwrap_or_else(|| base.join(slug))
}

fn archivo_recientes() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("nicostation").join("recientes.json"))
}

pub fn recientes() -> Vec<PathBuf> {
    let Some(f) = archivo_recientes() else { return vec![] };
    let l: Vec<String> = std::fs::read_to_string(f).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    l.into_iter().map(PathBuf::from).filter(|p| p.is_file()).take(MAX_RECIENTES).collect()
}

/// Pone `path` primero en la lista de juegos recientes (errores ignorados: es solo una comodidad).
pub fn recordar(path: &Path) {
    let Some(f) = archivo_recientes() else { return };
    let p = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut l = recientes();
    l.retain(|x| x != &p);
    l.insert(0, p);
    l.truncate(MAX_RECIENTES);
    let l: Vec<String> = l.iter().map(|p| p.to_string_lossy().into_owned()).collect();
    if let Some(d) = f.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(f, serde_json::to_string_pretty(&l).unwrap_or_default());
}

/// Problema con la carpeta elegida, en palabras simples (None = se puede usar).
pub fn problema_carpeta(dir: &Path) -> Option<String> {
    if dir.as_os_str().is_empty() {
        return Some("Escribe dónde guardar tu juego.".into());
    }
    if dir.join("juego.json").exists() {
        return Some("En esa carpeta ya hay un juego. Elige otra carpeta o cambia el nombre.".into());
    }
    if dir.is_file() {
        return Some("Eso es un archivo, no una carpeta. Elige otra carpeta.".into());
    }
    if dir.read_dir().is_ok_and(|mut d| d.next().is_some()) {
        return Some("Esa carpeta ya tiene otras cosas. Elige una carpeta vacía o cambia el nombre.".into());
    }
    None
}

/// Copia la plantilla `carpeta` a `dir`, le pone el nombre `nombre` y la guarda. Devuelve el juego.json.
pub fn crear(carpeta: &str, nombre: &str, dir: &Path) -> Result<PathBuf, String> {
    if let Some(p) = problema_carpeta(dir) {
        return Err(p);
    }
    if plantillas::dir().is_none() {
        return Err("No encontré las plantillas de juegos. Puedes empezar de cero mientras tanto.".into());
    }
    let json = plantillas::crear(carpeta, dir).map_err(|e| {
        if e.starts_with("no existe") { "No encontré esa plantilla. Prueba con otra.".into() } else { format!("No pude crear la carpeta. Elige otra carpeta. ({e})") }
    })?;
    let mut p: nicostation::model::Project =
        serde_json::from_str(&std::fs::read_to_string(&json).map_err(|e| e.to_string())?).map_err(|e| format!("La plantilla está dañada: {e}"))?;
    if !nombre.trim().is_empty() {
        p.name = nombre.trim().into();
    }
    std::fs::write(&json, serde_json::to_string_pretty(&p).map_err(|e| e.to_string())?).map_err(|e| format!("No pude guardar el juego: {e}"))?;
    Ok(json)
}

impl Inicio {
    pub fn new(puede_volver: bool) -> Self {
        let tarjetas = plantillas::lista()
            .into_iter()
            .map(|t| Tarjeta { carpeta: t.carpeta, nombre: t.nombre, miniatura: t.miniatura, tex: None })
            .collect();
        let recientes = recientes()
            .into_iter()
            .map(|p| {
                let v: Option<serde_json::Value> = std::fs::read_to_string(&p).ok().and_then(|s| serde_json::from_str(&s).ok());
                let n = v.as_ref().and_then(|v| v["name"].as_str()).map(String::from);
                let n = n.unwrap_or_else(|| p.parent().and_then(|d| d.file_name()).map_or("Juego".into(), |f| f.to_string_lossy().into_owned()));
                (p, n)
            })
            .collect();
        Inicio { tarjetas, recientes, dialogo: None, puede_volver }
    }

    /// Abre la ventanita de nombre y carpeta para la plantilla número `i`.
    pub fn elegir(&mut self, i: usize) {
        let Some(t) = self.tarjetas.get(i) else { return };
        let dir = carpeta_libre(&t.nombre);
        self.dialogo = Some(Dialogo { carpeta: t.carpeta.clone(), nombre: t.nombre.clone(), dir: dir.display().to_string(), dir_auto: true, error: None, foco: true });
    }

    fn textura(t: &mut Tarjeta, ctx: &egui::Context) -> Option<egui::TextureHandle> {
        t.tex
            .get_or_insert_with(|| {
                let img = image::open(&t.miniatura).ok()?.to_rgba8();
                let c = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw());
                Some(ctx.load_texture(format!("miniatura-{}", t.carpeta), c, egui::TextureOptions::LINEAR))
            })
            .clone()
    }

    pub fn ui(&mut self, ui: &mut Ui) -> Option<Accion> {
        let mut act = None;
        let ctx = ui.ctx().clone();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let avail = ui.available_width();
            ui.add_space(12.);
            if self.puede_volver {
                ui.horizontal(|ui| {
                    ui.add_space(8.);
                    if ui.button(it(ic::ARROW_LEFT, "Volver a mi juego")).clicked() {
                        act = Some(Accion::Volver);
                    }
                });
            } else {
                ui.add_space(30.);
            }
            ui.vertical_centered(|ui| {
                let (_, logo) = marca(ui.ctx());
                imagen_marca(ui, &logo, 64.);
                ui.add_space(12.);
                ui.label(RichText::new("¿Qué juego quieres hacer hoy?").font(sb(32.)).color(TEXT));
                ui.add_space(2.);
                ui.label(RichText::new("Elige uno para empezar. Después lo cambias como quieras.").size(16.).color(TEXT_SOFT));
            });
            ui.add_space(28.);

            // Tarjetas: plantillas y «Empezar de cero», en filas centradas.
            let n = self.tarjetas.len() + 1;
            let cols = (((avail - 32. + GAP) / (CARD_W + GAP)).floor() as usize).clamp(1, n);
            let row_w = cols as f32 * CARD_W + (cols - 1) as f32 * GAP;
            let mut elegida = None;
            for fila in (0..n).collect::<Vec<_>>().chunks(cols) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = GAP;
                    ui.add_space(((avail - row_w) / 2.).max(0.));
                    for &i in fila {
                        if i < self.tarjetas.len() {
                            let tex = Self::textura(&mut self.tarjetas[i], &ctx);
                            let t = &self.tarjetas[i];
                            let desc = descripcion(&t.carpeta).unwrap_or(&t.nombre);
                            if tarjeta(ui, tex.as_ref(), None, &t.nombre, desc).clicked() {
                                elegida = Some(i);
                            }
                        } else if tarjeta(ui, None, Some(ic::PLUS), "Empezar de cero", "Un juego vacío para inventar todo tú").clicked() {
                            act = Some(Accion::Vacio);
                        }
                    }
                });
                ui.add_space(GAP);
            }
            if let Some(i) = elegida {
                self.elegir(i);
            }

            ui.add_space(12.);
            ui.vertical_centered(|ui| {
                ui.spacing_mut().button_padding = Vec2::new(16., 10.);
                let b = ui.button((RichText::new(ic::FOLDER_OPEN).size(18.).color(TEXT_MUTED), RichText::new("Abrir un juego que ya hice…").font(sb(16.))));
                if b.clicked() {
                    act = Some(Accion::Abrir);
                }
            });
            if !self.recientes.is_empty() {
                ui.add_space(20.);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("Mis juegos recientes").font(sb(15.)).color(TEXT_MUTED));
                });
                ui.add_space(6.);
                let w = 520f32.min(avail - 32.);
                for (p, nombre) in &self.recientes {
                    ui.horizontal(|ui| {
                        ui.add_space(((avail - w) / 2.).max(0.));
                        if reciente(ui, p, nombre, w).clicked() {
                            act = Some(Accion::Reciente(p.clone()));
                        }
                    });
                }
            }
            ui.add_space(24.);
        });
        if let Some(d) = self.dialogo.as_mut() {
            match dialogo(&ctx, d) {
                Some(true) => match crear(&d.carpeta, &d.nombre, Path::new(d.dir.trim())) {
                    Ok(json) => {
                        self.dialogo = None;
                        return Some(Accion::Creado(json));
                    }
                    Err(e) => d.error = Some(e),
                },
                Some(false) => self.dialogo = None,
                None => {}
            }
        }
        act
    }
}

/// Tarjeta grande con imagen (o ícono), nombre y frase. Se puede elegir con el teclado (Tab + Enter).
fn tarjeta(ui: &mut Ui, tex: Option<&egui::TextureHandle>, icono: Option<&str>, nombre: &str, desc: &str) -> egui::Response {
    let (rect, r) = ui.allocate_exact_size(Vec2::new(CARD_W, CARD_H), Sense::click());
    let (hov, foco) = (r.hovered(), r.has_focus());
    let p = ui.painter();
    p.rect_filled(rect, 12, if hov { SURFACE_HOVER } else { SURFACE });
    let borde = if foco { Stroke::new(2., ACCENT) } else if hov { Stroke::new(1., BORDER_STRONG) } else { Stroke::new(1., BORDER) };
    p.rect_stroke(rect, 12, borde, egui::StrokeKind::Inside);
    let pic = Rect::from_min_size(rect.min + Vec2::splat(8.), Vec2::new(CARD_W - 16., PIC_H));
    p.rect_filled(pic, 8, BG);
    match (tex, icono) {
        (Some(t), _) => {
            let s = t.size_vec2();
            let k = (pic.width() / s.x).min(pic.height() / s.y);
            let img = Rect::from_center_size(pic.center(), s * k);
            let tint = if hov || foco { Color32::WHITE } else { Color32::from_gray(225) };
            p.add(egui::Shape::image(t.id(), img, Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)), tint));
        }
        (None, Some(i)) => {
            p.rect_stroke(pic.shrink(1.), 8, Stroke::new(1., BORDER_STRONG), egui::StrokeKind::Inside);
            p.text(pic.center(), Align2::CENTER_CENTER, i, FontId::proportional(44.), if hov || foco { TEXT } else { TEXT_MUTED });
        }
        _ => drop(p.text(pic.center(), Align2::CENTER_CENTER, ic::IMAGE, FontId::proportional(36.), TEXT_FAINT)),
    }
    let tx = rect.left() + 14.;
    let wrap = CARD_W - 28.;
    let g = p.layout(nombre.to_string(), sb(19.), TEXT, wrap);
    let y = pic.bottom() + 12.;
    let h = g.size().y;
    p.galley(Pos2::new(tx, y), g, TEXT);
    let g = p.layout(desc.to_string(), FontId::proportional(14.5), TEXT_SOFT, wrap);
    p.galley(Pos2::new(tx, y + h + 4.), g, TEXT_SOFT);
    if hov || foco {
        p.text(Pos2::new(rect.right() - 14., rect.bottom() - 14.), Align2::RIGHT_BOTTOM, ic::ARROW_RIGHT, FontId::proportional(18.), TEXT_MUTED);
    }
    r.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Fila de juego reciente: nombre del juego y carpeta atenuada.
fn reciente(ui: &mut Ui, path: &Path, nombre: &str, w: f32) -> egui::Response {
    let (rect, r) = ui.allocate_exact_size(Vec2::new(w, 44.), Sense::click());
    let p = ui.painter();
    let (hov, foco) = (r.hovered(), r.has_focus());
    if hov || foco {
        p.rect_filled(rect, 8, SURFACE_HOVER);
    }
    if foco {
        p.rect_stroke(rect, 8, Stroke::new(1.5, ACCENT), egui::StrokeKind::Inside);
    }
    let dir = path.parent().unwrap_or(path);
    p.text(Pos2::new(rect.left() + 14., rect.center().y), Align2::LEFT_CENTER, ic::GAMEPAD, FontId::proportional(18.), TEXT_MUTED);
    p.text(Pos2::new(rect.left() + 42., rect.top() + 13.), Align2::LEFT_CENTER, nombre, sb(15.), TEXT);
    let g = p.layout_no_wrap(dir.display().to_string(), FontId::proportional(12.5), TEXT_FAINT);
    let g = if g.size().x > w - 56. { p.layout(dir.display().to_string(), FontId::proportional(12.5), TEXT_FAINT, w - 56.) } else { g };
    p.galley_with_override_text_color(Pos2::new(rect.left() + 42., rect.top() + 22.), g, TEXT_FAINT);
    r.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(path.display().to_string())
}

/// Ventanita de nombre y carpeta. Devuelve Some(true) = crear, Some(false) = cancelar.
fn dialogo(ctx: &egui::Context, d: &mut Dialogo) -> Option<bool> {
    let mut res = None;
    let frame = egui::Frame::window(&ctx.global_style()).inner_margin(24).corner_radius(12);
    let r = egui::Modal::new(egui::Id::unique("nuevo_juego")).frame(frame).show(ctx, |ui| {
        ui.set_width(520.);
        ui.spacing_mut().item_spacing.y = 8.;
        ui.label(RichText::new("¿Cómo se llama tu juego?").font(sb(22.)));
        let antes = d.nombre.clone();
        // En el primer cuadro no se acepta Enter: puede ser el mismo que eligió la tarjeta.
        let primero = d.foco;
        let te = ui.add(TextEdit::singleline(&mut d.nombre).font(FontId::proportional(18.)).desired_width(f32::INFINITY).margin(Vec2::new(10., 8.)));
        if std::mem::take(&mut d.foco) {
            te.request_focus();
        }
        if d.nombre != antes {
            d.error = None;
            if d.dir_auto {
                d.dir = carpeta_libre(&d.nombre).display().to_string();
            }
        }
        ui.add_space(8.);
        ui.label(RichText::new("¿Dónde lo guardo?").font(sb(17.)));
        let mut elegir = false;
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                elegir |= ui.button(it(ic::FOLDER_OPEN, "Elegir carpeta…")).clicked();
                if ui.add(TextEdit::singleline(&mut d.dir).desired_width(ui.available_width()).margin(Vec2::new(8., 6.))).changed() {
                    d.dir_auto = false;
                    d.error = None;
                }
            });
        });
        // Aviso en vivo (antes de apretar Crear) o error al crear.
        let aviso = d.error.clone().or_else(|| problema_carpeta(Path::new(d.dir.trim())));
        if let Some(e) = &aviso {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(ic::WARNING_CIRCLE).color(WARN).size(16.));
                ui.label(RichText::new(e).color(TEXT));
            });
            if ui.link("Elegir otra carpeta…").clicked() {
                elegir = true;
            }
        } else {
            ui.label(RichText::new("Ahí van a quedar tu juego, sus dibujos y sus sonidos.").color(TEXT_MUTED));
        }
        if elegir {
            let base = Path::new(d.dir.trim()).parent().filter(|p| p.is_dir()).map(Path::to_path_buf).unwrap_or_else(dir_juegos);
            if let Some(f) = rfd::FileDialog::new().set_title("¿Dónde guardo tu juego?").set_directory(base).pick_folder() {
                // Carpeta vacía: se usa tal cual. Si tiene cosas, el juego va en una carpeta nueva adentro.
                let vacia = f.read_dir().is_ok_and(|mut r| r.next().is_none());
                d.dir = if vacia { f } else { f.join(carpeta_de(&d.nombre)) }.display().to_string();
                d.dir_auto = false;
                d.error = None;
            }
        }
        ui.add_space(12.);
        let ok = aviso.is_none() && !d.nombre.trim().is_empty();
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().button_padding = Vec2::new(16., 9.);
            let c = ui.add_enabled_ui(ok, |ui| primary(ui, (on_accent(ic::SPARKLE).size(17.), on_accent("Crear mi juego").font(sb(16.))))).inner;
            if c.clicked() {
                res = Some(true);
            }
            if ui.button(RichText::new("Cancelar").font(sb(15.))).clicked() {
                res = Some(false);
            }
        });
        if ok && !primero && ui.input(|i| i.key_pressed(Key::Enter)) {
            res = Some(true);
        }
    });
    if res.is_none() && r.should_close() {
        res = Some(false);
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nombre_de_carpeta() {
        assert_eq!(carpeta_de("Aventura en plataformas"), "aventura-en-plataformas");
        assert_eq!(carpeta_de("  ¡Mi Juegó Ñandú!  "), "mi-juego-nandu");
        assert_eq!(carpeta_de("???"), "mi-juego");
    }

    #[test]
    fn crear_juego_desde_plantilla() {
        std::env::set_var("NICOSTATION_PLANTILLAS", concat!(env!("CARGO_MANIFEST_DIR"), "/plantillas"));
        let dst = std::env::temp_dir().join(format!("nicostation-inicio-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dst);
        let json = crear("plataformas", "El salto de Nico", &dst).unwrap();
        let p: nicostation::model::Project = serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
        assert_eq!(p.name, "El salto de Nico");
        assert_eq!(p.plantilla, "plataformas");
        assert_eq!(nicostation::misiones::plantilla_de(&p), "plataformas");
        assert!(dst.join("img").is_dir());
        // Una carpeta con juego no se pisa, y el error se entiende.
        let e = crear("plataformas", "Otro", &dst).unwrap_err();
        assert!(e.contains("ya hay un juego"), "{e}");
        std::fs::remove_dir_all(&dst).unwrap();
    }
}
