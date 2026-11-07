# GDevelop: runtime (GDJS) y editor (newIDE). Especificación para un port a Rust + macroquad

Fuente: `/home/admin/gdevelop-src` (MIT, 4ian/GDevelop). Abreviaturas de rutas:
`R/` = `GDJS/Runtime/`, `E/` = `Extensions/`, `IDE/` = `newIDE/app/src/`, `GDJS/` = `GDJS/GDJS/`, `Core/` = `Core/GDCore/`.
Lo marcado como **[especulación]** o **[recomendación]** no sale del código, es interpretación mía para el port.

---

## 1. Bucle de juego (GDJS)

### 1.1 Ciclo de frame (nivel juego)
`R/runtimegame.ts:1272 startGameLoop()` → `R/pixi-renderers/runtimegame-pixi-renderer.ts:1025` (`requestAnimationFrame`). Callback (`runtimegame.ts:1344-1420`):
1. `accumulatedElapsedTime += dt`. **Limitador de FPS**: si `maxFPS > 0` y `1000/acumulado > maxFPS + 7`, se salta el frame sin hacer nada (margen de 7 FPS para no tirar 1 de cada 2 frames a 60 Hz) (`:1367-1378`).
2. Si cambió la resolución, se avisa a la pila de escenas (`:1383`).
3. Si está en pausa (depurador) solo se renderiza: `sceneStack.renderWithoutStep()` (`:1408`). Si no, `sceneStack.step(elapsed)`; si devuelve false se detiene el juego (`:1413`).
4. `inputManager.onFrameEnded()` (`:1419`) limpia "just pressed", "released", touches terminados, wheel y movimiento del ratón (`R/inputmanager.ts:657-679`).

Valores por defecto del proyecto: `maxFPS=60`, `minFPS=20`, ventana 800x600 (`Core/Project/Project.cpp:54`).

### 1.2 Orden dentro de un frame de escena
`R/runtimescene.ts:405 renderAndStep(elapsedTime)`:
1. `_requestedChange = CONTINUE`.
2. `timeManager.update(elapsed, minFPS)`: `dt = min(elapsed, 1000/minFPS) * timeScale`; actualiza los temporizadores de escena y `timeFromStart` (`R/timemanager.ts:45-61`). **El paso es variable**, con un tope (a 20 FPS mínimo, dt máx = 50 ms). No hay paso fijo en el núcleo.
3. `asyncTasksManager.processTasks()`: acciones asíncronas ("Wait X seconds") (`runtimescene.ts:420`, `R/AsyncTasksManager.ts`).
4. `_updateObjectsPreEvents()` (`R/RuntimeInstanceContainer.ts:613-636`), por cada instancia:
   a. Si tiene fuerzas: `x += avgForce.x*dt_s; y += avgForce.y*dt_s`, luego `obj.update()` y `updateForces()`; si no, solo `obj.update()` (avance de animación de sprite, etc.).
   b. `obj.updateTimers(dt)` (temporizadores del objeto).
   c. `obj.stepBehaviorsPreEvents()` → `doStepPreEvents` de cada comportamiento **activado**, en el orden del array `behaviors` del objeto (`R/runtimeobject.ts:2227`, `R/runtimebehavior.ts:158`).
   d. Al final: `_cacheOrClearRemovedInstances()`.
5. Callbacks globales pre-eventos (`gdjs.callbacksRuntimeScenePreEvents`, aquí se engancha p. ej. el gestor de tweens de escena: `E/TweenBehavior/tweentools.ts:160`).
6. **Eventos**: `this._eventsFunction(this)`, código JS generado a partir de la hoja de eventos (`GDJS/Events/CodeGeneration/LayoutCodeGenerator.cpp`, `EventsCodeGenerator.cpp`).
7. `_stepBehaviorsPostEvents()`: limpia eliminados → `doStepPostEvents` de todos → limpia otra vez (`RuntimeInstanceContainer.ts:649-661`).
8. Callbacks post-eventos.
9. `render()`: `_updateObjectsPreRender()` (culling y `updatePreRender`), efectos de capas y luego el dibujo (`runtimescene.ts:493-531`).
10. Devuelve `true` si hay un cambio de escena pendiente; la pila lo aplica **después** de renderizar (`R/scenestack.ts:144-165`).

Culling (`runtimescene.ts:541-603`): a partir del 2º frame, un objeto es visible si su `getVisibilityAABB()` intersecta el rectángulo de la cámara de su capa **ampliado x2**. Los sprites no se animan si están ocultos/culled, salvo `updateIfNotVisible` (`R/spriteruntimeobject.ts:219-233`).

### 1.3 Escala de tiempo
- Escena: `timeManager.setTimeScale(s>=0)` multiplica el dt global (`R/timemanager.ts:53,69`).
- Capa: `RuntimeLayer._timeScale`; `layer.getElapsedTime() = scene.getElapsedTime() * layerTimeScale` (`R/RuntimeLayer.ts:847-868`).
- Objeto: `obj.getElapsedTime()` = dt de su capa (`R/runtimeobject.ts:560`). Comportamientos y animaciones usan siempre `owner.getElapsedTime()`, por lo que la escala por capa permite cámaras lentas parciales.

