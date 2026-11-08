# Godot 3D: especificación para llevar un motor 3D a NicoStation

Fuente: `/home/admin/godot-src` (MIT, godotengine/godot master, clon superficial, nov. 2026).
Abreviaturas de rutas: `S3/` = `scene/3d/`, `SR/` = `scene/resources/`, `RS/` = `servers/rendering/`,
`RD/` = `servers/rendering/renderer_rd/`, `SH/` = `RD/shaders/`, `ED3/` = `editor/scene/3d/`, `M/` = `modules/`.
Formato de cita: `ruta:línea`. Lo marcado **(inferido)** es interpretación mía, no sale literal del código.
Las recomendaciones de la sección 9 son propuestas, no hechos de Godot.

---

## 1. Modelo de escena 3D

### 1.1 Node3D: transformaciones
- Guarda `local_transform` (Basis 3x3 + origin) y `global_transform` cacheados, más rotación Euler y escala por separado
  (`S3/node_3d.h:102-118`). El orden de Euler por defecto es **YXZ** (`node_3d.h:115`). Euler se guarda aparte porque pasar
  Basis→Euler→Basis pierde datos (720°, etc.) (`node_3d.h:68-83`).
- Bits sucios perezosos: `DIRTY_EULER_ROTATION_AND_SCALE`, `DIRTY_LOCAL_TRANSFORM`, `DIRTY_GLOBAL_TRANSFORM`,
  `DIRTY_GLOBAL_INTERPOLATED_TRANSFORM` (`node_3d.h:85-91`). Si se escribe el Euler se ensucia el local, y si se escribe
  el local se ensucia el Euler. Cada uno se recalcula **solo cuando alguien lo lee** (`node_3d.cpp:92-104`).
- `get_global_transform()`: si está sucio, `global = parent.global * local`, salvo `top_level`, que usa solo el local
  (`node_3d.cpp:640-669`). Con `disable_scale` se ortonormaliza.
- Propagación: `_propagate_transform_changed` recorre los hijos Node3D (y **no entra en los `top_level`**), los marca sucios
  y encola `NOTIFICATION_TRANSFORM_CHANGED` solo si el nodo pidió avisos o tiene gizmos (`node_3d.cpp:112-140`).
- Notificaciones: `TRANSFORM_CHANGED=2000`, `ENTER_WORLD=41`, `EXIT_WORLD=42`, `VISIBILITY_CHANGED=43`,
  `LOCAL_TRANSFORM_CHANGED=44` (`node_3d.h:234-239`).
- Convención de ejes: Y arriba, **−Z es "adelante"** (`Vector3::FORWARD = (0,0,-1)`, `core/math/vector3.h:232`).
  Los modelos importados miran a +Z (`MODEL_FRONT`, `vector3.h:238`). `look_at` arma
  `Basis::looking_at(target-pos, up, use_model_front)` y conserva la escala (`node_3d.cpp:1243-1260`, `core/math/basis.cpp:1034`).
- Unidad: 1 = 1 metro (sufijo `m` en las propiedades, p. ej. `S3/light_3d.cpp` spot_range).
- Bucle principal: hasta 8 pasos de física por frame a 60 Hz (`main/main.cpp:2244-2245`). Orden por cuadro:
  `physics_process` de la escena → navegación → `PhysicsServer3D::step` (`main.cpp:5044-5078`), después `process` (`:5105`)
  y al final `RenderingServer::draw` (`:5136`). Para que se vea fluido a cualquier Hz hay interpolación de física (FTI),
  que guarda `global_transform_interpolated` (`node_3d.h:104`).

### 1.2 Nodos visuales (todos heredan de `GeometryInstance3D`, `S3/visual_instance_3d.h`)
Comunes: `layers=1` (máscara de capas de render, `:44`), `cast_shadow` (OFF/ON/DOUBLE_SIDED/SHADOWS_ONLY, `:96-124`),
`material_override`, `material_overlay` (`:125-126`), rango de visibilidad (HLOD) begin/end/márgenes/fade (`:128-132`),
`transparency` (`:134`), `lod_bias=1` (`:136`) y `gi_mode=STATIC` (`:144`).

| Nodo | Datos clave (cita) |
|---|---|
| MeshInstance3D | `mesh`, `surface_override_materials[]`, `skin`, `skeleton_path`, pistas de blend shapes (`S3/mesh_instance_3d.h:49-56`) |
| Camera3D | PERSPECTIVE/ORTHOGONAL/FRUSTUM; `fov=75`, `size=1`, `near=0.05`, `far=4000`, `keep_aspect=KEEP_HEIGHT`; `environment`, `attributes` (CameraAttributes: exposición/DOF), `compositor` (`S3/camera_3d.h:44-87`). API: `project_ray_origin/normal`, `unproject_position`, `is_position_behind`, `cull_mask` (`:168-182`) |
| Light3D (base) | Por defecto: energía 1, indirecta 1, specular 0.5, range 5, size 0, attenuation 1, spot_angle 45, bias 0.1, normal_bias 1, blur 1, temperatura 6500 K, `bake_mode=DYNAMIC` (`S3/light_3d.cpp:504-533`, `light_3d.h:86`) |
| DirectionalLight3D | Modo de sombra ORTHOGONAL / PSSM 2 / **PSSM 4 (por defecto)**; `shadow_max_distance=100`, splits 0.1/0.2/0.5, normal_bias 2, `pancake_size=20`, specular 1, intensidad 100000 lux en modo físico (`light_3d.cpp:519-523, 635-641`) |
| OmniLight3D | Sombra DUAL_PARABOLOID o **CUBE (por defecto)** (`light_3d.cpp:682-685`). Atenuación: `(1-(d/r)^4)^2 · d^-decay` (`SH/scene_forward_lights_inc.glsl:457`) |
| SpotLight3D | `spot_range`, `spot_attenuation`, `spot_angle` (0-180), `spot_angle_attenuation`; bias 0.03 (`light_3d.cpp:705-716`). Avisa si hay sombra con ángulo ≥ 90° |
| AreaLight3D (nuevo) | `area_size` Vector2, `area_texture`, `area_normalize_energy` (`S3/light_3d.h:246-275`) |
| Decal | Texturas ALBEDO/NORMAL/ORM/EMISSION, `size=(2,2,2)`, `albedo_mix`, `upper/lower_fade=0.3`, `normal_fade`, `cull_mask` (`S3/decal.h:39-57`). No existe en Compatibility (`doc/classes/Decal.xml:11`) |
| Sprite3D / AnimatedSprite3D | `pixel_size=0.01`, flags SHADED/DOUBLE_SIDED/FIXED_SIZE, alpha_cut DISABLED/DISCARD/OPAQUE_PREPASS/HASH, `billboard_mode` (`S3/sprite_3d.h:43-94`) |
| Label3D | `pixel_size=0.005`, mismos alpha_cut, contorno (outline), billboard; agrupa las superficies por (textura, prioridad, contorno) (`S3/label_3d.h:51-92`) |
| MultiMeshInstance3D | Recurso MultiMesh: `transform_format`, `use_colors`, `use_custom_data`, `instance_count`, `visible_instance_count=-1` (`SR/multimesh.h:41-59`) |
| GPUParticles3D | `amount`, `lifetime`, `one_shot`, `explosiveness`, `fixed_fps`, `interpolate=true`, `sub_emitter`, trails, `process_material`, `draw_passes[]` (`S3/gpu_particles_3d.h:68-98`). Colisionadores (esfera/caja/SDF/heightfield) y atractores (`S3/gpu_particles_collision_3d.h:38-343`) |
| ParticleProcessMaterial | 18 parámetros con mín/máx/curva (velocidad inicial, angular, órbita, aceleración lineal/radial/tangencial, damping, ángulo, escala, hue, animación, turbulencia...) (`SR/particle_process_material.h:50-70`). Formas de emisión: punto, esfera, superficie de esfera, caja, puntos, puntos dirigidos, anillo (`:84-93`) |
| CPUParticles3D | Mismo modelo en CPU; se puede convertir desde uno GPU (`S3/cpu_particles_3d.cpp:1465`) |
| CSG (M/csg) | `CSGShape3D` con UNION/INTERSECTION/SUBTRACTION, `snap=0.001`, `use_collision` (`M/csg/csg_shape.h:46-73`). Primitivas: Box, Sphere, Cylinder, Torus, Polygon (extrusión/spin/path), Mesh; `CSGCombiner3D` (`:199-373`). Usa la librería **Manifold** (`M/csg/SCsub:16-30`, `csg_shape.cpp:55`). `bake_static_mesh()`, `bake_collision_shape()` (`csg_shape.h:176-178`) |
| GridMap (M/gridmap) | `cell_size=(2,2,2)`, `octant_size=8`, `center_x/y/z`, `cell_scale`, `MeshLibrary` (`M/gridmap/grid_map.h:169-184`). Celda = `item:16, rot:5, layer:8` bits (rot = índice de las 24 orientaciones ortogonales), clave = x,y,z int16 (`grid_map.h:56-95`). Cada octante se dibuja con **MultiMesh** (`grid_map.cpp:946-947`). Item de MeshLibrary: mesh, transform, formas de colisión, navmesh, preview (`SR/3d/mesh_library.h:62-75`) |
| Path3D / PathFollow3D | Curve3D; `progress`, `progress_ratio`, `loop=true`, `tilt_enabled`, `rotation_mode` NONE/Y/XY/XYZ/ORIENTED (por defecto XYZ), interpolación cúbica (`S3/path_3d.h:80-131`) |
| VisibleOnScreenNotifier3D | AABB + señales `screen_entered`/`screen_exited` (`S3/visible_on_screen_notifier_3d.cpp:44-61`); `VisibleOnScreenEnabler3D` activa o pausa un nodo (`.h:61`) |
| WorldEnvironment | Un solo `Environment` por escena; la cámara puede reemplazarlo (`camera_3d.h:85`) |

