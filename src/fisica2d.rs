//! Física 2D con rapier2d para el comportamiento `Physics`.
//! El mundo del juego está en píxeles con Y hacia abajo; rapier trabaja en metros (`PX_POR_METRO`).
//! Cada cuerpo se identifica por el id de su instancia. El motor (`runtime.rs`) sincroniza en cada paso:
//! instancia → cuerpo (creación perezosa, teletransportes con `set x`, cinemáticos que siguen a los eventos),
//! paso de la simulación, y cuerpo → instancia (los dinámicos mueven a su instancia).

use rapier2d::prelude::*;
use std::collections::HashMap;

/// 50 px = 1 metro.
pub const PX_POR_METRO: f32 = 50.;
const K: f32 = PX_POR_METRO;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tipo {
    Dinamico,
    Estatico,
    Cinematico,
}

/// Parámetros del comportamiento, ya interpretados.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Def {
    pub tipo: Tipo,
    pub density: f32,
    pub friction: f32,
    pub restitution: f32,
    pub fixed_rotation: bool,
    pub gravity_scale: f32,
    pub bullet: bool,
}

impl Def {
    pub fn de(b: &crate::model::Behavior) -> Option<Def> {
        let crate::model::Behavior::Physics { body, density, friction, restitution, fixed_rotation, gravity_scale, bullet, .. } = b else {
            return None;
        };
        let tipo = match body.as_str() {
            "static" => Tipo::Estatico,
            "kinematic" => Tipo::Cinematico,
            _ => Tipo::Dinamico,
        };
        Some(Def { tipo, density: density.max(0.001), friction: friction.max(0.), restitution: restitution.clamp(0., 1.), fixed_rotation: *fixed_rotation, gravity_scale: *gravity_scale, bullet: *bullet })
    }
}

/// Forma de choque en píxeles.
#[derive(Clone, PartialEq, Debug)]
pub enum Forma {
    /// Medio ancho y medio alto, centrada en el cuerpo.
    Caja(f32, f32),
    Circulo(f32),
    /// Polígono convexo relativo al centro del cuerpo (sin girar).
    Poligono(Vec<(f32, f32)>),
    /// Rectángulos en coordenadas del mundo (tiras sólidas de un mapa de baldosas; cuerpo en (0, 0)).
    Tiras(Vec<[(f32, f32); 4]>),
}

struct Cuerpo {
    h: RigidBodyHandle,
    def: Def,
    forma: Forma,
    /// Centro y ángulo (grados) de la instancia la última vez que se sincronizó: si cambian, la movieron los eventos.
    pose: (f32, f32, f32),
}

/// Junta entre dos instancias (se borra sola cuando se borra uno de los cuerpos).
struct Junta {
    h: ImpulseJointHandle,
    a: u64,
    b: u64,
}

#[derive(Default)]
pub struct Fisica {
    mundo: PhysicsWorld,
    cuerpos: HashMap<u64, Cuerpo>,
    juntas: Vec<Junta>,
}

fn v(x: f32, y: f32) -> Vector {
    Vector::new(x / K, y / K)
}

fn collider(forma: &Forma, def: &Def, id: u64) -> Vec<ColliderBuilder> {
    let base = |b: ColliderBuilder| b.density(def.density).friction(def.friction).restitution(def.restitution).user_data(id as u128);
    match forma {
        Forma::Caja(hw, hh) => vec![base(ColliderBuilder::cuboid((hw / K).max(0.001), (hh / K).max(0.001)))],
        Forma::Circulo(r) => vec![base(ColliderBuilder::ball((r / K).max(0.001)))],
        Forma::Poligono(pts) => {
            let pts: Vec<Vector> = pts.iter().map(|p| v(p.0, p.1)).collect();
            match ColliderBuilder::convex_hull(&pts) {
                Some(c) => vec![base(c)],
                None => vec![],
            }
        }
        Forma::Tiras(tiras) => tiras
            .iter()
            .map(|t| {
                let (x0, y0, x1, y1) = t.iter().fold((f32::MAX, f32::MAX, f32::MIN, f32::MIN), |b, p| (b.0.min(p.0), b.1.min(p.1), b.2.max(p.0), b.3.max(p.1)));
                base(ColliderBuilder::cuboid(((x1 - x0) / 2. / K).max(0.001), ((y1 - y0) / 2. / K).max(0.001)))
                    .position(Pose::translation((x0 + x1) / 2. / K, (y0 + y1) / 2. / K))
            })
            .collect(),
    }
}

const TOL: f32 = 1e-3;

impl Fisica {
    pub fn is_empty(&self) -> bool {
        self.cuerpos.is_empty()
    }

    pub fn tiene(&self, id: u64) -> bool {
        self.cuerpos.contains_key(&id)
    }

    pub fn tipo(&self, id: u64) -> Option<Tipo> {
        self.cuerpos.get(&id).map(|c| c.def.tipo)
    }

