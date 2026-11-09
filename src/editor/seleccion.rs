//! Selección múltiple en el lienzo: geometría de instancias (giro, origen), alinear, distribuir,
//! manijas de escala y giro, y el portapapeles de instancias. Lo puro tiene pruebas.

use eframe::egui::{Pos2, Rect, Vec2};
use nicostation::model::{Instance, ObjectDef};
use serde::{Deserialize, Serialize};

/// Dónde está una instancia en el mundo: posición guardada, tamaño, giro y punto de giro.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geo {
    /// x, y de la instancia.
    pub pos: Vec2,
    pub size: Vec2,
    /// Grados.
    pub angle: f32,
    /// Origen normalizado (0..1 del tamaño); `None` = sin origen: x, y es la esquina y gira en el centro.
    pub origin: Option<Vec2>,
}

impl Geo {
    /// Punto normalizado alrededor del que gira.
    pub fn u(&self) -> Vec2 {
        self.origin.unwrap_or(Vec2::splat(0.5))
    }
    /// Punto de giro en el mundo.
    pub fn pivot(&self) -> Vec2 {
        if self.origin.is_some() { self.pos } else { self.pos + self.size / 2. }
    }
    /// Posición (x, y) que corresponde a un punto de giro.
    pub fn pos_de_pivot(&self, pivot: Vec2) -> Vec2 {
        if self.origin.is_some() { pivot } else { pivot - self.size / 2. }
    }
    /// Esquina (fx, fy ∈ {0, 1}) o cualquier punto normalizado del rectángulo, en el mundo.
    pub fn punto(&self, f: Vec2) -> Vec2 {
        let u = self.u();
        self.pivot() + rot(Vec2::new((f.x - u.x) * self.size.x, (f.y - u.y) * self.size.y), self.angle)
    }
    /// Las cuatro esquinas (sup. izq., sup. der., inf. der., inf. izq.).
    pub fn quad(&self) -> [Vec2; 4] {
        [(0., 0.), (1., 0.), (1., 1.), (0., 1.)].map(|(x, y)| self.punto(Vec2::new(x, y)))
    }
    /// Caja alineada a los ejes que contiene la instancia (girada).
    pub fn aabb(&self) -> Rect {
        let q = self.quad();
        let mut r = Rect::from_min_max(q[0].to_pos2(), q[0].to_pos2());
        q.iter().for_each(|p| r.extend_with(p.to_pos2()));
        r
    }
    /// ¿Está el punto dentro del rectángulo girado?
    pub fn contiene(&self, p: Vec2) -> bool {
        let u = self.u();
        let l = rot(p - self.pivot(), -self.angle);
        let (x, y) = (l.x / self.size.x.max(1e-6) + u.x, l.y / self.size.y.max(1e-6) + u.y);
        (0. ..=1.).contains(&x) && (0. ..=1.).contains(&y)
    }
}

pub fn rot(v: Vec2, grados: f32) -> Vec2 {
    let (s, c) = grados.to_radians().sin_cos();
    Vec2::new(v.x * c - v.y * s, v.x * s + v.y * c)
}

// ---------- alinear y distribuir ----------

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Alinear {
    Izquierda,
    CentroH,
    Derecha,
    Arriba,
    CentroV,
    Abajo,
}

/// Cuánto mover cada caja para alinearlas (con respecto a la caja que las contiene a todas).
pub fn alinear(cajas: &[Rect], modo: Alinear) -> Vec<Vec2> {
    let Some(todo) = union(cajas) else { return vec![] };
    cajas
        .iter()
        .map(|r| match modo {
            Alinear::Izquierda => Vec2::new(todo.left() - r.left(), 0.),
            Alinear::Derecha => Vec2::new(todo.right() - r.right(), 0.),
            Alinear::CentroH => Vec2::new(todo.center().x - r.center().x, 0.),
            Alinear::Arriba => Vec2::new(0., todo.top() - r.top()),
            Alinear::Abajo => Vec2::new(0., todo.bottom() - r.bottom()),
            Alinear::CentroV => Vec2::new(0., todo.center().y - r.center().y),
        })
        .collect()
}

