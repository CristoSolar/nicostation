`mq_js_bundle.js` viene de macroquad 0.4.16 (`js/mq_js_bundle.js`, MIT/Apache-2.0) con un cambio y un agregado.

Cambio:
en el plugin `quad_net` se agregó `var` delante de `register_plugin=function` (y `;` en vez de `,` antes de `miniquad_add_plugin`), porque el original asigna
una variable sin declarar y en modo estricto (`"use strict"`) el navegador lanza `ReferenceError`.

Agregado (al final del archivo, después de `;`): el plugin `nicostation_storage` (partida guardada en `localStorage`),
con tres funciones que el reproductor importa desde `env`:
- `nicostation_storage_get(clave_ptr, clave_len) -> i32`: lee la clave y devuelve el largo en bytes UTF-8 del valor (0 si no hay nada);
- `nicostation_storage_take(ptr)`: copia ese valor a la memoria del wasm (`wasm_memory`) en `ptr`;
- `nicostation_storage_set(clave_ptr, clave_len, valor_ptr, valor_len)`: guarda el valor.

Si se actualiza macroquad, hay que volver a aplicar el cambio y copiar este bloque al final del bundle nuevo.
