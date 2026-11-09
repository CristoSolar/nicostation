//! Partículas: los emisores (objetos `particles`) sueltan partículas en `World::particles`; el motor las mueve
//! y las envejece. Los reproductores y el editor las dibujan con `particle_look`, así se ven igual en todos lados.

use crate::model::ParticleDef;
use crate::runtime::Particle;

/// Cómo se ve una partícula según su edad: (color, opacidad 0-255, tamaño en px).
/// Interpola de `*_start` a `*_end` a lo largo de su vida.
pub fn particle_look(def: &ParticleDef, p: &Particle) -> ([u8; 3], f32, f32) {
    let t = if p.life > 0. { (p.age / p.life).clamp(0., 1.) } else { 1. };
    let mix = |a: f32, b: f32| a + (b - a) * t;
    let rgb = [0, 1, 2].map(|k| mix(def.color_start[k] as f32, def.color_end[k] as f32).round().clamp(0., 255.) as u8);
    (rgb, mix(def.alpha_start, def.alpha_end).clamp(0., 255.), mix(def.size_start, def.size_end).max(0.))
}

/// Suelta `n` partículas desde (x, y). `giro` = ángulo del emisor (se suma a la dirección del cono).
#[allow(clippy::too_many_arguments)]
pub(crate) fn emitir(def: &ParticleDef, n: usize, (x, y, w, h): (f32, f32, f32, f32), giro: f32, obj: &str, layer: &str, emitter: u64, rnd: &dyn Fn() -> f32, out: &mut Vec<Particle>) {
    let rango = |r: [f32; 2], u: f32| r[0] + (r[1] - r[0]) * u;
    for _ in 0..n {
        // Con `area`, el punto de nacimiento es cualquiera dentro del rectángulo del emisor.
        let (x, y) = if def.area { (x + (rnd() - 0.5) * w, y + (rnd() - 0.5) * h) } else { (x, y) };
        let ang = (def.angle + giro + (rnd() - 0.5) * def.spread).to_radians();
        let speed = rango(def.speed, rnd());
        out.push(Particle {
            x,
            y,
            vx: ang.cos() * speed,
            vy: ang.sin() * speed,
            age: 0.,
            life: rango(def.life, rnd()).max(0.01),
            obj: obj.into(),
            layer: layer.into(),
            emitter,
        });
    }
}

/// Mueve y envejece; quita las que cumplieron su vida. `gravedad(obj)` da la gravedad del emisor.
pub(crate) fn mover(parts: &mut Vec<Particle>, dt: f32, gravedad: impl Fn(&str) -> [f32; 2]) {
    let mut cache: Option<(String, [f32; 2])> = None;
    for p in parts.iter_mut() {
        let g = match &cache {
            Some((o, g)) if *o == p.obj => *g,
            _ => {
                let g = gravedad(&p.obj);
                cache = Some((p.obj.clone(), g));
                g
            }
        };
        p.age += dt;
        let (pvx, pvy) = (p.vx, p.vy);
        p.vx += g[0] * dt;
        p.vy += g[1] * dt;
        p.x += (pvx + p.vx) / 2. * dt;
        p.y += (pvy + p.vy) / 2. * dt;
    }
    parts.retain(|p| p.age < p.life);
}
