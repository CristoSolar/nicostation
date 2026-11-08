# Tecnología para el 3D de NicoStation

Evaluación con prototipos reales (compilados y ejecutados) de qué pila Rust usar para agregar a NicoStation un motor 3D
con calidad visual tipo Godot, sin romper lo que ya existe: el reproductor 2D (macroquad 0.4), el editor (eframe/egui 0.37
sobre wgpu 30), la vista previa dentro del editor y la exportación web (wasm32 con el cargador JS de macroquad).

Fecha: 2026-11-08. Todos los prototipos están en `/tmp/spike3d/` (ver §11). En este repositorio solo se escribió este archivo.

---

## 1. Resumen y recomendación

**Pila recomendada**

| Pieza | Elección | Versión fijada |
|---|---|---|
| Render 3D | **kiss3d** (wgpu 30), detrás de una fachada propia `render3d` | `=0.47.0` |
| Física 3D | **rapier3d** (`KinematicCharacterController` más una capa nuestra estilo `move_and_slide`) | `0.36` |
| Modelos | **glTF 2.0** (cargador de kiss3d para dibujar, crate `gltf` para datos y animación propia); FBX y OBJ solo al importar en el editor (ufbx / tobj), convertidos a glTF | `gltf 1.4` |
| Audio 3D | **kira** (oyente más pistas espaciales) | `0.12` |
| Plan B del render | **Bevy 0.20** como biblioteca (mismo wgpu 30, misma integración con egui, también probada) | `0.20` |

**Por qué kiss3d.** Es la única opción que reúne las tres cosas que necesitamos:

1. **Funciona dentro del editor sin copias.** Usa wgpu 30, igual que eframe 0.37. Se probó que el editor crea el `wgpu::Device`, se lo
   pasa a kiss3d (`Context::init`), y la textura que dibuja kiss3d se registra en egui con `register_native_texture`.
2. **Tiene casi todo lo de Godot.** PBR metal/rugosidad (con clearcoat y anisotropía), IBL desde un HDR, sombras en cascada
   (hasta 4) y sombras de luces puntuales y focos, SSAO, SSR, bloom, tonemapping, profundidad de campo, niebla, y animación con
   esqueleto en GPU más morph targets desde glTF. Todo se vio funcionando en una captura.
3. **Pesa razonable.** 20.7 MB nativo, 4.5 MB de wasm (1.42 MB comprimido), 64 s de compilación desde cero. Bevy mide
   86 MB, 7.7 MB y 119 s.

Además lo mantiene dimforge, que también hace rapier, y ambos usan la misma biblioteca matemática (glamx 0.3). Su API es un grafo
de escena simple, no un ECS, así que encaja con el `World` propio de NicoStation: el runtime manda y el render solo refleja las instancias.

**Lo que cuesta.** El reproductor 3D tiene que ser **otro binario** (`nicostation3d`: winit, wgpu y wasm-bindgen). macroquad
no puede alojar un render 3D moderno: no tiene luces ni sombras, y meter three-d en su contexto GL corrompe su estado (medido, ver §4.6).
Además, en la web kiss3d usa **WebGPU**; su modo WebGL2 está pensado pero no se pudo comprobar aquí (§8). Los juegos 2D siguen
igual que hoy, con macroquad.

**Números clave** (Ryzen 7 5700X con 16 hilos, Radeon RX 6800 con Mesa 26.2, rustc 1.99.1, `--release`, `strip = true`):

| | Compilar desde cero | Binario nativo | wasm (opt-z + LTO, después de bindgen) | Cuadro 1280×720 |
|---|---|---|---|---|
| kiss3d 0.47 | 63.6 s | 20.7 MB | 4.50 MB / **1.42 MB gz** | 0.77 ms (sombras, SSAO, bloom, skinning) |
| Bevy 0.20 (`3d`) | 118.9 s | 86.3 MB | 7.66 MB / 1.47 MB gz | 1.45 ms (CSM, SSAO, bloom) |
| three-d 0.19 | 21.7 s | 4.8 MB | 1.17 MB / 0.37 MB gz | 0.67 ms (render más lectura a CPU) |
| rapier3d 0.36 | 18.4 s | 2.8 MB | 1.21 MB / 0.46 MB gz | 0.011 ms por paso (1 personaje) |
| *Hoy: reproductor 2D* | — | 7.2 MB | 4.22 MB / 1.00 MB gz | — |

---

## 2. Qué necesita NicoStation (requisitos que guiaron la evaluación)

- **Runtime sin ventana.** `src/runtime.rs` recibe `Input` y `dt` y produce estado y `Cmd`. Esto lo usan el MCP (`simular`), las
  pruebas y la vista previa. El 3D tiene que respetarlo: la física y la lógica van en la biblioteca, y el render es solo una vista.
- **Editor egui sobre wgpu**, con `--captura` fuera de pantalla (`src/editor/captura.rs`) y la vista previa dibujada con el
  `Painter` de egui (`src/editor/vista.rs`). Un visor 3D dentro del editor necesita **dibujar en una textura que egui muestre**.
- **Exportación web.** Hoy `nicostation.wasm` se carga con `mq_js_bundle.js` de macroquad, que no usa wasm-bindgen, más el
  plugin propio `nicostation_storage`.