### 1.3 Environment (`SR/environment.h`, valores por defecto)
- Fondo: CLEAR_COLOR (por defecto)/COLOR/SKY/CANVAS/KEEP/CAMERA_FEED (`:42-96`). Ambiente: BG/DISABLED/COLOR/SKY,
  `ambient_energy=1`, `ambient_sky_contribution=1`. Reflejos: BG/DISABLED/SKY (`:52-112`).
- Sky: `radiance_size=256` (32 a 2048), process_mode AUTOMATIC/QUALITY/... (`SR/sky.h:41-61`). Materiales de cielo:
  `ProceduralSkyMaterial`, `PanoramaSkyMaterial`, `PhysicalSkyMaterial` (`SR/3d/sky_material.h:36,124,164`), o un shader `sky`.
- Tonemap: LINEAR (por defecto)/REINHARDT/FILMIC/ACES/AGX, `exposure=1`, `white=1` (`:65-118`). Fórmulas en
  `SH/effects/tonemap.glsl:93-246`: Reinhard extendido, Filmic = Hable con exposure_bias 2, ACES (ajuste de MJP con bias 1.8) y AgX.
- SSR: `max_steps=64`, fade_in 0.15, fade_out 2, depth_tolerance 0.5 (`:124-128`). SSAO: radius 1, intensity 2, power 1.5,
  detail 0.5, horizon 0.06, sharpness 0.98 (`:132-140`); está basado en el ASSAO de Intel (`SH/effects/ssao.glsl:2-16`).
  SSIL: radius 5 (`:144-148`).
- SDFGI: 4 cascadas, min_cell 0.2, y_scale 75%, bounce_feedback 0.5, lee la luz del cielo (`:153-162`).
- Glow: intensity 0.3, strength 1, mix 0.05, bloom 0, modo **SCREEN**, hdr_threshold 1, hdr_scale 2, luminance_cap 12
  (`:166-177`). Niveles por defecto `[0, 0.8, 0.4, 0.1, 0, 0, 0]`: se usan los mips 1-3 (`SR/environment.cpp:1643-1648`).
- Niebla: EXPONENTIAL/DEPTH, density 0.01, color (0.518,0.553,0.608), sun_scatter, altura, aerial_perspective,
  sky_affect 1 (`:182-198`). Niebla volumétrica: density 0.05, anisotropy 0.2, length 64, reproyección temporal 0.9 (`:203-215`).
  Hay nodos `FogVolume` (`S3/fog_volume.cpp`).
- Ajustes de color: brillo, contraste, saturación y LUT 1D/3D (`:219-223`).
- **La "pinta Godot" del editor**: si la escena no trae WorldEnvironment, el editor agrega un sol con PSSM 4 splits
  (`ED3/node_3d_editor_plugin.cpp:4113-4114`) a 60° de altura y 150° de azimut, un ProceduralSky con cielo (0.385,0.454,0.55)
  y suelo (0.2,0.169,0.133), glow encendido (salvo en Compatibility) y sin tonemap ni SSAO ni GI (`:3125-3150`). Al
  activar el tonemap del preview se usa FILMIC (`:3122`).

---

## 2. Recursos

### 2.1 Mallas
- `Mesh` = lista de superficies. Cada superficie tiene un primitivo (POINTS/LINES/LINE_STRIP/TRIANGLES/TRIANGLE_STRIP,
  `SR/mesh.h:61-68`), arrays VERTEX, NORMAL, TANGENT, COLOR, TEX_UV, TEX_UV2, CUSTOM0-3, BONES, WEIGHTS, INDEX (`:99-115`),
  blend shapes, LODs y un material.
- `ArrayMesh` se arma con `mesh_add_surface_from_arrays` (`RS/rendering_server.h:219`). `ImmediateMesh`, `MeshDataTool`, SurfaceTool.
- Primitivas (`SR/3d/primitive_meshes.h:45-522`): Capsule, Box, Cylinder, Plane, Quad, Prism, Sphere, Torus, Point,
  TubeTrail, RibbonTrail, TextMesh.

