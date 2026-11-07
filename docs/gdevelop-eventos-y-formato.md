# GDevelop: formato de proyecto y semántica de eventos (spec para port a Rust)

Fuente: `/home/admin/gdevelop-src` (MIT, 4ian/GDevelop). Todas las rutas son relativas a esa raíz.
Convenciones: **[C]** = condición, **[A]** = acción, **[E]** = expresión numérica, **[S]** = expresión string.
"(inferido)" marca conclusiones deducidas del código y no verificadas ejecutando nada.

---

## 1. Formato JSON del proyecto

### 1.1 Serialización general
- Todo pasa por `gd::SerializerElement` (árbol genérico atributo/hijo → JSON). Los arrays se declaran con `ConsiderAsArrayOf("x")`; en JSON salen como arrays planos.
- Fuera del modo "canónico" se omiten los valores por defecto (`disabled`, `folded`, `events` vacíos, `variables` vacías...). Un lector tiene que tolerar campos ausentes (`Core/GDCore/Events/Serialization.cpp`, `SerializeEventsTo`; `Core/GDCore/Events/Builtin/StandardEvent.cpp`).
- Los nombres legacy en francés siguen aceptándose al leer (`"Contraire"` por `inverted`, `"Parametre"`, `"GroupesObjets"`, `"Positions"`...) (`Serialization.cpp`, `UnserializeInstructionsFrom`; `Core/GDCore/Project/Layout.cpp` líneas ~312-331).

### 1.2 Nivel superior (`Core/GDCore/Project/Project.cpp`, `Project::SerializeTo`)
| Clave | Contenido |
|---|---|
| `gdVersion` | `{major,minor,build,revision}`. Decide las migraciones al cargar (p.ej. `UpdateInstructionsFromGD2x/31x/4097` en `Serialization.cpp`). |
| `properties` | `name, description, author, version, windowWidth, windowHeight, maxFPS, minFPS, verticalSync, scaleMode ("linear"/"nearest"), pixelsRounding, adaptGameResolutionAtRuntime, sizeOnStartupMode, antialiasingMode, orientation, packageName, projectUuid, loadingScreen{...}, watermark{...}, platformSpecificAssets, authorIds[], categories[], playableDevices[], extensionProperties[], platforms[{name}], currentPlatform`. Flags de compatibilidad: `useDeprecatedZeroAsDefaultZOrder`, `useDeprecatedZeroAsDefaultStringVariable`. |
| `resources` | `{resources:[{name, kind, file, metadata, userAdded, smoothed?(image), preloadAsMusic/Sound/InCache?(audio), origin?}], resourceFolders:[]}`. `kind` ∈ image, audio, font, video, json, tilemap, tileset, bitmapFont, model3D, atlas, spine, javascript (`Core/GDCore/Project/ResourcesContainer.cpp` ~112-136, `ImageResource::SerializeTo` ~437, `AudioResource::SerializeTo` ~455). |
| `objects` | Objetos **globales** (mismo formato que los de escena). |
| `objectsGroups` | Grupos globales `[{name, objects:[{name}]}]`. |
| `variables` | Variables **globales** (ver §3). |
| `firstLayout` | Nombre de la escena inicial. `previewLayout` opcional. |
| `layouts` | Escenas (§1.3). |
| `externalEvents` | `[{name, associatedLayout, events:[...]}]` (`Core/GDCore/Project/ExternalEvents.cpp`). |
| `eventsFunctionsExtensions` | Extensiones definidas con eventos (§5). |
| `externalLayouts` | Instancias reutilizables (se crean con `CreateObjectsFromExternalLayout`). |
| `*FolderStructure`, `tests`, `namedEasings` | Metadatos de editor / opcionales. |

### 1.3 Escena (`Core/GDCore/Project/Layout.cpp`, `Layout::SerializeTo` l.239)
Campos: `name, mangledName, r, v, b` (color de fondo; ojo: verde = `v`), `title, standardSortMethod, stopSoundsOnStartup, disableInputWhenNotFocused, renderer3DWorldScale, resourcesPreloading?, resourcesUnloading?, uiSettings{...}`, y los hijos:
- `objectsGroups`: `[{name, objects:[{name}]}]`.
- `variables`: variables de escena.
- `instances`: instancias iniciales (abajo).
- `objects`: objetos de la escena.
- `events`: lista de eventos (§2).
- `layers`: capas (abajo).
- `behaviorsSharedData`: `[{name, type, ...props}]` (datos compartidos por comportamiento a nivel escena).

**Layer** (`Core/GDCore/Project/Layer.cpp`, `Layer::SerializeTo`): `name` ("" = capa base), `renderingType`, `cameraType`, `defaultCameraBehavior?`, `visibility`, `isLocked`, `isLightingLayer`, `followBaseLayerCamera`, `ambientLightColorR/G/B`, `camera3DNearPlaneDistance/FarPlaneDistance/FieldOfView`, `camera2DPlaneMaxDrawingDistance`, `cameras:[{defaultSize, width, height, defaultViewport, viewportLeft, viewportTop, viewportRight, viewportBottom}]`, `effects:[...]`.

**Instancia inicial** (`Core/GDCore/Project/InitialInstance.cpp`, `InitialInstance::SerializeTo`): siempre `name` (objeto), `x, y, zOrder, layer, angle, customSize, width, height, persistentUuid, numberProperties:[{name,value}], stringProperties:[{name,value}], initialVariables:[...]`. Sólo si ≠ defecto: `z, opacity (≠255), flippedX/Y/Z, rotationX/Y, depth, locked, sealed, keepRatio, hidden, behaviorOverridings`. Si `customSize=false`, `width/height` se ignoran y se usa el tamaño por defecto del objeto. `initialVariables` sobreescribe variables del objeto para esa instancia.