    /// Instancia → cuerpo. Crea el cuerpo si no existe; si la instancia se movió o cambió de forma desde la última
    /// sincronización, mueve o rehace el cuerpo. `vel0` = velocidad inicial en px/s (al crearlo).
    pub fn entrar(&mut self, id: u64, def: Def, centro: (f32, f32), angulo: f32, forma: Forma, vel0: (f32, f32)) {
        let pose = (centro.0, centro.1, angulo);
        let rot = if def.fixed_rotation { 0. } else { angulo.to_radians() };
        let rehacer = match self.cuerpos.get(&id) {
            None => true,
            Some(c) => c.def != def || c.forma != forma,
        };
        if rehacer {
            let vel = self.cuerpos.get(&id).map(|c| self.mundo.bodies[c.h].linvel()).unwrap_or(v(vel0.0, vel0.1));
            self.sacar(id);
            let b = match def.tipo {
                Tipo::Dinamico => RigidBodyBuilder::dynamic(),
                Tipo::Estatico => RigidBodyBuilder::fixed(),
                Tipo::Cinematico => RigidBodyBuilder::kinematic_position_based(),
            };
            let mut b = b.translation(v(centro.0, centro.1)).rotation(rot).gravity_scale(def.gravity_scale).ccd_enabled(def.bullet).user_data(id as u128);
            if matches!(forma, Forma::Tiras(_)) {
                b = b.translation(Vector::new(0., 0.)).rotation(0.);
            }
            if def.fixed_rotation {
                b = b.lock_rotations();
            }
            if def.tipo == Tipo::Dinamico {
                b = b.linvel(vel);
            }
            let h = self.mundo.insert_body(b);
            for c in collider(&forma, &def, id) {
                self.mundo.insert_collider(c, Some(h));
            }
            self.cuerpos.insert(id, Cuerpo { h, def, forma, pose });
            return;
        }
        let c = self.cuerpos.get_mut(&id).expect("cuerpo");
        if matches!(c.forma, Forma::Tiras(_)) {
            return;
        }
        let movido = (c.pose.0 - pose.0).abs() > TOL || (c.pose.1 - pose.1).abs() > TOL || (c.pose.2 - pose.2).abs() > TOL;
        let body = &mut self.mundo.bodies[c.h];
        match def.tipo {
            // Los cinemáticos siguen a la instancia con velocidad (empujan a los dinámicos).
            Tipo::Cinematico => body.set_next_kinematic_position(Pose::new(v(centro.0, centro.1), rot)),
            _ if movido => {
                body.set_translation(v(centro.0, centro.1), true);
                body.set_rotation(Rotation::new(rot), true);
            }
            _ => {}
        }
        c.pose = pose;
    }

    /// Quita los cuerpos cuyas instancias ya no existen (o perdieron el comportamiento).
    pub fn retener(&mut self, vivo: impl Fn(u64) -> bool) {
        let muertos: Vec<u64> = self.cuerpos.keys().copied().filter(|&id| !vivo(id)).collect();
        for id in muertos {
            self.sacar(id);
        }
    }

    fn sacar(&mut self, id: u64) {
        if let Some(c) = self.cuerpos.remove(&id) {
            self.mundo.remove_body(c.h); // también quita sus colisionadores y juntas
            self.juntas.retain(|j| j.a != id && j.b != id);
        }
    }

    /// Avanza la simulación. `gravedad` en px/s².
    pub fn paso(&mut self, dt: f32, gravedad: [f32; 2]) {
        if self.cuerpos.is_empty() || dt <= 0. {
            return;
        }
        self.mundo.gravity = v(gravedad[0], gravedad[1]);
        self.mundo.integration_parameters.dt = dt;
        self.mundo.step();
        // Las fuerzas valen por un paso (como «aplicar fuerza» en GDevelop).
        for c in self.cuerpos.values() {
            let b = &mut self.mundo.bodies[c.h];
            b.reset_forces(false);
            b.reset_torques(false);
        }
    }

    /// Centro (px) y ángulo (grados; None si no gira) de un cuerpo dinámico, para moverle la instancia.
    pub fn pose(&self, id: u64) -> Option<(f32, f32, Option<f32>)> {
        let c = self.cuerpos.get(&id).filter(|c| c.def.tipo == Tipo::Dinamico)?;
        let b = &self.mundo.bodies[c.h];
        let t = b.translation();
        Some((t.x * K, t.y * K, (!c.def.fixed_rotation).then(|| b.rotation().angle().to_degrees())))
    }

    /// Recuerda dónde quedó la instancia tras copiarle la pose (para detectar teletransportes).
    pub fn anotar(&mut self, id: u64, centro: (f32, f32), angulo: f32) {
        if let Some(c) = self.cuerpos.get_mut(&id) {
            c.pose = (centro.0, centro.1, angulo);
        }
    }