### 2.2 BaseMaterial3D (StandardMaterial3D / ORMMaterial3D) (`SR/material.h:140-931`)
- Texturas (19): albedo, metallic, roughness, emission, normal, rim, clearcoat, flowmap, AO, heightmap, SSS, transmittance,
  backlight, refraction, detail (mask/albedo/normal), **ORM** (solo ORMMaterial3D) y bent normal (`:147-168`).
- Transparencia: DISABLED, ALPHA, ALPHA_SCISSOR, ALPHA_HASH, ALPHA_DEPTH_PRE_PASS. Shading: UNSHADED/PER_PIXEL/PER_VERTEX.
  Blend: MIX/ADD/SUB/MUL/PREMULT. Depth draw: OPAQUE_ONLY/ALWAYS/DISABLED. Cull: BACK/FRONT/DISABLED. Diffuse: **BURLEY**/LAMBERT/
  LAMBERT_WRAP/TOON. Specular: **SCHLICK_GGX**/TOON/DISABLED. Billboard: DISABLED/ENABLED/FIXED_Y/PARTICLES (`:186-306`).
  Flags: albedo desde el color de vértice, triplanar (uv1/uv2, local o mundo), unshaded, no recibe sombras, fixed size, MSDF,
  no niebla, etc. (`:256-283`).
- Valores por defecto: albedo blanco, specular 0.5, **roughness 1**, metallic 0, emission negro ×1, rim 1, clearcoat 1/0.5,
  heightmap_scale 5, refraction 0.05, alpha_scissor 0.5 (el mismo que glTF), distance_fade 0-10, `texture_repeat` true
  (`SR/material.cpp:3965-4040`).
- **Cómo llega al renderer**: BaseMaterial3D no tiene un shader fijo. A partir de sus enums y flags arma una `MaterialKey`
  (`material.h:359`), **genera el código de un shader spatial** (`material.cpp:659`, que empieza con `shader_type spatial;\nrender_mode ...`
  en `:753`) y lo guarda en un `shader_map` compartido con conteo de usuarios (`:664-693`). Así, cada combinación de opciones
  es una variante de shader y los parámetros numéricos son uniforms.

### 2.3 ShaderMaterial y el lenguaje de shaders
- `shader_type spatial|canvas_item|particles|sky|fog`. Funciones `vertex()`, `fragment()` y `light()`.
- Built-ins de `fragment` (`RS/shader_types.cpp:137+`): VERTEX, NORMAL, TANGENT, BINORMAL, VIEW, UV, UV2, COLOR, **ALBEDO, ALPHA,
  METALLIC, SPECULAR, ROUGHNESS, EMISSION, NORMAL_MAP, AO, RIM, CLEARCOAT, ANISOTROPY, SSS_*, BACKLIGHT**, DEPTH, SCREEN_UV,
  FRAGCOORD, matrices MODEL/VIEW/PROJECTION (+ inversas), CAMERA_POSITION_WORLD, FOG, RADIANCE, IRRADIANCE, ALPHA_SCISSOR_THRESHOLD...
- `light()`: LIGHT, LIGHT_COLOR, ATTENUATION, LIGHT_IS_DIRECTIONAL, ALBEDO, ROUGHNESS, METALLIC, DIFFUSE_LIGHT/SPECULAR_LIGHT
  (que son inout).
- Render modes de spatial (`shader_types.cpp:241-263`): `blend_*`, `depth_draw_*`, `depth_prepass_alpha`, `depth_test_*`,
  `cull_*`, `unshaded`, `wireframe`, `diffuse_*`, `specular_*`, `skip_vertex_transform`, `world_vertex_coords`, `shadows_disabled`,
  `ambient_light_disabled`, `shadow_to_opacity`, `vertex_lighting`, `alpha_to_coverage`, `fog_disabled`...
- Cadena de compilación: `ShaderLanguage::compile` (parser y tipos, `RS/shader_language.cpp:11535`, archivo de más de 11k líneas),
  luego `ShaderCompiler::compile` → GLSL (`RS/shader_compiler.cpp:1570`). Ese código se inyecta en la plantilla del renderer
  (`RD/forward_clustered/scene_shader_forward_clustered.cpp:41`; GLES3: `drivers/gles3/storage/material_storage.cpp:2980`) y en RD
  se compila a SPIR-V con glslang. `M/visual_shader` es el editor de grafos y genera el mismo lenguaje.

---

## 3. Servidores de render

### 3.1 La forma de la API (`RS/rendering_server.h`)
Todo se maneja con **RID** opacos, sin nodos: `mesh_create` (`:200`), `material_create` (`:183`), `shader_create` (`:167`),
`multimesh_create` (`:259`), `skeleton_create` (`:303`), `directional_light_create` (`:314`), `camera_create` (`:537`),
`viewport_create` + `viewport_attach_camera` + `viewport_set_scenario` (`:551-584`), `environment_create` (`:660`),
`scenario_create` (`:730`) e `instance_create` / `instance_set_base` / `instance_set_scenario` / `instance_set_transform`
(`:741-747`). Al final del cuadro se llama `draw()` (`:981`).
Los nodos de escena son una capa delgada encima: cada nodo es dueño de un RID de instancia y empuja ahí sus cambios.

### 3.2 Culling (`RS/renderer_scene_cull.cpp`)
- Cada escenario tiene un `DynamicBVH` por tipo de indexador (`renderer_scene_cull.h:341`). `render_camera` (`:2709`) hace culling
  por frustum en paralelo (`_scene_cull`, `:2919`) y, opcionalmente, oclusión por raster en CPU (`renderer_scene_occlusion_cull.cpp`,
  con Embree en `M/raycast`).
- `_render_scene` (`:3325`): primero prepara el culling luz-cámara, después las direccionales (`:~3385`), las sombras
  direccionales y posicionales (`:~3500`), SDFGI (`:~3676`) y por último llama al renderer.
- Cascadas: `_light_instance_update_shadow` (`:2402`) reparte los splits según `shadow_max_distance × split_offsets`.
  Ajustes globales: atlas direccional de 4096 (`RS/rendering_server.cpp:3695`), calidad de soft shadows (`:3697, :3703`) y
  8 capas de roughness para el cielo (`:3720`).

### 3.3 Forward+ (clustered), orden real del cuadro
`RD/forward_clustered/render_forward_clustered.cpp`:
1. Prepare/Setup 3D (`:1805, :1857`). Construye los clusters: `cluster_builder->setup(...)` (`:167`).
2. Sombras: omni (`:1636`), y en paralelo direccional/spot + GI (`:1657`).
3. Setup Sky (`:2151`). **Depth prepass** (en paralelo con GI) (`:2207`) y resolve MSAA (`:2230`).
4. Pre-opaco: SSAO (`:1458`), SSIL (`:1484`), contact shadows (`:1546`) y niebla volumétrica (`:1756`).
5. **Opaque pass** (`:2281`) y motion vectors (`:2337`).
6. **Sky** (`:2389`), después SSS (`:2429`), merge specular (en este punto entra SSR, `:1518`, `:2437`) y copias de pantalla y profundidad para
   `SCREEN_TEXTURE`/refracción (`:2459-2470`).