**Objeto** (`Core/GDCore/Project/Object.cpp`, `Object::SerializeTo`): `name, type, assetStoreId, persistentUuid?, resourcesPreloading?, variables[], effects[], behaviors[{name, type, ...propiedades aplanadas}]` y luego la configuración específica del tipo volcada **al mismo nivel** (`configuration->SerializeTo(element)`).
- `Sprite`: `updateIfNotVisible, animations:[{name, useMultipleDirections, directions:[{looping, timeBetweenFrames, sprites:[{image, points[], originPoint, centerPoint{automatic}, hasCustomCollisionMask, customCollisionMask[]}]}]}]` (ver fixture abajo; código en `Core/GDCore/Extensions/Builtin/SpriteExtension/SpriteObject.cpp`).
- `TextObject::Text`: campos legacy de raíz (`string, font, characterSize, color{r,g,b}, bold, italic, underlined, smoothed`) + `content{text, font, textAlignment, verticalTextAlignment, characterSize, lineHeight, color:"r;g;b", ...}` (`Extensions/TextObject/TextObject.cpp`, `DoSerializeTo`). Leer `content` primero.
- Comportamientos "por defecto" implícitos (capabilities): Sprite tiene `Effect, Resizable, Scale, Flippable, Opacity, Animation` (`SpriteExtension.cpp` l.40-45); Text tiene `Text, Effect, Scale, Opacity` (`Extensions/TextObject/Extension.cpp` l.40-43). Los nombres de comportamiento (`"Animation"`, `"Text"`, `"Scale"`, `"Opacity"`, `"Flippable"`, `"Effect"`, `"Resizable"`) son los que aparecen como 2º parámetro de las instrucciones de capability (`Core/GDCore/Extensions/Builtin/Capabilities/*.cpp`, `AddBehavior(...)` 3er argumento).

### 1.4 Ejemplo real (recortado)
Escena tomada de `GDJS/tests/games/count-instances/Instances Count test.json` (se eliminaron instancias/objetos repetidos); la capa viene de `GDJS/tests/games/events-functions-nested-create/game.json`.
```json
{
  "firstLayout": "", "gdVersion": {"build": 99, "major": 4, "minor": 0, "revision": 0},
  "properties": {"name": "Counting instances of objects", "windowWidth": 800, "windowHeight": 600,
                 "maxFPS": 60, "minFPS": 20, "scaleMode": "linear", "...": "..."},
  "resources": {"resources": [{"alwaysLoaded": false, "file": "assets/Green laser (01).png",
     "kind": "image", "metadata": "", "name": "Green laser (01).png", "smoothed": true, "userAdded": false}],
     "resourceFolders": []},
  "objects": [], "objectsGroups": [], "variables": [],
  "layouts": [{
    "name": "Untitled scene", "mangledName": "Untitled_32scene", "r": 209, "v": 209, "b": 209,
    "standardSortMethod": true, "stopSoundsOnStartup": true, "title": "",
    "objectsGroups": [{"name": "Lasers", "objects": [{"name": "BlueLaser08"}, {"name": "GreenLaser01"}]}],
    "variables": [],
    "instances": [{"angle": 0, "customSize": false, "height": 0, "layer": "", "locked": false,
       "name": "GreenLaser01", "persistentUuid": "e060a322-...", "width": 0, "x": 205, "y": 465,
       "zOrder": 1, "numberProperties": [], "stringProperties": [], "initialVariables": []}],
    "objects": [{"name": "GreenLaser01", "tags": "", "type": "Sprite", "updateIfNotVisible": false,
       "variables": [], "effects": [],
       "behaviors": [{"name": "Draggable", "type": "DraggableBehavior::Draggable", "checkCollisionMask": true}],
       "animations": [{"name": "", "useMultipleDirections": false, "directions": [{"looping": true,
          "timeBetweenFrames": 0.025, "sprites": [{"hasCustomCollisionMask": false,
          "image": "Green laser (01).png", "points": [],
          "originPoint": {"name": "origine", "x": 0, "y": 0},
          "centerPoint": {"automatic": true, "name": "centre", "x": 0, "y": 0},
          "customCollisionMask": []}]}]}]}],
    "events": [{
      "disabled": false, "folded": false, "type": "BuiltinCommonInstructions::Standard",
      "conditions": [{"type": {"inverted": false, "value": "PosX"},
                      "parameters": ["Lasers", "<", "SceneWindowWidth()/2"], "subInstructions": []}],
      "actions": [{"type": {"inverted": false, "value": "TextObject::String"},
                   "parameters": ["PickedCount", "=", "ToString(PickedInstancesCount(Lasers))"],
                   "subInstructions": []}],
      "events": []}],
    "layers": [{"name": "", "visibility": true, "effects": [], "cameras": [{"defaultSize": true,
       "defaultViewport": true, "height": 0, "width": 0, "viewportBottom": 1, "viewportLeft": 0,
       "viewportRight": 1, "viewportTop": 0}]}],
    "behaviorsSharedData": []
  }],
  "externalEvents": [], "eventsFunctionsExtensions": [{"name": "SampleInstancesCounter", "...": "§5"}],
  "externalLayouts": []
}
```

---

## 2. Sistema de eventos

### 2.1 Serialización de eventos (`Core/GDCore/Events/Serialization.cpp`)
Cada evento: `{type, disabled?, folded?, aiGeneratedEventId?, ...campos del tipo}`. Tipo desconocido ⇒ `EmptyEvent` + warning (`UnserializeEventsFrom`).

| `type` | Campos propios | Fuente |
|---|---|---|
| `BuiltinCommonInstructions::Standard` | `conditions[], actions[], events[]?, variables[]?` | `Core/GDCore/Events/Builtin/StandardEvent.cpp` |
| `BuiltinCommonInstructions::Else` | igual que Standard | `.../ElseEvent.cpp` |
| `BuiltinCommonInstructions::Comment` | `color{r,g,b,textR,textG,textB}, comment, comment2?` | `.../CommentEvent.cpp` |
| `BuiltinCommonInstructions::Group` | `name, source, creationTime, colorR/G/B, events[], parameters[]` | `.../GroupEvent.cpp` |
| `BuiltinCommonInstructions::While` | `infiniteLoopWarning, whileConditions[], conditions[], actions[], events[]?, variables[]?, loopIndexVariable?` | `.../WhileEvent.cpp` |
| `BuiltinCommonInstructions::Repeat` | `repeatExpression` (string expr.), `conditions, actions, events?, variables?, loopIndexVariable?` | `.../RepeatEvent.cpp` |
| `BuiltinCommonInstructions::ForEach` | `object` (objeto o grupo), `conditions, actions, events?, variables?, loopIndexVariable?, orderBy?, order ("asc"/"desc")?, limit?` | `.../ForEachEvent.cpp` |
| `BuiltinCommonInstructions::ForEachChildVariable` | `iterableVariableName, valueIteratorVariableName, keyIteratorVariableName, conditions, actions, events?, variables?, loopIndexVariable?` | `.../ForEachChildVariableEvent.cpp` |
| `BuiltinCommonInstructions::Link` | `include{includeConfig: 0=todo,1=grupo,2=por índice(deprecado), eventsGroup?, start?, end?}, target` | `.../LinkEvent.cpp`, enum en `LinkEvent.h` l.19 |
| `BuiltinCommonInstructions::JsCode` | `inlineCode, parameterObjects, useStrict, eventsSheetExpanded` | `GDJS/GDJS/Events/Builtin/JsCodeEvent.cpp` |