/// Reparte las cajas con el mismo espacio entre ellas (la primera y la última no se mueven).
/// `horizontal`: a lo ancho; si no, a lo alto. Con menos de 3 cajas no mueve nada.
pub fn distribuir(cajas: &[Rect], horizontal: bool) -> Vec<Vec2> {
    let n = cajas.len();
    let mut out = vec![Vec2::ZERO; n];
    if n < 3 {
        return out;
    }
    let lo = |r: &Rect| if horizontal { r.left() } else { r.top() };
    let tam = |r: &Rect| if horizontal { r.width() } else { r.height() };
    let mut orden: Vec<usize> = (0..n).collect();
    orden.sort_by(|&a, &b| (lo(&cajas[a]) + tam(&cajas[a]) / 2.).total_cmp(&(lo(&cajas[b]) + tam(&cajas[b]) / 2.)));
    let (a, z) = (&cajas[orden[0]], &cajas[orden[n - 1]]);
    let span = lo(z) + tam(z) - lo(a);
    let ocupado: f32 = cajas.iter().map(tam).sum();
    let hueco = (span - ocupado) / (n - 1) as f32;
    let mut x = lo(a);
    for &k in &orden {
        let d = x - lo(&cajas[k]);
        out[k] = if horizontal { Vec2::new(d, 0.) } else { Vec2::new(0., d) };
        x += tam(&cajas[k]) + hueco;
    }
    out
}

pub fn union(cajas: &[Rect]) -> Option<Rect> {
    let mut it = cajas.iter();
    let first = *it.next()?;
    Some(it.fold(first, |a, b| a.union(*b)))
}

// ---------- manijas ----------

/// Qué se está arrastrando en el lienzo.
#[derive(Clone, Debug, PartialEq)]
pub enum Arrastre {
    /// Mover: posiciones al empezar y punto de partida (mundo).
    Mover { ini: Vec<(usize, Vec2)>, desde: Vec2 },
    /// Escalar desde la esquina `esq` (0..4, en el orden de `Geo::quad`).
    Escalar { esq: usize, ini: Vec<(usize, Geo)>, caja: Rect },
    /// Girar alrededor de `centro`; ángulo del puntero al empezar.
    Girar { centro: Vec2, a0: f32, ini: Vec<(usize, Geo)> },
    /// Rectángulo de selección (mundo); `sumar` = con Shift (se agrega a lo elegido).
    Caja { desde: Vec2, hasta: Vec2, sumar: bool },
}

/// Escala una sola instancia desde una esquina: la esquina opuesta queda quieta.
/// `p` es el puntero (mundo); `aspecto` mantiene la proporción. Devuelve (pos, tamaño).
pub fn escalar_una(g: &Geo, esq: usize, p: Vec2, aspecto: bool) -> (Vec2, Vec2) {
    let f = esquina(esq);
    let fija = Vec2::new(1. - f.x, 1. - f.y);
    let fw = g.punto(fija);
    // El puntero en el sistema de la instancia, desde la esquina fija.
    let l = rot(p - fw, -g.angle);
    let sx = if f.x > fija.x { 1. } else { -1. };
    let sy = if f.y > fija.y { 1. } else { -1. };
    let mut size = Vec2::new((l.x * sx).max(1.), (l.y * sy).max(1.));
    if aspecto && g.size.x > 0. && g.size.y > 0. {
        let k = (size.x / g.size.x).max(size.y / g.size.y);
        size = g.size * k;
    }
    let ng = Geo { size, ..*g };
    // Con el tamaño nuevo, la esquina fija tiene que seguir en `fw`.
    let u = ng.u();
    let pivot = fw - rot(Vec2::new((fija.x - u.x) * size.x, (fija.y - u.y) * size.y), g.angle);
    (ng.pos_de_pivot(pivot), size)
}

/// Escala varias instancias con la caja que las contiene (la esquina opuesta queda quieta).
pub fn escalar_varias(ini: &[(usize, Geo)], caja: Rect, esq: usize, p: Vec2, aspecto: bool) -> Vec<(usize, Vec2, Vec2)> {
    let f = esquina(esq);
    let fija = Vec2::new(caja.min.x + (1. - f.x) * caja.width(), caja.min.y + (1. - f.y) * caja.height());
    let orig = Vec2::new(caja.min.x + f.x * caja.width(), caja.min.y + f.y * caja.height());
    let d0 = orig - fija;
    let d1 = p - fija;
    let mut k = Vec2::new(if d0.x.abs() > 1e-3 { d1.x / d0.x } else { 1. }, if d0.y.abs() > 1e-3 { d1.y / d0.y } else { 1. });
    k = k.max(Vec2::splat(0.02));
    if aspecto {
        k = Vec2::splat(k.x.max(k.y));
    }
    ini.iter()
        .map(|(i, g)| {
            let size = (g.size * k).max(Vec2::splat(1.));
            let pivot = fija + (g.pivot() - fija) * k;
            let ng = Geo { size, ..*g };
            (*i, ng.pos_de_pivot(pivot), size)
        })
        .collect()
}

