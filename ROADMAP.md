# Hoja de ruta: de motor para niños a motor profesional

Meta: que alguien profesional pueda entregar con NicoStation un juego del nivel de calidad que se entrega con
GameMaker, sin perder el modo fácil para niños. Cada fase deja todo compilando, con pruebas y en git.

## Fase A: trabajar y depurar como profesional
- [ ] Vista previa dentro del editor: jugar, pausar, avanzar cuadro a cuadro, velocidad.
- [ ] Depurador: inspeccionar instancias (propiedades y variables en vivo), cajitas, avisos del motor.
- [ ] Recarga en caliente: los cambios del proyecto se ven sin cerrar la vista previa.
- [ ] Expresiones completas: comparaciones y lógica (`==`, `<`, `and`, `or`, `not`), `if(c, a, b)`,
      matemática de juegos (`atan2`, `lengthdir_x/y`, `sign`), texto (`substr`, `contains`, `replace`, `fixed`).
- [ ] Eventos «Si no» (else) y «Esperar N segundos» dentro de un evento.
- [ ] Alarmas por instancia (como las alarms de GameMaker).
- [ ] Guardar y cargar partida (archivo en escritorio, localStorage en web).
- [ ] Colisiones con grilla espacial (cientos de objetos sin bajar los FPS).
- [ ] Exportar a ejecutable de un solo archivo (Linux; Windows con el toolchain mingw instalado).

## Fase B: calidad visual y de sonido
- [ ] Sprites: punto de origen, máscara de colisión propia, escala, color (tint), visible, profundidad automática por Y.
- [ ] Cámara: seguir suave, límites, temblor, zoom animado.
- [ ] Tweens (movimientos suaves con curvas) y partículas.
- [ ] Fondos que se repiten y paralaje; fuentes propias (TTF) con alineación y salto de línea.
- [ ] Audio: canales, detener un sonido, volumen por sonido, música con fundido.
- [ ] Efectos (shaders): destello, silueta, escala de grises, ondas.

## Fase C: programar en serio
- [ ] Scripts con Rhai (lenguaje embebido en Rust): acción «Ejecutar script», objetos con código,
      acceso a instancias, variables y funciones del motor. Editor de código con resaltado.
- [ ] Variables listas y estructuras (inventarios, tablas).
- [ ] Eventos externos reutilizables entre escenas; búsqueda y reemplazo en eventos.
- [ ] Física 2D (rapier2d) y búsqueda de caminos (A* en grilla).
- [ ] Mandos (gamepad) en escritorio y web.

## Fase D: editor de estudio
- [ ] Selección múltiple, copiar/pegar, alinear y distribuir, rotar y escalar con manijas.
- [ ] Explorador de recursos con importación (arrastrar archivos), carpetas y renombrado seguro.
- [ ] Ajustes del proyecto: escalado pixel perfect, FPS, ícono, pantalla de carga.
- [ ] Versionado del formato del proyecto con migraciones.
- [ ] Perfilador (tiempos por evento y por comportamiento).

## Fase E: publicar
- [ ] Exportar a Windows, macOS y Android; publicación a itch.io.
- [ ] Manual de usuario y referencia de cada bloque.
