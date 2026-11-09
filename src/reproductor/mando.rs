//! Mandos (gamepads). Los botones llegan al motor como teclas («Pad1A», «Pad1Start», «Pad2Up»…) y los
//! palancas y gatillos como ejes en `Input.axes` («Pad1LX», «Pad1RT»…, de -1 a 1 con zona muerta).
//!
//! Escritorio: gilrs. Web: el plugin `nicostation_gamepad` de `mq_js_bundle.js` (navigator.getGamepads()).
//! Los dos llenan un [`Pad`] con la distribución «standard» del navegador y de ahí salen los nombres.

use std::collections::{HashMap, HashSet};

/// Botones en el orden de la distribución «standard» de la API Gamepad del navegador.
pub const BUTTONS: [&str; 17] = ["A", "B", "X", "Y", "L", "R", "LT", "RT", "Select", "Start", "LS", "RS", "Up", "Down", "Left", "Right", "Home"];
/// Ejes: palanca izquierda x/y, derecha x/y (y positivo = abajo).
pub const AXES: [&str; 4] = ["LX", "LY", "RX", "RY"];
pub const DEADZONE: f32 = 0.15;
/// Se leen hasta dos mandos.
pub const MAX_PADS: usize = 2;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pad {
    /// Valor de cada botón (0 a 1; los gatillos son analógicos).
    pub buttons: [f32; 17],
    pub axes: [f32; 4],
}

/// Zona muerta: por debajo de `dz` es 0, y el resto se reescala para llegar igual a 1.
pub fn deadzone(v: f32, dz: f32) -> f32 {
    let a = v.abs();
    if a <= dz { 0. } else { (v.signum() * (a - dz) / (1. - dz)).clamp(-1., 1.) }
}

