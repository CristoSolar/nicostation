//! Panel «Misiones»: desafíos guiados paso a paso sobre el juego abierto (ver `nicostation::misiones`).
//! Los pasos automáticos se revisan solos cuando cambia el proyecto (también al deshacer);
//! los demás los marca el niño con «¡Listo!». Las misiones cumplidas se guardan en `.misiones.json`
//! junto al juego (nunca dentro de juego.json).

use super::iconos as ic;
use super::tema::*;
use eframe::egui::{self, Align, Button, Color32, Frame, Layout, Margin, Pos2, Rect, RichText, Sense, Ui, Vec2};
use nicostation::misiones::{self as lib, Mision};
use nicostation::model::Project;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

pub const ANCHO: f32 = 320.;
const CONFETI_SEG: f64 = 1.8;
const SIDECAR: &str = ".misiones.json";

/// Avance de una misión empezada en esta sesión.
struct Estado {
    /// El proyecto como estaba al empezar (los pasos comparan contra esto).
    inicial: Project,
    /// Pasos marcados con «¡Listo!».
    manual: Vec<bool>,
    /// Último resultado de los pasos automáticos.
    auto: Vec<bool>,
}

impl Estado {
    fn hecho(&self, m: &Mision, i: usize) -> bool {
        if m.pasos[i].listo.is_some() { self.auto[i] } else { self.manual[i] }
    }

    fn hechos(&self, m: &Mision) -> usize {
        (0..m.pasos.len()).filter(|&i| self.hecho(m, i)).count()
    }

    /// Primer paso sin hacer (los pasos van en orden).
    fn actual(&self, m: &Mision) -> Option<usize> {
        (0..m.pasos.len()).find(|&i| !self.hecho(m, i))
    }

    fn evaluar(&mut self, m: &Mision, p: &Project) {
        for (i, s) in m.pasos.iter().enumerate() {
            if let Some(f) = s.listo {
                self.auto[i] = f(p, &self.inicial);
            }
        }
    }
}

#[derive(Default)]
pub struct Misiones {
    pub abierto: bool,
    /// Títulos de las misiones cumplidas.
    hechas: BTreeSet<String>,
    estados: HashMap<&'static str, Estado>,
    activa: Option<&'static str>,
    /// Pista visible para el paso número…
    pista: Option<usize>,
    /// Momento en que se cumplió la misión activa (muestra el festejo).
    festejo: Option<f64>,
    sidecar: Option<PathBuf>,
}

fn mision(titulo: &str) -> Option<&'static Mision> {
    lib::MISIONES.iter().find(|m| m.titulo == titulo)
}

impl Misiones {
    /// Estado para el juego en `path` (lee las misiones cumplidas de `.misiones.json`).
    pub fn cargar(path: Option<&Path>) -> Self {
        let sidecar = path.map(|p| p.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(Path::new(".")).join(SIDECAR));
        let hechas = sidecar
            .as_ref()
            .and_then(|f| std::fs::read_to_string(f).ok())
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .and_then(|v| v.get("hechas").and_then(|h| h.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()))
            .unwrap_or_default();
        Misiones { hechas, sidecar, ..Default::default() }
    }

    /// El juego se guardó con otro nombre: las misiones cumplidas van a la carpeta nueva.
    pub fn mover(&mut self, path: &Path) {
        let nuevo = path.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(Path::new(".")).join(SIDECAR);
        if self.sidecar.as_ref() != Some(&nuevo) {
            self.sidecar = Some(nuevo);
            if !self.hechas.is_empty() {
                self.guardar();
            }
        }
    }

    fn guardar(&self) {
        if let Some(f) = &self.sidecar {
            let v = serde_json::json!({ "hechas": self.hechas });
            let _ = std::fs::write(f, serde_json::to_string_pretty(&v).unwrap_or_default() + "\n");
        }
    }

