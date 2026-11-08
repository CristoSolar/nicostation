//! Catálogo de condiciones y acciones: nombre, frase y tipo de cada parámetro.
//! El editor construye su interfaz a partir de esto; el motor ejecuta por `op`.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum P {
    /// Nombre de objeto o grupo.
    Object,
    /// Expresión (número o texto): `Player.x + 10`, `"Hola " + nombre`.
    Expr,
    /// Texto literal (nombre de temporizador, etc.).
    Text,
    Key,
    Mouse,
    /// Operador de comparación.
    Op,
    Scene,
    /// Propiedad o variable de objeto: x, y, angle, o cualquier nombre.
    Prop,
    Anim,
    File,
    Layer,
    /// Nombre de variable de escena o global.
    Var,
    /// Función propia; los argumentos siguientes son sus parámetros.
    Func,
    /// Curva de suavizado: linear, ease_in, ease_out, ease_in_out, back, elastic, bounce.
    Ease,
    /// Código Rhai (varias líneas).
    Code,
}

pub struct Meta {
    pub op: &'static str,
    pub cond: bool,
    pub group: &'static str,
    pub label: &'static str,
    /// `_0_`, `_1_`... se reemplazan por los argumentos.
    pub sentence: &'static str,
    pub params: &'static [(&'static str, P)],
}

use P::*;

macro_rules! m {
    ($c:expr, $op:literal, $g:literal, $l:literal, $s:literal, [$($pn:literal : $pt:ident),*]) => {
        Meta { op: $op, cond: $c, group: $g, label: $l, sentence: $s, params: &[$(($pn, $pt)),*] }
    };
}

