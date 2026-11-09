# NicoStation

Motor de juegos 2D en Rust que se programa con **hojas de eventos** (condición → acción), como
GDevelop o Construct. El juego entero es un archivo JSON; no hace falta escribir código.

```
cargo run --release --bin nicostation -- examples/plataformas/juego.json   # jugar
cargo run --release --bin nicostation-editor -- examples/plataformas/juego.json   # editar
cargo test --lib
```

### Jugar

```
nicostation juego.json                                   # ventana
nicostation juego.json --captura foto.png --segundos 2   # juega 2 s sin entrada, guarda el último cuadro (PNG) y sale
```

`--captura` dibuja en una textura del tamaño del juego (no depende de que la ventana se vea), así sirve para pruebas
automáticas y para agentes. No suena nada mientras captura.

- **Ajustes del proyecto** (`settings`): `scale_mode` `fit` (franjas negras), `stretch` (llena la ventana) o `integer`
  (múltiplos enteros, pixel perfect); `pixel_art` (dibujo nítido; si no, suavizado); `max_fps` (0 = los de la pantalla; en web
  manda el navegador); `icon` (ícono de la ventana, cualquier PNG/JPG: se achica a 16/32/64); `loading_image` (fondo de la pantalla de carga).
- **Mandos**: los botones llegan como teclas `Pad1A`, `Pad1B`, `Pad1X`, `Pad1Y`, `Pad1L`, `Pad1R`, `Pad1LT`, `Pad1RT`, `Pad1Start`,
  `Pad1Select`, `Pad1LS`, `Pad1RS`, `Pad1Up/Down/Left/Right` (cruceta), `Pad1Home` (y `Pad2…` para el segundo mando); las palancas y
  gatillos como ejes `Pad1LX`, `Pad1LY`, `Pad1RX`, `Pad1RY` (-1 a 1, y positivo = abajo) y `Pad1LT`, `Pad1RT` (0 a 1), con zona muerta de 0.15.
  Escritorio con gilrs (en Linux usa libudev, que trae cualquier distribución); web con `navigator.getGamepads()`.
- **Sonido**: canales (un sonido con canal reemplaza al que sonaba en ese canal), volumen por sonido, música con fundido cruzado
  (`play_music` con segundos de fundido), «volumen del canal» con fundido (el canal `musica` es la música) y «detener canal».
  Escritorio con kira; web con WebAudio (el plugin de audio de `mq_js_bundle.js`).
- **Efectos** por objeto o instancia (propiedad `effect` y su intensidad `effect_amount` de 0 a 1): `flash`, `outline` (silueta del
  color de la instancia, blanca por defecto), `grayscale`, `wave`. En sprites todos; en fondos y mapas `flash` y `grayscale`; en cajas y
  textos por color. Son shaders GLSL ES 1.00: andan igual en la exportación web (WebGL1).

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

### Exportar a escritorio (un solo ejecutable)

El juego sale como **un único archivo** (sin json, imágenes ni sonidos sueltos), como las exportaciones de GameMaker:
el reproductor con el proyecto y sus archivos pegados al final (formato en `src/pack.rs`).

```
cargo build --release --bin nicostation
./target/release/nicostation --export-desktop examples/plataformas/juego.json mi-juego
./mi-juego        # funciona desde cualquier carpeta; se puede copiar a otra computadora con el mismo sistema
```

Para **Windows** (`mi-juego.exe`) hace falta, una sola vez, el reproductor compilado para Windows:

```
rustup target add x86_64-pc-windows-gnu
sudo pacman -S mingw-w64-gcc          # Arch/Omarchy (Debian/Ubuntu: sudo apt install mingw-w64)
cargo build --release --target x86_64-pc-windows-gnu --bin nicostation
./target/release/nicostation --export-desktop examples/plataformas/juego.json mi-juego --target windows
```

También se puede indicar el reproductor con `NICOSTATION_PLAYER_WINDOWS=/ruta/nicostation.exe`.
Desde código: `nicostation::export::export_desktop(proyecto, salida, DesktopTarget::Current | Windows)`.

En Linux el ejecutable usa `libasound.so.2` (sonido) y `libudev.so.1` (mandos), que vienen en cualquier distribución de escritorio.
El de Windows no necesita nada aparte (sonido con WASAPI, mandos con XInput/WGI).

### Exportar a Android (`.apk`)

Igual que en escritorio, el juego va pegado a un reproductor ya compilado: el **APK plantilla** (el reproductor para Android,
sin juego). Se compila una sola vez con docker (no hace falta instalar el SDK ni el NDK; la imagen pesa ~12 GB):