7. **Transparent pass** (`:2503`), ordenado de atrás hacia adelante. Resolve MSAA (`:2524`).
8. FSR2 o TAA (`:2575, :2643`), y por último la post (`RD/renderer_scene_render_rd.cpp:455+`): **DOF** (`:504`), **auto exposure** (`:557`),
   **Glow** (`:581`), **Tonemap** con FXAA dentro (`:676, :723`) y SMAA (`:820`).
Hay puntos de inyección del `Compositor` entre fases (`:2256, :2361, :2422, :2499, :2561`).

### 3.4 Iluminación clustered (`RD/cluster_builder_rd.*`)
- La pantalla se parte en tiles de **32 px** (`cluster_builder_rd.h:193`, `cluster_builder_rd.cpp:290`) y **32 cortes de profundidad**
  (un bit por corte, `SH/cluster_render.glsl:92-94`). Cada elemento (omni, spot, área, decal, reflection probe,
  `cluster_builder_rd.h:152-157`) se rasteriza como volumen en un pase de GPU que marca bits por celda.
  Hay un límite de 512 elementos (`RS/rendering_server.cpp:3829`).
- BRDF (`SH/scene_forward_lights_inc.glsl`): D_GGX (`:18`), V_GGX (`:56`), Schlick (`:76`), `light_compute` (`:101`), difuso Burley o Lambert
  (`:201-252`). Sombras PCF con kernel en disco rotado (`:317-407`), y búsqueda de bloqueador (PCSS) para luces con tamaño (`:410`).
- IBL: el cielo se pre-filtra a un **octmap** con mips de roughness (`RD/environment/sky.cpp:281-426`, importance sampling GGX). La
  especular usa `lod = sqrt(roughness)·MAX_ROUGHNESS_LOD` (`SH/forward_clustered/scene_forward_clustered.glsl:1720-1729`) y una
  **LUT DFG** con compensación multiscatter al estilo Filament (`scene_forward_clustered_inc.glsl:506-516`, uso en `.glsl:2335`).

### 3.5 Mobile y Compatibility
- Mobile (`RD/forward_mobile`): sin clusters. Cada instancia guarda listas de hasta **8** omni, 8 spot, 8 área, 8 decals y 8 probes
  (`render_forward_mobile.h:62, :562-570`). Rango HDR limitado a 2.0 (`doc/classes/Environment.xml:148`). No tiene SSAO, SSR, SSIL ni SDFGI.
- Compatibility (GLES3/WebGL2, `drivers/gles3/rasterizer_scene_gles3.cpp`): sombras (`:2222`), depth prepass (`:2785`),
  opaco (`:2880`), cielo (`:2937`) y transparente (`:2985`). Las luces con sombra se dibujan en **pases aditivos** por objeto (`:3395-3457`).
  Límites: 32 luces renderizables y 8 por objeto (`RS/rendering_server.cpp:3834-3835`). Tiene SSAO (`drivers/gles3/shaders/s4ao_*.glsl`) y glow,
  pero no SSR, SSIL, SDFGI, VoxelGI ni decals (`doc/classes/Decal.xml:11`, `VoxelGI.xml:8`).
- Selección: `rendering/renderer/rendering_method`. La primera opción, y por lo tanto la de escritorio por defecto, es `forward_plus`. En móvil
  por defecto es `mobile` (`main/main.cpp:2494-2513, :2661`).

### 3.6 GI y reflejos
| Técnica | Qué es | Renderers |
|---|---|---|
| ReflectionProbe | Cubemap local con caja/paralaje (`S3/reflection_probe.cpp`) | todos; máx. 8 por malla en Mobile y 2 en Compat (`ReflectionProbe.xml:10`) |
| LightmapGI | Bake en GPU (`M/lightmapper_rd`). Calidad LOW-ULTRA, `bounces=3`, denoiser, `texel_scale` (`S3/lightmap_gi.h:154-200`). Necesita UV2 (no funciona con CSG ni primitivas, `LightmapGI.xml:11`) | todos |
| VoxelGI | Voxeliza la escena en 64-512 subdivisiones (`S3/voxel_gi.h:98-106`), con bake | solo Forward+ |
| SDFGI | Cascadas SDF en tiempo real alrededor de la cámara (`RD/environment/gi.cpp`, `SH/environment/sdfgi_*.glsl`) | solo Forward+ (`Environment.xml:219`) |
| SSIL / SSR | Espacio de pantalla | solo Forward+ |

### 3.7 Qué hace que algo "se vea Godot" (mínimo) y qué es extra
**Mínimo (inferido, a partir del preview del editor, §1.3):** PBR metal/roughness (GGX + Burley/Lambert) con normal map →
un sol con **sombras PSSM 2-4 cascadas + PCF** → cielo procedural usado como fondo, **ambiente e IBL** (pre-filtrado + DFG) →
render en HDR (RGBA16F) → **glow** con mips 1-3 en modo screen → **tonemap** (Filmic/ACES/AgX) → sRGB. Con SSAO sube bastante la calidad.
**Extras de mayor costo:** SDFGI, VoxelGI, SSIL, SSR, niebla volumétrica, TAA/FSR, decals, SSS, clearcoat/anisotropía, partículas
con colisión SDF, área lights, lightmaps horneados.

---

## 4. Física 3D

### 4.1 Servidor y motor por defecto
- `PhysicsServer3D` también usa RID: `space_create`, `area_create`, `body_create`, `body_set_mode`, `*_shape_create`
  (`servers/physics_3d/physics_server_3d.h:76-166`) y **`body_test_motion`** (`:258`), que es la base del CharacterBody.
- Consultas (`servers/physics_3d/direct_states/physics_direct_space_state_3d.h:67-74`): `intersect_ray`, `intersect_point`,
  `intersect_shape`, `cast_motion`, `collide_shape` y `rest_info`.
- Formas: WORLD_BOUNDARY (plano), SEPARATION_RAY, SPHERE, BOX, CAPSULE, CYLINDER, CONVEX_POLYGON, CONCAVE_POLYGON, HEIGHTMAP
  (`servers/physics_3d/physics_server_3d_enums.h:38-48`).
- **Por defecto**: el módulo GodotPhysics3D se registra como servidor por defecto (`M/godot_physics_3d/register_types.cpp:58`), pero
  **los proyectos nuevos se crean con `physics/3d/physics_engine = Jolt Physics`** (`editor/editor_node.cpp:8521`, lista de
  "initial settings" para proyectos nuevos). Jolt queda integrado como `M/jolt_physics` (`register_types.cpp:60`) y tiene sus propios
  ajustes (velocity/position steps, enhanced internal edge removal, `M/jolt_physics/jolt_project_settings.cpp:37-41`).
- Ajustes: gravedad 9.8 (`servers/physics_3d/physics_server_3d.cpp:554`), damping lineal y angular 0.1 (`:556-557`), 60 ticks/s.

### 4.2 Cuerpos
- `StaticBody3D`, `AnimatableBody3D` (cinemático que arrastra a otros), `RigidBody3D` (mass 1, gravity_scale 1, `freeze` con modo STATIC o
  KINEMATIC, `can_sleep`, `ccd=false`, `lock_rotation`, damping COMBINE o REPLACE, contact monitor; `S3/physics/rigid_body_3d.h:42-87`),
  `CharacterBody3D`, `VehicleBody3D`, `SoftBody3D`, `PhysicalBone3D` y simulador de ragdoll (`S3/physics/`).