### 1.4 Pila de escenas
`R/scenestack.ts`. Peticiones desde eventos vía `scene.requestChange(SceneChangeRequest, name)` (`runtimescene.ts:820`): `CONTINUE, PUSH_SCENE, POP_SCENE, REPLACE_SCENE, CLEAR_SCENES, STOP_GAME`.
- `push(name)`: llama `onPause()` en la escena actual, crea `RuntimeScene`, `loadFromScene`, opcionalmente instancia un *external layout* y la apila (`scenestack.ts:213-325`). Si los assets no están listos, marca `_isNextLayoutLoading` y carga de forma asíncrona (mientras tanto `step` devuelve false).
- `pop(n)`: descarga la de arriba (nunca deja la pila vacía) y llama `onResume()` en la nueva cima (`:183-211`).
- `replace(name, clear)`: descarga la actual (o todas si `clear`) y carga la nueva.
- Solo la escena de la cima se actualiza y se renderiza (`:142-144`).

`loadFromScene` (`runtimescene.ts:149-255`), en este orden: título de ventana, color de fondo, capas, variables de escena, datos compartidos de comportamientos, registro de objetos (primero los globales y luego los de escena), creación de las instancias iniciales, `_setLayerDefaultZOrders()`, función de eventos, `OnceTriggers`, callbacks de "scene loaded", `stopSoundsOnStartup` y `timeManager.reset()`.

### 1.5 Creación y destrucción de instancias
- `createObject(name)` (`RuntimeInstanceContainer.ts:698-735`): reutiliza una instancia de la caché por nombre (`reinitialize`) o hace `new ctor(container, objectData, instanceData)`. Se añade de inmediato a `_instances` y será visible para los eventos siguientes.
- Desde eventos, `doCreateObjectOnScene` (`R/events-tools/objecttools.ts:1040-1066`) pone posición, capa y `zOrder = layer.getDefaultZOrder()`, y **añade el objeto a la lista de objetos "picked"** del evento, de modo que las acciones siguientes solo le afectan a él.
- Instancias iniciales (`RuntimeInstanceContainer.ts:386-414`): posición, ángulo, zOrder, capa, tamaño custom, variables iniciales y `extraInitializationFromInitialInstance`.
- `deleteFromScene()` → `markObjectForDeletion` (`:741-765`): se quita **en el acto** de las listas vivas, se llama `onDeletedFromScene()` y pasa a `_instancesRemoved`. Ahí se libera de verdad (`onDestroyed`, o vuelve a la caché, máx. 128 por objeto) en `_cacheOrClearRemovedInstances`, que se ejecuta tras los pre-eventos y antes y después de los post-eventos (`:548-562`). Motivo: los eventos pueden seguir teniendo referencias durante el frame.
- **[recomendación]** En Rust: `Vec<Entity>` por objeto, con borrado diferido mediante cola `pending_free` y *generational ids*.

### 1.6 Z-order, capas y cámaras
- Cada capa es un contenedor que ordena por `zIndex = zOrder` (`R/pixi-renderers/layer-pixi-renderer.ts:1505-1519`). Las capas se apilan según su índice en `sceneData.layers` (`runtimescene-pixi-renderer.ts:502-517`). Capa base = nombre `""`.
- Z-order por defecto de los objetos creados por eventos = (zOrder máximo de las instancias iniciales de esa capa) + 1 (`RuntimeInstanceContainer.ts:448-473`).
- `LayerData` (`R/types/project-data.d.ts:502-522`): `name, visibility, cameras[], effects[], ambientLightColor*, isLightingLayer, followBaseLayerCamera, renderingType, cameraType...`. `CameraData`: `defaultSize, defaultViewport, width, height, viewportLeft/Top/Right/Bottom` (`:524-533`).
- Cámara 2D (`R/layer.ts`): `_cameraX/_cameraY` = **centro** de la vista, inicialmente el centro de la resolución; `_zoomFactor`, `_cameraRotation` (grados). Pantalla a mundo (`layer.ts:327-357`): restar la mitad de la resolución, dividir por el zoom, rotar por el ángulo de la cámara y sumar la posición de la cámara. Ancho visible = `gameWidth / zoom` (`:149`).

---

## 2. Tipos de objeto y datos

Base `ObjectData = { name, type, variables[], behaviors[], effects[] }` (`R/types/project-data.d.ts:36-53`). `InstanceData = { name, layer, x, y, z?, angle, zOrder, opacity?, flippedX/Y?, customSize, width, height, locked?, hidden?, numberProperties[], stringProperties[], initialVariables[], persistentUuid }` (`:459-489`). Escena `LayoutData = { name, r,v,b (fondo), title, layers[], objects[], objectsGroups[], instances[], variables[], behaviorsSharedData[], stopSoundsOnStartup, uiSettings{grid, gridType, gridWidth/Height, gridOffsetX/Y, gridColor, gridAlpha, snap} }` (`:306-342`). Proyecto: `{ firstLayout, properties{windowWidth, windowHeight, maxFPS, minFPS, scaleMode 'linear'|'nearest', pixelsRounding, sizeOnStartupMode...}, resources{resources[]}, objects[] (globales), layouts[], externalLayouts[], eventsFunctionsExtensions[] }` (`:10-23`, `:557-587`).