pub const CATALOG: &[Meta] = &[
    // ---- condiciones ----
    m!(true, "start", "Escena", "Al iniciar la escena", "Al iniciar la escena", []),
    m!(true, "trigger_once", "Control", "Solo una vez", "Solo una vez mientras se cumpla lo anterior", []),
    m!(true, "or", "Control", "O (alguna de estas)", "Se cumple alguna de estas:", []),
    m!(true, "and", "Control", "Y (todas estas)", "Se cumplen todas estas:", []),
    m!(true, "every", "Tiempo", "Cada X segundos", "Cada _0_ segundos", ["Segundos": Expr]),
    m!(true, "timer", "Tiempo", "Temporizador", "El temporizador _0_ pasó los _1_ s", ["Nombre": Text, "Segundos": Expr]),
    m!(true, "key_down", "Teclado", "Tecla presionada", "Tecla _0_ presionada", ["Tecla": Key]),
    m!(true, "key_pressed", "Teclado", "Tecla recién presionada", "Tecla _0_ recién presionada", ["Tecla": Key]),
    m!(true, "key_released", "Teclado", "Tecla soltada", "Tecla _0_ soltada", ["Tecla": Key]),
    m!(true, "mouse_down", "Mouse", "Botón presionado", "Botón _0_ del mouse presionado", ["Botón": Mouse]),
    m!(true, "mouse_clicked", "Mouse", "Clic", "Clic _0_ del mouse", ["Botón": Mouse]),
    m!(true, "cursor_on", "Mouse", "Cursor sobre objeto", "El cursor está sobre _0_", ["Objeto": Object]),
    m!(true, "collides", "Objetos", "Colisión", "_0_ choca con _1_", ["Objeto": Object, "Otro": Object]),
    m!(true, "near", "Objetos", "Distancia", "_0_ está a menos de _2_ px de _1_", ["Objeto": Object, "Otro": Object, "Distancia": Expr]),
    m!(true, "compare_prop", "Objetos", "Comparar propiedad", "_1_ de _0_ _2_ _3_", ["Objeto": Object, "Propiedad": Prop, "Comparación": Op, "Valor": Expr]),
    m!(true, "animation_is", "Objetos", "Animación actual", "La animación de _0_ es _1_", ["Objeto": Object, "Animación": Anim]),
    m!(true, "animation_finished", "Objetos", "Animación terminada", "Terminó la animación de _0_", ["Objeto": Object]),
    m!(true, "pick_all", "Objetos", "Elegir todas", "Elegir todas las instancias de _0_", ["Objeto": Object]),
    m!(true, "pick_random", "Objetos", "Elegir una al azar", "Elegir una instancia al azar de _0_", ["Objeto": Object]),
    m!(true, "pick_nearest", "Objetos", "Elegir la más cercana", "Elegir la instancia de _0_ más cercana a (_1_, _2_)", ["Objeto": Object, "X": Expr, "Y": Expr]),
    m!(true, "on_floor", "Movimiento", "En el suelo", "_0_ está en el suelo", ["Objeto": Object]),
    m!(true, "jumping", "Movimiento", "Saltando", "_0_ está saltando", ["Objeto": Object]),
    m!(true, "falling", "Movimiento", "Cayendo", "_0_ está cayendo", ["Objeto": Object]),
    m!(true, "on_ladder", "Movimiento", "En una escalera", "_0_ está en una escalera", ["Objeto": Object]),
    m!(true, "grabbing", "Movimiento", "Agarrado a un borde", "_0_ está agarrado a un borde", ["Objeto": Object]),
    m!(true, "tweening", "Movimiento", "Moviéndose suave", "_0_ está en un movimiento suave", ["Objeto": Object]),
    m!(true, "is_moving", "Movimiento", "Moviéndose", "_0_ se está moviendo", ["Objeto": Object]),
    m!(true, "alarm", "Tiempo", "Alarma", "Sonó la alarma «_1_» de _0_", ["Objeto": Object, "Nombre": Text]),
    m!(true, "compare", "Variables", "Comparar dos valores", "_0_ _1_ _2_", ["Valor": Expr, "Comparación": Op, "Otro valor": Expr]),
    m!(true, "script_if", "Funciones", "Condición en código", "Código: _0_", ["Código Rhai": Code]),
    m!(true, "call_cond", "Funciones", "Condición propia", "_0_(_args_)", ["Función": Func]),
    // ---- acciones ----
    m!(false, "create", "Objetos", "Crear objeto", "Crear _0_ en (_1_, _2_) en la capa _3_", ["Objeto": Object, "X": Expr, "Y": Expr, "Capa": Layer]),
    m!(false, "delete", "Objetos", "Eliminar objeto", "Eliminar _0_", ["Objeto": Object]),
    m!(false, "set", "Objetos", "Cambiar propiedad", "Poner _1_ de _0_ en _2_", ["Objeto": Object, "Propiedad": Prop, "Valor": Expr]),
    m!(false, "add", "Objetos", "Sumar a propiedad", "Sumar _2_ a _1_ de _0_", ["Objeto": Object, "Propiedad": Prop, "Valor": Expr]),
    m!(false, "set_text", "Objetos", "Cambiar texto", "Texto de _0_ = _1_", ["Objeto": Object, "Texto": Expr]),
    m!(false, "set_animation", "Objetos", "Cambiar animación", "Animación de _0_ = _1_", ["Objeto": Object, "Animación": Anim]),
    m!(false, "flip_x", "Objetos", "Voltear horizontal", "Voltear _0_: _1_", ["Objeto": Object, "Sí (1) / No (0)": Expr]),
    m!(false, "look_at", "Objetos", "Mirar hacia", "Girar _0_ hacia (_1_, _2_)", ["Objeto": Object, "X": Expr, "Y": Expr]),
    m!(false, "separate", "Objetos", "Separar de", "Sacar a _0_ de encima de _1_", ["Objeto": Object, "Obstáculo": Object]),
    m!(false, "set_tile", "Objetos", "Cambiar baldosa", "Poner la baldosa _3_ en la columna _1_, fila _2_ de _0_", ["Mapa": Object, "Columna": Expr, "Fila": Expr, "Baldosa (-1 = vacía)": Expr]),
    m!(false, "tween", "Movimiento", "Cambio suave", "Llevar _1_ de _0_ a _2_ en _3_ s (curva _4_)", ["Objeto": Object, "Propiedad": Prop, "Valor final": Expr, "Segundos": Expr, "Curva": Ease]),
    m!(false, "move_forward", "3D", "Avanzar", "Mover _0_ hacia adelante _1_ metros", ["Objeto": Object, "Metros": Expr]),
    m!(false, "move_right", "3D", "Moverse de costado", "Mover _0_ hacia la derecha _1_ metros", ["Objeto": Object, "Metros": Expr]),
    m!(false, "look_at3", "3D", "Mirar hacia (3D)", "Girar _0_ para que mire a (_1_, _2_, _3_)", ["Objeto": Object, "X": Expr, "Y": Expr, "Z": Expr]),
    m!(false, "jump", "Movimiento", "Saltar", "Hacer saltar a _0_", ["Objeto": Object]),
    m!(false, "set_var", "Variables", "Variable de escena", "Variable _0_ = _1_", ["Variable": Var, "Valor": Expr]),
    m!(false, "add_var", "Variables", "Sumar a variable", "Sumar _1_ a la variable _0_", ["Variable": Var, "Valor": Expr]),
    m!(false, "set_global", "Variables", "Variable global", "Global _0_ = _1_", ["Variable": Var, "Valor": Expr]),
    m!(false, "add_global", "Variables", "Sumar a global", "Sumar _1_ a la global _0_", ["Variable": Var, "Valor": Expr]),
    m!(false, "set_alarm", "Tiempo", "Poner alarma", "Poner la alarma «_1_» de _0_ en _2_ segundos", ["Objeto": Object, "Nombre": Text, "Segundos (0 = apagar)": Expr]),
    m!(false, "wait", "Tiempo", "Esperar", "Esperar _0_ segundos", ["Segundos": Expr]),
    m!(false, "reset_timer", "Tiempo", "Reiniciar temporizador", "Reiniciar el temporizador _0_", ["Nombre": Text]),
    m!(false, "change_scene", "Escena", "Cambiar de escena", "Ir a la escena _0_", ["Escena": Scene]),
    m!(false, "restart_scene", "Escena", "Reiniciar escena", "Reiniciar la escena", []),
    m!(false, "camera", "Escena", "Centrar cámara", "Centrar la cámara en (_0_, _1_)", ["X": Expr, "Y": Expr]),
    m!(false, "camera_follow", "Escena", "Cámara sigue", "La cámara sigue a _0_ (suavidad _1_)", ["Objeto": Object, "Suavidad 0 a 0.99": Expr]),
    m!(false, "camera_bounds", "Escena", "Límites de cámara", "Cámara dentro de (_0_, _1_) a (_2_, _3_)", ["X izquierda": Expr, "Y arriba": Expr, "X derecha": Expr, "Y abajo": Expr]),
    m!(false, "camera_shake", "Escena", "Temblor de cámara", "Hacer temblar la cámara _0_ px durante _1_ s", ["Intensidad px": Expr, "Segundos": Expr]),
    m!(false, "camera_zoom_to", "Escena", "Zoom suave", "Llevar el zoom a _0_ en _1_ s", ["Zoom": Expr, "Segundos": Expr]),
    m!(false, "camera_zoom", "Escena", "Zoom de cámara", "Zoom de la cámara = _0_", ["Zoom": Expr]),
    m!(false, "play_sound", "Audio", "Reproducir sonido", "Reproducir _0_ al volumen _1_", ["Archivo": File, "Volumen 0-100": Expr]),
    m!(false, "play_music", "Audio", "Reproducir música", "Reproducir música _0_ en bucle al volumen _1_", ["Archivo": File, "Volumen 0-100": Expr]),
    m!(false, "stop_music", "Audio", "Detener música", "Detener la música", []),
    m!(false, "stop_sound", "Audio", "Detener un sonido", "Detener el sonido _0_", ["Archivo": File]),
    m!(false, "save_value", "Variables", "Guardar en la partida", "Guardar _1_ en la partida como «_0_»", ["Nombre": Text, "Valor": Expr]),
    m!(false, "erase_saved", "Variables", "Borrar de la partida", "Borrar «_0_» de la partida guardada", ["Nombre": Text]),
    m!(false, "script", "Funciones", "Ejecutar código", "Código: _0_", ["Código Rhai": Code]),
    m!(false, "call", "Funciones", "Acción propia", "_0_(_args_)", ["Función": Func]),
    m!(false, "return", "Funciones", "Devolver (condición propia)", "Devolver _0_", ["Verdadero/falso": Expr]),
    m!(false, "quit", "Escena", "Salir del juego", "Salir del juego", []),
];