- **Escritorio en un solo ejecutable** (`src/pack.rs`) y compilación cruzada para Windows.
- **Calidad tipo Godot:** PBR, sombras en cascada, IBL, posprocesado (tonemap, bloom, SSAO), personajes con esqueleto, física con
  personaje cinemático (`CharacterBody3D.move_and_slide`), audio 3D.

---

## 3. Tabla comparativa (medida)

«Medido» quiere decir que se compiló y ejecutó en esta máquina; «doc.» quiere decir que sale del código fuente o la documentación de la crate, sin ejecutarlo.

| | **kiss3d 0.47.0** | **Bevy 0.20.1** | **three-d 0.19.0** | **wgpu 30 propio** | **macroquad 0.4.16** |
|---|---|---|---|---|---|
| Última versión | 2026-10-04 (10 versiones en 2026) | 2026-10-28 | 2026-04-17 (último commit en junio de 2026) | 30.0.1 del 2026-08-22 | 2026-07-30 |
| Licencia | BSD-3-Clause | MIT/Apache-2.0 | MIT | MIT/Apache-2.0 | MIT/Apache-2.0 |
| Backend | wgpu 30: Vulkan, Metal, DX12; web WebGPU (WebGL2 parcial) | wgpu 30: igual; web WebGL2 o WebGPU | OpenGL 3.3 / WebGL2 (glow 0.17) | wgpu 30 | GL 2/3, WebGL1/2 propio |
| Compilar desde cero, nativo | 63.6 s | 118.9 s | 21.7 s | 36.4 s (con eframe) | 12.9 s |
| Recompilar (tocar main) | 4.1 s | 1.2 s | 1.6 s | 0.8 s | — |
| Binario nativo con `strip` | 20.7 MB | 86.3 MB | 4.8 MB | 15.4 MB (con eframe) | 2.5 MB |
| `target/` después de nativo y wasm | ~3.1 GB | ~4.1 GB (y se llenó la cuota de /tmp en debug) | 1.8 GB | 0.8 GB | 0.5 GB |
| wasm release (bindgen) | 6.79 MB / 1.91 MB gz | — | 1.49 MB / 0.46 MB gz | — | — |
| wasm opt-z + LTO (bindgen) | 4.50 MB / 1.42 MB gz | 7.66 MB / 1.47 MB gz (177 s) | 1.17 MB / 0.37 MB gz | — | 4.22 MB / 1.00 MB gz (reproductor actual) |
| PBR | sí, metal/rugosidad, clearcoat, anisotropía (medido) | sí, el más completo (medido) | sí, Cook-Torrance (medido) | a escribir | no (sin luces) |
| Sombras | direccional con cascadas (hasta 4), puntual y foco; atlas con suavizado (medido y doc.) | CSM, puntual, foco, PCSS (CSM medido) | un mapa por luz direccional o foco, sin cascadas (medido) | a escribir | no |
| IBL / cielo HDR | sí (medido) | sí (EnvironmentMapLight, doc.) | sí (medido) | a escribir | no |
| Posprocesado | tonemap, bloom, SSAO, SSR, DoF, niebla, color grading (SSAO y bloom medidos) | todo: TAA, SMAA, FXAA, bloom, SSAO, SSR, DoF, motion blur, niebla volumétrica, autoexposición (bloom y SSAO medidos) | tonemap ACES/Reinhard/Filmic, FXAA, niebla | a escribir | no |
| Esqueleto (skinning) | **sí, en GPU**, más morph (medido con Fox.glb) | sí, en GPU, más morph y grafo de animación (doc.) | **no** (solo animación de nodos) | a escribir | no |
| Mezcla de animaciones | no (un clip a la vez) | sí | no | a escribir | no |
| Dentro de egui 0.37 | **sí, mismo `wgpu::Device`, sin copias (medido)** | **sí, mismo `wgpu::Device`, sin copias (medido)**; hay que manejar `App` a mano y apagar `PipelinedRenderingPlugin` | sí con el backend glow de eframe (PaintCallback, medido) o leyendo a CPU (0.67 ms/cuadro) | sí, sin copias (medido, 0.067 ms/cuadro) | no |
| Web probado aquí | carga y corre sin panic; no se pudo capturar la imagen de WebGPU en Chromium sin ventana; WebGL2 no se pudo forzar | no ejecutado (solo compilado) | **sí, WebGL2 en Chromium sin ventana (captura)** | — | sí (hoy) |
| Encaje con el modelo NicoStation | grafo de escena retenido, fácil de reflejar | ECS más servidor de assets propio: dos «mundos» | API inmediata y simple | total, pero hay que escribirlo todo | — |

Descartados después de revisarlos:

- **rend3**: el repositorio está archivado (último push 2024-07, versión 0.3.0 de 2022).
- **Fyrox 1.0**: es un motor completo con su propio editor, no una biblioteca para incrustar.
- **avian3d**: es solo para Bevy.
- **blade-graphics**: es una capa GPU, no un render.

---

## 4. Detalle por candidato

### 4.1 kiss3d 0.47 (recomendado)

Prototipos: `/tmp/spike3d/kiss3d/src/main.rs` (fuera de pantalla) y `src/bin/egui_embed.rs` (dentro de egui).

