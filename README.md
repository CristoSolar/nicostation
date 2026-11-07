# NicoStation

Motor de juegos 2D en Rust que se programa con **hojas de eventos** (condición → acción), como
GDevelop o Construct. El juego entero es un archivo JSON; no hace falta escribir código.

```
cargo run --release --bin nicostation -- examples/plataformas/juego.json   # jugar
cargo run --release --bin nicostation-editor -- examples/plataformas/juego.json   # editar
cargo test --lib
```

### Exportar a web

El reproductor se compila una vez a WebAssembly y sirve para cualquier juego:

```
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown --bin nicostation
cargo run --release --bin nicostation -- --export-web examples/plataformas/juego.json web/
python3 -m http.server -d web 8000   # abrir http://localhost:8000
```

La carpeta `web/` queda lista para subir a cualquier hosting estático (itch.io, GitHub Pages, Netlify).
El editor también exporta desde su botón "Exportar web".

### Conector MCP (para agentes)

`nicostation-mcp` es un servidor MCP por stdio con el que un agente crea un juego completo: proyecto, escenas,
objetos, imágenes (pixel art o formas), sonidos, instancias, eventos y funciones; después lo valida, lo simula sin
ventana con teclas programadas, lo abre en una ventana o lo exporta a web.

```
cargo build --release --bin nicostation-mcp --bin nicostation
claude mcp add --scope user nicostation -- $PWD/target/release/nicostation-mcp
```

Herramientas: `catalogo`, `proyecto_crear`, `proyecto_abrir`, `proyecto_resumen`, `proyecto_json`, `proyecto_configurar`,
`escena_crear`, `escena_editar`, `escena_borrar`, `objeto_poner`, `objeto_borrar`, `mapa_pintar`, `instancias_agregar`, `instancias_quitar`,
`eventos_poner`, `eventos_ver`, `eventos_borrar`, `funcion_poner`, `recetas`, `receta_usar`, `imagen_crear`, `sonido_crear`, `validar`, `simular`,
`jugar`, `exportar_web`. Cada cambio responde con los problemas de validación que quedan.

## Piezas

| Archivo | Qué hace |
|---|---|
| `src/model.rs` | Formato del proyecto: escenas, capas, objetos, comportamientos, eventos, funciones |
| `src/catalog.rs` | Lista de condiciones y acciones con su frase y tipo de parámetros (el editor se arma con esto) |
| `src/expr.rs` | Expresiones: `Heroe.x + 10`, `"Puntos: " + puntos`, `random(0, 5)`, `count(Moneda)` |
| `src/runtime.rs` | Motor sin ventana: selección de objetos, eventos, comportamientos, colisiones (SAT) |
| `src/export.rs` | Exportación web: wasm + página + archivos del juego |
| `src/validate.rs` | Revisa un proyecto sin ejecutarlo: referencias, instrucciones, expresiones, archivos |
| `src/bin/nicostation-mcp.rs` | Conector MCP para agentes |
| `src/bin/nicostation.rs` | Reproductor (macroquad): imágenes, sonido, teclado y mouse |
| `src/bin/nicostation-editor.rs` | Editor visual (egui) |

## Cómo se arma un juego

- **Objetos**: `sprite` (animaciones con imágenes), `text` (con `{expresión}` que se actualiza sola), `box` (rectángulo de color),
  `tilemap` (mapa de baldosas pintado con un tileset; con `platform`, sus baldosas son suelo y paredes, salvo las de decoración en `pass`).
- **Comportamientos**: `platformer`, `platform`, `top_down`, `bullet`, `rotate`, `draggable`, `destroy_outside`, `bound_to_layout`, `screen_wrap`.
- **Plataformas** (como el PlatformerObject de GDevelop): aceleración y frenado, salto sostenido, pendientes
  (una plataforma rotada es una rampa; `slope_max_angle` limita cuánto se sube), escaleras (`ladder`),
  plataformas que se atraviesan desde abajo (`jumpthru`, flecha abajo para bajar), bordes de los que agarrarse
  (`grabbable` + `can_grab_platforms`) y plataformas móviles que llevan al personaje.
- **Eventos**: `conds` (todas deben cumplirse) → `acts`, con `sub`-eventos. Tipos: normal, `repeat`, `for_each`, `while`, `group`, `comment`.
- **Selección de objetos** (como GDevelop): una condición sobre un objeto se queda solo con las instancias que la cumplen,
  y las acciones del evento y sus sub-eventos actúan solo sobre esas. "Crear" deja elegida la instancia nueva.
- **Variables**: de escena, globales (sobreviven al cambiar de escena), de objeto y locales de funciones.
- **Funciones propias**: acciones o condiciones hechas con eventos (`functions` en el proyecto).

Lista completa de instrucciones: `src/catalog.rs`. Ejemplos: `examples/`.

## Créditos

El modelo de eventos, las reglas de selección de objetos y los valores por defecto de los comportamientos
siguen a [GDevelop](https://github.com/4ian/GDevelop) (licencia MIT, © Florian Rival y colaboradores),
estudiado desde su código fuente público. NicoStation es una implementación nueva en Rust y no incluye código de GDevelop.
El análisis está en `docs/`.

Fuentes incluidas en `assets/fonts/`: Geist (SIL Open Font License) e iconos Phosphor (MIT); licencias junto a cada archivo.
`assets/web/mq_js_bundle.js` viene de macroquad (MIT/Apache-2.0), con el arreglo descrito en `assets/web/README.md`.
