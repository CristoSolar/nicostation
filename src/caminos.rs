//! Búsqueda de caminos (comportamiento `Pathfinder`): A* en una grilla de celdas de `cell` px.
//! Las celdas bloqueadas son las donde el objeto (su caja sin girar, centrada en la celda) tocaría una forma
//! de un `Obstacle`; así el camino ya deja lugar para el tamaño del objeto. Después se suaviza (se saltan
//! los puntos intermedios cuando hay línea recta libre) y el último punto es el destino exacto.

use crate::runtime::sat;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// Máximo de celdas de una grilla: si el área es más grande, las celdas se agrandan.
pub const MAX_CELDAS: usize = 40_000;

/// Estado de un objeto con Pathfinder (lo guarda su instancia).
#[derive(Clone, Debug, Default)]
pub struct Camino {
    /// Destino pedido con `move_to`.
    pub destino: (f32, f32),
    /// Puntos que faltan (centro del objeto), el último es el destino.
    pub puntos: Vec<(f32, f32)>,
    /// La última búsqueda encontró camino.
    pub encontrado: bool,
    /// Llegó al destino (y no se pidió otro).
    pub llego: bool,
    /// Tamaño de celda con el que se buscó (para saber si el destino cambió de celda).
    pub celda: f32,
}

impl Camino {
    /// Lo que falta recorrer en px desde (x, y).
    pub fn largo(&self, x: f32, y: f32) -> f32 {
        let mut prev = (x, y);
        let mut s = 0.;
        for p in &self.puntos {
            s += (p.0 - prev.0).hypot(p.1 - prev.1);
            prev = *p;
        }
        s
    }
}

pub struct Grilla {
    pub x0: f32,
    pub y0: f32,
    pub cell: f32,
    pub cols: i32,
    pub rows: i32,
    bloq: Vec<bool>,
    /// Obstáculos con su caja (x0, y0, x1, y1) ya agrandada por el medio tamaño del objeto.
    obst: Vec<(Poly, (f32, f32, f32, f32))>,
    medio: (f32, f32),
}

type Poly = Vec<(f32, f32)>;

fn caja(cx: f32, cy: f32, hw: f32, hh: f32) -> [(f32, f32); 4] {
    [(cx - hw, cy - hh), (cx + hw, cy - hh), (cx + hw, cy + hh), (cx - hw, cy + hh)]
}

impl Grilla {
    /// Grilla que cubre `area` (x0, y0, x1, y1) con celdas de al menos `cell` px, para un objeto de medio tamaño
    /// (hw, hh) que esquiva los polígonos `obstaculos`.
    pub fn nueva(area: (f32, f32, f32, f32), cell: f32, obstaculos: &[Poly], medio: (f32, f32)) -> Grilla {
        let (w, h) = ((area.2 - area.0).max(1.), (area.3 - area.1).max(1.));
        let mut cell = cell.max(2.);
        while (w / cell).ceil() * (h / cell).ceil() > MAX_CELDAS as f32 {
            cell *= 1.5;
        }
        let (cols, rows) = ((w / cell).ceil() as i32, (h / cell).ceil() as i32);
        // Un pelito más chico que el objeto: que pueda pasar justo por un hueco de su tamaño.
        let (hw, hh) = ((medio.0 - 0.01).max(0.5), (medio.1 - 0.01).max(0.5));
        let mut g = Grilla { x0: area.0, y0: area.1, cell, cols, rows, bloq: vec![false; (cols * rows) as usize], obst: vec![], medio: (hw, hh) };
        for p in obstaculos.iter().filter(|p| p.len() >= 3) {
            let (bx0, by0, bx1, by1) = p.iter().fold((f32::MAX, f32::MAX, f32::MIN, f32::MIN), |b, q| (b.0.min(q.0), b.1.min(q.1), b.2.max(q.0), b.3.max(q.1)));
            g.obst.push((p.clone(), (bx0 - hw, by0 - hh, bx1 + hw, by1 + hh)));
            let c0 = (((bx0 - hw - g.x0) / cell).floor() as i32).max(0);
            let c1 = (((bx1 + hw - g.x0) / cell).floor() as i32).min(cols - 1);
            let r0 = (((by0 - hh - g.y0) / cell).floor() as i32).max(0);
            let r1 = (((by1 + hh - g.y0) / cell).floor() as i32).min(rows - 1);
            for r in r0..=r1 {
                for c in c0..=c1 {
                    let k = (r * cols + c) as usize;
                    if g.bloq[k] {
                        continue;
                    }
                    let (cx, cy) = g.centro((c, r));
                    if sat(&caja(cx, cy, hw, hh), p).is_some() {
                        g.bloq[k] = true;
                    }
                }
            }
        }
        g
    }