- **Escena probada:** DamagedHelmet (glTF PBR) más un zorro corriendo (Fox.glb, esqueleto de 24 huesos, skinning en GPU), sol con
  sombras, cielo HDR equirectangular (también da el IBL), SSAO y bloom. Carga en 257 ms, **0.77 ms por cuadro** a 1280×720.
  Captura: `/tmp/spike3d/shots/kiss3d.png`.
- **Dentro de egui:** el anfitrión crea instancia, adaptador y dispositivo, llama a `kiss3d::context::Context::init(...)` y después
  `OffscreenSurface::new`. kiss3d **reutiliza** ese contexto (`wgpu_canvas.rs`: «Reuse the wgpu context if one already exists»).
  `surface.output_view()` se registra en `egui_wgpu::Renderer` y se muestra con `ui.image`. Captura:
  `/tmp/spike3d/shots/kiss3d-in-egui.png`. eframe 0.37 expone `RenderState { instance, adapter, device, queue, .. }`, así que en el
  editor real es una sola llamada. Hay que pedirle a eframe un dispositivo con los límites y funciones del adaptador
  (`WgpuSetupCreateNew::device_descriptor`), porque kiss3d los usa.
- **Detalles:**
  - El contexto global es un `thread_local`: un dispositivo por hilo, lo cual sirve para el editor.
  - Formato de salida `Rgba8Unorm`: la corrección gamma la hace el shader, y los colores salieron bien en egui.
  - Animación: `AnimationPlayer` con `play`, `seek`, `speed` y bucle, pero **sin mezcla**. La mezcla la podemos hacer nosotros:
    muestrear el glTF en CPU y escribir las transformaciones de los nodos. Medido: 0.002 ms por cuadro para 24 huesos (§6).
  - La crate trae egui opcional en la versión **0.36**. No hay que activarlo: usamos nuestro egui 0.37.
- **Riesgos:** la API cambia mucho (10 versiones menores con cambios incompatibles en 2026), la mantiene básicamente una empresa
  pequeña, y la web depende de WebGPU. Mitigación: fijar `=0.47.0`, esconderla detrás de `render3d` (§7.2) y tener Bevy probado
  como plan B con el mismo patrón de integración.

### 4.2 Bevy 0.20 como biblioteca (plan B)

Prototipos: `/tmp/spike3d/bevy/src/main.rs` (sin winit, bucle manejado desde afuera) y `src/bin/egui_embed.rs`.

- **Escena probada:** casco, suelo, sol con **CSM de 3 cascadas**, luz puntual, **SSAO**, **bloom** y tonemapping TonyMcMapface.
  1.45 ms por cuadro a 1280×720. Captura: `/tmp/spike3d/shots/bevy.png`.
- **Dentro de egui con el mismo dispositivo:** `RenderPlugin { render_creation: RenderCreation::manual(RenderDevice::from(device), ...) }`
  más `.disable::<WinitPlugin>()` y `.disable::<PipelinedRenderingPlugin>()`. Sin lo último, el mundo de render se va a otro
  hilo y no se puede leer su `GpuImage`. Después se toma `RenderAssets<GpuImage>` del sub-app `RenderApp`, se registra su
  `texture_view` en egui, y queda sin copias. 2.47 ms por cuadro a 860×600. Captura: `/tmp/spike3d/shots/bevy-in-egui.png`. La
  imagen sale algo oscura porque es una textura sRGB muestreada por egui; se arregla con una vista `Rgba8Unorm` (detalle menor).
  Hay que crear el dispositivo con las funciones experimentales activadas, igual que hace Bevy.
- **Contras medidos:** 86 MB de binario, 119 s desde cero, 177 s para el wasm con LTO, y un `target/` de unos 4 GB (la compilación
  debug llenó la cuota de /tmp). Además es un ECS con su propio servidor de assets y su propio bucle: tendríamos dos mundos (el
  `World` de NicoStation y el de Bevy) que sincronizar. Sale una versión con cambios incompatibles cada trimestre.
- **Cuándo pasarse a Bevy:** si necesitamos TAA, niebla volumétrica, mezcla y grafos de animación, iluminación global en tiempo
  real, o si kiss3d se estanca o se rompe. La fachada `render3d` hace que el cambio afecte a un solo módulo.

### 4.3 three-d 0.19

Prototipos: `/tmp/spike3d/three-d/src/lib.rs` (escena), `main.rs` (ventana o PNG con EGL sin superficie) y `src/bin/egui_glow.rs`.

- **Escena probada:** casco PBR, IBL con HDR, sombra direccional, cielo y ACES. PNG fuera de pantalla en unos 360 ms con el
  contexto EGL de Mesa. Capturas: `/tmp/spike3d/shots/three-d.png` y `/tmp/spike3d/shots/three-d-web.png` (**la misma escena en
  WebGL2 dentro de Chromium sin ventana**).
- **Dentro de egui:** **eframe 0.37 usa glow 0.17 en egui_glow, la misma versión que three-d 0.19**. Con
  `egui_glow::CallbackFn` y `three_d::Context::from_gl_context(painter.gl().clone())`, three-d dibuja en el mismo contexto GL que
  egui. Funciona y no hace falta restaurar estado. Captura: `/tmp/spike3d/shots/three-d-in-egui.png`. Pero eso obliga a pasar el
  editor de `Renderer::Wgpu` a `Renderer::Glow` y a rehacer `--captura` (hoy usa wgpu). La otra opción, sin cambiar el
  editor, es leer la imagen a CPU: 0.67 ms por cuadro a 1280×720 con render y lectura, más la subida de la textura a egui.
