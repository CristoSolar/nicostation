//! Buscar y reemplazar en los eventos (Ctrl+F en Eventos): busca en las frases de las condiciones y acciones,
//! en sus argumentos y en los títulos y comentarios; salta al resultado; en modo experto reemplaza en los argumentos.

use super::eventos::Target;
use super::iconos as ic;
use super::tema::*;
use eframe::egui::{self, AtomExt, Button, Frame, Key, Margin, RichText, TextEdit, Ui, Vec2};
use nicostation::catalog;
use nicostation::model::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Hallazgo {
    pub target: Target,
    /// Escena o bloque donde está.
    pub donde: String,
    pub ev: Vec<usize>,
    pub cond: bool,
    /// Ruta de la instrucción en su columna (vacía = el texto del evento: comentario, grupo, repetir).
    pub ip: Vec<usize>,
    pub texto: String,
}

/// ¿`s` contiene `q` sin importar mayúsculas?
pub fn contiene(s: &str, q: &str) -> bool {
    !q.is_empty() && s.to_lowercase().contains(&q.to_lowercase())
}

/// Reemplaza `q` por `r` sin importar mayúsculas. Devuelve el texto nuevo y cuántas veces reemplazó.
pub fn reemplazar_texto(s: &str, q: &str, r: &str) -> (String, usize) {
    if q.is_empty() {
        return (s.to_string(), 0);
    }
    let low = |c: char| c.to_lowercase().next().unwrap_or(c);
    let qc: Vec<char> = q.chars().map(low).collect();
    let sc: Vec<char> = s.chars().collect();
    let (mut out, mut i, mut n) = (String::with_capacity(s.len()), 0, 0);
    while i < sc.len() {
        if i + qc.len() <= sc.len() && sc[i..i + qc.len()].iter().zip(&qc).all(|(a, b)| low(*a) == *b) {
            out.push_str(r);
            i += qc.len();
            n += 1;
        } else {
            out.push(sc[i]);
            i += 1;
        }
    }
    (out, n)
}

#[allow(clippy::too_many_arguments)]
fn en_instrs(l: &[Instr], t: Target, donde: &str, ev: &[usize], cond: bool, pre: &[usize], q: &str, facil: bool, out: &mut Vec<Hallazgo>) {
    for (k, i) in l.iter().enumerate() {
        let ip: Vec<usize> = pre.iter().copied().chain([k]).collect();
        let frase = if facil { super::facil::frase_simple(i) } else { catalog::sentence(i) };
        let label = catalog::meta(&i.op).map_or("", |m| m.label);
        if contiene(&frase, q) || contiene(label, q) || i.args.iter().any(|a| contiene(a, q)) {
            out.push(Hallazgo { target: t, donde: donde.into(), ev: ev.to_vec(), cond, ip: ip.clone(), texto: frase });
        }
        en_instrs(&i.sub, t, donde, ev, cond, &ip, q, facil, out);
    }
}

fn en_eventos(evs: &[Event], t: Target, donde: &str, pre: &[usize], q: &str, facil: bool, out: &mut Vec<Hallazgo>) {
    for (k, e) in evs.iter().enumerate() {
        let ev: Vec<usize> = pre.iter().copied().chain([k]).collect();
        if contiene(&e.text, q) {
            let tipo = match e.kind {
                EventKind::Comment => "Comentario",
                EventKind::Group => "Grupo",
                EventKind::Repeat => "Repetir",
                EventKind::ForEach => "Para cada",
                _ => "Evento",
            };
            out.push(Hallazgo { target: t, donde: donde.into(), ev: ev.clone(), cond: true, ip: vec![], texto: format!("{tipo}: {}", e.text) });
        }
        en_instrs(&e.conds, t, donde, &ev, true, &[], q, facil, out);
        en_instrs(&e.acts, t, donde, &ev, false, &[], q, facil, out);
        en_eventos(&e.sub, t, donde, &ev, q, facil, out);
    }
}

/// Busca en la escena `sc` (y, con `todo`, en todas las escenas y bloques). `facil`: con las frases del modo fácil.
pub fn buscar(p: &Project, sc: usize, q: &str, todo: bool, facil: bool) -> Vec<Hallazgo> {
    let mut out = vec![];
    if q.trim().is_empty() {
        return out;
    }
    for (k, s) in p.scenes.iter().enumerate() {
        if todo || k == sc {
            en_eventos(&s.events, Target::Scene(k), &format!("Escena «{}»", s.name), &[], q, facil, &mut out);
        }
    }
    if todo {
        for (k, f) in p.functions.iter().enumerate() {
            en_eventos(&f.events, Target::Func(k), &format!("Bloque «{}»", f.name), &[], q, facil, &mut out);
        }
    }
    out
}

fn eventos_de(p: &mut Project, t: Target) -> Option<&mut Vec<Event>> {
    match t {
        Target::Scene(i) => p.scenes.get_mut(i).map(|s| &mut s.events),
        Target::Func(i) => p.functions.get_mut(i).map(|f| &mut f.events),
    }
}