/// Teclas apretadas y ejes de los mandos conectados (el primero es Pad1).
pub fn pad_inputs(pads: &[Pad]) -> (HashSet<String>, HashMap<String, f32>) {
    let (mut down, mut axes) = (HashSet::new(), HashMap::new());
    for (i, p) in pads.iter().take(MAX_PADS).enumerate() {
        let n = i + 1;
        for (b, name) in BUTTONS.iter().enumerate() {
            if p.buttons[b] > 0.5 {
                down.insert(format!("Pad{n}{name}"));
            }
        }
        for (a, name) in AXES.iter().enumerate() {
            axes.insert(format!("Pad{n}{name}"), deadzone(p.axes[a], DEADZONE));
        }
        axes.insert(format!("Pad{n}LT"), deadzone(p.buttons[6], DEADZONE));
        axes.insert(format!("Pad{n}RT"), deadzone(p.buttons[7], DEADZONE));
    }
    (down, axes)
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::Native as Platform;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::Pad;
    use gilrs::{Axis, Button, Gilrs};

    /// Botón de gilrs → lugar en la distribución standard.
    const MAP: [(Button, usize); 17] = [
        (Button::South, 0), (Button::East, 1), (Button::West, 2), (Button::North, 3),
        (Button::LeftTrigger, 4), (Button::RightTrigger, 5), (Button::LeftTrigger2, 6), (Button::RightTrigger2, 7),
        (Button::Select, 8), (Button::Start, 9), (Button::LeftThumb, 10), (Button::RightThumb, 11),
        (Button::DPadUp, 12), (Button::DPadDown, 13), (Button::DPadLeft, 14), (Button::DPadRight, 15), (Button::Mode, 16),
    ];

    pub struct Native(Option<Gilrs>);

    impl Native {
        pub fn new() -> Native {
            Native(Gilrs::new().map_err(|e| eprintln!("sin mandos: {e}")).ok())
        }

        pub fn read(&mut self) -> Vec<Pad> {
            let Some(g) = &mut self.0 else { return vec![] };
            while g.next_event().is_some() {} // actualiza el estado
            let mut ids: Vec<_> = g.gamepads().filter(|(_, p)| p.is_connected()).map(|(id, _)| id).collect();
            ids.sort_by_key(|id| usize::from(*id));
            ids.into_iter().take(super::MAX_PADS).map(|id| {
                let gp = g.gamepad(id);
                let mut p = Pad::default();
                for (b, i) in MAP {
                    p.buttons[i] = gp.button_data(b).map_or(0., |d| d.value()).max(if gp.is_pressed(b) { 1. } else { 0. });
                }
                // Cruceta como ejes (algunos mandos la informan así).
                let (dx, dy) = (gp.value(Axis::DPadX), gp.value(Axis::DPadY));
                for (i, on) in [(12, dy > 0.5), (13, dy < -0.5), (14, dx < -0.5), (15, dx > 0.5)] {
                    if on {
                        p.buttons[i] = 1.;
                    }
                }
                // gilrs: y positivo = arriba; en el juego (y en el navegador) y positivo = abajo.
                p.axes = [gp.value(Axis::LeftStickX), -gp.value(Axis::LeftStickY), gp.value(Axis::RightStickX), -gp.value(Axis::RightStickY)];
                p
            }).collect()
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use web::Web as Platform;

#[cfg(target_arch = "wasm32")]
mod web {
    use super::Pad;

    #[link(wasm_import_module = "env")]
    extern "C" {
        /// Escribe 21 f32 (17 botones y 4 ejes) del mando número `pad` (0, 1… entre los conectados) en `out`.
        /// Devuelve 1 si ese mando está conectado.
        fn nicostation_pad_read(pad: i32, out: *mut f32) -> i32;
    }

    pub struct Web;

    impl Web {
        pub fn new() -> Web {
            Web
        }
        pub fn read(&mut self) -> Vec<Pad> {
            let mut out = vec![];
            for i in 0..super::MAX_PADS {
                let mut buf = [0f32; 21];
                if unsafe { nicostation_pad_read(i as i32, buf.as_mut_ptr()) } == 0 {
                    break;
                }
                let mut p = Pad::default();
                p.buttons.copy_from_slice(&buf[..17]);
                p.axes.copy_from_slice(&buf[17..]);
                out.push(p);
            }
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zona_muerta() {
        assert_eq!(deadzone(0.1, 0.15), 0.);
        assert_eq!(deadzone(-0.15, 0.15), 0.);
        assert_eq!(deadzone(1., 0.15), 1.);
        assert_eq!(deadzone(-1., 0.15), -1.);
        assert!((deadzone(0.575, 0.15) - 0.5).abs() < 1e-6);
        assert!((deadzone(-0.575, 0.15) + 0.5).abs() < 1e-6);
    }

    #[test]
    fn nombres_de_botones_y_ejes() {
        let mut a = Pad::default();
        a.buttons[0] = 1.; // A
        a.buttons[9] = 1.; // Start
        a.buttons[12] = 1.; // cruceta arriba
        a.buttons[7] = 0.8; // gatillo derecho
        a.axes = [1., -0.05, 0., 0.5];
        let mut b = Pad::default();
        b.buttons[15] = 1.;
        b.buttons[4] = 0.4; // apenas tocado: no cuenta
        let (down, axes) = pad_inputs(&[a, b, Pad { buttons: [1.; 17], axes: [1.; 4] }]);
        let mut d: Vec<_> = down.into_iter().collect();
        d.sort();
        assert_eq!(d, ["Pad1A", "Pad1RT", "Pad1Start", "Pad1Up", "Pad2Right"], "el tercer mando se ignora");
        assert_eq!(axes["Pad1LX"], 1.);
        assert_eq!(axes["Pad1LY"], 0.);
        assert!((axes["Pad1RY"] - (0.5 - 0.15) / 0.85).abs() < 1e-6);
        assert!((axes["Pad1RT"] - (0.8 - 0.15) / 0.85).abs() < 1e-6);
        assert_eq!(axes["Pad1LT"], 0.);
        assert_eq!(axes["Pad2LX"], 0.);
        assert_eq!(axes.len(), 12);
        assert!(pad_inputs(&[]).0.is_empty());
    }
}