- **Web:** para cargar los assets en la web, three-d-asset necesita reqwest. En el prototipo se usó `fetch` propio con web-sys,
  que es como lo haríamos nosotros.
- **Por qué no:** **no tiene skinning** (solo animación de nodos; en el código no aparecen `skin` ni `joint`), no tiene cascadas,
  ni bloom, ni SSAO. Lo mantiene una sola persona y hubo poca actividad en 2026. Además ata la versión de glow a la de eframe. Sirve
  para un «3D sencillo», pero no llega a la meta de Godot.

### 4.4 wgpu propio

Prototipo: `/tmp/spike3d/wgpu-egui/src/render3d.rs` (unas 150 líneas) y `main.rs`.

- **Qué hace:** malla glTF con textura base, luz direccional y depth, dibujada en una textura propia y mostrada en egui con
  `register_native_texture`. **0.067 ms por cuadro** a 860×600. Captura: `/tmp/spike3d/shots/wgpu-egui.png`. Confirma que el editor
  ya tiene todo para alojar cualquier render wgpu sin copias.
- **Esfuerzo estimado para igualar a kiss3d** (una persona):

  | Tarea | Semanas |
  |---|---|
  | PBR con IBL (preintegración de BRDF, mapas prefiltrados) | 3–4 |
  | Sombras en cascada con PCF | 2–3 |
  | Skinning y morph | 2 |
  | Posprocesado HDR (tonemap, bloom, SSAO) | 3 |
  | Materiales y transparencias | 2 |
  | Instancing y culling | 2 |
  | Web (WebGPU y límites de WebGL2) | 2 |
  | **Total** | **unas 16–20 semanas**, más el mantenimiento |

  No se justifica teniendo kiss3d y Bevy sobre la misma versión de wgpu.

### 4.5 macroquad 0.4 (línea base)

Prototipo: `/tmp/spike3d/macroquad/src/main.rs`.

- **3D propio:** `Camera3D`, `draw_mesh`, `draw_cube` y `draw_grid`, **sin luces, sin sombras y sin materiales**. No trae cargador de
  glTF: en el prototipo el casco se carga a mano con la crate `gltf`.
- **Problemas medidos:**
  - Con la configuración por defecto, `draw_mesh` **recorta** mallas grandes: «geometry() exceeded max drawcall size, clamping», 90
    veces con un casco de 14 556 vértices. Hay que subir `draw_call_vertex_capacity` y `draw_call_index_capacity` en
    `macroquad::conf::Conf`.
  - Los índices son u16, así que el máximo es de 65 535 vértices por malla.
- **Capturas:** `/tmp/spike3d/shots/macroquad-base.png`.

### 4.6 ¿Poner three-d dentro de macroquad? (convivencia en un mismo contexto GL)

Se probó en escritorio. Con `glXGetProcAddressARB`, glow y `three_d::Context::from_gl_context` sobre el contexto de miniquad,
three-d dibuja PBR, sombras e IBL correctamente en media ventana. Pero **el texto que macroquad dibuja después sale corrupto**
(`/tmp/spike3d/shots/macroquad-three-d.png`), aunque se restauren `UNPACK_ALIGNMENT` y `PACK_ALIGNMENT`. miniquad guarda en caché
el estado GL (texturas, programas y buffers ligados), three-d lo cambia por detrás, y miniquad no ofrece una forma pública de
invalidar esa caché. En la web es peor: glow necesita un `WebGl2RenderingContext` de web-sys, es decir wasm-bindgen, y el cargador
de macroquad no usa wasm-bindgen. **Conclusión: no mezclar en el mismo contexto.** El 3D va en su propio reproductor.

Nota: en esta sesión el monitor estaba apagado (DPMS), así que `grim` no servía. Las ventanas de macroquad se ejecutaron con
`swap_interval = 0` sobre Xwayland y la imagen se sacó con `get_screen_data()`. Todo lo demás se dibujó fuera de pantalla.

---

## 5. Física: rapier3d 0.36

Prototipo: `/tmp/spike3d/rapier/src/lib.rs`. El mismo código corre en nativo y en wasm (con Node) y da **exactamente los mismos
resultados**, algo importante para que `simular` sea determinista.

| Caso | Resultado medido |
|---|---|
| Caer y apoyarse | y = 0.910 (cápsula de 0.8 más offset de 0.01 sobre un suelo en 0.1), `grounded = true` |
| Escalón de 0.25 con `autostep` de 0.3 | sube: el centro llega a y = 1.163 |
| Pared en diagonal | se detiene en x = 7.49 (pared en 7.8 menos el radio de 0.3) y **desliza** dz = 1.93; `on_wall = true` |
| Rampa de 30° (máximo 45°) | sube hasta y = 2.12 |
| Rampa de 60° (máximo 45°) | no sube (y = 0.91), `is_sliding_down_slope = true` |
| Empujar una caja dinámica | `solve_character_collision_impulses` la mueve dz = 1.61 |
| Costo | 0.011 ms por paso (1 personaje con el mundo) |
| Tamaño | 2.8 MB nativo; wasm de 1.21 MB / 0.46 MB gz (opt-z + LTO); 18 s desde cero |

**Comparación con `CharacterBody3D.move_and_slide()` de Godot:**