No existe un tipo "SubEvent": los sub-eventos son el array `events` de un evento que `CanHaveSubEvents()`.

### 2.2 Instrucciones (condiciones y acciones)
```json
{"type": {"value": "PosX", "inverted": false, "await": false},
 "parameters": ["Lasers", "<", "SceneWindowWidth()/2"],
 "subInstructions": []}
```
- `type.value`: nombre completo. Las extensiones builtin "sin namespace" (BuiltinObject, Sprite, audio, ratón, teclado, tiempo, variables, cámara, ventana, escena, avanzadas, conversiones, strings, matemáticas) usan el nombre pelado (`PosX`, `Create`); el resto lleva `Extension::Nombre` y los de comportamiento `Extension::Behavior::Nombre` (`Core/GDCore/Extensions/PlatformExtension.cpp`, `SetNameSpace` l.595-615 y `AddCondition` l.42). Al leer se reemplaza `Automatism`→`Behavior`.
- `parameters`: **array de strings posicional que incluye los parámetros "code-only"** (escena, contexto) serializados como `""`. Ej. real: `KeyPressed` → `["", "Space"]`; `Create` → `["", "Pt", "Obj.PointX(\"Pt\")", "Obj.PointY(\"Pt\")", ""]`; `SetNumberVariable` → `["Damage", "+", "10"]` (`GDJS/tests/games/**`). Faltantes se rellenan con `""` en codegen (`Core/GDCore/Events/CodeGeneration/EventsCodeGenerator.cpp` l.336-341). Cada string se interpreta según el tipo del parámetro en la metadata: `expression`/`number` → expresión numérica; `string` → expresión string (los literales llevan comillas: `"\"s\""`); `object/objectList/objectPtr` → nombre plano; `operator` ∈ `= + - * /` (y `True/False/Toggle` para booleanos); `relationalOperator` ∈ `= < > <= >= !=` + `startsWith/endsWith/contains` para strings; `yesorno` → `"yes"`/`"no"`; `trueorfalse` → `"True"`/`"False"`; `key`, `mouse`, `layer`... (`EventsCodeGenerator.cpp` `GenerateParameterCodes` l.871-990; `GenerateRelationalOperatorCodes` l.98).
- `inverted`: niega la condición (ver §2.4).
- `subInstructions`: sólo para `BuiltinCommonInstructions::Or`, `::And`, `::Not` (y las legacy `subConditions`/`subActions` al leer).
- Alias legacy: muchos nombres viejos se registran con `AddDuplicated*` y siguen en proyectos (`Cache`→`Hide`, `Montre`→`Show`, `MettreX/Y/XY`, `DepartScene`→`SceneJustBegins`, `SourisSurObjet`→`IsCursorOnObject`, `AjoutObjConcern`→`PickAllInstances`, `AjoutHasard`→`PickRandomInstance`, `Egal`/`StrEqual`, ...). Además hay instrucciones ocultas legacy (no duplicadas) muy frecuentes: `VarScene/ModVarScene`, `VarGlobal/ModVarGlobal`, `VarObjet/ModVarObjet`, `KeyPressed/KeyReleased`, `MouseButtonPressed/Released`, `ChangeAnimation`, `FlipX`, `Opacity`, `TextObject::String`. Un port debe tener una tabla de alias (`grep AddDuplicated Core/GDCore/Extensions/Builtin`).
- Operadores en acciones: con `op != "="` se genera `setter(getter() op (rhs))`; con `=` `setter(rhs)` (`EventsCodeGenerator.cpp` `GenerateOperatorCall` l.127-186).

### 2.3 Modelo de ejecución por frame
`RuntimeScene.renderAndStep` (`GDJS/Runtime/runtimescene.ts` ~l.405-480): actualizar tiempo → tareas async (Wait) → `_updateObjectsPreEvents` (aplicar fuerzas, `update`, timers de objeto, `doStepPreEvents` de behaviors; `GDJS/Runtime/RuntimeInstanceContainer.ts` l.613) → callbacks pre-eventos → **función de eventos de la escena** → `_stepBehaviorsPostEvents` → callbacks post → render.
La función de eventos generada empieza con `runtimeScene.getOnceTriggers().startNewFrame()` (`GDJS/GDJS/Events/CodeGeneration/EventsCodeGenerator.cpp` `GenerateLayoutCode` l.104-128).
Orden: los eventos se ejecutan secuencialmente de arriba a abajo; sub-eventos inmediatamente tras las acciones del padre si las condiciones del padre fueron verdaderas.

### 2.4 Selección de objetos (picking) — la regla central
Implementación: `Core/GDCore/Events/CodeGeneration/EventsCodeGenerationContext.h` + `EventsCodeGenerator.cpp` (Core y GDJS).

