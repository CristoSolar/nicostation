# Hoja de ruta: de motor para niños a motor profesional

Meta: que alguien profesional pueda entregar con NicoStation un juego del nivel de calidad que se entrega con
GameMaker, sin perder el modo fácil para niños. Cada fase deja todo compilando, con pruebas y en git.

## Fase A: trabajar y depurar como profesional
- [x] Vista previa dentro del editor: jugar, pausar, avanzar cuadro a cuadro, velocidad.
- [x] Depurador: inspeccionar instancias (propiedades y variables en vivo), cajitas, avisos del motor.
- [x] Recarga en caliente: los cambios del proyecto se ven sin cerrar la vista previa.
- [x] Expresiones completas: comparaciones y lógica (`==`, `<`, `and`, `or`, `not`), `if(c, a, b)`,
      matemática de juegos (`atan2`, `lengthdir_x/y`, `sign`), texto (`substr`, `contains`, `replace`, `fixed`).
- [x] Eventos «Si no» (else) y «Esperar N segundos» dentro de un evento.
- [x] Alarmas por instancia (como las alarms de GameMaker).
- [x] Guardar y cargar partida (archivo en escritorio, localStorage en web).
- [x] Colisiones con grilla espacial (cientos de objetos sin bajar los FPS).
- [x] Exportar a ejecutable de un solo archivo (Linux; Windows con el toolchain mingw instalado).

## Fase B: calidad visual y de sonido
- [x] Escala (scale_x/scale_y), color (tint) y visible por instancia.
- [x] Sprites: punto de origen y máscara de colisión propia (profundidad automática por Y: con «set z = y»).
- [x] Cámara: seguir suave, límites, temblor, zoom animado.
- [x] Tweens (movimientos suaves con curvas).
- [x] Partículas.
- [x] Fondos que se repiten y paralaje; fuentes propias (TTF) con alineación y salto de línea.
- [x] Audio: canales, volumen por sonido, música con fundido.
- [x] Efectos (shaders): destello, silueta, escala de grises, ondas.

## Fase C: programar en serio
- [x] Scripts con Rhai (lenguaje embebido en Rust): acción «Ejecutar script», objetos con código,
      acceso a instancias, variables y funciones del motor. Editor de código con resaltado.
- [x] Variables listas y estructuras (inventarios, tablas).
- [x] Búsqueda y reemplazo en eventos.
- [ ] Eventos externos reutilizables entre escenas.
- [x] Física 2D (rapier2d) y búsqueda de caminos (A* en grilla).
- [x] Mandos (gamepad) en escritorio y web.

## Fase D: editor de estudio
- [x] Selección múltiple, copiar/pegar, alinear y distribuir, rotar y escalar con manijas.
- [x] Explorador de recursos con importación (arrastrar archivos), carpetas y renombrado seguro.
- [x] Ajustes del proyecto: escalado pixel perfect, FPS, ícono, pantalla de carga.
- [x] Versionado del formato del proyecto con migraciones.
- [x] Perfilador (tiempos por evento y por comportamiento).

## Fase E: publicar
- [x] Exportar a Android (APK firmada de depuración; falta probar en un teléfono).
- [x] Exportar a Windows: un solo .exe sin consola ni DLL externas (falta probarlo en un Windows real).
- [ ] Exportar a macOS; publicación a itch.io.
- [ ] Manual de usuario y referencia de cada bloque.

## Fase F: 3D (ver docs/godot-3d.md y docs/tecnologia-3d.md)
Tecnología elegida tras probarla: kiss3d 0.47 (render wgpu, el mismo que usa el editor), rapier3d 0.36 (física),
glTF (modelos; FBX/OBJ se convierten al importar), kira 0.12 (audio 3D). Plan B: Bevy detrás del mismo módulo `render3d`.
Los juegos 3D usan un reproductor propio (`nicostation3d`); los 2D siguen en el actual. Las hojas de eventos y los
nodos programan ambos.
- [x] F1 Modelo 3D en el proyecto: escenas 3D, nodos con transformación jerárquica, mallas, luces, cámara, entorno.
- [x] F2 Reproductor `nicostation3d`: ventana, entrada y audio fuera de macroquad; mismo `World` y eventos.
- [ ] F3 Render «aspecto Godot»: PBR, sombras en cascadas, cielo con luz ambiente, HDR, glow, tonemap, SSAO.
- [ ] F4 Física: cuerpos estáticos/rígidos/cinemáticos, áreas, raycast; comportamiento `character3d` (move_and_slide).
- [ ] F5 Importar glTF con materiales, esqueletos y animaciones; reproducción y mezcla de animaciones.
- [ ] F6 Bloques 3D: mover hacia adelante, rotar hacia, raycast golpea, animación terminó, en el suelo, etc.
- [ ] F7 Editor 3D: visor con manijas mover/rotar/escalar, cámara libre (clic derecho + WASD), árbol de escena.
- [ ] F8 Exportar 3D (escritorio de un archivo y web; probar WebGPU/WebGL2 en navegadores reales).
- [ ] F9 Paridad: partículas, GridMap, CSG, máquina de estados de animación, navegación, lightmaps, efectos.