    fn cuerpo(&mut self, id: u64) -> Option<&mut RigidBody> {
        let h = self.cuerpos.get(&id)?.h;
        self.mundo.bodies.get_mut(h)
    }

    /// Velocidad en px/s.
    pub fn velocidad(&self, id: u64) -> Option<(f32, f32)> {
        let c = self.cuerpos.get(&id)?;
        let l = self.mundo.bodies[c.h].linvel();
        Some((l.x * K, l.y * K))
    }

    pub fn poner_velocidad(&mut self, id: u64, vx: f32, vy: f32) {
        if let Some(b) = self.cuerpo(id) {
            b.set_linvel(v(vx, vy), true);
        }
    }

    /// Velocidad de giro en grados por segundo.
    pub fn giro(&self, id: u64) -> Option<f32> {
        let c = self.cuerpos.get(&id)?;
        Some(self.mundo.bodies[c.h].angvel().to_degrees())
    }

    pub fn poner_giro(&mut self, id: u64, grados_s: f32) {
        if let Some(b) = self.cuerpo(id) {
            b.set_angvel(grados_s.to_radians(), true);
        }
    }

    /// Fuerza durante el próximo paso, en «px·kg/s²» (980 sostiene a una caja de 50x50 con densidad 1).
    pub fn fuerza(&mut self, id: u64, fx: f32, fy: f32) {
        if let Some(b) = self.cuerpo(id) {
            b.add_force(v(fx, fy), true);
        }
    }

    /// Golpe instantáneo: cambia la velocidad en (ix, iy) / masa px/s.
    pub fn impulso(&mut self, id: u64, ix: f32, iy: f32) {
        if let Some(b) = self.cuerpo(id) {
            b.apply_impulse(v(ix, iy), true);
        }
    }

    /// Fuerza de giro durante el próximo paso (en px²·kg/s²).
    pub fn torque(&mut self, id: u64, t: f32) {
        if let Some(b) = self.cuerpo(id) {
            b.add_torque(t / (K * K), true);
        }
    }

    pub fn masa(&self, id: u64) -> f32 {
        self.cuerpos.get(&id).map(|c| self.mundo.bodies[c.h].mass()).unwrap_or(0.)
    }

    /// Junta tipo cuerda: los centros no se alejan más de `largo` px (None = la distancia actual).
    pub fn junta_distancia(&mut self, a: u64, b: u64, largo: Option<f32>) -> bool {
        let (Some(ca), Some(cb)) = (self.cuerpos.get(&a), self.cuerpos.get(&b)) else { return false };
        let (ha, hb) = (ca.h, cb.h);
        let d = (self.mundo.bodies[ha].translation() - self.mundo.bodies[hb].translation()).length();
        let largo = largo.map(|l| l / K).unwrap_or(d).max(0.);
        let h = self.mundo.insert_impulse_joint(ha, hb, RopeJointBuilder::new(largo).contacts_enabled(true));
        self.juntas.push(Junta { h, a, b });
        true
    }

    /// Junta de giro (bisagra) en el punto (x, y) del mundo, en px.
    pub fn junta_giro(&mut self, a: u64, b: u64, x: f32, y: f32) -> bool {
        let (Some(ca), Some(cb)) = (self.cuerpos.get(&a), self.cuerpos.get(&b)) else { return false };
        let (ha, hb) = (ca.h, cb.h);
        let p = v(x, y);
        let local = |h: RigidBodyHandle| self.mundo.bodies[h].position().inverse_transform_point(p);
        let (la, lb) = (local(ha), local(hb));
        let h = self.mundo.insert_impulse_joint(ha, hb, RevoluteJointBuilder::new().local_anchor1(la).local_anchor2(lb).contacts_enabled(false));
        self.juntas.push(Junta { h, a, b });
        true
    }

    /// Quita las juntas de una instancia.
    pub fn sacar_juntas(&mut self, id: u64) {
        let (fuera, quedan): (Vec<Junta>, Vec<Junta>) = std::mem::take(&mut self.juntas).into_iter().partition(|j| j.a == id || j.b == id);
        self.juntas = quedan;
        for j in fuera {
            self.mundo.remove_impulse_joint(j.h);
        }
    }

    pub fn juntas(&self) -> usize {
        self.juntas.len()
    }

    /// Dos cuerpos se están tocando según rapier (contacto activo entre algún par de sus formas).
    pub fn tocando(&self, a: u64, b: u64) -> bool {
        let (Some(ca), Some(cb)) = (self.cuerpos.get(&a), self.cuerpos.get(&b)) else { return false };
        let (la, lb) = (self.mundo.bodies[ca.h].colliders(), self.mundo.bodies[cb.h].colliders());
        la.iter().any(|&x| lb.iter().any(|&y| self.mundo.contact_pair(x, y).is_some_and(|p| p.has_any_active_contact())))
    }
}
