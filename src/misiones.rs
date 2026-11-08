//! Misiones: pequeños desafíos guiados sobre una plantilla. Cada paso se comprueba solo,
//! comparando el proyecto actual con como estaba al empezar la misión.

use crate::model::*;

pub struct Paso {
    pub texto: &'static str,
    pub pista: &'static str,
    /// `None` = el niño lo marca con «¡Listo!» (p. ej. «prueba el juego»).
    pub listo: Option<fn(&Project, &Project) -> bool>,
}

pub struct Mision {
    pub titulo: &'static str,
    pub descripcion: &'static str,
    /// Carpeta de la plantilla para la que está pensada ("" = cualquier proyecto).
    pub plantilla: &'static str,
    pub pasos: &'static [Paso],
}

const PROBAR: Paso = Paso { texto: "Prueba el juego con el botón Probar (o F5).", pista: "Está arriba a la derecha, en naranja.", listo: None };

fn objects(p: &Project) -> impl Iterator<Item = &ObjectDef> {
    p.objects.iter().chain(p.scenes.iter().flat_map(|s| s.objects.iter()))
}

fn count(p: &Project, obj: &str) -> usize {
    p.scenes.iter().flat_map(|s| &s.instances).filter(|i| i.obj == obj).count()
}

fn platformer_jump(p: &Project, obj: &str) -> f32 {
    objects(p).filter(|o| o.name == obj).flat_map(|o| &o.behaviors).find_map(|b| match b {
        Behavior::Platformer { jump_speed, .. } => Some(*jump_speed),
        _ => None,
    }).unwrap_or(0.)
}

fn bullet_speed(p: &Project, obj: &str) -> f32 {
    objects(p).filter(|o| o.name == obj).flat_map(|o| &o.behaviors).find_map(|b| match b {
        Behavior::Bullet { speed } => Some(*speed),
        _ => None,
    }).unwrap_or(0.)
}

fn scene_var(p: &Project, name: &str) -> f64 {
    p.scenes.iter().find_map(|s| s.vars.get(name)).map(Value::num).unwrap_or(0.)
}

/// Todos los eventos del proyecto, incluidos los que salen de los grafos de nodos.
fn all_events(p: &Project) -> Vec<Event> {
    fn walk(l: &[Event], out: &mut Vec<Event>) {
        for e in l {
            out.push(e.clone());
            walk(&e.sub, out);
        }
    }
    let p = p.compiled();
    let mut out = vec![];
    for s in &p.scenes {
        walk(&s.events, &mut out);
    }
    for f in &p.functions {
        walk(&f.events, &mut out);
    }
    out
}

/// Primer número que aparece en una expresión: "320 * dt" → 320.
fn first_number(s: &str) -> f64 {
    let t: String = s.chars().skip_while(|c| !c.is_ascii_digit()).take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    t.parse().unwrap_or(0.)
}

/// Mayor número que suma un evento que toca `obj` a la variable `var`.
fn reward(p: &Project, obj: &str, var: &str) -> f64 {
    all_events(p).iter()
        .filter(|e| e.conds.iter().any(|c| c.op == "collides" && c.args.iter().any(|a| a == obj)))
        .flat_map(|e| &e.acts)
        .filter(|a| a.op == "add_var" && a.args.first().is_some_and(|v| v == var))
        .map(|a| a.args.get(1).map(|x| first_number(x)).unwrap_or(0.))
        .fold(0., f64::max)
}

fn fastest_move(p: &Project, obj: &str) -> f64 {
    all_events(p).iter().flat_map(|e| &e.acts)
        .filter(|a| a.op == "add" && a.args.first().is_some_and(|o| o == obj) && a.args.get(1).is_some_and(|x| x == "x"))
        .map(|a| a.args.get(2).map(|x| first_number(x)).unwrap_or(0.))
        .fold(0., f64::max)
}