fn instr_mut<'a>(l: &'a mut [Instr], ip: &[usize]) -> Option<&'a mut Instr> {
    let (f, rest) = ip.split_first()?;
    let i = l.get_mut(*f)?;
    if rest.is_empty() { Some(i) } else { instr_mut(&mut i.sub, rest) }
}

/// Reemplaza en los argumentos (y textos de evento) de los hallazgos. Devuelve cuántos reemplazos hizo.
pub fn reemplazar(p: &mut Project, hs: &[Hallazgo], q: &str, r: &str) -> usize {
    let mut n = 0;
    for h in hs {
        let Some(evs) = eventos_de(p, h.target) else { continue };
        let Some(e) = super::eventos::ev_mut(evs, &h.ev) else { continue };
        if h.ip.is_empty() {
            let (t, k) = reemplazar_texto(&e.text, q, r);
            e.text = t;
            n += k;
            continue;
        }
        let l = if h.cond { &mut e.conds } else { &mut e.acts };
        if let Some(i) = instr_mut(l, &h.ip) {
            for a in &mut i.args {
                let (t, k) = reemplazar_texto(a, q, r);
                *a = t;
                n += k;
            }
        }
    }
    n
}

// ---------- barra ----------

#[derive(Default)]
pub struct Buscador {
    pub abierto: bool,
    pub texto: String,
    pub reemplazo: String,
    pub todo: bool,
    pub resultados: Vec<Hallazgo>,
    pub actual: usize,
    /// Texto y JSON con que se buscó (para no repetir).
    hecho: (String, bool, u64),
    pub foco: bool,
    /// Ya se saltó al resultado actual (el primer Enter va al primero, los siguientes avanzan).
    visto: bool,
}

pub enum Accion {
    Ir(Hallazgo),
    /// Reemplazar en uno (el actual) o en todos.
    Reemplazar(bool),
}

impl Buscador {
    pub fn abrir(&mut self) {
        self.abierto = true;
        self.foco = true;
    }
    /// Marca el resultado `k` como el actual (ya mostrado).
    pub fn ir(&mut self, k: usize) {
        self.actual = k.min(self.resultados.len().saturating_sub(1));
        self.visto = true;
    }
    /// Vuelve a buscar si cambió el texto o el proyecto.
    pub fn actualizar(&mut self, p: &Project, sc: usize, json: &str, facil: bool) {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (sc, json, facil).hash(&mut h);
        let k = (self.texto.clone(), self.todo, h.finish());
        if k != self.hecho {
            self.resultados = buscar(p, sc, &self.texto, self.todo, facil);
            if k.0 != self.hecho.0 || k.1 != self.hecho.1 {
                self.actual = 0;
                self.visto = false;
            }
            self.actual = self.actual.min(self.resultados.len().saturating_sub(1));
            self.hecho = k;
        }
    }
}