- `Area3D`: señales `body_entered/exited` y `*_shape_entered/exited` (`S3/physics/area_3d.cpp:205-221`); sobreescribe gravedad
  (incluida la gravedad puntual), damping y prioridad (`area_3d.h:40-60`).
- `RayCast3D`: `target_position=(0,-1,0)`, `collision_mask=1`, `exclude_parent_body`, `collide_with_areas=false`, `hit_from_inside`
  (`S3/physics/ray_cast_3d.h:51-73`). También `ShapeCast3D` y `SpringArm3D` (para cámaras en 3ª persona).
- Joints (`S3/physics/joints/`): Pin, Hinge, Slider, ConeTwist, Generic6DOF.

### 4.3 CharacterBody3D::move_and_slide, paso a paso (`S3/physics/character_body_3d.cpp`)
Valores por defecto (`character_body_3d.h:117-152`): `margin=0.001`, modo GROUNDED, `floor_stop_on_slope=true`, `floor_constant_speed=false`,
`floor_block_on_wall=true`, `slide_on_ceiling=true`, `max_slides=6`, `floor_snap_length=0.1`, `floor_max_angle=45°`, `wall_min_slide_angle=15°`,
`up_direction=(0,1,0)` y `platform_on_leave=ADD_VELOCITY`. Usa `FLOOR_ANGLE_THRESHOLD=0.01` rad (`.cpp:41`).
1. `delta` es el de física o el de idle, según desde dónde se llame (`:45`). Pone en 0 los ejes bloqueados (`:47-51`).
2. **Plataforma**: si en el cuadro anterior estaba en suelo o pared con una plataforma (y su capa no está excluida), toma la velocidad
   de la plataforma en el punto de contacto (`:58-91`) y primero se mueve con ella usando `move_and_collide` (`:100-116`).
3. Guarda `was_on_floor` y reinicia los estados (`:95-96`). En GROUNDED llama `_move_and_slide_grounded` y en FLOATING
   `_move_and_slide_floating` (`:118-122`).
4. Grounded (`:140-400`): `motion = velocity·delta`. El primer intento **no desliza** cuando `stop_on_slope` está activo (`:156`).
   El bucle corre hasta `max_slides` veces con `max_collisions=6` (`:165-167`):
   - Clasifica cada colisión con `_set_collision_direction` (`:528-614`). Es **suelo** si el ángulo con `up` es ≤ `floor_max_angle+0.01`,
     **techo** si el ángulo con `-up` cumple lo mismo y **pared** en cualquier otro caso. Varias paredes cuyas normales sumadas apuntan
     hacia arriba cuentan como suelo (`:592-611`).
   - Si es suelo con `stop_on_slope` y la velocidad apunta casi exactamente contra `up`, se queda quieto y no resbala (`:196-206`).
   - Pared con `floor_block_on_wall` y ángulo de movimiento menor a 90°: cancela el avance horizontal, hace snap al suelo, desliza solo
     de forma lateral y en una esquina contra una segunda pared se detiene (`:217-300`). Por debajo de `wall_min_slide_angle` se anula
     el movimiento horizontal (`:303-312`).
   - Deslizamiento normal: `remainder.slide(normal)`. Sobre suelo se usa la intersección del plano del movimiento con el plano del suelo
     para conservar la dirección (`:315-347`). Techo: desliza o corta la velocidad vertical (`:339-356`).
   - `constant_speed` evita que el personaje vaya más lento en las subidas (`:362-383`).
5. Al final, `_snap_on_floor` (`:394`, `:495-501`): si estaba en el suelo, no está subiendo y ahora no toca suelo, hace un
   `test_motion` hacia abajo de `max(snap_length, margin)` y baja **solo en el eje up** (`apply_floor_snap`, `:459-493`).
6. Si toca suelo y no sube, se quita la componente vertical de la velocidad, lo que "resetea" la gravedad (`:397-399`).
7. `real_velocity = Δpos/delta` (`:125`). Si acaba de dejar la plataforma, suma su velocidad (`:127-135`).
   Devuelve true si hubo alguna colisión.
Consultas: `is_on_floor/wall/ceiling(_only)`, `get_floor_normal`, `get_wall_normal`, `get_floor_angle`, `get_slide_collision(i)` (`:655-741`).
Este algoritmo vive **en la capa de escena** y solo necesita `body_test_motion` del servidor. Por eso funciona igual con GodotPhysics y con Jolt.

---

## 5. Animación
- `Animation`: pistas VALUE, POSITION_3D, ROTATION_3D, SCALE_3D, BLEND_SHAPE, METHOD, BEZIER, AUDIO y ANIMATION (`SR/animation.h:50-60`).
  Interpolación NEAREST, LINEAR, CUBIC, LINEAR_ANGLE, CUBIC_ANGLE, MAKIMA (`:63-69`). Las pistas 3D se pueden comprimir.
- `AnimationMixer` es la base de AnimationPlayer y AnimationTree (`scene/animation/animation_mixer.*`): librerías, `root_motion_track`,
  modo determinista (`animation_mixer.h:329-332`). Señales `animation_started` y `animation_finished` (`animation_mixer.cpp:2547-2548`).
- `AnimationTree` (`scene/animation/`): BlendTree con nodos Animation, OneShot, Add2/3, Blend2/3, Sub2, TimeScale, TimeSeek, Transition y Output
  (`animation_blend_tree.h:36-433`). `BlendSpace1D/2D` triangulados, con modo de blend interpolado o discreto y snap (`animation_blend_space_2d.h:43-88`).
  **StateMachine**: las transiciones tienen switch IMMEDIATE/SYNC/AT_END, advance DISABLED/ENABLED/AUTO, `advance_condition`, `advance_expression`
  y `xfade_time` (`animation_node_state_machine.h:41-63`). La reproducción se controla con `AnimationNodeStateMachinePlayback.travel()`, que
  busca un camino A* entre estados (`:234`).
- `Skeleton3D`: cada `Bone` tiene parent, rest, global_rest y pose (pos/quat/escala) con caché global (`S3/skeleton_3d.h:104-123`).
  Los modificadores corren en `_process_modifiers` (`:199`). `Skin` guarda los binds (nombre o índice de hueso + pose inversa,
  `SR/3d/skin.h:38-65`).
- **Skinning en GPU**: en RD con un compute shader (`SH/skeleton.glsl:1`) que deforma los vértices a un buffer; en GLES3 con
  transform feedback (`drivers/gles3/shaders/skeleton.glsl`).
- `BoneAttachment3D`: un Node3D que sigue a un hueso (o lo sobreescribe) (`S3/bone_attachment_3d.h:35`).
- Modificadores e IK (`SkeletonModifier3D`, `S3/skeleton_modifier_3d.h:36`): `TwoBoneIK3D`, `CCDIK3D`, `FABRIK3D`, `JacobianIK3D`,
  `SplineIK3D`, `LookAtModifier3D`, `AimModifier3D`, `SpringBoneSimulator3D` (huesos con resorte y colisiones), `RetargetModifier3D`,
  copy/convert transform, `BoneConstraint3D`. El viejo `SkeletonIK3D` sigue presente (`S3/*.h`).

