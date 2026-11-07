`mq_js_bundle.js` viene de macroquad 0.4.16 (`js/mq_js_bundle.js`, MIT/Apache-2.0) con un único cambio:
en el plugin `quad_net` se agregó `var` delante de `register_plugin=function` (y `;` en vez de `,` antes de `miniquad_add_plugin`), porque el original asigna
una variable sin declarar y en modo estricto (`"use strict"`) el navegador lanza `ReferenceError`.
