//! Teclado y mouse → `Input` del runtime, en pasos fijos. También las teclas programadas de `--teclas`.

use nicostation::runtime::Input;
use std::collections::BTreeSet;

/// Estado del teclado entre pasos del mundo.
/// - `down` de un paso = teclas sostenidas + las que se apretaron y soltaron entre dos pasos (un toque corto
///   cuenta como apretada durante un paso, así no se pierde).
/// - `pressed` / `released` salen de comparar con el paso anterior.
#[derive(Default)]
pub struct Teclado {
    sostenidas: BTreeSet<String>,
    toques: BTreeSet<String>,
    antes: BTreeSet<String>,
    mouse_antes: [bool; 3],
}

impl Teclado {
    pub fn tecla(&mut self, nombre: &str, abajo: bool) {
        if abajo {
            self.sostenidas.insert(nombre.into());
            self.toques.insert(nombre.into());
        } else {
            self.sostenidas.remove(nombre);
        }
    }

    /// Reemplaza las teclas sostenidas (teclas programadas).
    pub fn poner(&mut self, teclas: &[String]) {
        let nuevas: BTreeSet<String> = teclas.iter().cloned().collect();
        self.toques.extend(nuevas.difference(&self.sostenidas).cloned());
        self.sostenidas = nuevas;
    }

    /// La entrada de un paso del mundo. `mouse` ya en coordenadas del juego.
    pub fn paso(&mut self, mouse: (f32, f32), botones: [bool; 3]) -> Input {
        let down: BTreeSet<String> = self.sostenidas.union(&std::mem::take(&mut self.toques)).cloned().collect();
        let input = Input {
            pressed: down.difference(&self.antes).cloned().collect(),
            released: self.antes.difference(&down).cloned().collect(),
            down: down.iter().cloned().collect(),
            mouse,
            mouse_down: botones,
            mouse_pressed: [0, 1, 2].map(|k| botones[k] && !self.mouse_antes[k]), axes: Default::default() };
        self.antes = down;
        self.mouse_antes = botones;
        input
    }
}

/// Tramos de teclas programadas: `"Up:1.0,Left:0.5"` = Up durante 1 s y después Left durante 0.5 s.
/// `"Up+Left:0.5"` = las dos a la vez; `"-:0.5"` = nada durante 0.5 s.
pub fn tramos(s: &str) -> Result<Vec<(Vec<String>, f32)>, String> {
    let mut out = vec![];
    for parte in s.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let (teclas, dur) = parte.rsplit_once(':').ok_or_else(|| format!("«{parte}»: falta «:segundos» (ej. Up:1.5)"))?;
        let dur: f32 = dur.trim().parse().map_err(|_| format!("«{parte}»: «{dur}» no es un número de segundos"))?;
        if !(dur >= 0.) {
            return Err(format!("«{parte}»: los segundos no pueden ser negativos"));
        }
        let teclas = teclas.split('+').map(str::trim).filter(|t| !t.is_empty() && *t != "-").map(String::from).collect();
        out.push((teclas, dur));
    }
    Ok(out)
}

/// Teclas sostenidas en el segundo `t` según los tramos (vacío después del último).
pub fn teclas_en(tramos: &[(Vec<String>, f32)], t: f32) -> &[String] {
    let mut fin = 0.;
    for (teclas, dur) in tramos {
        fin += dur;
        if t < fin {
            return teclas;
        }
    }
    &[]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(v: &[&str]) -> std::collections::HashSet<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn apretar_sostener_soltar() {
        let mut t = Teclado::default();
        t.tecla("Up", true);
        let a = t.paso((0., 0.), [false; 3]);
        assert_eq!((a.down, a.pressed, a.released), (set(&["Up"]), set(&["Up"]), set(&[])));
        let b = t.paso((0., 0.), [false; 3]);
        assert_eq!((b.down, b.pressed, b.released), (set(&["Up"]), set(&[]), set(&[])));
        t.tecla("Up", false);
        let c = t.paso((0., 0.), [false; 3]);
        assert_eq!((c.down, c.pressed, c.released), (set(&[]), set(&[]), set(&["Up"])));
    }

    #[test]
    fn toque_corto_entre_pasos_no_se_pierde() {
        let mut t = Teclado::default();
        t.tecla("Space", true);
        t.tecla("Space", false);
        let a = t.paso((0., 0.), [false; 3]);
        assert!(a.pressed.contains("Space") && a.down.contains("Space"));
        let b = t.paso((0., 0.), [false; 3]);
        assert!(b.released.contains("Space") && b.down.is_empty());
    }

    #[test]
    fn clic_del_mouse() {
        let mut t = Teclado::default();
        assert_eq!(t.paso((1., 2.), [true, false, false]).mouse_pressed, [true, false, false]);
        let b = t.paso((1., 2.), [true, false, false]);
        assert_eq!((b.mouse_pressed, b.mouse_down, b.mouse), ([false; 3], [true, false, false], (1., 2.)));
    }

    #[test]
    fn teclas_programadas() {
        let tr = tramos("Up:1.0, Left+Up:0.5,-:0.25,Space:0").unwrap();
        assert_eq!(tr.len(), 4);
        assert_eq!(teclas_en(&tr, 0.2), ["Up"]);
        assert_eq!(teclas_en(&tr, 1.2), ["Left", "Up"]);
        assert!(teclas_en(&tr, 1.6).is_empty());
        assert!(teclas_en(&tr, 9.).is_empty());
        assert!(tramos("Up").is_err());
        assert!(tramos("Up:x").is_err());
        assert!(tramos("Up:-1").is_err());
        let mut t = Teclado::default();
        t.poner(&["Up".into()]);
        assert!(t.paso((0., 0.), [false; 3]).pressed.contains("Up"));
        t.poner(&["Up".into(), "Left".into()]);
        let b = t.paso((0., 0.), [false; 3]);
        assert_eq!((b.pressed, b.down.len()), (set(&["Left"]), 2));
        t.poner(&[]);
        assert_eq!(t.paso((0., 0.), [false; 3]).released, set(&["Left", "Up"]));
    }
}
