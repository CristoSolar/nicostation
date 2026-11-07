//! Recetas: trozos de juego listos para usar. El niño elige qué objetos van en cada hueco y se agregan los eventos.

use crate::catalog::P;
use crate::model::{Event, Value};

pub struct Receta {
    pub nombre: &'static str,
    pub descripcion: &'static str,
    /// Categoría del catálogo (para el color e icono en el editor).
    pub grupo: &'static str,
    /// Huecos a completar: (pregunta, tipo, valor por defecto).
    pub huecos: &'static [(&'static str, P, &'static str)],
    /// Variables de escena que la receta necesita (se crean si no existen).
    pub variables: &'static [(&'static str, f64)],
    /// Consejo extra (p. ej. qué comportamiento ponerle a un objeto).
    pub nota: &'static str,
    /// Eventos en JSON; `$0`, `$1`... son los huecos.
    eventos: &'static str,
}

pub const RECETAS: &[Receta] = &[
    Receta {
        nombre: "Moverse con las flechas",
        descripcion: "El personaje camina hacia donde apuntan las flechas del teclado.",
        grupo: "Teclado",
        huecos: &[("¿Quién se mueve?", P::Object, ""), ("¿Qué tan rápido?", P::Expr, "200")],
        variables: &[],
        nota: "Para un juego de plataformas (con saltos) es mejor ponerle al personaje el comportamiento «Plataformas».",
        eventos: r#"[
            {"conds":[{"op":"key_down","args":["Left"]}],"acts":[{"op":"add","args":["$0","x","-($1) * dt"]},{"op":"flip_x","args":["$0","1"]}]},
            {"conds":[{"op":"key_down","args":["Right"]}],"acts":[{"op":"add","args":["$0","x","($1) * dt"]},{"op":"flip_x","args":["$0","0"]}]},
            {"conds":[{"op":"key_down","args":["Up"]}],"acts":[{"op":"add","args":["$0","y","-($1) * dt"]}]},
            {"conds":[{"op":"key_down","args":["Down"]}],"acts":[{"op":"add","args":["$0","y","($1) * dt"]}]}
        ]"#,
    },
    Receta {
        nombre: "Recoger cosas y sumar puntos",
        descripcion: "Cuando el personaje toca una moneda, la moneda desaparece y ganas un punto.",
        grupo: "Objetos",
        huecos: &[("¿Quién recoge?", P::Object, ""), ("¿Qué se recoge?", P::Object, "")],
        variables: &[("puntos", 0.)],
        nota: "Los puntos quedan guardados en la cajita «puntos». Usa la receta «Mostrar los puntos» para verlos.",
        eventos: r#"[{"conds":[{"op":"collides","args":["$0","$1"]}],"acts":[{"op":"delete","args":["$1"]},{"op":"add_var","args":["puntos","1"]}]}]"#,
    },
    Receta {
        nombre: "Mostrar los puntos",
        descripcion: "Un texto en la pantalla muestra cuántos puntos llevas.",
        grupo: "Variables",
        huecos: &[("¿Qué texto muestra los puntos?", P::Object, "")],
        variables: &[("puntos", 0.)],
        nota: "Pon el texto en una capa «fija» (HUD) para que no se mueva con la cámara.",
        eventos: r#"[{"acts":[{"op":"set_text","args":["$0","\"Puntos: \" + puntos"]}]}]"#,
    },
    Receta {
        nombre: "Perder al tocar un enemigo",
        descripcion: "Si el personaje toca un enemigo, el nivel empieza de nuevo.",
        grupo: "Objetos",
        huecos: &[("¿Quién pierde?", P::Object, ""), ("¿Qué lo hace perder?", P::Object, "")],
        variables: &[],
        nota: "",
        eventos: r#"[{"conds":[{"op":"collides","args":["$0","$1"]}],"acts":[{"op":"restart_scene"}]}]"#,
    },
    Receta {
        nombre: "Perder al caer al vacío",
        descripcion: "Si el personaje cae por debajo de la pantalla, el nivel empieza de nuevo.",
        grupo: "Movimiento",
        huecos: &[("¿Quién puede caerse?", P::Object, "")],
        variables: &[],
        nota: "",
        eventos: r#"[{"conds":[{"op":"compare_prop","args":["$0","y",">","screen_h + 200"]}],"acts":[{"op":"restart_scene"}]}]"#,
    },
    Receta {
        nombre: "Ganar al llegar a la meta",
        descripcion: "Cuando el personaje toca la meta, pasa a la escena que elijas (el siguiente nivel).",
        grupo: "Escena",
        huecos: &[("¿Quién llega?", P::Object, ""), ("¿Cuál es la meta?", P::Object, ""), ("¿A qué escena pasa?", P::Scene, "")],
        variables: &[],
        nota: "",
        eventos: r#"[{"conds":[{"op":"collides","args":["$0","$1"]}],"acts":[{"op":"change_scene","args":["$2"]}]}]"#,
    },
    Receta {
        nombre: "Disparar",
        descripcion: "Al apretar una tecla aparece una bala donde está el personaje.",
        grupo: "Teclado",
        huecos: &[("¿Quién dispara?", P::Object, ""), ("¿Qué sale disparado?", P::Object, ""), ("¿Con qué tecla?", P::Key, "Space")],
        variables: &[],
        nota: "Ponle a la bala los comportamientos «Bala» (para que avance) y «Destruir al salir de la pantalla».",
        eventos: r#"[{"conds":[{"op":"key_pressed","args":["$2"]}],"acts":[{"op":"create","args":["$1","$0.cx","$0.cy",""]}]}]"#,
    },
    Receta {
        nombre: "Aparecer cada tantos segundos",
        descripcion: "Cada cierto tiempo aparece un objeto nuevo en un lugar al azar de la pantalla.",
        grupo: "Tiempo",
        huecos: &[("¿Qué aparece?", P::Object, ""), ("¿Cada cuántos segundos?", P::Expr, "2")],
        variables: &[],
        nota: "",
        eventos: r#"[{"conds":[{"op":"every","args":["$1"]}],"acts":[{"op":"create","args":["$0","random(0, screen_w - 40)","random(0, screen_h - 40)",""]}]}]"#,
    },
    Receta {
        nombre: "La cámara sigue al personaje",
        descripcion: "La pantalla se mueve junto con el personaje, para niveles más grandes que la pantalla.",
        grupo: "Escena",
        huecos: &[("¿A quién sigue?", P::Object, "")],
        variables: &[],
        nota: "",
        eventos: r#"[{"acts":[{"op":"camera","args":["$0.cx","$0.cy"]}]}]"#,
    },
    Receta {
        nombre: "Enemigo que persigue",
        descripcion: "El enemigo camina siempre hacia el personaje.",
        grupo: "Movimiento",
        huecos: &[("¿Quién persigue?", P::Object, ""), ("¿A quién persigue?", P::Object, ""), ("¿Qué tan rápido?", P::Expr, "80")],
        variables: &[],
        nota: "",
        eventos: r#"[{"acts":[
            {"op":"add","args":["$0","x","cos(angle($0.cx, $0.cy, $1.cx, $1.cy)) * ($2) * dt"]},
            {"op":"add","args":["$0","y","sin(angle($0.cx, $0.cy, $1.cx, $1.cy)) * ($2) * dt"]}
        ]}]"#,
    },
    Receta {
        nombre: "Cronómetro",
        descripcion: "Cuenta los segundos que llevas jugando en la cajita «tiempo».",
        grupo: "Tiempo",
        huecos: &[],
        variables: &[("tiempo", 0.)],
        nota: "",
        eventos: r#"[{"conds":[{"op":"every","args":["1"]}],"acts":[{"op":"add_var","args":["tiempo","1"]}]}]"#,
    },
    Receta {
        nombre: "Botón para cambiar de escena",
        descripcion: "Al hacer clic sobre un objeto (por ejemplo un botón «Jugar») pasas a otra escena.",
        grupo: "Mouse",
        huecos: &[("¿Cuál es el botón?", P::Object, ""), ("¿A qué escena lleva?", P::Scene, "")],
        variables: &[],
        nota: "",
        eventos: r#"[{"conds":[{"op":"cursor_on","args":["$0"]},{"op":"mouse_clicked","args":["Left"]}],"acts":[{"op":"change_scene","args":["$1"]}]}]"#,
    },
];