1. **Listas por nombre de objeto real.** Cada evento mantiene una lista de instancias "seleccionadas" por *objeto* (no por grupo). Un grupo se expande a sus objetos (`ObjectsContainersList::ExpandObjectName`, `Core/GDCore/Project/ObjectsContainersList.cpp` l.348); filtrar el grupo `Lasers` filtra las listas de `BlueLaser08` y `GreenLaser01`, y eso afecta a usos posteriores de esos objetos por su nombre.
2. **Primera referencia ⇒ todas las instancias.** Cuando una instrucción necesita la lista de `X` (`ObjectsListNeeded`) y ningún contexto padre la declaró, se inicializa con `runtimeScene.getObjects("X")` = todas las instancias vivas (`GDJS/.../EventsCodeGenerator.cpp` `GenerateObjectsDeclarationCode` l.1146-1151, `GenerateAllInstancesGetterCode` l.1182). Si el padre ya la tenía, se **copia** la del padre (`gdjs.copyArray(padre, hijo)`), o se reutiliza la misma si es el último evento de la lista (optimización `Reuse`, `Core/.../EventsCodeGenerator.cpp` l.1124-1131). Por tanto: un objeto nunca mencionado antes en la cadena de eventos padres se trata como "todas las instancias".
3. **Cada evento hermano tiene su propio contexto** que hereda del padre (`InheritsFrom`): lo seleccionado en un evento no afecta a sus hermanos, sólo a sus sub-eventos (`Core/.../EventsCodeGenerator.cpp` `GenerateEventsListCode` l.1113-1118).
4. **Condiciones de objeto filtran in-place:** para cada objeto real, `for i: if (pred(list[i])) list[k++]=list[i]; list.length=k;` y la condición es verdadera si quedó ≥1 instancia (de cualquiera de los objetos del grupo) (`GDJS/.../EventsCodeGenerator.cpp` `GenerateObjectCondition` l.852-895; igual para behaviors l.897). Con `inverted`, el predicado se niega *por instancia*: quedan las instancias que NO cumplen.
5. **Las condiciones se evalúan en cascada (AND con cortocircuito):** cada condición está dentro de `if (isConditionTrue)` de la anterior (`GDJS/.../EventsCodeGenerator.cpp` `GenerateConditionsListCode` l.1226-1256). Una condición posterior ve las listas ya filtradas por las anteriores. Si una es falsa, las restantes no se evalúan (relevante para Trigger once).
6. **Acciones de objeto** iteran la lista seleccionada: `for (i...) list[i].metodo(args)` (`GenerateObjectAction` l.961-1026). Los argumentos se re-evalúan por instancia y, dentro de ellos, `Obj.X()` del mismo objeto se refiere a la instancia actual `list[i]` (`GenerateObjectFunctionCall` l.774-792). Si no hubo condición sobre el objeto, la acción afecta a todas las instancias.
7. **Expresiones sobre otro objeto** (`Otro.X()` en una instrucción que no es de `Otro`) usan la **primera** instancia seleccionada `list[0]`, o un valor por defecto (0/"") si la lista está vacía (mismo sitio, l.788-791). Igual para parámetros `objectPtr` (`GenerateObject` l.1372-1392) y variables de objeto (`GenerateGetVariable` l.1490-1508).
8. **Condiciones con dos listas** (colisión, distancia, `IsTurnedTowardObject`...): reciben mapas de listas y usan `twoListsTest`: marcan `pick` en ambos objetos de cada par que cumple y luego recortan ambas listas; invertida ⇒ sólo se filtra la primera lista, quedando los objetos que no cumplen con *ningún* objeto de la segunda (`GDJS/Runtime/events-tools/objecttools.ts` l.60-181).
9. **Tipos de parámetro de objeto** (`GDJS/.../EventsCodeGenerator.cpp` `GenerateObject` l.1290-1396):
   - `object`: instrucción "de objeto", se itera por instancia (puntos 4/6).
   - `objectList`: se pasa un mapa `{nombre: lista}`; la función puede filtrar las listas (picking).
   - `objectListOrEmptyIfJustDeclared` (usado por `Create`): si nadie declaró antes la lista en este evento ni en padres, se declara **vacía** en lugar de "todas".
   - `objectListOrEmptyWithoutPicking` (usado por `SceneInstancesCount`, `PickedInstancesCount`): no fuerza la declaración; objetos no declarados se pasan como lista vacía.
   - `objectPtr`: primera instancia seleccionada (o la actual).
10. **Crear añade a la selección:** `Create` crea la instancia, le pone posición/capa/zOrder por defecto de la capa y hace `push` a la lista del objeto; como la lista empieza vacía si no estaba declarada (punto 9), tras `Create` las acciones siguientes y los sub-eventos operan **sólo** sobre la nueva instancia (o sobre lo previamente seleccionado + la nueva) (`objecttools.ts` `doCreateObjectOnScene` l.1040-1066; metadata en `Core/GDCore/Extensions/Builtin/BaseObjectExtension.cpp` l.1718-1735).
11. **Pick all** (`PickAllInstances`, acción y condición) recopia `getObjects(name)` en cada lista del mapa ⇒ "reinicia" la selección (`objecttools.ts` `pickAllObjects` l.878-890). `PickRandomInstance` deja una sola; `PickNearest` la más cercana.
12. **Borrar**: `Delete` → `deleteFromScene` → la instancia sale inmediatamente de la lista viva de la escena (`RuntimeInstanceContainer.ts` `markObjectForDeletion` l.741-768) pero (inferido) sigue en las listas ya copiadas del evento en curso hasta que éste termina.
13. **Bucles** (`While`, `Repeat`, `ForEach`, `ForEachChildVariable`) usan `ForbidReuse`: en cada iteración las listas se vuelven a copiar desde el padre, así que el filtrado de una iteración no contamina la siguiente (`GDJS/GDJS/Extensions/Builtin/CommonInstructionsExtension.cpp` l.436-440, 709-713, 797-802). `ForEach` construye la lista combinada de los objetos (del padre o todas) y en cada iteración deja exactamente 1 instancia en la lista (l.780-1127); con `orderBy` ordena por la expresión y aplica `limit`.

### 2.5 Or / And / Not (`GDJS/GDJS/Extensions/Builtin/CommonInstructionsExtension.cpp`)
- **Or** (l.249-367): cada sub-condición se evalúa sobre una *copia* de las listas de entrada; si es verdadera, sus instancias seleccionadas se unen (sin duplicados) en una lista `...final`. Al final, las listas del evento = unión. Resultado = OR de las sub-condiciones.
- **And** (l.369-385): la lista de sub-condiciones se genera como una lista normal (cascada) y su resultado se propaga.
- **Not** (l.387-404): evalúa las sub-condiciones en cascada **sobre las mismas listas** y niega el resultado. Observado en el código: el filtrado que hicieron las sub-condiciones no se deshace, de modo que `Not(cond de objeto)` puede dejar las listas vacías aunque el resultado sea verdadero. Para objetos usar `inverted` en vez de `Not`.
- Condiciones "libres" (sin objeto) invertidas: `!(pred)`, salvo que la metadata tenga un parámetro `conditionInverted`, que entonces la gestiona la propia función (`GDJS/.../EventsCodeGenerator.cpp` `GenerateFreeCondition` l.817-850).