/// Gira instancias alrededor de `centro`: cada una gira sobre sí misma y su punto de giro da la vuelta.
/// Con `paso` (Shift) el giro va de a 15°. Devuelve (índice, pos, ángulo).
pub fn girar(ini: &[(usize, Geo)], centro: Vec2, a0: f32, p: Vec2, paso: bool) -> Vec<(usize, Vec2, f32)> {
    let a = (p - centro).y.atan2((p - centro).x).to_degrees();
    let mut d = a - a0;
    if paso {
        d = (d / 15.).round() * 15.;
    }
    ini.iter()
        .map(|(i, g)| {
            let mut ang = g.angle + d;
            if paso && ini.len() == 1 {
                ang = (ang / 15.).round() * 15.;
            }
            let ang = norm_ang(ang);
            let pivot = centro + rot(g.pivot() - centro, if ini.len() == 1 { 0. } else { d });
            (*i, g.pos_de_pivot(pivot), ang)
        })
        .collect()
}

/// Ángulo entre -180 y 180 (los proyectos guardan números cortos).
pub fn norm_ang(a: f32) -> f32 {
    let r = (a + 180.).rem_euclid(360.) - 180.;
    if (r - r.round()).abs() < 1e-3 { r.round() } else { r }
}

/// Esquina normalizada en el orden de `Geo::quad`.
pub fn esquina(k: usize) -> Vec2 {
    [Vec2::new(0., 0.), Vec2::new(1., 0.), Vec2::new(1., 1.), Vec2::new(0., 1.)][k % 4]
}

// ---------- portapapeles ----------

/// Lo copiado: instancias (posición relativa a la primera) y los objetos que hacen falta para pegarlas en otra escena.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Portapapeles {
    pub instancias: Vec<Instance>,
    pub objetos: Vec<ObjectDef>,
}

/// Marca del texto en el portapapeles del sistema.
pub const MARCA: &str = "nicostation-instancias:";

impl Portapapeles {
    pub fn texto(&self) -> String {
        format!("{MARCA}{}", serde_json::to_string(self).unwrap_or_default())
    }
    pub fn de_texto(s: &str) -> Option<Portapapeles> {
        serde_json::from_str(s.trim().strip_prefix(MARCA)?).ok()
    }
    /// Esquina de lo copiado (mínimo x, y).
    pub fn esquina(&self) -> Vec2 {
        self.instancias.iter().fold(Vec2::splat(f32::MAX), |m, i| m.min(Vec2::new(i.x, i.y)))
    }
}