/// Barra de búsqueda arriba de la hoja. `facil`: sin reemplazar.
pub fn barra(ui: &mut Ui, b: &mut Buscador, facil: bool) -> Option<Accion> {
    let mut act = None;
    Frame::new().fill(SURFACE).stroke((1., BORDER_STRONG)).corner_radius(8).inner_margin(Margin::symmetric(10, 6)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(ic::SEARCH).size(16.).color(ACCENT));
            let r = ui.add(TextEdit::singleline(&mut b.texto).hint_text(if facil { "¿Qué bloque buscas?" } else { "Buscar en eventos" }).desired_width(220.));
            if std::mem::take(&mut b.foco) {
                r.request_focus();
            }
            let n = b.resultados.len();
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
            let cuenta = if b.texto.trim().is_empty() { String::new() } else if n == 0 { "sin resultados".into() } else { format!("{} de {n}", b.actual + 1) };
            ui.label(RichText::new(cuenta).color(if n == 0 && !b.texto.is_empty() { WARN } else { TEXT_MUTED }));
            if ib_on(ui, n > 0, ic::CARET_UP, "Anterior (Shift+Enter)").clicked() {
                b.visto = true;
                b.actual = (b.actual + n - 1) % n.max(1);
                act = b.resultados.get(b.actual).cloned().map(Accion::Ir);
            }
            let siguiente = ib_on(ui, n > 0, ic::CARET_DOWN, "Siguiente (Enter)").clicked();
            if (siguiente || enter) && n > 0 {
                if !b.visto {
                    b.visto = true;
                } else if enter && ui.input(|i| i.modifiers.shift) {
                    b.actual = (b.actual + n - 1) % n;
                } else {
                    b.actual = (b.actual + 1) % n;
                }
                act = b.resultados.get(b.actual).cloned().map(Accion::Ir);
                if enter {
                    r.request_focus();
                }
            }
            super::seg(ui, |ui| {
                if super::seg_btn(ui, !b.todo, if facil { "Esta escena" } else { "Escena" }).clicked() {
                    b.todo = false;
                }
                if super::seg_btn(ui, b.todo, if facil { "Todo el juego" } else { "Todo" }).on_hover_text("Todas las escenas y bloques").clicked() {
                    b.todo = true;
                }
            });
            if !facil {
                ui.add_space(8.);
                ui.label(muted("Reemplazar por"));
                ui.add(TextEdit::singleline(&mut b.reemplazo).desired_width(140.));
                if ui.add_enabled(n > 0, Button::new("Reemplazar")).on_hover_text("En los argumentos del resultado actual").clicked() {
                    act = Some(Accion::Reemplazar(false));
                }
                if ui.add_enabled(n > 0, Button::new("Todos")).on_hover_text("En los argumentos de todos los resultados").clicked() {
                    act = Some(Accion::Reemplazar(true));
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ib(ui, ic::X, "Cerrar (Esc)").clicked() {
                    b.abierto = false;
                }
            });
        });
        if !b.resultados.is_empty() {
            ui.add_space(4.);
            egui::ScrollArea::vertical().max_height(150.).id_salt("buscar_res").show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 1.;
                for (k, h) in b.resultados.iter().enumerate() {
                    let col = if h.ip.is_empty() { WARN } else if h.cond { CAT_INPUT } else { CAT_MOV };
                    let pre = if h.ip.is_empty() { "" } else if h.cond { if facil { "CUANDO" } else { "Cond." } } else if facil { "ENTONCES" } else { "Acción" };
                    let atoms = (
                        RichText::new(pre).small().color(col),
                        RichText::new(&h.texto).atom_grow(true).atom_align(egui::Align2::LEFT_CENTER),
                        RichText::new(format!("{} · evento {}", h.donde, h.ev.iter().map(|x| (x + 1).to_string()).collect::<Vec<_>>().join("."))).small().color(TEXT_MUTED),
                    );
                    let r = ui.add(Button::selectable(k == b.actual, atoms).min_size(Vec2::new(ui.available_width(), 22.)));
                    if r.clicked() {
                        b.actual = k;
                        b.visto = true;
                        act = Some(Accion::Ir(h.clone()));
                    }
                }
            });
        }
    });
    if ui.input(|i| i.key_pressed(Key::Escape)) {
        b.abierto = false;
    }
    act
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proyecto() -> Project {
        serde_json::from_str(r#"{"name":"t","width":10,"height":10,"first_scene":"A","scenes":[
            {"name":"A","events":[
              {"kind":"comment","text":"Mover al Heroe"},
              {"conds":[{"op":"key_down","args":["Right"]}],"acts":[{"op":"set","args":["Heroe","x","Heroe.x + 5"]}],
               "sub":[{"acts":[{"op":"play_sound","args":["snd/paso.wav"]}]}]}]},
            {"name":"B","events":[{"acts":[{"op":"set","args":["Heroe","y","0"]}]}]}],
            "functions":[{"name":"f","events":[{"acts":[{"op":"play_sound","args":["snd/heroe.wav"]}]}]}]}"#).unwrap()
    }

    #[test]
    fn busca_en_frases_args_y_textos() {
        let p = proyecto();
        let h = buscar(&p, 0, "heroe", false, false);
        assert_eq!(h.len(), 2);
        assert!(h[0].ip.is_empty() && h[0].texto.contains("Mover al Heroe"));
        assert_eq!((h[1].ev.clone(), h[1].cond, h[1].ip.clone()), (vec![1], false, vec![0]));
        // Sub-eventos.
        let s = buscar(&p, 0, "paso.wav", false, false);
        assert_eq!(s[0].ev, vec![1, 0]);
        // Todo el juego: también la escena B y el bloque.
        let t = buscar(&p, 0, "HEROE", true, false);
        assert_eq!(t.len(), 4);
        assert!(t.iter().any(|x| x.target == Target::Func(0)));
        assert!(buscar(&p, 0, "  ", true, false).is_empty());
    }

    #[test]
    fn reemplaza_en_argumentos() {
        assert_eq!(reemplazar_texto("Heroe.x + heroe", "HEROE", "Nave"), ("Nave.x + Nave".into(), 2));
        assert_eq!(reemplazar_texto("ñandú", "Ñ", "n"), ("nandú".into(), 1));
        let mut p = proyecto();
        let h = buscar(&p, 0, "Heroe", true, false);
        let n = reemplazar(&mut p, &h, "Heroe", "Nave");
        assert_eq!(n, 5);
        assert_eq!(p.scenes[0].events[0].text, "Mover al Nave");
        assert_eq!(p.scenes[0].events[1].acts[0].args, vec!["Nave", "x", "Nave.x + 5"]);
        assert_eq!(p.scenes[1].events[0].acts[0].args[0], "Nave");
        assert_eq!(p.functions[0].events[0].acts[0].args[0], "snd/Nave.wav");
    }
}