### 2.1 Sprite (`type: "Sprite"`)
Datos (`R/spriteruntimeobject.ts:11-18`, `R/SpriteAnimator.ts:6-84`):
```
SpriteObjectData { updateIfNotVisible, preScale?, animations: [
  { name, useMultipleDirections, directions: [
    { timeBetweenFrames /*s*/, looping, sprites: [
      { image /*nombre recurso*/, originPoint{name,x,y}, centerPoint{name,automatic,x,y},
        points:[{name,x,y}], hasCustomCollisionMask, customCollisionMask: [[{x,y}...]...] } ] } ] } ] }
```
- Avance (`SpriteAnimator.ts:471-501`): si no está en pausa y (es loop o no ha llegado al final) y `timeBetweenFrames > 0`: `t += dt*speedScale`; con loop `t = mod(t, dur)`, sin loop `t = clamp(t, 0, dur)`; `dur = frames*timeBetweenFrames`; `frame = floor(t/timeBetweenFrames)` (`:653`). Cuando cambia el frame se invalidan las hitboxes (`spriteruntimeobject.ts:229-233`).
- Direcciones: con `useMultipleDirections`, 8 direcciones y `dir = round(angle/45) % 8`; el ángulo visual es `dir*45` (`SpriteAnimator.ts:688-757`). Sin ellas, el ángulo rota el sprite.
- Origen: `drawableX = x - origin.x*|scaleX*preScale|` (con flip se refleja) (`spriteruntimeobject.ts:595-610`). El centro es el punto de rotación; `automatic` = centro de la imagen.
- Hitboxes: sin máscara custom, rectángulo del tamaño del objeto rotado alrededor del centro (`R/runtimeobject.ts:2052-2083`). Con máscara custom, cada polígono (deben ser **convexos**) se transforma con origen, escala, flip y rotación (`spriteruntimeobject.ts:275-311`). Se recalculan de forma perezosa (`hitBoxesDirty`) en `getHitBoxes()/getAABB()` (`runtimeobject.ts:1992, 2108`).

### 2.2 Otros objetos
- **Text** (`TextObject::Text`), `content{ text, font, characterSize, bold, italic, underlined, color "r;g;b", textAlignment, verticalTextAlignment, lineHeight, isOutlineEnabled, outlineThickness, outlineColor, isShadowEnabled, shadowColor, shadowOpacity, shadowDistance, shadowAngle, shadowBlurRadius }`; en runtime, `_wrapping`, `_wrappingWidth` (`E/TextObject/textruntimeobject.ts:10-43,106`).
- **TiledSprite** `{ width, height, texture }`; en runtime `_xOffset/_yOffset` desplazan la textura repetida (scroll parallax) (`E/TiledSpriteObject/tiledspriteruntimeobject.ts:10-54`).
- **PanelSprite (9-patch)** `{ leftMargin, rightMargin, topMargin, bottomMargin, tiled, width, height, texture }` (`E/PanelSpriteObject/panelspriteruntimeobject.ts:9-26`). Renderizado: centro y 4 bordes como `Sprite` estirado o `TilingSprite` si `tiled`, y esquinas fijas (`panelspriteruntimeobject-pixi-renderer.ts:35-69`).
- **ShapePainter** `{ fillColor, outlineColor, fillOpacity, outlineOpacity, outlineSize, absoluteCoordinates, clearBetweenFrames, antialiasing }` (`E/PrimitiveDrawing/shapepainterruntimeobject.ts:28-45`). Primitivas inmediatas: rectangle, circle, line, ellipse, roundedRectangle, star, arc, bezier y paths (moveTo...). Con `clearBetweenFrames` el lienzo se borra en cada frame.
- **ParticleEmitter** `{ emitterAngleB (cono, grados), emitterForceMin/Max, zoneRadius, particleLifeTimeMin/Max, particleGravityX/Y, particleColor1/2, particleSize1/2, particleSizeRandomness1/2, particleAlpha1/2, particleAngle1/2 (vel. rotación °/s), rendererType 'Point'|'Line'|imagen, rendererParam1/2, textureParticleName, maxParticleNb, additive, tank (-1 = infinito), flow (part/s), destroyWhenNoParticles, jumpForwardInTimeOnCreation }` (`E/ParticleSystem/particleemitterobject.ts:11-55`). Usa `PIXI.particles.Emitter` (`particleemitterobject-pixi-renderer.ts:212`). 'Point' genera un disco y 'Line' un rectángulo (`:42-60`).

### 2.3 Colisiones
- `gdjs.Polygon.collisionTest(p1, p2, ignoreTouchingEdges)`: **SAT** sobre las normales de todas las aristas de ambos polígonos; devuelve `{collision, move_axis}` (MTV orientado según los centros) (`R/polygon.ts:215-300`). Requiere polígonos convexos (`:201`). También `raycastTest` (`:319`).
- `RuntimeObject.collisionTest(o1, o2)` (`R/runtimeobject.ts:2835-2900`): fase ancha con **círculos envolventes** (radio desde el centro), luego `getHitBoxesAround` y SAT entre cada par de hitboxes.
- Condición de colisión en eventos: `twoListsTest` sobre los productos de las listas picked (`R/events-tools/objecttools.ts:60`). **No hay índice espacial global**.
- **R-tree (rbush)** solo en comportamientos concretos: `PlatformObjectsManager` mantiene un `rbush` de plataformas (`E/PlatformBehavior/platformruntimebehavior.ts:17-20,64-100`). Lib en `R/libs/rbush.js`; wrapper `BehaviorRBushAABB` en `R/runtimebehavior.ts:8-15`.
- `separateFromObjects` usa el `move_axis` del SAT para expulsar al objeto (`runtimeobject.ts:2475`, helpers `:160-190`).
- Fuerzas: multiplicador 0 = instantánea (dura 1 frame), 1 = permanente; valores intermedios están obsoletos (decaimiento) (`runtimeobject.ts:1895-1915`).

---

## 3. Comportamientos

`BehaviorData = { name, type, ...props }` (`R/types/project-data.d.ts:212`). Ciclo de vida: `onCreated`, `doStepPreEvents`, `doStepPostEvents`, `onActivate/onDeActivate`, `onDestroy` (`R/runtimebehavior.ts`).