### 2.6 Trigger once (`BuiltinCommonInstructions::Once`)
- Codegen: `isConditionTrue = runtimeScene.getOnceTriggers().triggerOnce(<id>)`, con un id entero único por instrucción original (`GenerateSingleUsageUniqueIdFor`) (`CommonInstructionsExtension.cpp` l.406-420). En funciones de eventos se usa `eventsFunctionContext.getOnceTriggers()`; en comportamientos por eventos cada instancia de behavior tiene su `_onceTriggers` (`GDJS/GDJS/Events/CodeGeneration/BehaviorCodeGenerator.cpp` l.102, 200, 471).
- Runtime (`GDJS/Runtime/oncetriggers.ts`): dos conjuntos `thisFrame` y `lastFrame`. `startNewFrame()` mueve `thisFrame`→`lastFrame` y vacía `thisFrame`. `triggerOnce(id)` marca `thisFrame[id]=true` y devuelve `!lastFrame.has(id)`. Es decir: verdadero el primer frame consecutivo en que se alcanza; como sólo se alcanza si las condiciones previas fueron verdaderas (cascada §2.4.5), equivale a "flanco de subida de las condiciones anteriores". No es por instancia de objeto (documentado en `Core/GDCore/Extensions/Builtin/CommonInstructionsExtension.cpp` l.107-116).

### 2.7 Codegen de cada tipo de evento (GDJS)
Todos en `GDJS/GDJS/Extensions/Builtin/CommonInstructionsExtension.cpp`:
- **Standard** (l.128-182): `[init variables locales] condiciones; if (isConditionTrue) { declarar listas; acciones; sub-eventos }`. Sin condiciones ⇒ siempre.
- **Else** (l.184-238 + `Core/.../EventsCodeGenerator.cpp` l.1133-1192): una cadena Standard→Else→Else comparte `elseEventsChainSatisfied`; un Else se ejecuta sólo si ningún eslabón previo se cumplió y sus propias condiciones son verdaderas. Un Else sin Standard previo actúa como Standard. Comentarios/eventos deshabilitados son transparentes.
- **While** (l.422-514): `do { redeclarar listas; whileConditions; if (true) { conditions; if { actions; subeventos } } else stop } while(!stop)`. Si está vacío no se genera (evita bucle infinito).
- **Repeat** (l.696-778): la expresión se evalúa **una vez** antes del bucle; `for (i<count) { redeclarar listas; [indexVar=i]; condiciones; acciones; subeventos }`.
- **ForEach** (l.780-1127): ver §2.4.13. `loopIndexVariable` recibe el índice.
- **ForEachChildVariable** (l.516-694): no itera primitivos; para structure itera claves (key → string), para array índices (key → número); copia el valor del hijo a la variable "value" (primitivos por valor; structure/array reemplazando hijos, compartiendo referencias).
- **Group** (l.1129-1142): sólo genera sus sub-eventos (más marcas de profiler). Sin condiciones.
- **Comment**: no genera código (l.240-247).
- **Link** (l.109-126; `Core/GDCore/Events/Builtin/LinkEvent.cpp` `ReplaceLinkByLinkedEvents`): en preprocesado se **inlinean** (copian) los eventos del `target` (external events o eventos de otra escena; opcionalmente sólo los sub-eventos de un Group con ese nombre) en el lugar del link. Los links anidados se resuelven porque el preprocesado continúa por los eventos insertados.
- **JsCode** (l.1144+): inserta JS crudo; `parameterObjects` pasa la lista seleccionada de ese objeto. Un port en Rust no puede soportarlo salvo con un motor JS (marcar como no soportado).
- Variables locales de evento (`variables` en el evento): se apilan en `localVariables` durante el evento y se hace `pop` al final (l.136-139, 176-179); tienen prioridad sobre las de escena/globales en la resolución de nombres (§3.2).
- Cada lista de eventos se emite como una función JS separada `eventsListN(runtimeScene)`; las listas de objetos son variables estáticas `GDObjN_Depth` (`GDJS/.../EventsCodeGenerator.cpp` l.1193-1224, `GetObjectListName` l.1098). Detalle de implementación, no semántico.

---

## 3. Variables y expresiones

### 3.1 Variables
- JSON (`Core/GDCore/Project/Variable.cpp` `SerializeTo` l.264-297; `VariablesContainer.cpp` l.193): `{"name","type","value"}` para primitivos; `type` ∈ `number|string|boolean|structure|array`; structure: `children:[{name, type, ...}]`; array: `children:[{type, ...}]` (sin name). Extras: `folded`, `persistentUuid`. Tipo por defecto al leer: `"string"` (`UnserializeFrom`).
- Ámbitos: global (`project.variables`), escena (`layout.variables`), objeto (`object.variables`, sobreescribibles por `instance.initialVariables`), local de evento, y en extensiones: globales/escena de extensión, parámetros y propiedades (`Core/GDCore/Project/VariablesContainersList.cpp` l.17-110).
- Runtime (`GDJS/Runtime/variable.ts`): por defecto `number 0`. Conversiones: `getAsNumber` (string→`parseFloat`, NaN→0; bool→1/0) l.579; `getAsString` (number→`toString()`, bool→"true"/"false", structure→"[Structure]", array→"[Array]") l.606; `getAsBoolean` (number≠0; string ∉ {"0","","false"}; colecciones→true) l.646. `getChild(name)` convierte a structure si no lo es; en arrays usa índice entero (l.483-493). Las setters cambian el tipo (`setNumber`, `setString`...).
- Acceder a una variable no declarada la **crea** con valor por defecto (`GDJS/Runtime/variablescontainer.ts` `get` l.154-168).

### 3.2 Resolución de identificadores
`ProjectScopedContainers::MatchIdentifierWithName` (`Core/GDCore/Project/ProjectScopedContainers.h` l.127-152): objeto/grupo > variable (local > escena > global, búsqueda en orden inverso de apilado, `VariablesContainersList::Has` l.129) > parámetro > propiedad. Así `Foo` es variable salvo que exista un objeto `Foo`; `Obj.Foo` sin paréntesis es variable de objeto (`ExpressionCodeGenerator.cpp` `OnVisitIdentifierNode`).

