`mq_js_bundle.js` viene de macroquad 0.4.16 (`js/mq_js_bundle.js`, MIT/Apache-2.0) con un cambio y dos agregados.

Cambio:
en el plugin `quad_net` se agregó `var` delante de `register_plugin=function` (y `;` en vez de `,` antes de `miniquad_add_plugin`), porque el original asigna
una variable sin declarar y en modo estricto (`"use strict"`) el navegador lanza `ReferenceError`.

Agregado 1 (al final del archivo, después de `;`): el plugin `nicostation_storage` (partida guardada en `localStorage`),
con tres funciones que el reproductor importa desde `env`:
- `nicostation_storage_get(clave_ptr, clave_len) -> i32`: lee la clave y devuelve el largo en bytes UTF-8 del valor (0 si no hay nada);
- `nicostation_storage_take(ptr)`: copia ese valor a la memoria del wasm (`wasm_memory`) en `ptr`;
- `nicostation_storage_set(clave_ptr, clave_len, valor_ptr, valor_len)`: guarda el valor.

Agregado 2 (después del anterior): el plugin `nicostation_gamepad` (mandos con `navigator.getGamepads()`), con una función:
- `nicostation_pad_read(mando, ptr) -> i32`: entre los mandos conectados, toma el número `mando` (0, 1…) y escribe en la memoria
  del wasm, en `ptr`, 21 `f32`: los 17 botones de la distribución «standard» (A, B, X, Y, L, R, LT, RT, Select, Start, LS, RS,
  arriba, abajo, izquierda, derecha, Home; de 0 a 1) y los 4 ejes (palanca izquierda x/y, derecha x/y). Devuelve 1 si existe ese mando.

Sonido: el reproductor no usa la feature `audio` de macroquad (su ALSA choca con el de kira en el workspace), pero en web llama
directamente a las funciones del plugin `macroquad_audio` que ya trae el bundle (`audio_init`, `audio_add_buffer`,
`audio_source_is_loaded`, `audio_play_buffer`, `audio_playback_set_volume`, `audio_playback_stop`; ver `src/reproductor/audio.rs`).
Si se actualiza macroquad, revisar que sigan con la misma firma.

Si se actualiza macroquad, hay que volver a aplicar el cambio y copiar estos dos bloques al final del bundle nuevo.