### 3.1 PlatformerObject (`PlatformBehavior::PlatformerObjectBehavior`)
Valores por defecto (`E/PlatformBehavior/PlatformerObjectBehavior.cpp:24-40`): `gravity 1000, maxFallingSpeed 700, ladderClimbingSpeed 150, acceleration 1500, deceleration 1500, maxSpeed 250, jumpSpeed 600, jumpSustainTime 0.2, slopeMaxAngle 60, canGrabPlatforms false, canGrabWithoutMoving true, yGrabOffset 0, xGrabTolerance 10, useLegacyTrajectory false, useRepeatedJump false, canGoDownFromJumpthru true, ignoreDefaultControls false`. Unidades en px/s y px/s².
Plataforma (`PlatformBehavior::PlatformBehavior`): `platformType 'NormalPlatform'|'Jumpthru'|'Ladder', canBeGrabbed, yGrabOffset` (`platformruntimebehavior.ts:136-145`). Cada vez que la plataforma se mueve, rota o cambia de tamaño se reinserta en el rbush (`:162-190`).

Máquina de estados: `Falling` (inicial), `OnFloor`, `Jumping`, `GrabbingPlatform`, `OnLadder` (`platformerobjectruntimebehavior.ts:170-229`). Algoritmo de `doStepPreEvents` (`:414-578`), con `dt = owner.getElapsedTime()/1000`:
0.1. **Entrada**: flags OR con la simulación desde eventos. Teclas por defecto: ←37, →39, ↑38 (escalera/up), ↓40 (down y soltar el borde), salto = LShift 1016, RShift 2016 o Space 32 (`:415-468`). `_jumpKeyHeldSinceJumpStart` pasa a false si se suelta la tecla.
   `requestedDeltaX = _updateSpeed(dt)` (`:582-627`):
   - Si solo se pulsa una dirección: acelera `acceleration*dt`; si va en sentido contrario usa `max(acceleration, deceleration)` (el giro es al menos tan rápido como la frenada).
   - Sin tecla (o ambas): frena `deceleration*dt` hacia 0 sin pasarse.
   - Clamp a ±maxSpeed. `ΔX = (v + v_prev)/2 * dt` (Verlet).
0.2. `state.beforeUpdatingObstacles`: en OnFloor, si cambió la altura del objeto se corrige Y para mantener los pies; sigue el ΔY de la plataforma del suelo si `|ΔY| <= maxFallingSpeed*dt` (`:1964-2003`).
0.3. Candidatos: `rbush.search(AABB del objeto ± max(ΔX, maxFallingSpeed*dt))` y se filtran los que ya no están cerca (`platformruntimebehavior.ts:64-100`). `_overlappedJumpThru` = jumpthrus que ya se solapan (`:1276-1292`).
1. **Eje X**: `state.checkTransitionBeforeX()` (OnFloor: si el suelo ya no está cerca → Falling; ↓ sobre jumpthru con `canGoDownFromJumpthru` → lo marca como solapado y pasa a Falling; comprueba salto (`:2005-2035`)). `beforeMovingX` (OnFloor: `ΔX += Δx` de la plataforma (`:2037`); Grabbing: ΔX/ΔY = movimiento de la plataforma agarrada). Desatascar: `separateFromObjects(no-ladder, no-jumpthru)`. `_moveX()` (`:632-677`): `x += ΔX`; mientras colisione con un sólido (los jumpthrus nunca bloquean en X; se ignora el polígono del suelo actual): primero redondear x y después retroceder 1 px por iteración (`round(x) ∓ 1`); si se pasaría de `oldX`, se queda en `oldX`.
2. **Eje Y**: `checkTransitionBeforeY` (Falling/Jumping: escalera, salto y grab; OnLadder: salir si ya no solapa, salto, soltar). `beforeMovingY`:
   - **Falling**: `_fall`: `vFall = min(vFall + gravity*dt, maxFallingSpeed)`; `ΔY += (vFall + vPrev)/2*dt` (o `vFall*dt` en legacy) (`:877-890`).
   - **Jumping** (`:2321-2399`): al entrar, `vJump = jumpSpeed`, `vFall = 0`, `canJump = false`. Cada frame: si (tecla mantenida desde el inicio y `t < jumpSustainTime`) no se resta gravedad; si no, `vJump -= gravity*dt`. `ΔY += (-vJumpPrev - vJump)/2*dt` y además `_fall(dt)`, de modo que la gravedad cuenta doble tras el sustain y la trayectoria efectiva es `vJump - vFall`. Cuando `vJump < 0` → Falling.
   - **OnFloor** (`:2051-2215`): si no chocó en X, sigue la pendiente con `_findHighestFloorAndMoveOnTop(±|ΔX|*tan(slopeMaxAngle))`; si es demasiado empinada vuelve a `oldX`. Si chocó en X, intenta subir el escalón en 2 pasos (1 px a la junta y luego ≥1 px de pendiente). Si no puede, se detiene y `currentSpeed = 0`. `slopeClimbingFactor = tan(angle)`, con 45° → 1 y mínimo 1/1024 (`:1619-1641`).
   - **OnLadder**: `ΔY ∓= ladderClimbingSpeed*dt` según ↑/↓ (`:2543-2553`).
   - **Grabbing**: memoriza la posición de la plataforma.
   `_moveY()` (`:679-737`): si ΔY>0 (bajando) usa `_findHighestFloorAndMoveOnTop(0, ΔY)` para aterrizar exactamente encima; si no hay suelo, `y += ΔY`. Si ΔY<0 (subiendo), `y += ΔY` y, mientras choque con un sólido (los jumpthrus no cuentan al subir), Jumping→Falling y retrocede 1 px (`floor(y) + 1`).
3. `_checkTransitionOnFloorOrFalling()` (`:841-875`): busca suelo en [-1, +1] px. En OnFloor: sin suelo → Falling; otro suelo → `setOnFloor`. En el aire solo aterriza si `ΔY >= 0`.
   Si chocó con una pared sin cambiar de estado y no está en OnFloor → `currentSpeed = 0` (`:527-543`).