| Godot | rapier `KinematicCharacterController` | Qué hay que agregar |
|---|---|---|
| `up_direction` | `up` | — |
| `floor_max_angle` | `max_slope_climb_angle` / `min_slope_slide_angle` | — |
| `floor_snap_length` | `snap_to_ground` | — |
| Escalones (en Godot casi siempre a mano) | `autostep { max_height, min_width, include_dynamic_bodies }` | — (rapier tiene más) |
| `safe_margin` | `offset` | — |
| `is_on_floor()` | `EffectiveCharacterMovement.grounded` | — |
| `is_on_wall()` / `is_on_ceiling()`, `get_wall_normal()` | lista de `CharacterCollision` con su normal | derivar los flags de las normales (5 líneas; el prototipo ya lo hace) |
| `velocity` que se conserva o se recorta | se pasa un desplazamiento | capa `mover(vel, dt)`: gravedad, recortar la velocidad contra las normales |
| Plataformas móviles (heredar su velocidad) | no | guardar el suelo actual y sumar su desplazamiento (el 2D ya lo hace con `floor` en `Inst`) |
| `floor_stop_on_slope`, `floor_constant_speed`, `floor_block_on_wall` | no | ajustes en la capa `mover` |
| `max_slides`, `wall_min_slide_angle` | internos (`slide = true`) | casi nunca hace falta |
| `motion_mode = FLOATING` | `slide` sin gravedad ni snap | — |
| Empujar `RigidBody3D` | `solve_character_collision_impulses` | — (en Godot es manual) |

Resumen: rapier trae lo difícil (barrido de la forma, deslizamiento, escalones, pendientes, snap) y faltan unas 200 líneas para
tener la semántica exacta de Godot. **avian3d** no sirve aquí porque funciona solo dentro de Bevy.

---

## 6. Modelos y animación

Prototipo: `/tmp/spike3d/anim-audio/src/main.rs`.

- **glTF 2.0 como formato del motor.** La crate `gltf` 1.4 (2024-05, estable, con 3 millones de descargas en 90 días) lee mallas,
  materiales, **pieles (skins), huesos y animaciones**. Con Fox.glb: carga en 4 ms, 24 huesos, clips «Survey», «Walk» y «Run» (21
  canales, 1.16 s).
- **Costo en CPU (medido):** muestrear y calcular las matrices de huesos cuesta **0.002 ms por cuadro**. Deformar 1 728 vértices
  en CPU cuesta 0.0076 ms, unos 4.4 ns por vértice: un personaje de 50 000 vértices costaría unos 0.22 ms. Conclusión: **calcular
  los huesos en CPU** (barato, y así podemos mezclar y hacer máquinas de estado nosotros) y **deformar en GPU** (kiss3d ya lo hace).
- **FBX y OBJ:**
  - `ufbx` 0.11.5 (MIT/Unlicense) compila y enlaza en nativo, pero **no compila para wasm32-unknown-unknown** porque es C y no hay
    toolchain C para wasm.
  - `tobj` 4.0.5 (MIT) funciona en los dos.
  - Decisión: FBX y OBJ se **importan solo en el editor** y se convierten a `.glb` dentro del proyecto. El reproductor solo lee glTF.

---

## 7. Arquitectura de integración

### 7.1 Vista general

```
                 ┌──────────────────────── biblioteca nicostation (sin ventana) ─────────────────────┐
 proyecto.json → │ model.rs (+ escenas 3D)   runtime.rs (eventos, selección, variables, Cmd)          │
                 │                              ├─ mundo 2D (lo de hoy)                             │
                 │                              └─ mundo 3D: Inst con transform 3D + rapier3d          │
                 │ catalog.rs (+ condiciones/acciones 3D)   validate.rs   expr.rs   script.rs (Rhai)    │
                 └───────────────▲───────────────────────────▲──────────────────────────▲────────────┘
                                 │ Input, dt → estado, Cmd   │                          │
      ┌──────────────────────────┴───┐   ┌───────────────────┴───────────┐   ┌──────────┴───────────┐
      │ nicostation (2D, macroquad)  │   │ nicostation3d (winit+kiss3d)  │   │ nicostation-editor     │
      │ escritorio y web (mq_js)     │   │ escritorio y web (bindgen)    │   │ eframe/egui con wgpu   │
      │ sin cambios                  │   │ render3d::sync(World) + kira  │   │ visor 3D = textura de  │
      └──────────────────────────────┘   └───────────────────────────────┘   │ kiss3d en el MISMO     │
                                                                             │ wgpu::Device           │
                                                                             └────────────────────────┘
```

- **Un solo sistema de eventos.** Las hojas de eventos, las expresiones, las variables, las funciones, Rhai, las alarmas y los
  tweens son los mismos. Las condiciones y acciones nuevas se agregan al `catalog.rs` con el mismo formato, por ejemplo:
  - «Mover en 3D», «Mirar hacia», «En el suelo», «Contra la pared».
  - «Rayo choca con», «Reproducir animación», «Sonido en posición».
- **El estado 3D vive en el runtime.** `Inst` ya tiene `x`, `y` y `z` (la `z` hoy es profundidad). Se agregan rotación (cuaternión,
  y en el editor ángulos en grados), escala en xyz y `collider`. rapier se queda dentro de `World` para que `simular` del MCP y las
  pruebas corran sin GPU, de forma determinista (comprobado en nativo y wasm).