/// Pasa un rectángulo del mundo a pantalla.
pub fn a_pantalla(r: Rect, org: Pos2, z: f32) -> Rect {
    Rect::from_min_max(org + r.min.to_vec2() * z, org + r.max.to_vec2() * z)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h))
    }

    #[test]
    fn alinear_a_la_caja_comun() {
        let c = [r(0., 0., 10., 10.), r(50., 20., 20., 10.), r(30., 100., 10., 40.)];
        assert_eq!(alinear(&c, Alinear::Izquierda), vec![Vec2::ZERO, Vec2::new(-50., 0.), Vec2::new(-30., 0.)]);
        assert_eq!(alinear(&c, Alinear::Derecha), vec![Vec2::new(60., 0.), Vec2::new(0., 0.), Vec2::new(30., 0.)]);
        assert_eq!(alinear(&c, Alinear::CentroH), vec![Vec2::new(30., 0.), Vec2::new(-25., 0.), Vec2::new(0., 0.)]);
        assert_eq!(alinear(&c, Alinear::Arriba), vec![Vec2::ZERO, Vec2::new(0., -20.), Vec2::new(0., -100.)]);
        assert_eq!(alinear(&c, Alinear::Abajo), vec![Vec2::new(0., 130.), Vec2::new(0., 110.), Vec2::ZERO]);
        assert_eq!(alinear(&c, Alinear::CentroV)[0], Vec2::new(0., 65.));
        assert!(alinear(&[], Alinear::Izquierda).is_empty());
    }

    #[test]
    fn distribuir_con_huecos_iguales() {
        // Anchos 10, 20, 10 entre x=0 y x=100: hueco (100 - 40) / 2 = 30.
        let c = [r(0., 0., 10., 5.), r(90., 0., 10., 5.), r(15., 9., 20., 5.)];
        let d = distribuir(&c, true);
        assert_eq!(d[0], Vec2::ZERO);
        assert_eq!(d[1], Vec2::ZERO);
        assert_eq!(d[2], Vec2::new(25., 0.)); // queda en x = 40
        let v = distribuir(&[r(0., 0., 5., 10.), r(0., 12., 5., 10.), r(0., 90., 5., 10.)], false);
        assert_eq!(v[1], Vec2::new(0., 33.)); // y = 45: hueco (100 - 30) / 2 = 35
        // Con dos no se mueve nada.
        assert_eq!(distribuir(&c[..2], true), vec![Vec2::ZERO; 2]);
    }

    #[test]
    fn geometria_con_y_sin_origen() {
        let g = Geo { pos: Vec2::new(10., 20.), size: Vec2::new(40., 20.), angle: 0., origin: None };
        assert_eq!(g.pivot(), Vec2::new(30., 30.));
        assert_eq!(g.quad()[2], Vec2::new(50., 40.));
        assert!(g.contiene(Vec2::new(11., 21.)) && !g.contiene(Vec2::new(9., 21.)));
        // Girada 90°: el rectángulo queda parado alrededor del centro.
        let g90 = Geo { angle: 90., ..g };
        let a = g90.aabb();
        assert!((a.width() - 20.).abs() < 1e-3 && (a.height() - 40.).abs() < 1e-3);
        assert!((a.center() - Pos2::new(30., 30.)).length() < 1e-3);
        // Con origen abajo al medio: x, y es ese punto.
        let o = Geo { pos: Vec2::new(100., 100.), size: Vec2::new(20., 30.), angle: 0., origin: Some(Vec2::new(0.5, 1.)) };
        assert_eq!(o.quad()[0], Vec2::new(90., 70.));
        assert_eq!(o.pos_de_pivot(o.pivot()), o.pos);
    }

    #[test]
    fn escalar_deja_quieta_la_esquina_opuesta() {
        let g = Geo { pos: Vec2::new(0., 0.), size: Vec2::new(10., 10.), angle: 0., origin: None };
        // Arrastrar la esquina inferior derecha a (30, 20).
        let (pos, size) = escalar_una(&g, 2, Vec2::new(30., 20.), false);
        assert_eq!((pos, size), (Vec2::ZERO, Vec2::new(30., 20.)));
        // Con Shift: mantiene la proporción.
        let (_, size) = escalar_una(&g, 2, Vec2::new(30., 20.), true);
        assert_eq!(size, Vec2::new(30., 30.));
        // Esquina superior izquierda: la inferior derecha (10, 10) queda quieta.
        let (pos, size) = escalar_una(&g, 0, Vec2::new(-10., 5.), false);
        assert_eq!((pos, size), (Vec2::new(-10., 5.), Vec2::new(20., 5.)));
        // Girada 30°: la esquina fija no se mueve.
        let gr = Geo { angle: 30., origin: Some(Vec2::new(0.5, 1.)), ..g };
        let fija = gr.punto(Vec2::new(0., 0.));
        let (pos, size) = escalar_una(&gr, 2, gr.punto(Vec2::new(1., 1.)) + Vec2::new(5., 5.), false);
        let n = Geo { pos, size, ..gr };
        assert!((n.punto(Vec2::new(0., 0.)) - fija).length() < 1e-3);
    }

    #[test]
    fn escalar_y_girar_varias() {
        let a = Geo { pos: Vec2::new(0., 0.), size: Vec2::new(10., 10.), angle: 0., origin: None };
        let b = Geo { pos: Vec2::new(20., 0.), ..a };
        let caja = r(0., 0., 30., 10.);
        let v = escalar_varias(&[(0, a), (1, b)], caja, 2, Vec2::new(60., 20.), false);
        assert_eq!(v[0], (0, Vec2::ZERO, Vec2::new(20., 20.)));
        assert_eq!(v[1], (1, Vec2::new(40., 0.), Vec2::new(20., 20.)));
        // Girar 90° alrededor del centro (15, 5): el de la izquierda pasa arriba.
        let g = girar(&[(0, a), (1, b)], Vec2::new(15., 5.), 0., Vec2::new(15., 50.), false);
        assert_eq!(g[0].2, 90.);
        let p0 = Geo { pos: g[0].1, angle: 90., ..a }.pivot();
        assert!((p0 - Vec2::new(15., -5.)).length() < 1e-3);
        // Una sola con Shift: ángulo redondo de 15 en 15.
        let g = girar(&[(0, a)], a.pivot(), 0., a.pivot() + Vec2::new(10., 2.9), true);
        assert_eq!(g[0].2, 15.);
        assert_eq!(g[0].1, a.pos);
        assert_eq!(norm_ang(270.), -90.);
    }

    #[test]
    fn portapapeles_ida_y_vuelta() {
        let p = Portapapeles { instancias: vec![Instance { obj: "A".into(), x: 5., y: 7., ..Default::default() }], objetos: vec![] };
        let t = p.texto();
        assert!(t.starts_with(MARCA));
        let q = Portapapeles::de_texto(&t).unwrap();
        assert_eq!(q.instancias[0].obj, "A");
        assert_eq!(q.esquina(), Vec2::new(5., 7.));
        assert!(Portapapeles::de_texto("hola").is_none());
    }
}