4. Reset de teclas (si `_clearInputsBetweenFrames`) y cálculo de `hasReallyMoved` (epsilon 2^-20).

Búsqueda de suelo `_findHighestFloorAndMoveOnTop(up, down)` (`:1028-1112`) + `FollowConstraintContext` (`:2569-2770`): con el AABB de las hitboxes del personaje, para cada plataforma (se saltan escaleras; los jumpthrus solo cuentan si no estaban ya solapados o si son el suelo actual) se recorren los vértices dentro de [minX, maxX] y las intersecciones de las aristas con x = minX y x = maxX. Cada Y se clasifica así: bajo los pies (restringe `allowedMaxDeltaY = min(y - bottom)`), sobre la cabeza (restringe `allowedMinDeltaY`) o entre ambos ("floorIsTooHigh" = obstáculo). Si hay hueco, mueve `y += allowedMaxDeltaY` (la Y más alta).
Grab de bordes (`:792-839, 907-919`): desplaza X `±xGrabTolerance` y es agarrable si `platform.y + platform.yGrabOffset` está entre la Y del objeto (más `yGrabOffset`) del frame anterior y la actual, y la posición de agarre está libre.

### 3.2 TopDownMovement (`TopDownMovementBehavior::TopDownMovementBehavior`)
Valores por defecto (`E/TopDownMovementBehavior/TopDownMovementBehavior.cpp:26-37`): `allowDiagonals true, acceleration 400, deceleration 800, maxSpeed 200, angularMaxSpeed 180, rotateObject true, angleOffset 0, viewpoint 'TopDown'|'PixelIsometry'|'TrueIsometry'|'CustomIsometry', customIsometryAngle 30, movementAngleOffset 0, useLegacyTurnBack false`.
`doStepPreEvents` (`topdownmovementruntimebehavior.ts:348-589`):
1. Teclas de flecha (37-40) OR simulación. Se acumula cuánto tiempo lleva pulsada cada tecla.
2. Dirección 0..7 (0 = derecha, 2 = abajo, 4 = izquierda, 6 = arriba; sentido horario en coordenadas de pantalla). Sin diagonales, gana la tecla pulsada **más recientemente** (`:396-423`). Con diagonales: 1, 3, 5, 7.
3. Ángulo = `(dir + movementAngleOffset/45)*π/4`, `targetSpeed = maxSpeed`. Con stick: `stickAngle` y `targetSpeed = maxSpeed*stickForce`. Sin entrada pero con velocidad: la dirección actual y `target = 0`.
4. `speed = |v|`; si `dot(v, dirUnit) < 0` (dando la vuelta) `speed = dot`. `getAcceleratedSpeed` (`:591-650`): acelera hasta `target` con `acceleration`; el giro usa `max(acc, dec)`; con target 0 frena con `deceleration` hasta 0. `v = speed*dirUnit` y se limita |v| ≤ maxSpeed.
5. Posición: `Δ = (vPrev + v)/2*dt`; en isometría se transforma con la base (`:551-565`, ángulos `atan(0.5)`, `π/6` o el custom: `:227-245`).
6. Si se mueve y `rotateObject`: `rotateTowardAngle(dirDeg + angleOffset, angularMaxSpeed)`.
**No gestiona colisiones** (0 referencias a collision/separate en el archivo). Las paredes se resuelven en los eventos con "Separate objects".

### 3.3 Resto
- **Physics2** (Box2D 2.3.1 compilado a WASM, `E/Physics2Behavior/Box2D_v2.3.1_min.wasm.*`): paso fijo `1/60` con acumulador, máximo 5 subpasos por frame, `world.Step(timeStep*timeScale, 8, 10)` (`physics2runtimebehavior.ts:83, 201-215`). Datos compartidos de escena: `gravityX/Y, worldScale`. **[recomendación]** usar `rapier2d`.
- **Pathfinding** (`E/PathfindingBehavior/pathfindingruntimebehavior.ts`): por defecto `allowDiagonals true, acceleration 400, maxSpeed 200, angularMaxSpeed 180, rotateObject true, angleOffset 0, cellWidth/Height 20, gridOffsetX/Y 0, extraBorder, smoothingMaxCellGap` (`PathfindingBehavior.cpp:16-25`). `moveTo(x, y)` (`:402-475`): A* sobre una rejilla virtual (lista abierta ordenada por inserción, heurística euclídea con diagonales y Manhattan sin ellas (`:575-590, 651-1015`)), luego simplificación (`gdjs.pathfinding.simplifyPath`) si hay diagonales. Movimiento por segmentos con `speed += acc*dt` hasta maxSpeed y distancia Verlet (`:510-560`). Obstáculo: `impassable true, cost 2` (`PathfindingObstacleBehavior.cpp:14-15`).
- **Draggable** `{ checkCollisionMask }`: en pre-eventos `_tryBeginDrag` (ratón o touches nuevos), `_shouldEndDrag` y `_updateObjectPosition`; `wasJustDropped` se limpia en post-eventos (`E/DraggableBehavior/draggableruntimebehavior.ts:27,58-121`).
- **DestroyOutside** `{ extraBorder 200, unseenGraceDistance 10000 }` (`DestroyOutsideBehavior.cpp:15-16`). En post-eventos: con círculo envolvente frente a la cámara de su capa, si sale de `cam ± extraBorder` y ya estuvo en pantalla se destruye; si nunca estuvo, solo se destruye fuera de `unseenGraceDistance` (`destroyoutsideruntimebehavior.ts:36-75`).
- **Anchor** `{ relativeToOriginalWindowSize, left/right/top/bottomEdgeAnchor (None=0, WindowLeft/Top, WindowRight/Bottom, Proportional, WindowCenter), useLegacyBottomAndRightAnchors }` (`E/AnchorBehavior/anchorruntimebehavior.ts:7-20,60-88`). En el primer frame guarda las distancias a los bordes de la ventana o cámara y después reposiciona y redimensiona (`:101-330`).
- **Tween** (`E/TweenBehavior/`): `TweenManager.step()`, `t = min(t + dt, duration)` y aplica `easing(t/duration)` (`TweenManager.ts:859-875`). Easings: linear, easeIn/Out/InOut{Quad, Cubic, Quart, Quint, Sine, Expo, Circ, Back}, easeOutBounce, elastic, swingFromTo, bounce, bouncePast... (`TweenManager.ts:20+`). El comportamiento se actualiza en `doStepPreEvents` (`tweenruntimebehavior.ts:435`) y los tweens de escena o capa por callback pre-eventos (`tweentools.ts:160`). Hay tweens de posición, X/Y, ángulo, escala, opacidad, color, tamaño, variables, etc.
- **LinkedObjects**: `LinksManager` por escena, con `Map<objId, Set/Map<nombre, objetos>>` bidireccional; se limpia al borrar un objeto; sirve para seleccionar ("pick") objetos enlazados en eventos (`E/LinkedObjects/linkedobjects.ts:11-170, 302`).
- **ScreenWrap**: **no está en este repositorio**. Es una extensión comunitaria escrita con eventos (repo GDevelop-extensions) **[no verificable aquí]**. Algoritmo trivial: si x > borde derecho, x = izquierdo, etc.