- **Comportamientos nuevos:**
  - `character3d`: rapier KCC más la capa `mover` (§5).
  - `rigidbody3d`, `static3d`.
  - `camera_follow3d` y `orbit_camera`.
- **Comandos nuevos (`Cmd`):**
  - `PlaySound3d { file, volume, pos }`: el reproductor lo manda a una pista espacial de kira.
  - Las animaciones van como estado de la instancia (`anim` ya existe), y el reproductor o el editor las aplican.
- **El render solo refleja.** `render3d::Escena` guarda un `SceneNode3d` por instancia. En cada cuadro, `sync(&World)` crea, borra
  y mueve nodos, y cambia de clip de animación. Nada de la lógica depende de kiss3d.

### 7.2 Fachada `render3d` (para poder cambiar de motor)

Un módulo propio, con una API mínima que elegimos nosotros, para que kiss3d no se filtre al resto del código:

- `Escena::new(dispositivo_opcional)`
- `sync(&World)`
- `dibujar(&Camara) -> wgpu::TextureView` (editor) o `presentar()` (reproductor)
- `pick(rayo)`
- `set_cielo(hdr)`
- `set_post(PostConfig)`

Toda la API de kiss3d queda dentro de ese módulo. Si hay que pasar a Bevy, se reescribe solo ese módulo: el patrón de dispositivo
compartido ya está probado con los dos.

### 7.3 Editor

- **Visor 3D:** en `CreationContext`, tomar `wgpu_render_state` y llamar a
  `kiss3d::context::Context::init(rs.instance, rs.adapter, rs.device, rs.queue, ...)`. Crear un `OffscreenSurface` del tamaño del
  panel, registrar `output_view()` con `rs.renderer.write().register_native_texture(...)` y mostrarlo con `ui.image`. Al cambiar el
  tamaño del panel, `surface.resize(...)` y `update_egui_texture_from_wgpu_texture`. No hay copias a CPU.
- **Navegación y edición:** cámara orbital con el mouse sobre la imagen (ya probado en el prototipo wgpu con `drag_delta`).
  Gizmos para mover, rotar y escalar: hay una crate `transform-gizmo-egui` 0.11 (2026-08) que todavía no probamos con egui 0.37.
  Selección con rayos contra los colliders de rapier/parry.
- **«Jugar aquí» en 3D:** el mismo `World` con paso fijo de 60 Hz, como en `vista.rs`, pero dibujado con `render3d` en lugar del
  `Painter`.
- **`--captura`:** sigue funcionando sin ventana, porque kiss3d dibuja fuera de pantalla con el mismo dispositivo (probado).
- **Importar:** glTF directo; FBX y OBJ con ufbx/tobj convertidos a `.glb`; HDR para el cielo.
- **Ojo:** el dispositivo de eframe tiene que pedirse con `required_limits: adapter.limits()` y las funciones del adaptador; los
  valores por defecto de eframe se quedan cortos para kiss3d. Esto se configura una sola vez en `NativeOptions`.

### 7.4 Reproductor

- **Juegos 2D:** `nicostation` sigue con macroquad. No cambian ni el tamaño, ni el web, ni el empaquetado.
- **Juegos 3D:** un binario nuevo, `nicostation3d`, con winit (vía kiss3d), `render3d`, kira y la misma lectura de `juego.json` o
  `pack.rs`:
  - La exportación a escritorio en un solo archivo se reutiliza tal cual: el pack se pega al final del ejecutable 3D.
  - Nombres de teclas: hay que traducir de winit a los nombres actuales (`"Left"`, `"Space"`…, que hoy siguen el `Debug` de
    `KeyCode` de macroquad). Es una tabla, como `vista::tecla` en el editor.
  - HUD y textos: kiss3d tiene un overlay 2D; los objetos `text` del 2D se pueden dibujar en una capa de interfaz.
  - Partida guardada: en escritorio, el mismo archivo; en la web, `localStorage` con web-sys en vez del plugin de `mq_js_bundle`.
- **Más adelante (opcional):** pasar también el 2D a wgpu (kiss3d tiene escena 2D) para tener un solo reproductor. No hace falta
  para el 3D, y hoy no conviene arriesgar el 2D.
- **Cómo se elige:** el proyecto declara `"dimension": "3d"` (o se deduce si tiene escenas 3D), y `export.rs` elige el reproductor
  que corresponde.

---

## 8. Consecuencias para la exportación web

- **El 3D no puede usar el cargador de macroquad.** wgpu y winit en la web necesitan wasm-bindgen. La exportación 3D escribe
  `nicostation3d_bg.wasm`, el `nicostation3d.js` que genera wasm-bindgen (unos 95 KB en el prototipo) y un `index.html` con un
  `<canvas>`. three-d, por ejemplo, entra en pánico si no hay canvas. Los assets se bajan con `fetch` y la partida se guarda con
  `localStorage` vía web-sys. El `.wasm` de cada juego sigue siendo el mismo binario para todos, como hoy: se compila una vez y se
  copia.
- **Tamaño:** el 3D completo (kiss3d más rapier más kira) quedaría en unos 6–7 MB sin comprimir y **unos 2 MB comprimido** (suma
  aproximada de 1.42 MB, 0.46 MB y unos 0.2 MB). El reproductor 2D actual mide 1.0 MB comprimido.