pub const EASES: &[&str] = &["linear", "ease_in", "ease_out", "ease_in_out", "back", "elastic", "bounce"];
pub const OPS: &[&str] = &["=", "!=", "<", "<=", ">", ">="];
pub const MOUSE: &[&str] = &["Left", "Right", "Middle"];
pub const KEYS: &[&str] = &[
    "Left", "Right", "Up", "Down", "Space", "Enter", "Escape", "Tab", "Backspace", "LeftShift",
    "LeftControl", "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P",
    "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z", "Key0", "Key1", "Key2", "Key3", "Key4",
    "Key5", "Key6", "Key7", "Key8", "Key9",
];
/// Propiedades de objeto conocidas (además, cualquier otro nombre es una variable del objeto).
pub const PROPS: &[&str] = &["x", "y", "cx", "cy", "w", "h", "angle", "opacity", "z", "speed", "visible", "scale_x", "scale_y", "tint", "d", "pitch", "roll", "scale_z"];

pub fn meta(op: &str) -> Option<&'static Meta> {
    CATALOG.iter().find(|m| m.op == op)
}

/// Frase legible: `"_0_ choca con _1_"` + args → `"Player choca con Coin"`.
pub fn sentence(i: &crate::model::Instr) -> String {
    let Some(m) = meta(&i.op) else { return format!("¿{}? {:?}", i.op, i.args) };
    let mut s = m.sentence.to_string();
    for (n, a) in i.args.iter().enumerate() {
        s = s.replace(&format!("_{n}_"), if a.is_empty() { "\"\"" } else { a });
    }
    if m.params.last().is_some_and(|p| p.1 == Func) {
        s = s.replace("_args_", &i.args.iter().skip(1).cloned().collect::<Vec<_>>().join(", "));
    }
    let s = if i.not { format!("NO: {s}") } else { s };
    s
}