---

## 4. Audio, recursos y entrada

### Audio (`R/howler-sound-manager/howler-sound-manager.ts`)
- Dos familias: **sounds** y **musics**, cada una con canales enteros (`_sounds/_musics: Record<channel, HowlerSound>`) y una lista de "free" sin canal (`:504-509`). Volumen global 0-100.
- Diferencia técnica: las músicas usan `html5: true` (streaming, sin espacialización) y los sonidos usan WebAudio en buffer (`:676, 994`).
- Reproducir en un canal detiene lo que hubiera en él (`:918, 985`). API de eventos: `playSound(file, loop, volume, pitch)`, `playSoundOnChannel(file, channel, loop, volume, pitch)` y lo mismo para Music (`R/events-tools/soundtools.ts:29-250`). Pitch limitado a [0.5, 4] (`:610-618`). Precarga por recurso: `preloadAsSound/preloadAsMusic/preloadInCache` (`project-data.d.ts:628-639`).
- **[recomendación]** macroquad::audio tiene volumen y loop pero no pitch; para el pitch, `kira`.

### Recursos (`R/ResourceLoader.ts`)
- `ResourceData { name, file, kind: image|audio|font|video|json|tilemap|tileset|bitmapFont|model3D|atlas|spine, smoothed?, disablePreload?, ... }` (`project-data.d.ts:628-658`).
- `loadGlobalAndFirstSceneResources` al arrancar y después `loadAllSceneInBackground` (`:354, 399`). `sceneResourcesPreloading 'at-startup'|'never'`, `sceneResourcesUnloading 'at-scene-exit'|'never'` (por proyecto o escena). Push y replace esperan a `areSceneAssetsReady` (`scenestack.ts:250-276`).

### Entrada (`R/inputmanager.ts`)
- Teclado: tablas `pressed`, `justPressed`, `released` indexadas por **keycode con ubicación**: `keyCode + 1000*location` (1 = izquierda, 2 = derecha, 3 = numpad). Shift, Ctrl, Alt y Meta sin ubicación cuentan como izquierda (+1000) (`:27, 115-130`).
- Nombres de tecla → código (`R/events-tools/inputtools.ts:22-130`): `a..z` (65-90), `Num0..Num9` (48-57), `Numpad0..9` (96-105), `LShift 1016, RShift 2016, LControl 1017, RControl 2017, LAlt 1018, RAlt 2018, LSystem 1091, RSystem 2091, SemiColon 186, Comma 188, Period 190, Quote 222, Slash 191, BackSlash 220, Equal 187, Dash 189, Menu 93, LBracket 219, RBracket 221, Tilde 192, Space 32, Back 8, Tab 9, Delete 46, Insert 45, Escape 27, PageUp 33, PageDown 34, End 35, Home 36, Return 13, NumpadPageUp 3033, NumpadPageDown 3034, NumpadEnd 3035, NumpadHome 3036, NumpadReturn 3013, Add 107, Subtract 109, Multiply 106, Divide 111, NumpadAdd 3107, NumpadSubtract 3109, NumpadMultiply 3106, NumpadDivide 3111, Left 37, Up 38, Right 39, Down 40, NumpadLeft 3037, NumpadUp 3038, NumpadRight 3039, NumpadDown 3040, F1..F12 (112-123), Pause 19`.
- Ratón: botones `LEFT 0, RIGHT 1, MIDDLE 2, BACK 3, FORWARD 4` (`:15-19`); posición en coordenadas de juego, que se convierten a mundo con `layer.convertCoords`; rueda; `isMouseInsideCanvas`.
- Táctil: touches con id; `getStartedTouchIdentifiers`, `popStartedTouch`; **touchSimulateMouse = true** por defecto (el primer toque mueve el ratón y simula el botón izquierdo; `MOUSE_TOUCH_ID = 1`) (`:20, 86, 538-640`).
- Todo lo "just/released" se limpia en `onFrameEnded` (§1.1).

---