pub const MISIONES: &[Mision] = &[
    // ---- Plataformas
    Mision {
        titulo: "Salta más alto",
        descripcion: "El héroe salta poquito. ¡Hagamos que llegue más arriba!",
        plantilla: "plataformas",
        pasos: &[
            Paso { texto: "En la pestaña Escena, haz clic en el objeto «Heroe» de la lista.", pista: "La lista de objetos está a la izquierda.", listo: None },
            Paso { texto: "Busca el comportamiento «Plataformas» y sube «jump_speed» a 800 o más.", pista: "Está a la derecha, en Comportamientos. jump_speed es la fuerza del salto.",
                   listo: Some(|p, _| platformer_jump(p, "Heroe") >= 800.) },
            PROBAR,
        ],
    },
    Mision {
        titulo: "Más monedas",
        descripcion: "Pon más monedas en el nivel para que haya más que recoger.",
        plantilla: "plataformas",
        pasos: &[
            Paso { texto: "Coloca 3 monedas más en el nivel.", pista: "Arrastra «Moneda» desde la lista de objetos hasta el lienzo, o usa el modo Colocar.",
                   listo: Some(|p, ini| count(p, "Moneda") >= count(ini, "Moneda") + 3) },
            PROBAR,
        ],
    },
    Mision {
        titulo: "Monedas que valen más",
        descripcion: "Cada moneda da 1 punto. ¿Y si diera 5?",
        plantilla: "plataformas",
        pasos: &[
            Paso { texto: "En Eventos, busca el evento CUANDO «Heroe choca con Moneda».", pista: "Tiene el bloque «Sumar 1 a la cajita puntos».", listo: None },
            Paso { texto: "Cambia el 1 por un número más grande.", pista: "Haz clic en el 1 y escribe otro número, o usa el botón +.",
                   listo: Some(|p, _| reward(p, "Moneda", "puntos") >= 2.) },
            PROBAR,
        ],
    },
    // ---- Recolector
    Mision {
        titulo: "Más tiempo para jugar",
        descripcion: "30 segundos se pasan volando. Dale más tiempo al gato.",
        plantilla: "recolector",
        pasos: &[
            Paso { texto: "Ve a la pestaña Cajitas y cambia «tiempo» a 60.", pista: "Es una cajita de esta escena. Usa el botón + o escribe el número.",
                   listo: Some(|p, _| scene_var(p, "tiempo") >= 60.) },
            PROBAR,
        ],
    },
    Mision {
        titulo: "¡Otro perro!",
        descripcion: "Un solo perro es fácil de esquivar. Agrega otro.",
        plantilla: "recolector",
        pasos: &[
            Paso { texto: "Coloca un perro más en la escena.", pista: "Arrastra «Perro» desde la lista de objetos hasta el lienzo.",
                   listo: Some(|p, ini| count(p, "Perro") > count(ini, "Perro")) },
            PROBAR,
        ],
    },
    // ---- Naves
    Mision {
        titulo: "Nave más rápida",
        descripcion: "Las rocas te alcanzan. Haz que la nave se mueva más rápido.",
        plantilla: "naves",
        pasos: &[
            Paso { texto: "En Eventos, busca los bloques que suman a la X de la Nave.", pista: "Dicen «Sumar 320 * dt a x de Nave».", listo: None },
            Paso { texto: "Cambia el 320 por un número más grande (por ejemplo 500) en los dos.", pista: "Haz clic en el número para cambiarlo. dt significa «por segundo».",
                   listo: Some(|p, ini| fastest_move(p, "Nave") > fastest_move(ini, "Nave")) },
            PROBAR,
        ],
    },
    Mision {
        titulo: "Rocas más rápidas",
        descripcion: "¿Muy fácil? Que las rocas caigan más rápido.",
        plantilla: "naves",
        pasos: &[
            Paso { texto: "Haz clic en el objeto «Roca» y sube la velocidad de su comportamiento «Bala».", pista: "Está en Comportamientos, a la derecha.",
                   listo: Some(|p, ini| bullet_speed(p, "Roca") > bullet_speed(ini, "Roca")) },
            PROBAR,
        ],
    },
    // ---- Atrapa
    Mision {
        titulo: "Más vidas",
        descripcion: "Con 3 vidas se pierde rápido. Empieza con 5.",
        plantilla: "atrapa",
        pasos: &[
            Paso { texto: "En Cajitas, cambia «vidas» a 5 o más.", pista: "Usa el botón + de la cajita vidas.", listo: Some(|p, _| scene_var(p, "vidas") >= 5.) },
            PROBAR,
        ],
    },
    Mision {
        titulo: "Una fruta nueva",
        descripcion: "Dibuja una fruta nueva (¿una frutilla?) y haz que también caiga.",
        plantilla: "atrapa",
        pasos: &[
            Paso { texto: "Crea un sprite nuevo y dibújalo con «Dibujar».", pista: "Botón «+ Sprite» y luego «Dibujar uno nuevo».",
                   listo: Some(|p, ini| objects(p).count() > objects(ini).count()) },
            Paso { texto: "Agrégalo al grupo «Fruta» para que la canasta lo atrape.", pista: "En la lista de la izquierda, sección Grupos.",
                   listo: Some(|p, _| p.scenes.iter().any(|s| s.groups.get("Fruta").is_some_and(|g| g.len() > 2))) },
            Paso { texto: "Copia el evento «Cada 3.1 segundos: crear Banana» y cambia Banana por tu fruta.", pista: "En el menú de tres puntos del evento está «Duplicar».",
                   listo: Some(|p, _| all_events(p).iter().flat_map(|e| &e.acts).filter(|a| a.op == "create").count() >= 3) },
            PROBAR,
        ],
    },
    // ---- Cualquier proyecto
    Mision {
        titulo: "Tu primer sonido",
        descripcion: "Haz que suene algo cuando aprietes una tecla.",
        plantilla: "",
        pasos: &[
            Paso { texto: "Crea un evento nuevo con CUANDO «Tecla recién presionada».", pista: "Botón «+ Nuevo evento» y luego «+ Cuando», categoría Teclado y mouse.",
                   listo: Some(|p, ini| {
                       let n = |p: &Project| all_events(p).iter().filter(|e| e.conds.iter().any(|c| c.op == "key_pressed")).count();
                       n(p) > n(ini)
                   }) },
            Paso { texto: "En ENTONCES agrega «Hacer sonar» y elige un sonido.", pista: "Categoría Sonido.",
                   listo: Some(|p, ini| {
                       let n = |p: &Project| all_events(p).iter().filter(|e| e.conds.iter().any(|c| c.op == "key_pressed")
                           && e.acts.iter().any(|a| a.op == "play_sound" && a.args.first().is_some_and(|f| !f.is_empty()))).count();
                       n(p) > n(ini)
                   }) },
            PROBAR,
        ],
    },
];