### 3.3 Gramática de expresiones (`Core/GDCore/Events/Parsers/ExpressionParser2.h`, `GrammarTerminals.h`)
Parser LL(1) recursivo, sin tokenizer separado:
```
Start      := Expression EOF
Expression := Term ( ('+'|'-') Expression )?      # recursión a la DERECHA
Term       := Factor ( ('*'|'/') Factor )*        # bucle (izquierda)
Factor     := '"' texto '"'                       # escapes \" y \\ ; otro \x se deja literal
            | ('+'|'-') Factor                    # '-' seguido de dígito ⇒ número negativo
            | Número                              # [0-9.]+ , un solo punto, ".5"→"0.5", sin exponente
            | '(' Expression ')'
            | Identifier
Identifier := Nombre ('::' Nombre)?               # namespace de extensión: Ext::Func
              ( '(' Params ')'                    # función libre:   ToString(x)
              | '.' Nombre ( '::' Nombre '(' Params ')'   # behavior: Obj.Beh::Func(a)
                           | '(' Params ')'                # objeto:   Obj.X()
                           | ('.'|'[') Accesores )         # variable de objeto: Obj.V.child
                           | ε )                           # Obj.V  (variable/hijo)
              | '[' Accesores                      # Var[expr]...
              | ε )                                # identificador simple (variable)
Accesores  := ( '.' Nombre | '[' Expression ']' )*
Params     := (Expression (',' Expression)*)?
```
- Operadores binarios válidos sólo `+ - * /` (`ValidateOperator` l.563-574); `<, >, =, ?, ^, \, :, !` se reconocen léxicamente para dar error. Unarios `+ -`. No hay comparadores, ni booleanos, ni `%` (usar `mod(a,b)`).
- **Ojo precedencia:** el AST de `+/-` es asociativo a la derecha (`a-b-c` → `a-(b-c)`), pero el generador vuelve a imprimir `lhs op rhs` sin paréntesis y deja que JS reaplique la asociatividad izquierda (`Core/GDCore/Events/CodeGeneration/ExpressionCodeGenerator.cpp` `OnVisitOperatorNode` l.93-99). Un evaluador en Rust que recorra el AST directamente daría resultados distintos: hay que re-asociar a izquierda.
- Identificadores: cualquier carácter salvo separadores/operadores/comillas/corchetes/espacios y `~'%#@|&\`$;` (Unicode permitido); espacios internos tolerados por compatibilidad en nombres de función/objeto (`GrammarTerminals.h` `IsAllowedInIdentifier`).
- Strings: en contexto string sólo se permite `+` (concatenación); números en contexto string requieren `ToString()` (`Core/GDCore/IDE/Events/ExpressionValidator.h` l.104-140). Las variables se leen con `getAsNumber()`/`getAsString()` según el tipo esperado por el parámetro (`EventsCodeGenerator.h` `GenerateVariableValueAs` l.614).
- En expresiones **no** se escriben los parámetros code-only; el primer parámetro "objeto" va antes del punto y el behavior antes de `::` (`ExpressionParser2.h` l.71-86).
- Ejemplos válidos: `Player.X() + 10`, `Enemy.Behavior::Speed()`, `"Score: " + ToString(Score)`, `Inventory[Index].Name`, `Player.Stats.HP`, legacy `Variable(Score)`, `VariableString(Name)`, `GlobalVariable(Lives)`, `Player.Variable(HP)`, `SampleInstancesCounter::CountInstancesOnScene(Lasers)`.

---

## 4. Núcleo de instrucciones built-in (≈65)
Firmas sin parámetros code-only. `op(t)`/`relop(t)` = par operador + valor de tipo `t`. Fuentes: `Core/GDCore/Extensions/Builtin/*.cpp` y `.../Capabilities/*.cpp`. Expresiones de objeto se escriben `Obj.Nombre(args)`.

**Objeto base** (`BaseObjectExtension.cpp`)
- [C] `PosX`/`PosY`(object, relop(number)); [A] `SetX`/`SetY`(object, op(number)); [A] `SetXY`(object, operator, x, operator, y); [E] `X`, `Y`, `Width`, `Height`, `Angle`, `ZOrder`.
- [A] `SetCenterX`/`SetCenterY`; [C/E] `CenterX`/`CenterY` (generadas por `AddExpressionAndConditionAndAction`: condición `N`, acción `SetN`, expresión `N`; `Core/GDCore/Extensions/PlatformExtension.cpp` l.152-210).
- [A] `SetAngle`(object, op), `Rotate`(object, degPerSec), `RotateTowardPosition`(object, x, y, speed), `RotateTowardObject`(object, objectPtr, speed); [C] `Angle`(object, relop).
- [A] `AddForceXY`(object, vx, vy, multiplier), `AddForceAL`(object, angle, speed, multiplier), `AddForceTowardPosition`(object, x, y, speed, mult), `AddForceTowardObject`(object, objectPtr, speed, mult), `ClearForces`(object); [C] `HasNoForces`, `Speed`. Multiplicador: 0 = instantánea (se borra tras aplicarse un frame), 1 = permanente, otro valor = decae (deprecado) (`GDJS/Runtime/runtimeobject.ts` `updateForces` l.1895-1915). Las fuerzas se aplican antes de los eventos (§2.3).
- [A] `Create`(objectListOrEmpty, x, y, layer); `CreateByName`(group, nameExpr, x, y, layer); `Delete`(object).
- [A] `Hide`/`Show`(object); [C] `Visible`(object); [A] `SetZOrder`(object, op); [A] `ChangeLayer`(object, layer); [C] `Layer`(object, layer).
- [C] `CollisionNP`(objectList, objectList, ignoreTouchingEdges?) ; [C] `Distance`(objectList, objectList, dist); [C] `IsCursorOnObject`(objectList, accurate?); [C] `CollisionPoint`(object, x, y); [A] `SeparateFromObjects`(object, objectList, ignoreTouching?).
- [A/C] `PickAllInstances`(objectList), `PickRandomInstance`(objectList), `PickNearest`(objectList, x, y); [C/E] `SceneInstancesCount`(obj, relop) / `PickedInstancesCount`(obj, relop).
- Variables de objeto: [A] `SetNumberObjectVariable`/`SetStringObjectVariable`(object, objectvar, op(t)), `SetBooleanObjectVariable`(object, objectvar, True|False|Toggle); [C] `NumberObjectVariable`/`StringObjectVariable`(object, objectvar, relop(t)), `BooleanObjectVariable`(object, objectvar, trueorfalse); [A] `ObjectVariablePushNumber/String/Bool`, `ObjectVariableRemoveAt`, `ObjectVariableClearChildren`; legacy `VarObjet`/`ModVarObjet`.
- Timers de objeto: [C] `CompareObjectTimer`(object, name, relop, seconds); [A] `ResetObjectTimer`/`PauseObjectTimer`/`UnPauseObjectTimer`/`RemoveObjectTimer`(object, name); [E] `ObjectTimerElapsedTime`.
- [E] `Distance`(object, objectPtr), `DistanceToPosition`(object, x, y), `AngleToObject`, `AngleToPosition`; [S] `ObjectName`, `Layer`.
- Behaviors: [A] `ActivateBehavior`(object, behavior, yesorno); [C] `BehaviorActivated`.