---

## 6. Importación
- **glTF 2.0** (`M/gltf/gltf_document.cpp`). Orden de parseo (`:7145-7230`): buffers → bufferViews → accessors → extensiones → scenes →
  nodes → images → samplers → textures → materials → skins → `SkinTool::_determine_skeletons` (agrupa los joints en Skeleton3D) →
  nombres → meshes → lights → cameras → animations. Después, `generate_scene` (`:7253`) arma el árbol: Node3D, ImporterMeshInstance3D,
  Light3D, Camera3D, Skeleton3D, BoneAttachment3D y AnimationPlayer (`:4159-4776`, `_import_animation` `:5608`).
- Materiales: el mapeo va a **StandardMaterial3D**, con metallic en el canal **B** y roughness en el **G** de la textura metallicRoughness
  (`:3117-3119`), normal map con su escala (`:3141-3142`) y AO en el canal R (`:3161`). Se ajustan alpha mode (opaco, mask o blend), cull,
  emisión y unlit. Extensiones: KHR_lights_punctual, materials_unlit, materials_emissive_strength, texture_transform, node_visibility,
  animation_pointer, además de spec-gloss, KTX2 y WebP (`M/gltf/extensions/`).
- Opciones del importador de escena (`editor/import/3d/resource_importer_scene.cpp:2685-2721`): root_type, ensure_tangents, **generate_lods**
  (meshoptimizer), **create_shadow_meshes**, light_baking, use_named_skins, `animation/fps=30`, import_rest_as_RESET.
  Sufijos en el nombre de los nodos: `-col`, `-convcol`, `-colonly`, `-convcolonly`, `-rigid`, `-navmesh`, `-occ`, `-noimp` (`:690-975`).
- FBX: `M/fbx/fbx_document.cpp` con ufbx, que reutiliza el mismo GLTFState. OBJ: `editor/import/3d/resource_importer_obj.cpp`. También Collada.
- Texturas (`editor/import/resource_importer_texture.cpp`): modo Lossless/Lossy/VRAM Compressed/VRAM Uncompressed/Basis Universal (`:266`),
  normal map detectado automáticamente (`:276`), mipmaps encendidos en el preset 3D (`:278`) y roughness desde el canal elegido (`:282`). Si la
  textura se detecta usada en 3D, se reimporta como VRAM con mipmaps (`:126-169`). Para la compresión usa BPTC/S3TC (cvtt, betsy en GPU),
  ETC2/ASTC y Basis (`modules/`).

---

## 7. Navegación, audio 3D y entrada
- **NavigationServer3D** (`M/navigation_3d`, `servers/navigation_3d`): mapas, regiones, links y obstáculos. El bake usa **Recast**: heightfield →
  rasterizado → compact → erode por radio → regiones (watershed o monotone) → contornos → polymesh → detail
  (`M/navigation_3d/3d/nav_mesh_generator_3d.cpp:427-526`). Valores por defecto del NavigationMesh: cell 0.25×0.25, agent_height 1.5, radius 0.5,
  max_climb 0.25, max_slope 45° (`SR/navigation_mesh.h:80-89`, `servers/navigation_3d/navigation_constants_3d.h:66-81`).
- `NavigationAgent3D`: A* sobre polígonos + corridor-funnel (`S3/navigation/navigation_agent_3d.h:56-57`), `path_desired_distance=1`,
  `target_desired_distance=1`, y evasión **RVO2** opcional (2D o 3D, `nav_agent_3d.h:63-64`) con `max_speed=10`. Señales `target_reached`,
  `navigation_finished`, `velocity_computed(safe_velocity)`, etc. (`navigation_agent_3d.cpp:192-197`). El agente no mueve el cuerpo: el usuario
  lee `get_next_path_position()` y mueve su CharacterBody.
- **AudioStreamPlayer3D** (`S3/audio_stream_player_3d.*`): atenuación INVERSE (por defecto), INVERSE_SQUARE, LOG o DISABLED, con `unit_size=10`
  y `max_db=3` (`.h:51-80`). Fórmulas: `linear_to_db(1/(d/unit))`, `1/(d/unit)^2` y `-20·log(d/unit)` (`.cpp:235-260`). Filtro paso bajo
  por distancia (5000 Hz, −24 dB), cono de emisión de 45° (`.h:108-112`) y `max_distance`. **Doppler**:
  `pitch·343/(343+v·approaching)`, limitado a [1/8, 8] (`.cpp:534-539`). La cámara o un AudioListener3D hacen de oyente.
- Entrada: InputMap con acciones y deadzone de 0.2 (`core/input/input_map.h:55`). `Input.get_vector(-x,+x,-y,+y)` para mover en 3D
  (`core/input/input.cpp:586`) y `set_mouse_mode` (capturado, para FPS) (`:104`).

---

## 8. UX del editor 3D
- Herramientas: Transform (Q), Move (W), Rotate (E), Scale (R) y Select (V) (`ED3/node_3d_editor_plugin.cpp:3415-3460`). **Espacio local** con
  T (`:3543`) y **snap** con Y (`:3578`). Valores por defecto del snap: **1 m, 15° y 10%** (`:3053-3055`). Snap al suelo con PgDn (`:3691`).
- Gizmo de transformación: se pica en `_transform_gizmo_select` (`ED3/node_3d_editor_viewport.cpp:1488`) y se aplica en `update_transform`
  (`:6464`). También se pueden escribir valores numéricos (`:6763`). Para seleccionar un objeto, el rayo se consulta contra un BVH
  de gizmos y después contra los triángulos de colisión de cada gizmo (`_select_ray`, `:744-780`).
- Gizmos por tipo (`ED3/gizmos/`): cámara, luces, decal, partículas, formas de colisión, notifier, sprites, IK, etc. Las manijas de las primitivas CSG
  (radio, tamaño, altura) están en `M/csg/editor/csg_gizmos.cpp:194-206`.
- Navegación (`scene/debugger/view_3d_controller.*`): esquemas Godot, Maya, Modo, Custom y Tablet (`view_3d_controller.h:71-76`). Botón medio orbita,
  Shift pan y Ctrl zoom (`node_3d_editor_viewport.cpp:6903-6910`). **Freelook** mientras se mantiene el botón derecho, con WASD + Q/E,
  Shift para ir rápido y Alt para ir lento (`:6912-6919`, `view_3d_controller.cpp:155`). Shift+F lo alterna (`node_3d_editor_plugin.cpp:3675`). La rueda
  cambia la velocidad del freelook (`view_3d_controller.cpp:91-99`). F enfoca la selección (`node_3d_editor_viewport.cpp:5389`).