    pub fn celda(&self, x: f32, y: f32) -> Option<(i32, i32)> {
        let (c, r) = (((x - self.x0) / self.cell).floor() as i32, ((y - self.y0) / self.cell).floor() as i32);
        (c >= 0 && r >= 0 && c < self.cols && r < self.rows).then_some((c, r))
    }

    pub fn centro(&self, (c, r): (i32, i32)) -> (f32, f32) {
        (self.x0 + (c as f32 + 0.5) * self.cell, self.y0 + (r as f32 + 0.5) * self.cell)
    }

    pub fn libre(&self, (c, r): (i32, i32)) -> bool {
        c >= 0 && r >= 0 && c < self.cols && r < self.rows && !self.bloq[(r * self.cols + c) as usize]
    }

    /// El objeto centrado en (x, y) no toca ningún obstáculo. Cerca de celdas bloqueadas se prueba la forma exacta.
    fn seguro(&self, x: f32, y: f32, start: Option<(i32, i32)>) -> bool {
        let Some(c) = self.celda(x, y) else { return false };
        if Some(c) == start {
            return true;
        }
        if !self.libre(c) {
            return false;
        }
        let near = (-1..=1).any(|dy| (-1..=1).any(|dx| {
            let n = (c.0 + dx, c.1 + dy);
            n.0 >= 0 && n.1 >= 0 && n.0 < self.cols && n.1 < self.rows && self.bloq[(n.1 * self.cols + n.0) as usize]
        }));
        !near || self.obst.iter().filter(|(_, b)| x > b.0 && x < b.2 && y > b.1 && y < b.3).all(|(p, _)| sat(&caja(x, y, self.medio.0, self.medio.1), p).is_none())
    }

    /// Se puede ir en línea recta (la celda de partida no cuenta: el objeto puede estar rozando algo).
    fn visible(&self, a: (f32, f32), b: (f32, f32)) -> bool {
        let start = self.celda(a.0, a.1);
        let d = (b.0 - a.0).hypot(b.1 - a.1);
        let n = (d / (self.cell / 4.).min(2.)).ceil().max(1.) as usize;
        (0..=n).all(|k| {
            let t = k as f32 / n as f32;
            self.seguro(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, start)
        })
    }