**Sprite y capabilities** (`SpriteExtension/SpriteExtension.cpp`, `Capabilities/*.cpp`; nombre real `Cap::Behavior::X`, 2º parámetro = nombre del behavior)
- `AnimatableCapability::AnimatableBehavior::` [A] `SetName`/`SetIndex`(object, beh, op), `PauseAnimation`, `PlayAnimation`, `SetSpeedScale`, `SetElapsedTime`; [C] `Name`/`Index`, `HasAnimationEnded`, `IsAnimationPaused`. Legacy: `ChangeAnimation`, `SetAnimationName`, `AnimationEnded`.
- `FlippableCapability::FlippableBehavior::` [A] `FlipX`/`FlipY`(object, beh, yesorno); [C] `FlippedX`/`FlippedY`.
- `ScalableCapability::ScalableBehavior::` [A] `SetValue`/`SetX`/`SetY`(op); `OpacityCapability::OpacityBehavior::SetValue`(op) / [C] `Value`.
- Sprite: [E] `PointX`/`PointY`(object, pointName); [A] `ChangeColor`(object, "r;g;b").

**Texto**: `TextContainerCapability::TextContainerBehavior::SetValue`(object, "Text", op(string)) / [C] `Value`; legacy muy común `TextObject::String`(object, op(string)). Otras: `TextObject::ChangeColor`, `TextObject::Font`, `TextObject::SetFontSize`(op) / [C/E] `TextObject::FontSize` (`Extensions/TextObject/Extension.cpp` l.444, `String` l.669).

**Teclado** (`KeyboardExtension.cpp`): [C] `KeyFromTextPressed`(stringExpr p.ej. `"Left"`), `KeyFromTextJustPressed`, `KeyFromTextReleased`, `AnyKeyPressed`, `AnyKeyReleased`; [S] `LastPressedKey()`. Legacy ocultas: `KeyPressed`/`KeyReleased`(key literal sin comillas).

**Ratón/táctil** (`MouseExtension.cpp`): [C] `MouseButtonFromTextPressed`/`Released`(`"Left"|"Right"|"Middle"`); [E/C] `CursorX`/`CursorY`(layer, camera) (alias `MouseX`); [C] `IsMouseWheelScrollingUp/Down`; [E] `TouchX/TouchY`(id, layer, cam), `StartedTouchOrMouseCount`, `StartedTouchOrMouseId`; [C] `HasAnyTouchOrMouseStarted`, `HasTouchEnded`.

**Escena** (`SceneExtension.cpp`): [C] `SceneJustBegins` (legacy `DepartScene`) = primer frame (`GDJS/Runtime/events-tools/runtimescenetools.ts` l.9-13); [A] `Scene`(sceneName, stopOthers?), `PushScene`, `PopScene`, `Quit`, `SceneBackground`(color); [S] `CurrentSceneName()`.

**Variables** (`VariablesExtension.cpp`): [C] `NumberVariable`/`StringVariable`(var, relop(t)), `BooleanVariable`(var, trueorfalse); [A] `SetNumberVariable`/`SetStringVariable`(var, op(t)), `SetBooleanVariable`(var, True|False|Toggle); [A] `PushNumber/PushString/PushBoolean/PushVariable`(arrayVar, v), `RemoveVariableAt`, `ClearVariableChildren`, `RemoveVariableChild`; [C] `VariableChildCount`, `VariableChildExists2`; [E] `VariableChildCount`. Legacy: `VarScene/ModVarScene`, `VarGlobal/ModVarGlobal` (+ `Txt`).

**Tiempo** (`TimeExtension.cpp`): [C] `CompareTimer`(name, relop, seconds) — un timer inexistente se crea y devuelve falso (`runtimescenetools.ts` l.62-75); [A] `ResetTimer`, `PauseTimer`, `UnPauseTimer`, `RemoveTimer`, `ChangeTimeScale`, `Wait`(seconds, async); [E] `TimeDelta()`, `TimerElapsedTime(name)`, `TimeFromStart()`.

**Audio** (`AudioExtension.cpp`): [A] `PlaySound`/`PlayMusic`(file, loop, volume, pitch), `PlaySoundOnChannel`/`PlayMusicOnChannel`(file, channel, loop, volume, pitch), `StopSoundChannel`/`StopMusicChannel`, `PauseSoundChannel`, `ResumeSoundChannel`, `SetSoundChannelVolume`/`SetMusicChannelVolume`(ch, op), `SetGlobalVolume`(op); [C] `MusicPlaying`/`SoundPlaying`(ch).

**Cámara/capas** (`CameraExtension.cpp`): [A/C/E] `CameraCenterX`/`CameraCenterY` (acción `SetCameraCenterX`; params op, layer, camera), `CameraAngle`; [A] `CenterCameraOnObject`(objectPtr, anticipate, layer, cam), `ZoomCamera`(value, layer, cam), `ClampCamera`(l, t, r, b, layer, cam), `ShowLayer`/`HideLayer`(layer); [C] `LayerVisible`; [E] `CameraWidth/Height`, `CameraBorderLeft/Right/Top/Bottom`, `CameraZoom`.

**Ventana** (`WindowExtension.cpp`): [E] `SceneWindowWidth()`, `SceneWindowHeight()`; [A] `SetFullScreen`, `SetWindowTitle`.

**Comunes** (`CommonInstructionsExtension.cpp`): [C] `BuiltinCommonInstructions::Or/And/Not/Once`, `CompareNumbers`(a, relop, b), `CompareStrings`(a, relop, b).