```
docker build -t nicostation-android assets/android        # una vez (~4 min): NDK r25 + SDK 31 + Rust estable + cargo-quad-apk
docker run --rm -v $PWD:/root/src -v nico-cargo-registry:/usr/local/cargo/registry -w /root/src nicostation-android \
  bash -c 'cargo quad-apk build --release --bin nicostation; chown -R $(id -u):$(id -g) target'
# → target/android-artifacts/release/apk/nicostation.apk (arm64-v8a + armeabi-v7a, Android 8.0+)
./target/release/nicostation --export-android examples/plataformas/juego.json plataformas.apk
adb install -r plataformas.apk
```

`--export-android` copia la plantilla, le pone `juego.json` y sus archivos en `assets/` (de ahí los lee el reproductor en
Android), y la alinea y firma con `zipalign` y `apksigner`: los del SDK si están (`$ANDROID_HOME/build-tools/*` o el PATH), si no
dentro de docker con la imagen `notfl3/cargo-apk` (o `NICOSTATION_ANDROID_IMAGE`). La firma es de depuración: la clave se crea
una vez en `~/.local/share/nicostation/android-debug.keystore` (contraseña `android`; otra con `NICOSTATION_ANDROID_KEYSTORE`).
La plantilla también se busca en `NICOSTATION_ANDROID_TEMPLATE`. Desde código: `nicostation::export::export_android(proyecto, salida)`.

Límites por ahora: todos los juegos salen con el mismo id (`com.nicostation.nicostation`, nombre «NicoStation», sin ícono propio;
instalar uno reemplaza al anterior), en horizontal y sin mandos (los controles táctiles sí). Para la Play Store hace falta firmar
con una clave propia (y Google prefiere AAB). La partida se guarda en la carpeta privada de la app.

### Partida guardada

Lo que el juego guarda con «Guardar en la partida» (`save_value`, se lee con `saved("nombre", 0)`) queda guardado al cerrar:

- Linux: `~/.local/share/nicostation/<nombre-del-juego>/guardado.json` (o `$XDG_DATA_HOME/...`)
- Windows: `%APPDATA%\nicostation\<nombre-del-juego>\guardado.json`; macOS: `~/Library/Application Support/nicostation/...`
- Web: `localStorage` del navegador, clave `nicostation:<nombre-del-juego>`
- Android: carpeta privada de la app, `/data/data/com.nicostation.nicostation/files/nicostation/<nombre-del-juego>/guardado.json`

`<nombre-del-juego>` es el nombre del proyecto en minúsculas, sin acentos y con guiones («Lluvia de asteroides» → `lluvia-de-asteroides`).
Si cambias el nombre del juego, empieza con una partida nueva.

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

### Instalar como aplicación (Linux)

```
./instalar.sh
```

Compila todo y lo instala para tu usuario, sin sudo: programas en `~/.local/share/nicostation/`, accesos directos
en `~/.local/bin/` y la entrada «NicoStation» en el lanzador de aplicaciones. Correrlo de nuevo actualiza la instalación.

## Piezas

| Archivo | Qué hace |
|---|---|
| `src/model.rs` | Formato del proyecto: escenas, capas, objetos, comportamientos, eventos, funciones |
| `src/catalog.rs` | Lista de condiciones y acciones con su frase y tipo de parámetros (el editor se arma con esto) |
| `src/expr.rs` | Expresiones: `Heroe.x + 10`, `"Puntos: " + puntos`, `random(0, 5)`, `count(Moneda)` |
| `src/runtime.rs` | Motor sin ventana: selección de objetos, eventos, comportamientos, colisiones (SAT) |
| `src/export.rs` | Exportación web (wasm + página + archivos) y a escritorio (un solo ejecutable) |
| `src/pack.rs` | Paquete pegado al final del reproductor: el juego entero en un ejecutable |
| `src/validate.rs` | Revisa un proyecto sin ejecutarlo: referencias, instrucciones, expresiones, archivos |
| `src/bin/nicostation-mcp.rs` | Conector MCP para agentes |
| `src/bin/nicostation.rs` | Reproductor (macroquad): dibujo, sonido, teclado, mouse, toques y mandos |
| `src/reproductor/` | Partes del reproductor: escalado/paralaje/fondos (`geometria`), canales y fundidos (`audio`), shaders (`efectos`), mandos (`mando`) |
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
`assets/web/mq_js_bundle.js` viene de macroquad (MIT/Apache-2.0), con el arreglo y el agregado descritos en `assets/web/README.md`.