impl Receta {
    /// Eventos con los huecos completados. `valores` va en el mismo orden que `huecos`.
    pub fn aplicar(&self, valores: &[String]) -> Result<Vec<Event>, String> {
        if let Some((q, ..)) = self.huecos.get(valores.len()) {
            return Err(format!("falta responder: {q}"));
        }
        if valores.len() > self.huecos.len() {
            return Err(format!("la receta tiene {} preguntas y recibió {} respuestas", self.huecos.len(), valores.len()));
        }
        if let Some((q, ..)) = self.huecos.iter().zip(valores).find(|(_, v)| v.trim().is_empty()).map(|x| x.0) {
            return Err(format!("falta responder: {q}"));
        }
        let mut src = self.eventos.to_string();
        // De atrás para adelante, para que $1 no pise a $10.
        for (k, v) in valores.iter().enumerate().rev() {
            let esc = serde_json::to_string(v).map_err(|e| e.to_string())?;
            src = src.replace(&format!("${k}"), &esc[1..esc.len() - 1]);
        }
        serde_json::from_str(&src).map_err(|e| format!("receta «{}»: {e}", self.nombre))
    }

    pub fn variables_iniciales(&self) -> impl Iterator<Item = (String, Value)> + '_ {
        self.variables.iter().map(|(n, v)| (n.to_string(), Value::Num(*v)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todas_las_recetas_quedan_validas() {
        for r in RECETAS {
            let valores: Vec<String> = r.huecos.iter().enumerate().map(|(k, (_, t, def))| match t {
                P::Object => ["Heroe", "Moneda", "Texto"][k.min(2)].to_string(),
                P::Scene => "S".into(),
                _ => def.to_string(),
            }).collect();
            let evs = r.aplicar(&valores).unwrap_or_else(|e| panic!("{e}"));
            let mut p: crate::model::Project = serde_json::from_str(r#"{"name":"t","width":100,"height":100,"first_scene":"S","scenes":[{"name":"S",
                "objects":[{"name":"Heroe","type":"box","w":10,"h":10,"color":[0,0,0]},{"name":"Moneda","type":"box","w":10,"h":10,"color":[0,0,0]},
                           {"name":"Texto","type":"text","text":""}]}]}"#).unwrap();
            p.scenes[0].events = evs;
            p.scenes[0].vars.extend(r.variables_iniciales());
            assert_eq!(crate::validate::validate(&p, None), Vec::<String>::new(), "receta «{}»", r.nombre);
            let mut w = crate::runtime::World::new(p);
            for _ in 0..30 {
                w.step(1. / 60., Default::default());
            }
            assert!(w.log.borrow().is_empty(), "receta «{}»: {:?}", r.nombre, w.log.borrow());
        }
        assert!(RECETAS[0].aplicar(&["".into(), "1".into()]).is_err()); // hueco sin responder
    }
}