**Matemáticas / strings / conversión** (expresiones): `Random(max)` (entero 0..max inclusive, `GDJS/Runtime/gd.ts` l.255), `RandomInRange(min,max)`, `RandomFloat`, `abs, min, max, clamp, floor, ceil, round, roundTo, sqrt, pow, mod, sin/cos/tan` (radianes), `atan2, lerp, AngleDifference, ToRad, ToDeg, Pi()`; `ToString(n)`, `ToNumber(s)`, `StrLength`, `SubStr(s,start,len)`, `StrFind`, `StrReplaceAll`, `ToUpperCase`, `NewLine()` (`MathematicalToolsExtension.cpp`, `StringInstructionsExtension.cpp`, `CommonConversionsExtension.cpp`).

**Funciones** (`AdvancedExtension.cpp`): [A] `SetReturnNumber`, `SetReturnString`, `SetReturnBoolean` → `eventsFunctionContext.returnValue = ...` (`GDJS/GDJS/Extensions/Builtin/AdvancedExtension.cpp` l.34-67).

---

## 5. Funciones de eventos / extensiones

### 5.1 Almacenamiento
`eventsFunctionsExtensions[]` (`Core/GDCore/Project/EventsFunctionsExtension.cpp` `SerializeTo`): `name` (= namespace de las instrucciones), `version, extensionNamespace, fullName, shortDescription, description, category, tags[], author, dependencies[], sourceFiles[]?, globalVariables[], sceneVariables[], eventsFunctions[], eventsBasedBehaviors[], eventsBasedObjects[]`.

Cada función (`Core/GDCore/Project/EventsFunction.cpp` `SerializeTo`): `name, fullName, description?, sentence, group?, getterName? (para ActionWithOperator), private?, async?, functionType ∈ Action|Condition|Expression|StringExpression|ExpressionAndCondition|ActionWithOperator, expressionType{...}? (si expresión), parameters[{name, type, description, optional, defaultValue, supplementaryInformation, codeOnly}], objectGroups[], events[]`. Ejemplo real (`GDJS/tests/games/count-instances/Instances Count test.json`):
```json
{"name": "CountInstancesOnScene", "functionType": "Expression", "sentence": "",
 "parameters": [{"name": "ObjectToCount", "type": "objectList", "optional": false, "description": "Objects to count"}],
 "events": [{"type": "BuiltinCommonInstructions::Standard", "conditions": [],
   "actions": [{"type": {"inverted": false, "value": "SetReturnNumber"},
                "parameters": ["SceneInstancesCount(ObjectToCount)"]}]}],
 "objectGroups": []}
```
Se invoca como `SampleInstancesCounter::CountInstancesOnScene(Lasers)` en expresiones o con `type.value = "Ext::Func"` en instrucciones.

### 5.2 Declaración e invocación (`GDJS/GDJS/Events/CodeGeneration/MetadataDeclarationHelper.cpp`, `EventsCodeGenerator.cpp`)
- Cada función se registra como instrucción/expresión `Ext::Func` con: parámetro code-only `currentScene` al inicio (l.560-673), luego los parámetros del usuario, y siempre un code-only `eventsFunctionContext` al final (l.1631, 1686). Por eso en JSON una llamada se ve como `["", "MyObject", ""]` (`GDJS/tests/games/events-functions-nested-create/game.json`). `ActionWithOperator` inserta `operator`+valor (tipo del getter) tras los primeros parámetros (l.1663-1677).
- Código: `gdjs.evtsExt__<Ext>__<Func>.func(runtimeScene, args..., parentEventsFunctionContext)` (`GetFreeFunctionCodeName` l.1746-1753, `GenerateEventsFunctionParameterDeclarationsList` l.396-420). Condición ⇒ devuelve `!!returnValue`; expresión numérica ⇒ `Number(returnValue)||0`; string ⇒ `""+returnValue` (`GenerateEventsFunctionReturn` l.727-745).
- Contexto (`GenerateEventsFunctionContext` l.545-725): `_objectsMap` (mapas de listas recibidos por parámetro), `_objectArraysMap` (= `gdjs.objectsListsToArray(param)`, **copia plana** creada al entrar; `GDJS/Runtime/gd.ts` l.665-679), `_behaviorNamesMap` (nombre de behavior del parámetro → real), `getObjects(name)` (lo que usan los eventos internos como "todas las instancias"), `createObject` (crea vía el contexto padre o la escena y hace `push` en la lista del llamador), `getArgument(name)` para parámetros no-objeto, variables de extensión (`globalVariablesForExtension`, `sceneVariablesForExtension`), `localVariables`, y `getOnceTriggers()` (los de la escena para funciones libres, l.164).
- Picking a través de funciones (inferido de lo anterior): dentro de la función, "todas las instancias" de un parámetro objeto = las instancias que el llamador tenía seleccionadas. Como los eventos internos trabajan sobre la copia plana, filtrar dentro de una función libre **no** reduce las listas del llamador; la única mutación visible fuera es `createObject` (push). Para las instrucciones de comportamiento/objeto definidas por eventos el filtrado sí ocurre en el llamador, porque se generan como condición de behavior iterada por instancia (§2.4.4, `GenerateBehaviorCondition`).
- Comportamientos por eventos (`eventsBasedBehaviors`) y objetos por eventos (`eventsBasedObjects`) usan el mismo formato de función, con funciones de ciclo de vida (`onCreated`, `doStepPreEvents`, etc.; `EventsFunctionsExtension::IsExtensionLifecycleEventsFunction`) y sus propios once-triggers por instancia (`BehaviorCodeGenerator.cpp` l.200, 471).

### 5.3 Recomendaciones para el port (opinión, no del código)
1. Cargar JSON tolerando campos ausentes y aplicar tabla de alias legacy.
2. Compilar la metadata de instrucciones a una tabla `nombre → (tipo, [ParamKind], impl)` donde `ParamKind` incluya los code-only para alinear índices.
3. Implementar el picking como `HashMap<ObjectName, Vec<InstanceId>>` por contexto con herencia/copia, más los cuatro modos de parámetro de objeto (§2.4.9).
4. Parser de expresiones propio con re-asociación a izquierda de `+/-`; tipado guiado por el tipo del parámetro (number/string).
5. Inlinear Links y expandir funciones de eventos en carga; no soportar JsCode.