- Scene tree dock (`editor/docks/scene_tree_dock.cpp`), inspector genérico que se arma por reflexión de propiedades (`editor/inspector/editor_inspector.cpp`),
  previews de materiales y mallas (`editor/inspector/editor_preview_plugins.h:76,121`) y editor de material con una esfera o cubo
  (`editor/scene/material_editor_plugin.cpp`).
- **Bloqueo con CSG**: se arman las piezas con primitivas CSG, sustracciones para puertas y ventanas, `use_collision` para jugar enseguida y
  después "Bake Mesh/Collision" (`csg_shape.h:176-178`).
- **GridMap**: modos Transform (T), Select (Q), Erase (W), Paint (E) y Pick (R), con acciones Fill (Z) y Move (X)
  (`M/gridmap/editor/grid_map_editor_plugin.cpp:1779-1849`). Piso anterior y siguiente con 1 y 3 y eje de edición con Shift+Z/X/C
  (`:1739-1743`). Rotación del cursor en X/Y/Z (`grid_map_editor_plugin.h:205-211`). La paleta sale de la MeshLibrary.

---

## 9. Cómo lo llevaríamos a NicoStation

### 9.0 Puntos de partida y decisiones de base (propuesta)
- Hoy: el proyecto es JSON (`src/model.rs`), el runtime es un `World` sin ventana (`src/runtime.rs`), el reproductor usa macroquad/miniquad
  (también en wasm) y el editor usa eframe/egui con **wgpu** (`src/editor/vista.rs:30`). La vista previa del editor dibuja con egui.
- **Renderer**: hay que escribir uno solo y que lo usen el editor y el reproductor. Opciones:
  (A) **wgpu** (recomendado, inferido): el editor lo embebe con un `egui_wgpu` callback, el reproductor 3D pasa a winit+wgpu y en web usa
  WebGPU o el fallback WebGL2. WGSL; compute en escritorio y WebGPU. Costo: un reproductor nuevo para 3D (ventana, entrada y audio fuera de macroquad).
  (B) miniquad: 0.4.11 ya trae RGBA16F, texturas de profundidad, MRT, cubemaps e instancing (`miniquad/src/graphics.rs:209,321-379,1229`) y
  crea un contexto WebGL2 (`js/gl.js:91`). Sirve para el reproductor, pero **no se puede meter dentro del editor wgpu**.
  Sea cual sea, el modelo correcto es el **Compatibility de Godot** (forward con N luces por objeto y sin compute obligatorio), no Forward+.
- Física: **rapier3d** (mismas consultas que §4.1: `cast_ray`, `cast_shape`, `intersections_with_shape`). El `move_and_slide` de §4.3 se porta
  tal cual sobre `cast_shape`. Así lo hace Godot: el algoritmo está encima del servidor.
- Importación: crate `gltf` (con mapeo de materiales B=metal y G=rough como en §6). Navegación: grilla o navmesh simplificado, o bindings de Recast.

### 9.1 Modelo de datos propuesto (proyecto JSON)
```jsonc
"scenes": [{ "name": "Nivel1", "dim": "3d",                       // 2d por defecto, compatible con lo actual
  "environment": { "sky": {"type":"procedural","top":[98,116,140],"ground":[51,43,34]},
                   "ambient":"sky", "tonemap":"filmic", "exposure":1, "glow":true, "ssao":false,
                   "fog": {"density":0.01,"color":[132,141,155]} },
  "instances": [{ "obj":"Heroe", "pos":[0,1,0], "rot":[0,90,0], "scale":[1,1,1], "parent": null }] }],
"objects": [
  { "name":"Heroe", "type":"model", "src":"modelos/heroe.glb", "anim_default":"idle",
    "collider":{"shape":"capsule","r":0.4,"h":1.8}, "behaviors":[{"type":"character3d"}] },
  { "name":"Caja", "type":"mesh", "mesh":{"prim":"box","size":[1,1,1]},
    "material":{"albedo":[200,80,60],"roughness":0.6,"metallic":0,"albedo_tex":"tex/caja.png"},
    "body":"static" },
  { "name":"Sol", "type":"light", "kind":"directional", "energy":1, "shadows":true },
  { "name":"Camara", "type":"camera3d", "fov":75, "near":0.05, "far":4000 } ]
```
- `rot` va en grados Euler, orden YXZ como Godot, y se guarda así en el JSON. En el runtime se usa quaternion (inferido: es lo que hace Node3D, §1.1).
- `parent` (id de instancia) da jerarquía y `top_level` en el runtime. Adelante = −Z.
- Las expresiones existentes se extienden con `Heroe.z`, `Heroe.rot_y`, `distance3d(...)`.

### 9.2 Plan por prioridad (S ≈ días, M ≈ 1-2 semanas, L ≈ 3-6 semanas, XL ≈ 2+ meses)
| # | Pieza | Qué incluye (referencia en Godot) | Esfuerzo |
|---|---|---|---|
| **MVP** | | | |
| 1 | Escena 3D + transformaciones | Instancias con pos/rot/escala, padre e hijos, caché global perezosa (§1.1) | M |
| 2 | Renderer base | Mallas primitivas (box, sphere, plane, cylinder, capsule) y Lambert o Burley + GGX, 1 direccional + 8 omni/spot por objeto (como `max_lights_per_object=8`, §3.5), color y textura de albedo, RGBA8 con tonemap en el shader | L |
| 3 | Cámara 3D + eventos | Perspectiva u ortográfica, seguir objeto, 3ª persona con brazo (como SpringArm3D) | M |
| 4 | Física y "Personaje 3D" | rapier3d: estático, rígido y cinemático; formas box, sphere, capsule, cylinder y trimesh; **move_and_slide** portado (§4.3) con los mismos valores por defecto | L |
| 5 | Importar glTF | Mallas, materiales PBR, jerarquía, texturas y animaciones de nodos | M |
| 6 | Editor 3D mínimo | Viewport wgpu, orbitar, pan y zoom + freelook con clic derecho y WASD, gizmo de mover, rotar y escalar con snap de 1 m, 15° y 10% y espacio local con T (§8), picking con rayos | L |
| **Calidad "Godot"** | | | |
| 7 | PBR completo | metallic/roughness/normal/emission/AO, ORM, alpha (scissor, blend, hash), cull, unshaded, billboard (§2.2) | M |
| 8 | Sombras | PSSM de 2 a 4 cascadas en un atlas, PCF con disco rotado, bias y normal bias (§3.4), sombras de spot y omni (cubemap) | L |
| 9 | Cielo + IBL | Cielo procedural (con los valores del editor, §1.3), cubemap pre-filtrado por roughness, LUT DFG y luz ambiente desde el cielo (§3.4) | M |
| 10 | HDR + post | RGBA16F, glow con cadena de mips (niveles 0.8, 0.4 y 0.1), tonemap Filmic/ACES/AgX tomando las fórmulas de `tonemap.glsl`, FXAA, niebla exponencial | M |
| 11 | Animación esquelética | Skin, Skeleton, skinning en el vertex shader (no hace falta compute), mezcla con crossfade y "animación terminó" | L |
| 12 | SSAO | Media resolución, kernel de hemisferio + blur (GLES3 también lo tiene, §3.5) | M |
| **Hacia paridad** | | | |
| 13 | Partículas 3D en CPU | Equivalente a CPUParticles3D con billboards, por instancing (§1.2) | M |
| 14 | Sprite3D, Label3D y decorado | Billboards con pixel_size y texto en 3D | S-M |
| 15 | Bloques (CSG ligero) | Box, cilindro y rampa con unión y resta usando una crate de booleanas (o solo unión, sin booleanas) | M (XL con booleanas robustas) |
| 16 | GridMap / "tilemap 3D" | Celdas int16 con item y una de las 24 rotaciones, MeshLibrary, dibujo instanciado, pintar, borrar, elegir y rellenar por pisos (§1.2, §8) | L |
| 17 | Máquina de estados de animación | Estados, transiciones por condición o al terminar, blend 1D por velocidad (§5) | M |
| 18 | Navegación | Navmesh horneado (Recast) o grilla + A* con funnel; agente con `target_reached` (§7) | L |
| 19 | Audio 3D | Atenuación inversa con unit_size 10, paneo por dirección y doppler opcional (§7) | S |
| 20 | Path3D | Curvas, "seguir camino" y rotación orientada | S-M |
| 21 | Lightmaps | Bake en CPU con UV2 (xatlas); da GI estática barata | XL |
| 22 | Shaders de usuario | Un **subconjunto** de spatial (fragment con ALBEDO, ROUGHNESS, METALLIC, EMISSION, NORMAL_MAP, ALPHA, más TIME y UV) traducido a WGSL/GLSL, o efectos predefinidos con parámetros | L (subconjunto) |