## 5. Editor (newIDE, React + Electron)

### 5.1 Estructura general
- `IDE/MainFrame/index.js`: barra de pestañas de editores (`EditorTabs/`), cada pestaña un contenedor (`MainFrame/EditorContainers/`): `SceneEditorContainer`, `EventsEditorContainer`, `ExternalLayoutEditorContainer`, `ExternalEventsEditorContainer`, `ResourcesEditorContainer`, `DebuggerEditorContainer`, `HomePage`. Una escena abre **dos pestañas**: escena y eventos.
- **Project Manager** (`IDE/ProjectManager/index.js`): árbol con propiedades del juego (`ProjectPropertiesDialog.js`), carpetas de escenas, external events, external layouts, extensiones y gameplay tests (acciones "Add a scene", "Add external events"..., `:1596-1749`).
- Modelo de datos: el IDE manipula objetos C++ de `Core/` compilados a WASM (`GDevelop.js/`, `gd.*`). La serialización a JSON es la del §2 (`Core/Events/Serialization.cpp`).

### 5.2 Editor de escena
- Layout tipo mosaico (`IDE/SceneEditor/MosaicEditorsDisplay/index.js:42-70`): por defecto `properties | instances-editor | objects-list`; paneles opcionales `object-groups-list`, `instances-list`, `layers-list`.
- Lienzo (`IDE/InstancesEditor/`): `InstancesRenderer` (PIXI; cada tipo de objeto tiene un `Rendered*Instance` en `IDE/ObjectsRendering/Renderers/`: Sprite, Text, TiledSprite, PanelSprite, ParticleEmitter, ShapePainter...), `SelectionRectangle`, `InstancesSelection`, `InstancesMover`, `InstancesResizer`, `InstancesRotator`, `Grid.js`, `WindowBorder` (marco de la resolución), `ViewPosition` (pan/zoom), `InstancesAdder` (soltar desde la lista de objetos), `StatusBar`.
- Arrastre (`InstancesMover.js:78-150`): acumula delta total, imanta la **esquina del AABB de la selección** más cercana al punto de agarre, redondea a la rejilla (`roundPositionsToGrid`, `IDE/Utils/GridHelpers.js:5-33`; sin snap, redondea a px) y aplica el mismo delta a todas las instancias no bloqueadas; `followAxis` bloquea un eje (Shift).
- Rejilla: `uiSettings{grid, gridType rectangular|isometric, gridWidth/Height, gridOffsetX/Y, gridColor, gridAlpha, snap}` (`SetupGridDialog.js`).
- Panel de propiedades: `InstanceOrObjectPropertiesEditorContainer.js` y `InstancesEditor/CompactInstancePropertiesEditor` (X, Y, ángulo, capa, Z, bloqueo, tamaño custom, variables de instancia), además de las propiedades de capa (`LayersList/CompactLayerPropertiesEditor`) y de escena (`CompactScenePropertiesEditor`).
- Capas (`IDE/LayersList/`): lista ordenable con visibilidad, bloqueo, editor de capa y "fondo" como pseudo-item (`BackgroundColorTreeViewItemContent.js`).
- Objetos (`IDE/ObjectsList/`): árbol con carpetas; doble clic abre el editor del tipo (`IDE/ObjectEditor/Editors/`). El de Sprite (`SpriteEditor/`) tiene `AnimationList`, `SpritesList` (frames), `PointsEditor`, `CollisionMasksEditor`, `AnimationPreview` y `DirectionTools` (timeBetweenFrames, loop).
- Nota: hay un editor in-game reciente que edita dentro del propio runtime (`R/InGameEditor/`, comandos `updateInstances`, `addInstances`, `dragNewInstance`... en `R/debugger-client/abstract-debugger-client.ts:460-540`).

### 5.3 Hoja de eventos (`IDE/EventsSheet/`)
- Formato JSON de un evento (`Core/Events/Serialization.cpp:237-384`):
```json
{ "type":"BuiltinCommonInstructions::Standard", "disabled":false, "folded":false,
  "conditions":[{ "type":{"value":"PlatformBehavior::IsOnFloor","inverted":false},
                  "parameters":["Player","PlatformerObject"] }],
  "actions":[...], "events":[ /* subeventos */ ] }
```
  Tipos (`Core/Events/Builtin/`): Standard, Else, Comment, Group, Link (eventos externos), Repeat, While, ForEach, ForEachChildVariable, Async y JsCode.