    /// Camino de `desde` a `hasta` (puntos del mundo, sin el de partida; el último es `hasta`). None = no hay.
    pub fn buscar(&self, desde: (f32, f32), hasta: (f32, f32), diagonales: bool) -> Option<Vec<(f32, f32)>> {
        let s = self.celda(desde.0, desde.1)?;
        let t = self.celda(hasta.0, hasta.1)?;
        if !self.libre(t) {
            return None;
        }
        if s == t || self.visible(desde, hasta) {
            return Some(vec![hasta]);
        }
        let idx = |c: (i32, i32)| (c.1 * self.cols + c.0) as usize;
        let n = (self.cols * self.rows) as usize;
        let mut g = vec![f32::INFINITY; n];
        let mut from = vec![u32::MAX; n];
        let mut open = BinaryHeap::new();
        let h = |c: (i32, i32)| {
            let (dx, dy) = ((c.0 - t.0).abs() as f32, (c.1 - t.1).abs() as f32);
            if diagonales { dx.max(dy) + (std::f32::consts::SQRT_2 - 1.) * dx.min(dy) } else { dx + dy }
        };
        g[idx(s)] = 0.;
        open.push(Nodo { f: h(s), c: s });
        const D: f32 = std::f32::consts::SQRT_2;
        const VEC: [(i32, i32, f32); 8] = [(1, 0, 1.), (-1, 0, 1.), (0, 1, 1.), (0, -1, 1.), (1, 1, D), (1, -1, D), (-1, 1, D), (-1, -1, D)];
        let mut found = false;
        while let Some(Nodo { f, c }) = open.pop() {
            if c == t {
                found = true;
                break;
            }
            let gc = g[idx(c)];
            if f > gc + h(c) + 1e-3 {
                continue; // entrada vieja del montón
            }
            for &(dx, dy, cost) in VEC.iter().take(if diagonales { 8 } else { 4 }) {
                let nc = (c.0 + dx, c.1 + dy);
                if !self.libre(nc) {
                    continue;
                }
                // En diagonal no se cortan esquinas.
                if dx != 0 && dy != 0 && (!self.libre((c.0 + dx, c.1)) || !self.libre((c.0, c.1 + dy))) {
                    continue;
                }
                let ng = gc + cost;
                if ng < g[idx(nc)] {
                    g[idx(nc)] = ng;
                    from[idx(nc)] = idx(c) as u32;
                    open.push(Nodo { f: ng + h(nc), c: nc });
                }
            }
        }
        if !found {
            return None;
        }
        // Celdas desde el destino hacia atrás.
        let mut cells = vec![t];
        let mut k = idx(t);
        while from[k] != u32::MAX {
            k = from[k] as usize;
            cells.push(((k as i32) % self.cols, (k as i32) / self.cols));
        }
        cells.reverse();
        let mut pts: Vec<(f32, f32)> = cells.iter().skip(1).map(|&c| self.centro(c)).collect();
        if let Some(last) = pts.last_mut() {
            *last = hasta;
        }
        // Suavizado: desde cada punto, saltar al más lejano que se vea en línea recta.
        let mut out = vec![];
        let mut cur = desde;
        let mut i = 0;
        while i < pts.len() {
            let mut j = pts.len() - 1;
            while j > i && !self.visible(cur, pts[j]) {
                j -= 1;
            }
            out.push(pts[j]);
            cur = pts[j];
            i = j + 1;
        }
        Some(out)
    }
}

#[derive(PartialEq)]
struct Nodo {
    f: f32,
    c: (i32, i32),
}
impl Eq for Nodo {}
impl Ord for Nodo {
    fn cmp(&self, o: &Self) -> Ordering {
        o.f.total_cmp(&self.f).then_with(|| self.c.cmp(&o.c))
    }
}
impl PartialOrd for Nodo {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rodea_una_pared() {
        // Pared vertical de x 190..210, y 0..300, en un área de 400x400. Objeto de 20x20.
        let pared = vec![(190., 0.), (210., 0.), (210., 300.), (190., 300.)];
        let g = Grilla::nueva((0., 0., 400., 400.), 20., &[pared], (10., 10.));
        let p = g.buscar((100., 100.), (300., 100.), true).expect("hay camino por abajo");
        assert_eq!(*p.last().unwrap(), (300., 100.));
        assert!(p.iter().any(|q| q.1 > 300.), "pasa por debajo de la pared: {p:?}");
        // Sin salida: la pared llega hasta abajo.
        let pared = vec![(190., 0.), (210., 0.), (210., 400.), (190., 400.)];
        let g = Grilla::nueva((0., 0., 400., 400.), 20., &[pared], (10., 10.));
        assert!(g.buscar((100., 100.), (300., 100.), true).is_none());
        // Áreas enormes: las celdas crecen para no pasar del máximo.
        let g = Grilla::nueva((0., 0., 100_000., 100_000.), 8., &[], (5., 5.));
        assert!((g.cols * g.rows) as usize <= MAX_CELDAS);
    }
}