**Lo que no se puede igualar (o no vale la pena) y su alternativa:**
- **SDFGI y VoxelGI** (compute pesado, solo Forward+): ambiente desde el cielo + SSAO + reflection probes estáticos + lightmaps (#21).
  Con eso se logra la mayor parte de lo que se ve.
- **Iluminación clustered** (cluster builder con raster en GPU, §3.4): forward con luces por objeto elegidas en CPU, como Mobile y Compatibility.
- **Lenguaje de shaders completo** (parser de más de 11k líneas, §2.3): subconjunto o materiales parametrizados.
- **SSR, SSIL, niebla volumétrica, TAA y FSR**: posponer. FXAA y la niebla de profundidad o exponencial cubren lo básico.
- **GPUParticles** con colisión SDF y atractores: partículas en CPU con colisión contra el plano del suelo.
- **IK completo** (CCD, FABRIK, Jacobian, Spline): solo TwoBoneIK y "mirar hacia" (LookAt) al principio.
- **Jolt-level física** (soft bodies, vehículos): rapier cubre rígidos, joints y personajes.

### 9.3 Hoja de eventos 3D (propuesta de catálogo; los `op` siguen el estilo de `src/catalog.rs`)
**Acciones**
| op | Frase |
|---|---|
| `move_forward3d` | Mover _0_ hacia adelante _1_ m/s (eje −Z local) |
| `move_local3d` | Mover _0_ en su espacio local (_1_, _2_, _3_) |
| `set_pos3d` / `add_pos3d` | Poner o sumar la posición de _0_ a (_1_, _2_, _3_) |
| `rotate3d` | Rotar _0_ _1_° por segundo en el eje _2_ (X/Y/Z, local o global) |
| `look_at3d` | Hacer que _0_ mire hacia _1_ (usa `Basis::looking_at` y conserva la escala, §1.1) |
| `rotate_toward3d` | Rotar _0_ hacia _1_ como máximo _2_°/s (solo yaw o completo) |
| `apply_impulse3d` / `set_velocity3d` | Aplicar un impulso o fijar la velocidad del cuerpo rígido _0_ |
| `play_anim3d` | Reproducir la animación _1_ en _0_ con mezcla de _2_ s (y repetir sí/no) |
| `set_anim_state` | Ir al estado _1_ de la máquina de estados de _0_ (`travel`, §5) |
| `camera_follow3d` | La cámara sigue a _0_ en 3ª persona a una distancia de _1_ y una altura de _2_, con colisión (SpringArm) |
| `set_env` | Cambiar el ambiente: niebla, exposición o color del cielo |
| `nav_go_to` | Ir con _0_ hasta (_1_, _2_, _3_) por el navmesh |
| `spawn3d` | Crear _0_ en (_1_, _2_, _3_) |

**Condiciones**
| op | Frase |
|---|---|
| `raycast_hits` | Un rayo desde _0_ hacia adelante de _1_ m golpea a _2_ (elige la instancia golpeada; deja `hit_x/y/z`, `hit_normal`, `hit_distance`) |
| `mouse_ray_hits` | El clic del mouse toca a _0_ (`project_ray_origin/normal`, §1.2) |
| `collides3d` | _0_ choca con _1_ (contactos del motor de física) |
| `in_area3d` | _0_ está dentro del área _1_ (entró o salió en este cuadro, como las señales de Area3D, §4.2) |
| `on_floor3d` / `on_wall3d` / `on_ceiling3d` | _0_ está en el suelo, la pared o el techo (directo de `is_on_floor()` y similares, §4.3) |
| `anim_finished3d` | Terminó la animación _1_ de _0_ (`animation_finished`, §5) |
| `on_screen3d` | _0_ está visible en pantalla (VisibleOnScreenNotifier3D) |
| `nav_arrived` | _0_ llegó a su destino (`target_reached`) |
| `distance3d` | _0_ está a menos de _2_ m de _1_ |

**Comportamiento `character3d`** (el equivalente 3D del `platformer` actual):
- Parámetros: `max_speed` 5 m/s, `acceleration` 30, `deceleration` 40, `gravity` 9.8 (o la del proyecto), `jump_speed` 4.5,
  `jump_sustain_time`, `max_falling_speed` 50, `slope_max_angle` 45, `snap_length` 0.1, `max_slides` 6, `stop_on_slope` true,
  `camera_relative` true y `turn_speed` 720 °/s. Los cinco de `move_and_slide` (slope 45, snap 0.1, slides 6, stop_on_slope y la constante de ángulo) salen
  de los valores por defecto de Godot (§4.3). Los demás son sugerencias.
- Cada paso: (1) arma la intención con `get_vector`, con WASD o flechas relativas al yaw de la cámara; (2) acelera o frena en el plano horizontal;
  (3) suma gravedad si no está en el suelo y salta si está en el suelo y se pulsa la tecla; (4) **move_and_slide** (§4.3, pasos 1-7); (5) rota la malla hacia la
  dirección en que se mueve; (6) expone `on_floor`, `jumping`, `falling`, `speed` para los eventos y para elegir la animación
  (idle/run/jump/fall) con una receta.
- Plataformas móviles: con `platform_on_leave=ADD_VELOCITY` se hereda la velocidad de la plataforma (§4.3 paso 2 y 7). Es el mismo concepto
  que ya existe en 2D, donde `floor: Option<(u64,f32,f32)>` está en `src/runtime.rs:66`.
- Variantes: `top_down3d`, como el TopDown actual pero en el plano XZ, y `fps3d`, con mouse capturado, yaw en el cuerpo y pitch en la cámara.