- **WebGPU o WebGL2:**
  - kiss3d en la web usa WebGPU. En Chromium sin ventana, el wasm **carga, consigue adaptador y corre sin panic**, pero la
    presentación de WebGPU no se pudo capturar en esta máquina (error de `SharedImage` del propio Chromium sin ventana).
  - Con la feature `webgl` de wgpu activada (sube a 9.8 MB sin LTO), wgpu igual eligió WebGPU, así que el camino WebGL2 de kiss3d
    **quedó sin comprobar**. Su código dice que en WebGL2 cae a un modo con 8 luces fijas y sin compute (sin luces agrupadas, sin
    trazado de rayos, deformación por un camino alternativo).
  - Tarea concreta de la fase 4: probar en Chrome, Firefox y Safari reales, y si hace falta forzar `Backends::GL` cuando
    `navigator.gpu` no exista.
  - Bevy tiene WebGL2 bien probado por su comunidad, pero en WebGL2 también pierde lo que usa compute.
  - three-d es la única opción con WebGL2 **comprobado aquí**.
- **Para itch.io y GitHub Pages no cambia nada:** sigue siendo una carpeta estática.

---

## 9. Riesgos

| Riesgo | Prob. | Impacto | Mitigación |
|---|---|---|---|
| kiss3d cambia la API o se estanca | media | medio | fijar `=0.47.0`, fachada `render3d`, Bevy probado como plan B con el mismo patrón |
| Web 3D sin WebGPU (navegadores viejos) | media | medio | probar WebGL2 en la fase 4; si falla, mostrar «necesita un navegador con WebGPU»; como último recurso, three-d (WebGL2 probado) detrás de `render3d` |
| Versiones de wgpu desalineadas (eframe frente a kiss3d o Bevy) | media | alto | actualizar egui, eframe y kiss3d juntos; la fachada aísla; si no coinciden, el plan de emergencia es leer a CPU (0.67 ms) |
| Límites del dispositivo de eframe insuficientes para kiss3d | baja | medio | pedir `adapter.limits()` y las funciones del adaptador al crear el dispositivo (probado) |
| Tamaño del web 3D (unos 2 MB gz) | baja | bajo | opt-z y LTO; `wasm-opt` (no instalado aquí, suele bajar otro 10–20 %) |
| Compilar más lento | segura | bajo | kiss3d suma unos 64 s desde cero y 4 s al recompilar; `opt-level = 2` para dependencias ya está en `Cargo.toml` |
| Falta mezcla de animaciones | segura | medio | animador propio sobre `gltf` (muestreo medido a 0.002 ms) que escribe los nodos de kiss3d |
| FBX en la web | segura | bajo | importar solo en el editor (ufbx) y convertir a glb |
| Corrupción de estado GL si alguien mezcla macroquad con GL ajeno | — | alto | no hacerlo; el 3D tiene su propio reproductor (§4.6) |

---

## 10. Plan por fases

Estimación para una persona, en semanas de trabajo. Cada fase deja todo compilando, con pruebas y en git, como pide `ROADMAP.md`.

| Fase | Contenido | Semanas |
|---|---|---|
| **0. Bases** | Dependencias fijadas (`kiss3d =0.47.0`, `rapier3d 0.36`, `kira 0.12`); módulo `render3d` (fachada) y prueba de render fuera de pantalla; CI que compila nativo y wasm de `nicostation3d` | 1 |
| **1. Runtime 3D sin ventana** | `model.rs`: escenas 3D, objetos `model3d` (glb), `light3d`, `camera3d`, primitivas; `Inst` con transform 3D; rapier en `World`; `character3d` con la capa `mover`, más `rigidbody3d` y `static3d`; condiciones y acciones 3D en `catalog.rs`; `validate.rs`; `simular` del MCP en 3D; pruebas deterministas | 3 |
| **2. Reproductor 3D de escritorio** | `nicostation3d` con kiss3d: `sync(World)`, cámaras, cielo HDR, sombras y posprocesado configurables, animación (clip por instancia), kira espacial, teclas winit traducidas a nuestros nombres, HUD de texto, partida guardada, `--export-desktop` con `pack.rs` | 2 |
| **= MVP jugable (escritorio)** | | **6** |
| **3. Editor 3D** | Visor con dispositivo compartido; cámara orbital; selección por rayo; gizmos; panel de luces, cielo y post; importar glTF, FBX y OBJ; «Jugar aquí» 3D; `--captura` 3D; modo fácil para chicos (bloques 3D básicos, plantillas) | 4 |
| **4. Web 3D** | Exportación con wasm-bindgen (html, js, wasm), `fetch` de assets, `localStorage`, controles táctiles, prueba en Chrome, Firefox y Safari reales; decisión sobre WebGL2 | 2 |
| **= MVP completo (escritorio, editor y web)** | | **12** |
| **5. Calidad tipo Godot** | Mezcla de animaciones y máquina de estados, plataformas móviles y ajustes finos de `move_and_slide`, partículas 3D, LOD y culling, navegación (navmesh, crate por elegir), materiales avanzados en el editor, presets de post (TAA y similares solo si pasamos a Bevy) | 4–6 |
| **Total estimado** | | **16–18** |

