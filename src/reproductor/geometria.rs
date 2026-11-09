//! Cuentas puras del reproductor 2D (sin ventana): escalado a la pantalla, paralaje, fondos que se repiten y
//! alineación de texto. Todas tienen pruebas. (Partir texto, origen de sprites y aspecto de partículas son del
//! motor: `runtime::wrap_lines`, `Inst::draw_rect`, `particulas::particle_look`.)

/// Dónde y a qué escala se dibuja el área del juego dentro de la ventana.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    /// Esquina superior izquierda del área de juego en la ventana.
    pub off: (f32, f32),
    /// Píxeles de ventana por píxel de juego (x, y). Solo «stretch» los hace distintos.
    pub scale: (f32, f32),
}

/// `fit`: lo más grande posible sin deformar, con franjas; `stretch`: llena la ventana deformando;
/// `integer`: múltiplo entero (pixel perfect) centrado; si la ventana es más chica que el juego, como `fit`.
pub fn viewport(mode: &str, sw: f32, sh: f32, pw: f32, ph: f32) -> Viewport {
    let (pw, ph) = (pw.max(1.), ph.max(1.));
    let fit = (sw / pw).min(sh / ph);
    let (sx, sy) = match mode {
        "stretch" => (sw / pw, sh / ph),
        "integer" if fit >= 1. => (fit.floor(), fit.floor()),
        _ => (fit, fit),
    };
    let off = ((sw - pw * sx) / 2., (sh - ph * sy) / 2.);
    // En modo entero, también la posición en píxeles enteros: si no, el pixel art se ve borroso.
    let off = if mode == "integer" { (off.0.floor(), off.1.floor()) } else { off };
    Viewport { off, scale: (sx, sy) }
}

/// Centro de la cámara para una capa con paralaje. Con [1, 1] es la cámara del juego; con [0, 0] la capa
/// queda quieta donde se ve al empezar (cámara en el centro del juego); con 0.5 se mueve a la mitad.
pub fn parallax_camera(cam: (f32, f32), parallax: [f32; 2], pw: f32, ph: f32) -> (f32, f32) {
    let home = (pw / 2., ph / 2.);
    (home.0 + (cam.0 - home.0) * parallax[0], home.1 + (cam.1 - home.1) * parallax[1])
}

/// Un trozo de baldosa: destino en el mundo (x, y, w, h) y origen en la imagen (x, y, w, h).
pub type Tile = ([f32; 4], [f32; 4]);

/// Tramos 1D: el área [a0, a1) se llena con copias de largo `t` que empiezan en `origin` (más/menos múltiplos de t).
/// Devuelve (inicio en destino, largo, inicio en la imagen).
fn spans(a0: f32, a1: f32, origin: f32, t: f32) -> Vec<(f32, f32, f32)> {
    let mut out = vec![];
    if t <= 0. || a1 <= a0 {
        return out;
    }
    let mut x = origin - ((origin - a0) / t).ceil() * t; // primera copia que empieza en a0 o antes
    while x < a1 && out.len() < 100_000 {
        let (s, e) = (x.max(a0), (x + t).min(a1));
        if e > s {
            out.push((s, e - s, s - x));
        }
        x += t;
    }
    out
}

/// Baldosas para llenar `area` (x, y, w, h) con una imagen de `tex` (w, h) corrida `offset` píxeles
/// (desplazamiento automático), dibujando solo lo que cae dentro de `view` (lo que ve la cámara).
pub fn tiles(area: [f32; 4], tex: (f32, f32), offset: (f32, f32), view: [f32; 4]) -> Vec<Tile> {
    let x0 = area[0].max(view[0]);
    let x1 = (area[0] + area[2]).min(view[0] + view[2]);
    let y0 = area[1].max(view[1]);
    let y1 = (area[1] + area[3]).min(view[1] + view[3]);
    let ox = area[0] + offset.0.rem_euclid(tex.0.max(1e-3));
    let oy = area[1] + offset.1.rem_euclid(tex.1.max(1e-3));
    let (cols, rows) = (spans(x0, x1, ox, tex.0), spans(y0, y1, oy, tex.1));
    let mut out = Vec::with_capacity(cols.len() * rows.len());
    for &(y, h, sy) in &rows {
        for &(x, w, sx) in &cols {
            out.push(([x, y, w, h], [sx, sy, w, h]));
        }
    }
    out
}