    fn lista(p: &Project) -> Vec<&'static Mision> {
        lib::para(lib::plantilla_de(p)).collect()
    }

    /// Misiones del juego que faltan por cumplir (para la insignia del botón).
    pub fn pendientes(&self, p: &Project) -> usize {
        Self::lista(p).iter().filter(|m| !self.hechas.contains(m.titulo)).count()
    }

    /// Empieza (o retoma) una misión. Una misión ya cumplida empieza de nuevo.
    pub fn empezar(&mut self, m: &'static Mision, p: &Project) {
        let nueva = self.hechas.contains(m.titulo) || !self.estados.contains_key(m.titulo);
        if nueva {
            let n = m.pasos.len();
            let mut e = Estado { inicial: p.clone(), manual: vec![false; n], auto: vec![false; n] };
            e.evaluar(m, p);
            self.estados.insert(m.titulo, e);
        } else if let Some(e) = self.estados.get_mut(m.titulo) {
            e.evaluar(m, p);
        }
        self.activa = Some(m.titulo);
        self.festejo = None;
        self.pista = None;
        self.abierto = true;
    }

    /// Empieza la primera misión del juego (para `--abrir mision-activa`).
    pub fn empezar_primera(&mut self, p: &Project) {
        if let Some(m) = Self::lista(p).first() {
            self.empezar(m, p);
        }
    }

    /// Marca con «¡Listo!» el paso actual si no se revisa solo.
    pub fn listo(&mut self, now: f64) {
        let Some((m, e)) = self.activa.and_then(|t| Some((mision(t)?, self.estados.get_mut(t)?))) else { return };
        if let Some(i) = e.actual(m).filter(|&i| m.pasos[i].listo.is_none()) {
            e.manual[i] = true;
        }
        self.revisar_fin(now);
    }

    /// Se apretó «Probar»: cumple el paso «Prueba el juego» si es el que toca.
    pub fn probado(&mut self, now: f64) {
        let toca = self.activa.and_then(|t| Some((mision(t)?, self.estados.get(t)?))).and_then(|(m, e)| e.actual(m).map(|i| m.pasos[i].texto));
        if toca.is_some_and(|t| t.starts_with("Prueba el juego")) {
            self.listo(now);
        }
    }

    /// Revisa los pasos automáticos (el editor lo llama solo cuando cambió el proyecto, también al deshacer).
    pub fn actualizar(&mut self, p: &Project, now: f64) {
        for (t, e) in self.estados.iter_mut() {
            if let Some(m) = mision(t) {
                e.evaluar(m, p);
            }
        }
        self.revisar_fin(now);
    }

    fn revisar_fin(&mut self, now: f64) {
        let Some(t) = self.activa else { return };
        let (Some(m), Some(e)) = (mision(t), self.estados.get(t)) else { return };
        if self.festejo.is_none() && e.actual(m).is_none() {
            self.festejo = Some(now);
            self.pista = None;
            if self.hechas.insert(t.to_string()) {
                self.guardar();
            }
        }
    }

    /// Botón de la barra superior, con la cantidad de misiones que faltan.
    pub fn boton(&self, ui: &mut Ui, p: &Project, corto: bool) -> egui::Response {
        let n = self.pendientes(p);
        let icono = RichText::new(ic::TROPHY).size(17.).color(if self.abierto { ACCENT } else { TEXT_MUTED });
        let b = if corto { Button::selectable(self.abierto, icono) } else { Button::selectable(self.abierto, (icono, RichText::new("Misiones").font(sb(14.)))) };
        let r = ui.add(b.frame_when_inactive(self.abierto).min_size(Vec2::new(30., 30.)));
        if n > 0 {
            let c = Pos2::new(r.rect.right() - 3., r.rect.top() + 4.);
            let t = n.to_string();
            let g = ui.painter().layout_no_wrap(t, sb(11.), ON_CAT);
            let rad = (g.size().x / 2. + 4.).max(8.);
            ui.painter().rect_filled(Rect::from_center_size(c, Vec2::new(rad * 2., 16.)), 8, CAT_MOV);
            ui.painter().galley(c - g.size() / 2., g, ON_CAT);
        }
        let tip = match n {
            0 => "Misiones: ¡las cumpliste todas!".to_string(),
            1 => "Misiones: te falta 1".to_string(),
            n => format!("Misiones: te faltan {n}"),
        };
        r.on_hover_text(tip)
    }

    /// Contenido del panel lateral.
    pub fn ui(&mut self, ui: &mut Ui, p: &Project) {
        let now = ui.input(|i| i.time);
        ui.horizontal(|ui| {
            ui.label(RichText::new(ic::TROPHY).size(20.).color(ACCENT));
            ui.label(RichText::new("Misiones").font(sb(18.)));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ib(ui, ic::X, "Cerrar misiones").clicked() {
                    self.abierto = false;
                }
            });
        });
        ui.add_space(4.);
        let activa = self.activa.and_then(|t| Some((mision(t)?, t)));
        match activa {
            Some((m, _)) if self.festejo.is_some() => self.festejo_ui(ui, m, p, now),
            Some((m, t)) if self.estados.contains_key(t) => self.activa_ui(ui, m, now),
            _ => self.lista_ui(ui, p),
        }
    }

    fn lista_ui(&mut self, ui: &mut Ui, p: &Project) {
        ui.label(RichText::new("Pequeños desafíos para cambiar tu juego. Elige uno.").color(TEXT_SOFT));
        ui.add_space(8.);
        let mut elegida = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for m in Self::lista(p) {
                let hecha = self.hechas.contains(m.titulo);
                let e = self.estados.get(m.titulo);
                let total = m.pasos.len();
                let hechos = if hecha { total } else { e.map_or(0, |e| e.hechos(m)) };
                Frame::new().fill(SURFACE).stroke((1., BORDER)).corner_radius(10).inner_margin(12).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        if hecha {
                            ui.label(RichText::new(ic::TROPHY).size(18.).color(WARN)).on_hover_text("¡Misión cumplida!");
                        }
                        ui.add(egui::Label::new(RichText::new(m.titulo).font(sb(16.))).wrap());
                    });
                    ui.label(RichText::new(m.descripcion).color(TEXT_SOFT));
                    ui.add_space(4.);
                    barra(ui, hechos, total, if hecha { OK } else { CAT_MOV });
                    ui.horizontal(|ui| {
                        let txt = if hecha { "¡Cumplida!".to_string() } else { format!("{hechos} de {total} pasos") };
                        ui.label(RichText::new(txt).color(if hecha { OK } else { TEXT_MUTED }));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let (icono, t) = if hecha { (ic::REPEAT, "Otra vez") } else if e.is_some() { (ic::ARROW_RIGHT, "Seguir") } else { (ic::ARROW_RIGHT, "Empezar") };
                            let b = Button::new((RichText::new(t).font(sb(15.)), RichText::new(icono).color(TEXT_MUTED))).min_size(Vec2::new(0., 32.));
                            if ui.add(b).clicked() {
                                elegida = Some(m);
                            }
                        });
                    });
                });
                ui.add_space(8.);
            }
        });
        if let Some(m) = elegida {
            self.empezar(m, p);
        }
    }

    fn activa_ui(&mut self, ui: &mut Ui, m: &'static Mision, now: f64) {
        if ui.add(Button::new(it(ic::CARET_LEFT, "Todas las misiones")).frame_when_inactive(false)).clicked() {
            self.activa = None;
            return;
        }
        let Some(e) = self.estados.get(m.titulo) else { return };
        let (total, hechos, actual) = (m.pasos.len(), e.hechos(m), e.actual(m));
        let hecho: Vec<bool> = (0..total).map(|i| e.hecho(m, i)).collect();
        ui.add_space(4.);
        ui.add(egui::Label::new(RichText::new(m.titulo).font(sb(20.))).wrap());
        ui.label(RichText::new(m.descripcion).color(TEXT_SOFT));
        ui.add_space(6.);
        barra(ui, hechos, total, CAT_MOV);
        ui.label(muted(format!("{hechos} de {total} pasos")));
        ui.add_space(8.);
        let mut listo = false;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for (i, s) in m.pasos.iter().enumerate() {
                if Some(i) == actual {
                    let fill = blend(CAT_MOV, SURFACE, 0.14);
                    Frame::new().fill(fill).stroke((1.5, CAT_MOV)).corner_radius(10).inner_margin(12).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(caps(&format!("PASO {}", i + 1), 12., CAT_MOV));
                        ui.add(egui::Label::new(RichText::new(s.texto).font(sb(17.)).color(TEXT)).wrap());
                        ui.add_space(4.);
                        let ver = self.pista == Some(i);
                        let b = Button::new((RichText::new(ic::LIGHTBULB).color(WARN), RichText::new(if ver { "Ocultar pista" } else { "Ver pista" })))
                            .frame_when_inactive(false);
                        if ui.add(b).clicked() {
                            self.pista = if ver { None } else { Some(i) };
                        }
                        if ver {
                            Frame::new().fill(SURFACE).corner_radius(8).inner_margin(Margin::symmetric(10, 8)).show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.add(egui::Label::new(RichText::new(s.pista).color(TEXT_SOFT)).wrap());
                            });
                        }
                        ui.add_space(4.);
                        if s.listo.is_none() {
                            ui.spacing_mut().button_padding = Vec2::new(18., 8.);
                            if primary(ui, (on_accent(ic::CHECK).size(16.), on_accent("¡Listo!").font(sb(16.)))).on_hover_text("Apriétalo cuando lo hayas hecho").clicked() {
                                listo = true;
                            }
                        } else {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new(ic::SPARKLE).color(TEXT_MUTED));
                                ui.label(muted("Este paso se marca solo cuando lo hagas."));
                            });
                        }
                    });
                } else {
                    let (icono, col, tc) = if hecho[i] {
                        (ic::CHECK_CIRCLE, OK, TEXT_MUTED)
                    } else if actual.is_some_and(|a| i > a) {
                        (ic::LOCK, TEXT_FAINT, TEXT_FAINT)
                    } else {
                        (ic::CIRCLE, TEXT_MUTED, TEXT_SOFT)
                    };
                    Frame::new().inner_margin(Margin::symmetric(12, 6)).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal_top(|ui| {
                            ui.label(RichText::new(icono).size(18.).color(col));
                            ui.add(egui::Label::new(RichText::new(s.texto).color(tc)).wrap());
                        });
                    });
                }
                ui.add_space(4.);
            }
            ui.add_space(8.);
            if ui.add(Button::new(muted("Dejar esta misión para después")).frame_when_inactive(false)).clicked() {
                self.activa = None;
            }
        });
        if listo {
            self.listo(now);
        }
    }

    fn festejo_ui(&mut self, ui: &mut Ui, m: &'static Mision, p: &Project, now: f64) {
        let t0 = self.festejo.unwrap_or(now);
        let area = ui.available_rect_before_wrap();
        // Los papelitos van detrás del texto (se reserva su lugar en la lista de dibujo antes del contenido).
        let fondo = ui.painter().add(egui::Shape::Noop);
        ui.add_space(24.);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(ic::TROPHY).size(64.).color(WARN));
            ui.add_space(4.);
            ui.label(RichText::new("¡Misión cumplida!").font(sb(24.)));
            ui.add(egui::Label::new(RichText::new(format!("Terminaste «{}». ¡Tu juego ahora es un poco más tuyo!", m.titulo)).size(15.).color(TEXT_SOFT)).wrap());
        });
        ui.add_space(20.);
        let siguiente = Self::lista(p).into_iter().find(|x| !self.hechas.contains(x.titulo) && x.titulo != m.titulo);
        ui.vertical_centered(|ui| {
            ui.spacing_mut().button_padding = Vec2::new(18., 9.);
            match siguiente {
                Some(s) => {
                    if primary(ui, (on_accent("Siguiente misión").font(sb(16.)), on_accent(ic::ARROW_RIGHT).size(16.))).on_hover_text(s.titulo).clicked() {
                        self.empezar(s, p);
                    }
                }
                None => drop(ui.label(RichText::new("¡Cumpliste todas las misiones de este juego!").font(sb(15.)).color(OK))),
            }
            ui.add_space(4.);
            if ui.add(Button::new("Ver todas las misiones").frame_when_inactive(false)).clicked() {
                self.activa = None;
                self.festejo = None;
            }
        });
        ui.painter().set(fondo, confeti(ui, area, now - t0));
    }
}