- Semántica (**[resumen]**, `GDJS/Events/CodeGeneration/EventsCodeGenerator.cpp:1114-1170`): cada evento declara listas de objetos; las condiciones **filtran** las instancias ("picking") y las acciones se aplican solo a las seleccionadas; los subeventos heredan la selección. "Trigger once" con `OnceTriggers` (`Core/Extensions/Builtin/CommonInstructionsExtension.cpp:108-116`).
- Render (`EventsTree/`): árbol virtualizado (`SortableEventsTree.js`, alturas en `EventHeightsCache.js`). `Renderers/StandardEvent.js` usa `ConditionsActionsColumns.js`: **dos columnas**, condiciones a la izquierda con ancho = `ratio*sheetWidth - indent`, donde el ratio va de 0.5 (≤800 px) a 0.35 (≥1600 px) (`:29-82`). En pantallas estrechas pasa a una sola columna. Sangría por nivel de subevento.
- Instrucción (`EventsTree/Instruction.js`): frase traducible con marcadores `_PARAM0_`, por ejemplo `"_PARAM0_ is on floor"` (`E/PlatformBehavior/Extension.cpp:52-64`). Se trocea con `getAsFormattedText` y cada parámetro es un `<span>` clicable coloreado por tipo y validado (`gd.InstructionValidator`, `Instruction.js:300-400`); un clic abre el editor de parámetro inline (`InlineParameterEditor.js`). Los iconos de la instrucción y de inversión van delante.
- Metadatos de instrucción (`AddCondition/AddAction(id, nombre, descripción, frase, grupo, icono, iconoPequeño).AddParameter(tipo, etiqueta, extra).SetFunctionName(...)`) (`E/PlatformBehavior/Extension.cpp:52-89`). **[recomendación]** Para el port, un registro de instrucciones `struct InstrMeta { id, sentence, group, params: Vec<ParamMeta>, is_condition, func }`.
- Selector de instrucciones (`InstructionEditor/InstructionEditorDialog.js`, `InstructionOrObjectSelector.js`): pestañas **"objects"** (primero el objeto, luego sus instrucciones: comunes, de su tipo y de sus behaviors) y **"free-instructions"** (árbol por grupo/extensión), con búsqueda. Al elegir, a la derecha aparece `InstructionParametersEditor.js` con un campo por parámetro.
- Campos de parámetro (`EventsSheet/ParameterFields/`, mapeo en `ParameterRenderingService.js`): `ObjectField`, `ExpressionField` (número con autocompletado `IDE/ExpressionAutocompletion`), `StringField`, `OperatorField` (=, +, -, *, /), `RelationalOperatorField` (=, <, >, <=, >=, !=), `YesNoField/TrueFalseField`, `KeyboardKeyField`, `MouseButtonField`, `LayerField`, `SceneNameField`, `ObjectAnimationNameField`, `ObjectPointNameField`, `ImageResourceField/AudioResourceField`, `VariableField`, `BehaviorField` (automático), `ColorExpressionField` y `EasingField`.
- Barra (`ToolbarCommands.js`): añadir evento, subevento, comentario u otro tipo; activar/desactivar; deshacer; buscar (`SearchPanel.js`).

### 5.4 Flujo de preview
1. El IDE exporta el proyecto a una carpeta temporal: `exporter.exportProjectForPixiPreview(previewExportOptions)` (`IDE/ExportAndShare/LocalExporters/LocalPreviewLauncher/index.js:393`), con opciones `layoutName`, `externalLayoutName`, hot reload, etc. (`GDJS/IDE/ExporterHelper.h:74-346`). Se genera el JS de los eventos y `data.js` (el JSON del proyecto).
2. Electron abre una `BrowserWindow` (`ipcRenderer.invoke('preview-open')`, `index.js:124`; `newIDE/electron-app/app/PreviewWindow.js`).
3. El juego se conecta al IDE por WebSocket (`R/debugger-client/websocket-debugger-client.ts`; servidor `LocalPreviewDebuggerServer.js`, `electron-app/app/DebuggerServer.js`). Comandos: `play, pause, refresh, getStatus, set, call, profiler.start/stop, hotReload, hotReloadObjects, hotReloadLayers...` (`abstract-debugger-client.ts:321-400`). Hot reload: `R/debugger-client/hot-reloader.ts`.
**[recomendación egui]** El editor y el juego pueden ir en el mismo binario: el botón "Preview" serializa el proyecto en memoria, crea un `Game` nuevo y lo ejecuta en otra ventana o proceso (`cargo run -- --preview proj.json --scene X`); el debugger va por un canal o socket con los mismos verbos (play/pause/set/hotReload).

---

## 6. Exportación (resumen)
`GDJS/IDE/Exporter.cpp:57-200 ExportWholePixiProject`: copia los recursos, las librerías del runtime y las extensiones usadas, genera el código de eventos de cada escena (`ExportScenesEventsCode`) y `data.js`, y adapta un `index.html` según el destino (`R/index.html`, `R/Cordova/www/index.html`, `R/FacebookInstantGames/index.html`).
- **Web/HTML5**: carpeta estática con PixiJS y GDJS (`IDE/ExportAndShare/GenericExporters/HTML5Export.js`).
- **Electron** (escritorio): plantilla `R/Electron/{main.js, package.json}` + `ExportElectronFiles`; build local o en la nube (`OnlineElectronExport.js`).
- **Cordova** (Android/iOS): `R/Cordova/{config.xml, package.json, www/}` + `ExportCordovaFiles`; build en la nube de GDevelop (`OnlineCordovaExport.js`, `OnlineCordovaIosExport.js`).
- También Facebook Instant Games (`ExportFacebookInstantGamesFiles`).
**[recomendación]** En Rust/macroquad: binario nativo (Win/Linux/macOS), `wasm32-unknown-unknown` + `mq_js_bundle` para web y `cargo-quad-apk` para Android. El proyecto JSON y los assets se empaquetan junto al binario.

---

## 7. Checklist para el port (**[recomendación]**)
1. `Game { scene_stack: Vec<Scene>, input, sound, resources, max_fps, min_fps }`; bucle macroquad con `dt = min(get_frame_time(), 1/min_fps) * time_scale` por escena y capa.
2. `Scene::step`: async tasks → por instancia (fuerzas, `update` de animación, timers, behaviors pre) → flush de borrados → eventos (intérprete del JSON o bytecode) → flush → behaviors post → flush → culling y dibujo por capa ordenado por `z_order` → aplicar `requested_change`.
3. Colisión: polígonos convexos + SAT con MTV + fase ancha por círculo; `rstar` solo para las plataformas del Platformer.
4. Portar el Platformer tal cual (estados + `FollowConstraintContext`). Es lo más delicado; los tests de referencia están en `E/PlatformBehavior/tests/`.
5. Mantener el JSON de proyecto compatible (los campos del §2) para poder abrir proyectos de GDevelop sencillos.