/// Misiones para un proyecto hecho con la plantilla `carpeta` (más las generales).
pub fn para(carpeta: &str) -> impl Iterator<Item = &'static Mision> + '_ {
    MISIONES.iter().filter(move |m| m.plantilla.is_empty() || m.plantilla == carpeta)
}

/// La plantilla de la que viene un proyecto: el campo `plantilla`, o su nombre si es una plantilla sin copiar.
pub fn plantilla_de(p: &Project) -> &str {
    if !p.plantilla.is_empty() {
        return &p.plantilla;
    }
    match p.name.as_str() {
        "Aventura en plataformas" => "plataformas",
        "Gato goloso" => "recolector",
        "Lluvia de asteroides" => "naves",
        "Atrapa la fruta" => "atrapa",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plantilla(c: &str) -> Project {
        let f = format!("{}/plantillas/{c}/juego.json", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(f).unwrap()).unwrap()
    }

    fn pendientes(m: &Mision, p: &Project, ini: &Project) -> Vec<&'static str> {
        m.pasos.iter().filter(|s| s.listo.is_some_and(|f| !f(p, ini))).map(|s| s.texto).collect()
    }

    #[test]
    fn misiones_se_cumplen_al_hacer_lo_pedido() {
        for c in ["plataformas", "recolector", "naves", "atrapa"] {
            let ini = plantilla(c);
            assert_eq!(plantilla_de(&ini), c);
            for m in para(c) {
                // Recién empezada, ningún paso automático está cumplido.
                for s in m.pasos.iter().filter_map(|s| s.listo) {
                    assert!(!s(&ini, &ini), "«{}» ya cumplida al empezar", m.titulo);
                }
            }
        }
        // Hacer lo que pide cada misión las completa.
        let ini = plantilla("plataformas");
        let mut p = ini.clone();
        let heroe = p.scenes[0].objects.iter_mut().find(|o| o.name == "Heroe").unwrap();
        if let Behavior::Platformer { jump_speed, .. } = &mut heroe.behaviors[0] { *jump_speed = 850. }
        for _ in 0..3 { p.scenes[0].instances.push(Instance { obj: "Moneda".into(), ..Default::default() }) }
        fn bump(l: &mut [Event]) {
            for e in l {
                if e.conds.iter().any(|c| c.op == "collides" && c.args.contains(&"Moneda".to_string())) {
                    e.acts.iter_mut().filter(|a| a.op == "add_var").for_each(|a| a.args[1] = "5".into());
                }
                bump(&mut e.sub);
            }
        }
        bump(&mut p.scenes[0].events);
        for m in para("plataformas").filter(|m| !m.plantilla.is_empty()) {
            assert_eq!(pendientes(m, &p, &ini), Vec::<&str>::new(), "«{}»", m.titulo);
        }

        let ini = plantilla("atrapa");
        let mut p = ini.clone();
        p.scenes[0].vars.insert("vidas".into(), Value::Num(5.));
        assert!(pendientes(&MISIONES[7], &p, &ini).is_empty());
    }
}