Primer paso concreto: pasar `/tmp/spike3d/kiss3d/src/bin/egui_embed.rs` al editor (un panel «Escena 3D» con el casco), con el
dispositivo de eframe pedido con los límites del adaptador. Así se valida la integración en el editor real antes de tocar el
runtime.

---

## 11. Prototipos y capturas

Todos en `/tmp/spike3d/` (assets CC0/CC-BY de Khronos glTF-Sample-Assets y un HDR CC0 de Poly Haven en `/tmp/spike3d/assets/`).
Tiempos de compilación en `/tmp/spike3d/timings.txt` (medidos con `tbuild.sh`).

| Prototipo | Qué prueba |
|---|---|
| `/tmp/spike3d/three-d/` | three-d: escena PBR (`src/lib.rs`), PNG con EGL sin superficie (`src/main.rs --png`), dentro de egui_glow (`src/bin/egui_glow.rs`), web (`web/`) |
| `/tmp/spike3d/wgpu-egui/` | render wgpu propio mínimo dentro de egui-wgpu 0.37 (`src/render3d.rs`, `src/main.rs --png`) |
| `/tmp/spike3d/bevy/` | Bevy manejado desde afuera, sin winit (`src/main.rs`) y con el dispositivo compartido dentro de egui (`src/bin/egui_embed.rs`) |
| `/tmp/spike3d/kiss3d/` | kiss3d fuera de pantalla con PBR, IBL, sombras, SSAO, bloom y skinning (`src/main.rs`); dentro de egui con el dispositivo compartido (`src/bin/egui_embed.rs`) |
| `/tmp/spike3d/macroquad/` | 3D propio de macroquad y three-d en el contexto GL de miniquad (`--features threed`) |
| `/tmp/spike3d/rapier/` | personaje cinemático contra escalón, pared, rampas y caja; nativo y wasm (`web/`, con Node) |
| `/tmp/spike3d/anim-audio/` | huesos y animación glTF en CPU (`src/main.rs`); kira espacial con backend propio (`src/bin/audio3d.rs`) |
| `/tmp/spike3d/fbx/` | ufbx y tobj (nativo bien, ufbx en wasm falla) |
| `/tmp/spike3d/wasmz/` | wasm opt-z + LTO pasados por bindgen (kiss3d, three-d, rapier) |

Capturas (`/tmp/spike3d/shots/`):

- `kiss3d.png`: casco y zorro animado, IBL, sombras, SSAO y bloom.
- `kiss3d-in-egui.png`: kiss3d en un panel de egui con el mismo `wgpu::Device`.
- `bevy.png`: Bevy con CSM, SSAO, bloom y luz puntual.
- `bevy-in-egui.png`: Bevy en egui con el mismo dispositivo.
- `three-d.png`: three-d nativo con PBR, IBL y sombra.
- `three-d-web.png`: three-d en WebGL2 (Chromium sin ventana).
- `three-d-in-egui.png`: three-d dentro de egui_glow con PaintCallback.
- `wgpu-egui.png`: render wgpu propio mínimo en egui.
- `macroquad-base.png`: 3D de macroquad (sin luces).
- `macroquad-three-d.png`: three-d en el contexto de macroquad; texto de macroquad corrupto (§4.6).

Datos de las crates (API de crates.io, 2026-11-08):

| Crate | Versión | Fecha |
|---|---|---|
| kiss3d | 0.47.0 | 2026-10-04 |
| bevy | 0.20.1 | 2026-10-28 |
| three-d | 0.19.0 | 2026-04-17 |
| three-d-asset | 0.10.0 | — |
| wgpu | 30.0.1 | 2026-08-22 |
| macroquad | 0.4.16 | — |
| rapier3d | 0.36.1 | 2026-10-10 |
| parry3d | 0.31.1 | — |
| avian3d | 0.8.0 | (solo Bevy) |
| gltf | 1.4.1 | 2024-05-10 |
| ufbx | 0.11.5 | 2026-09-25 |
| tobj | 4.0.5 | — |
| kira | 0.12.6 | 2026-10-28 |
| rodio | 0.23.0 | — |
| rend3 | 0.3.0 | 2022, repositorio archivado |
| fyrox-impl | 1.0.1 | — |

## 12. Audio 3D

Prototipo: `/tmp/spike3d/anim-audio/src/bin/audio3d.rs`, con un `Backend` propio para leer las muestras.

kira 0.12 tiene un oyente (`add_listener`) y pistas espaciales (`add_spatial_sub_track`, con distancias mínima y máxima y curva
de atenuación). Picos medidos con un tono de 440 Hz:

| Emisor en | Izquierda | Derecha |
|---|---|---|
| x = −3 | 0.302 | 0.086 |
| x = +3 | 0.086 | 0.302 |
| z = −10 | 0.045 | 0.045 |
| z = −25 | 0.001 | 0.001 |

Es decir, paneo y atenuación correctos. Binario de 0.59 MB; compila para wasm (0.73 MB sin LTO; en la web usa cpal con WebAudio y
wasm-bindgen, que el reproductor 3D ya tendría).

Las alternativas:

- **rodio 0.23:** `SpatialSink` solo panea en estéreo, y kira tiene mejor mezcla, tweens y pistas.
- **Audio de macroquad (quad-snd):** no tiene espacial.

**kira para el reproductor 3D**, tanto para los sonidos 2D como para los 3D.