// ---------- Textos para niños (modo fácil del editor) ----------

/// Explicación corta con ejemplo, y si el bloque se muestra en el modo fácil (los demás van en "Más bloques").
pub struct Kid {
    pub help: &'static str,
    pub basic: bool,
}

pub fn kid(op: &str) -> Kid {
    let (help, basic) = match op {
        "start" => ("Pasa una sola vez, justo cuando empieza la escena. Sirve para preparar cosas: poner los puntos en 0, poner música.", true),
        "trigger_once" => ("Hace que el evento pase una sola vez aunque lo demás se siga cumpliendo. Ejemplo: sonar un sonido solo cuando llegas a 10 puntos, no a cada rato.", true),
        "or" => ("Se cumple si pasa AL MENOS UNA de las cosas de adentro. Ejemplo: tocar un enemigo O caer al vacío.", false),
        "and" => ("Se cumple solo si pasan TODAS las cosas de adentro. Se usa dentro de un bloque O.", false),
        "every" => ("Pasa una y otra vez, cada tantos segundos. Ejemplo: cada 2 segundos aparece una moneda.", true),
        "timer" => ("Un cronómetro con nombre. Se cumple cuando ya pasaron esos segundos desde que empezó (o desde que lo reiniciaste).", false),
        "key_down" => ("Se cumple mientras mantienes apretada la tecla. Ideal para moverse.", true),
        "key_pressed" => ("Pasa solo en el instante en que aprietas la tecla. Ideal para saltar o disparar una vez por toque.", true),
        "key_released" => ("Pasa en el instante en que sueltas la tecla.", false),
        "mouse_down" => ("Se cumple mientras mantienes apretado el botón del mouse.", false),
        "mouse_clicked" => ("Pasa en el instante en que haces clic.", true),
        "cursor_on" => ("Se cumple cuando la flechita del mouse está encima del objeto. Junto con «clic» sirve para hacer botones.", true),
        "collides" => ("Se cumple cuando dos objetos se tocan. Ejemplo: el héroe toca una moneda.", true),
        "near" => ("Se cumple cuando un objeto está cerca de otro. Ejemplo: el enemigo te persigue si estás a menos de 200.", false),
        "compare_prop" => ("Mira un dato de un objeto, como su posición. Ejemplo: si el héroe está más abajo que 700 (se cayó).", true),
        "animation_is" => ("Se cumple si el objeto está mostrando esa animación.", false),
        "animation_finished" => ("Se cumple cuando la animación del objeto llegó al final (solo las que no se repiten).", false),
        "pick_all" => ("Vuelve a tener en cuenta a todos los objetos de ese tipo, aunque una condición anterior haya elegido solo algunos.", false),
        "pick_random" => ("Elige uno solo de esos objetos, al azar.", false),
        "pick_nearest" => ("Elige el objeto de ese tipo que esté más cerca de un punto.", false),
        "on_floor" => ("Se cumple cuando el personaje de plataformas está parado en el suelo.", true),
        "jumping" => ("Se cumple mientras el personaje de plataformas va subiendo en un salto.", true),
        "falling" => ("Se cumple mientras el personaje de plataformas va cayendo.", true),
        "on_ladder" => ("Se cumple cuando el personaje está trepando una escalera.", false),
        "grabbing" => ("Se cumple cuando el personaje está colgado del borde de una plataforma.", false),
        "is_moving" => ("Se cumple si el objeto se está moviendo.", true),
        "alarm" => ("Pasa una vez cuando suena la alarma de un objeto. Cada copia del objeto tiene sus propias alarmas. Ejemplo: cada enemigo dispara cuando suena su alarma «disparo».", false),
        "set_alarm" => ("Pone una alarma en un objeto que suena dentro de tantos segundos. Para repetir, vuelve a ponerla cuando suene.", false),
        "wait" => ("Espera antes de hacer los bloques que vienen después en el mismo evento. Ejemplo: aparecer un cartel, esperar 2 segundos y borrarlo.", true),
        "compare" => ("Compara dos valores. Ejemplo: si los puntos son mayores o iguales que 10.", true),
        "call_cond" => ("Usa una condición que inventaste en «Mis bloques».", true),
        "create" => ("Hace aparecer un objeto nuevo en un lugar. Ejemplo: crear una bala donde está el héroe.", true),
        "delete" => ("Hace desaparecer el objeto. Ejemplo: la moneda que tocaste.", true),
        "set" => ("Cambia un dato del objeto a un valor exacto. Ejemplo: poner la X del héroe en 100 (lo teletransporta).", true),
        "add" => ("Suma (o resta, con un número negativo) a un dato del objeto. Ejemplo: sumar 5 a la X para moverlo a la derecha.", true),
        "set_text" => ("Cambia lo que dice un objeto de texto. Ejemplo: \"¡Ganaste!\".", true),
        "set_animation" => ("Cambia la animación que muestra el objeto. Ejemplo: «correr» cuando se mueve.", true),
        "flip_x" => ("Da vuelta el dibujo como en un espejo, para que mire al otro lado. 1 = dado vuelta, 0 = normal.", true),
        "look_at" => ("Gira el objeto para que mire hacia un punto. Ejemplo: el enemigo mira al héroe.", false),
        "separate" => ("Empuja al objeto hacia afuera para que no atraviese el otro. Ejemplo: que el héroe no atraviese las paredes.", true),
        "set_tile" => ("Cambia una baldosa del mapa: sirve para romper bloques o abrir puertas. -1 la deja vacía.", false),
        "tween" => ("Cambia un dato poco a poco en vez de golpe: mover, agrandar o hacer transparente con suavidad. La curva decide cómo: «bounce» rebota, «elastic» es como un resorte.", true),
        "tweening" => ("Se cumple mientras el objeto está en un cambio suave.", false),
        "camera_follow" => ("La cámara sigue al objeto con suavidad. 0 = pegada, 0.9 = muy suave.", true),
        "camera_bounds" => ("La cámara no muestra nada fuera de este rectángulo: pon el tamaño de tu nivel.", true),
        "camera_shake" => ("Hace temblar la pantalla, como en una explosión.", true),
        "camera_zoom_to" => ("Acerca o aleja la cámara poco a poco.", false),
        "move_forward" => ("Mueve el objeto hacia donde está mirando. Usa «por segundo» (dt) para que se mueva parejo: 5 * dt.", true),
        "move_right" => ("Mueve el objeto de costado, sin girarlo (para caminar como cangrejo).", false),
        "look_at3" => ("Gira el objeto 3D para que mire a un punto. Ejemplo: el enemigo mira al jugador.", false),
        "jump" => ("Hace saltar al personaje de plataformas, como si apretaras la tecla de salto.", false),
        "set_var" => ("Guarda un valor en una cajita (variable) de esta escena. Ejemplo: poner vidas en 3.", true),
        "add_var" => ("Suma a una cajita (variable) de esta escena. Ejemplo: sumar 1 a los puntos.", true),
        "set_global" => ("Guarda un valor en una cajita del juego entero: no se borra al cambiar de escena. Ejemplo: el récord.", false),
        "add_global" => ("Suma a una cajita del juego entero.", false),
        "reset_timer" => ("Pone un cronómetro de vuelta en cero.", false),
        "change_scene" => ("Pasa a otra escena. Ejemplo: del menú al nivel 1, o al nivel 2 cuando llegas a la meta.", true),
        "restart_scene" => ("Empieza la escena de nuevo, como al perder.", true),
        "camera" => ("Mueve la cámara para mirar a un punto. Ejemplo: centrarla en el héroe para que lo siga.", true),
        "camera_zoom" => ("Acerca o aleja la cámara. 1 = normal, 2 = el doble de cerca.", false),
        "play_sound" => ("Hace sonar un efecto de sonido una vez.", true),
        "play_music" => ("Pone una música que se repite todo el tiempo.", true),
        "stop_music" => ("Detiene la música.", true),
        "stop_sound" => ("Detiene un sonido que está sonando.", false),
        "save_value" => ("Guarda un valor para la próxima vez que juegues, aunque cierres el juego. Ejemplo: guardar el récord. Para leerlo usa saved(\"récord\", 0).", true),
        "erase_saved" => ("Borra un valor guardado de la partida.", false),
        "script" => ("Para programadores: ejecuta código Rhai. Puede leer y cambiar cajitas, objetos y más.", false),
        "script_if" => ("Para programadores: código Rhai que responde verdadero o falso.", false),
        "call" => ("Usa un bloque que inventaste en «Mis bloques».", true),
        "return" => ("Dentro de un bloque-pregunta de «Mis bloques»: responde sí (1) o no (0).", false),
        "quit" => ("Cierra el juego.", false),
        _ => ("", false),
    };
    Kid { help, basic }
}