/// Corrimiento horizontal de una línea de ancho `line_w` dentro de un bloque de ancho `block_w`.
pub fn align_x(align: &str, line_w: f32, block_w: f32) -> f32 {
    match align {
        "center" => (block_w - line_w) / 2.,
        "right" => block_w - line_w,
        _ => 0.,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escalado_fit_stretch_integer() {
        // 320x180 en una ventana de 1000x700.
        let v = viewport("fit", 1000., 700., 320., 180.);
        assert_eq!(v.scale, (3.125, 3.125));
        assert_eq!(v.off, (0., (700. - 180. * 3.125) / 2.));
        let v = viewport("stretch", 1000., 700., 320., 180.);
        assert_eq!(v.scale, (1000. / 320., 700. / 180.));
        assert_eq!(v.off, (0., 0.));
        let v = viewport("integer", 1000., 700., 320., 180.);
        assert_eq!(v.scale, (3., 3.));
        assert_eq!(v.off, (20., 80.)); // (1000-960)/2, (700-540)/2
        // Ventana más chica que el juego: como fit (no se escala a 0).
        let v = viewport("integer", 200., 150., 320., 180.);
        assert_eq!(v.scale, (0.625, 0.625));
        // Modo desconocido = fit.
        assert_eq!(viewport("raro", 800., 600., 400., 300.), viewport("fit", 800., 600., 400., 300.));
    }

    #[test]
    fn paralaje() {
        let (w, h) = (800., 600.);
        // Al empezar (cámara en el centro) todas las capas coinciden.
        assert_eq!(parallax_camera((400., 300.), [0.5, 0.2], w, h), (400., 300.));
        assert_eq!(parallax_camera((1400., 300.), [1., 1.], w, h), (1400., 300.));
        assert_eq!(parallax_camera((1400., 900.), [0.5, 0.], w, h), (900., 300.));
        assert_eq!(parallax_camera((1400., 900.), [0., 0.], w, h), (400., 300.));
    }

    #[test]
    fn baldosas_cubren_el_area_visible() {
        // Área 100x50 con imagen 32x32, sin corrimiento, todo visible.
        let t = tiles([10., 20., 100., 50.], (32., 32.), (0., 0.), [-1000., -1000., 5000., 5000.]);
        let area: f32 = t.iter().map(|(d, _)| d[2] * d[3]).sum();
        assert!((area - 5000.).abs() < 1e-3);
        assert_eq!(t[0], ([10., 20., 32., 32.], [0., 0., 32., 32.]));
        // Columnas: 32+32+32+4.
        assert_eq!(t.iter().filter(|(d, _)| d[1] == 20.).count(), 4);
        for (d, s) in &t {
            assert!(s[0] >= 0. && s[0] + s[2] <= 32.001 && s[1] >= 0. && s[1] + s[3] <= 32.001, "{d:?} {s:?}");
            assert_eq!((d[2], d[3]), (s[2], s[3]), "sin estirar");
        }
        // Corrido 8 px a la derecha: la primera columna muestra el final de la imagen.
        let t = tiles([0., 0., 64., 32.], (32., 32.), (8., 0.), [0., 0., 64., 32.]);
        assert_eq!(t[0], ([0., 0., 8., 32.], [24., 0., 8., 32.]));
        assert_eq!(t[1], ([8., 0., 32., 32.], [0., 0., 32., 32.]));
        // Corrimiento negativo y grande da lo mismo que su resto.
        assert_eq!(tiles([0., 0., 64., 32.], (32., 32.), (8. - 320., 0.), [0., 0., 64., 32.]), t);
        // Solo lo visible: cámara mirando un trozo de un fondo enorme.
        let t = tiles([0., 0., 100000., 600.], (64., 64.), (0., 0.), [5000., 0., 800., 600.]);
        assert!(t.len() <= 14 * 10, "{}", t.len());
        assert!(t.iter().all(|(d, _)| d[0] >= 5000. && d[0] + d[2] <= 5800.));
        // Fuera de la vista: nada.
        assert!(tiles([0., 0., 100., 100.], (32., 32.), (0., 0.), [500., 500., 10., 10.]).is_empty());
    }

    #[test]
    fn alinear() {
        assert_eq!(align_x("left", 30., 100.), 0.);
        assert_eq!(align_x("center", 30., 100.), 35.);
        assert_eq!(align_x("right", 30., 100.), 70.);
    }
}