/// Barra de avance.
fn barra(ui: &mut Ui, n: usize, total: usize, c: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 6.), Sense::hover());
    ui.painter().rect_filled(rect, 3, BORDER);
    if total > 0 && n > 0 {
        let w = rect.width() * n as f32 / total as f32;
        ui.painter().rect_filled(Rect::from_min_size(rect.min, Vec2::new(w, rect.height())), 3, c);
    }
}

/// Lluvia corta de papelitos sobre el panel al cumplir una misión (se apaga sola).
fn confeti(ui: &Ui, area: Rect, t: f64) -> egui::Shape {
    if !(0. ..CONFETI_SEG).contains(&t) || ui.style().animation_time <= 0. {
        return egui::Shape::Noop;
    }
    ui.ctx().request_repaint();
    let colores = [CAT_MOV, CAT_INPUT, CAT_SCENE, CAT_AUDIO, CAT_FUNC, ACCENT];
    let fade = (((CONFETI_SEG - t) / 0.5).min(1.)) as f32;
    let t = t as f32;
    let mut out = vec![];
    for i in 0..40u32 {
        // Pseudoazar fijo por papelito.
        let h = i.wrapping_mul(2654435761);
        let fx = (h % 1000) as f32 / 1000.;
        let vel = 160. + ((h >> 10) % 120) as f32;
        let x = area.left() + fx * area.width() + (t * 3. + i as f32).sin() * 10.;
        let y = area.top() - 20. - ((h >> 20) % 80) as f32 + vel * t;
        let giro = (t * 6. + i as f32).cos().abs();
        let r = Rect::from_center_size(Pos2::new(x, y), Vec2::new(7. * giro + 1., 5.));
        if area.contains_rect(r) {
            out.push(egui::Shape::rect_filled(r, 1, colores[i as usize % colores.len()].gamma_multiply(fade)));
        }
    }
    egui::Shape::Vec(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nicostation::model::Behavior;

    fn plantilla(c: &str) -> Project {
        let f = format!("{}/plantillas/{c}/juego.json", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(f).unwrap()).unwrap()
    }

    fn salto(p: &mut Project, v: f32) {
        let o = p.scenes[0].objects.iter_mut().find(|o| o.name == "Heroe").unwrap();
        if let Behavior::Platformer { jump_speed, .. } = &mut o.behaviors[0] {
            *jump_speed = v;
        }
    }

    #[test]
    fn mision_paso_a_paso_con_deshacer() {
        let dir = std::env::temp_dir().join(format!("nicostation-misiones-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("juego.json");
        let mut p = plantilla("plataformas");
        let mut ms = Misiones::cargar(Some(&path));
        let total = ms.pendientes(&p);
        assert!(total >= 3);
        ms.empezar_primera(&p);
        let m = mision(ms.activa.unwrap()).unwrap();
        assert_eq!(m.titulo, "Salta más alto");
        let e = |ms: &Misiones| ms.estados[m.titulo].actual(m);
        assert_eq!(e(&ms), Some(0));
        ms.listo(0.); // «haz clic en Heroe»
        assert_eq!(e(&ms), Some(1));
        ms.listo(0.); // un paso automático no se marca con «¡Listo!»
        assert_eq!(e(&ms), Some(1));
        salto(&mut p, 850.);
        ms.actualizar(&p, 0.);
        assert_eq!(e(&ms), Some(2));
        // Deshacer: el paso automático vuelve a quedar sin hacer.
        salto(&mut p, 600.);
        ms.actualizar(&p, 0.);
        assert_eq!(e(&ms), Some(1));
        salto(&mut p, 900.);
        ms.actualizar(&p, 0.);
        ms.probado(1.);
        assert!(ms.festejo.is_some());
        assert_eq!(ms.pendientes(&p), total - 1);
        // Las cumplidas sobreviven a cerrar el editor (archivo junto al juego).
        let ms2 = Misiones::cargar(Some(&path));
        assert!(ms2.hechas.contains("Salta más alto"));
        assert!(!std::fs::read_to_string(dir.join(SIDECAR)).unwrap().is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