/// Comparaciones en palabras.
pub fn op_words(op: &str) -> &'static str {
    match op {
        "=" => "es igual a",
        "!=" => "es distinto de",
        "<" => "es menor que",
        "<=" => "es menor o igual que",
        ">" => "es mayor que",
        ">=" => "es mayor o igual que",
        _ => "?",
    }
}

/// Nombre de tecla para mostrar.
pub fn key_label(k: &str) -> String {
    match k {
        "Left" => "Flecha izquierda".into(),
        "Right" => "Flecha derecha".into(),
        "Up" => "Flecha arriba".into(),
        "Down" => "Flecha abajo".into(),
        "Space" => "Barra espaciadora".into(),
        "Enter" => "Enter".into(),
        "Escape" => "Esc".into(),
        "Tab" => "Tab".into(),
        "Backspace" => "Borrar".into(),
        "LeftShift" => "Shift".into(),
        "LeftControl" => "Ctrl".into(),
        _ => k.strip_prefix("Key").unwrap_or(k).into(),
    }
}

/// Propiedades de objeto explicadas.
pub fn prop_label(p: &str) -> String {
    match p {
        "x" => "posición X (izquierda a derecha)".into(),
        "y" => "posición Y (arriba a abajo)".into(),
        "cx" => "centro X".into(),
        "cy" => "centro Y".into(),
        "w" => "ancho".into(),
        "h" => "alto".into(),
        "angle" => "ángulo (giro)".into(),
        "opacity" => "transparencia (0 invisible, 255 normal)".into(),
        "z" => "adelante o atrás (más alto = más adelante)".into(),
        "speed" => "velocidad".into(),
        "visible" => "visible (1 sí, 0 no)".into(),
        "d" => "profundidad (3D)".into(),
        "pitch" => "inclinación arriba/abajo (3D)".into(),
        "roll" => "giro de costado (3D)".into(),
        "scale_z" => "tamaño a lo profundo (3D)".into(),
        "scale_x" => "tamaño a lo ancho (1 normal, 2 el doble)".into(),
        "scale_y" => "tamaño a lo alto (1 normal, 2 el doble)".into(),
        "tint" => "color (\"rojo\", \"#ff0000\"...)".into(),
        _ => format!("variable «{p}» del objeto"),
    }
}

pub fn ease_label(e: &str) -> &'static str {
    match e {
        "ease_in" => "empieza lento",
        "ease_out" => "termina lento",
        "ease_in_out" => "lento al empezar y al terminar",
        "back" => "se pasa un poquito y vuelve",
        "elastic" => "como un resorte",
        "bounce" => "rebota",
        _ => "parejo",
    }
}

pub fn mouse_label(b: &str) -> &'static str {
    match b {
        "Right" => "botón derecho",
        "Middle" => "botón del medio (rueda)",
        _ => "botón izquierdo",
    }
}
