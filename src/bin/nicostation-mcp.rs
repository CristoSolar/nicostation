//! Servidor MCP (stdio, JSON-RPC 2.0) para que un agente cree juegos NicoStation completos:
//! proyecto, escenas, objetos, imágenes, sonidos, eventos, validación, simulación y exportación.
//! Registro: `claude mcp add nicostation -- /ruta/nicostation-mcp`.

use nicostation::model::*;
use nicostation::nodos::Graph;
use nicostation::runtime::{Input, World};
use nicostation::{catalog, export, validate};
use serde::de::DeserializeOwned;
use serde_json::{json, Value as J};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

const INSTRUCCIONES: &str = "NicoStation es un motor 2D que se programa con hojas de eventos (condición -> acción), \
como GDevelop. Flujo recomendado para crear un juego: 1) catalogo (lee el formato y las instrucciones válidas), \
2) proyecto_crear, 3) imagen_crear / sonido_crear para los recursos, 4) objeto_poner, 5) instancias_agregar, \
6) eventos_poner (y funcion_poner si conviene), 7) validar hasta que no haya problemas, \
8) simular con teclas programadas para comprobar que el juego se comporta como esperas, 9) jugar o exportar_web. \
Todas las rutas de imágenes y sonidos son relativas a la carpeta del proyecto.";

fn main() {
    let mut srv = Srv { cur: None };
    let mut out = std::io::stdout().lock();
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let msg: J = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(e) => {
                let r = json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": e.to_string()}});
                let _ = writeln!(out, "{r}");
                continue;
            }
        };
        let id = msg.get("id").cloned();
        let method = msg["method"].as_str().unwrap_or("");
        let result: Result<J, (i64, String)> = match method {
            "initialize" => Ok(json!({
                "protocolVersion": msg["params"]["protocolVersion"].as_str().unwrap_or("2025-06-18"),
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "nicostation", "version": env!("CARGO_PKG_VERSION")},
                "instructions": INSTRUCCIONES,
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": tools()})),
            "tools/call" => {
                let name = msg["params"]["name"].as_str().unwrap_or("");
                let args = msg["params"].get("arguments").cloned().unwrap_or(json!({}));
                Ok(match srv.call(name, &args) {
                    Ok(text) => json!({"content": [{"type": "text", "text": text}]}),
                    Err(e) => json!({"content": [{"type": "text", "text": e}], "isError": true}),
                })
            }
            _ if id.is_none() => continue, // notificaciones (initialized, cancelled...)
            _ => Err((-32601, format!("método desconocido: {method}"))),
        };
        let Some(id) = id else { continue };
        let r = match result {
            Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
            Err((code, m)) => json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": m}}),
        };
        let _ = writeln!(out, "{r}");
        let _ = out.flush();
    }
}

// ---------- definición de herramientas ----------

fn t(name: &str, desc: &str, props: J, req: &[&str]) -> J {
    let mut props = props;
    props["proyecto"] = json!({"type": "string", "description": "Ruta del proyecto .json (opcional si ya abriste uno)"});
    json!({"name": name, "description": desc, "inputSchema": {"type": "object", "properties": props, "required": req}})
}

fn tools() -> Vec<J> {
    let s = |d: &str| json!({"type": "string", "description": d});
    let n = |d: &str| json!({"type": "number", "description": d});
    let o = |d: &str| json!({"type": "object", "description": d});
    let a = |d: &str| json!({"type": "array", "description": d});
    let target = json!({"escena": s("Nombre de la escena"), "funcion": s("Nombre de la función (en vez de escena)")});
    let mut ev_props = target.clone();
    ev_props["eventos"] = a("Lista de eventos (formato en catalogo)");
    ev_props["modo"] = json!({"type": "string", "enum": ["agregar", "reemplazar"], "description": "agregar (por defecto) o reemplazar toda la hoja"});
    let mut ev_del = target.clone();
    ev_del["ruta"] = s("Número del evento como lo muestra eventos_ver, p. ej. \"3\" o \"2.1\"");
    vec![
        t("catalogo", "Formato del proyecto, tipos de objeto, comportamientos con sus valores por defecto, todas las condiciones y acciones con sus parámetros, expresiones y teclas. Léelo antes de crear un juego.", json!({}), &[]),
        t("proyecto_crear", "Crea un proyecto nuevo (carpeta + juego .json) con una escena vacía y lo deja abierto.",
            json!({"ruta": s("Ruta del archivo .json a crear, p. ej. /home/u/juegos/mi-juego/juego.json"), "nombre": s("Nombre del juego"),
                   "ancho": n("Ancho en px (800)"), "alto": n("Alto en px (600)"), "escena": s("Nombre de la primera escena (Principal)"),
                   "sobrescribir": json!({"type": "boolean", "description": "Reemplazar si ya existe"}),
                   "plantilla": s("Opcional: empezar desde un juego completo (ver herramienta plantillas). La ruta debe estar en una carpeta nueva.")}), &["ruta", "nombre"]),
        t("plantillas", "Juegos completos listos para modificar (plataformas, vista de arriba, naves, atrapar fruta).", json!({}), &[]),
        t("proyecto_abrir", "Abre un proyecto existente y devuelve su resumen.", json!({"ruta": s("Ruta del .json")}), &["ruta"]),
        t("proyecto_resumen", "Resumen del proyecto abierto: escenas, objetos, comportamientos, instancias, eventos y problemas.", json!({}), &[]),
        t("proyecto_json", "JSON completo del proyecto, o solo de una escena.", json!({"escena": s("Opcional: solo esta escena")}), &[]),
        t("proyecto_configurar", "Cambia nombre, tamaño, escena inicial o variables globales.",
            json!({"nombre": s("Nombre"), "ancho": n("Ancho px"), "alto": n("Alto px"), "escena_inicial": s("Escena con la que arranca"),
                   "globales": o("Variables globales a poner (número, texto o booleano); se combinan con las existentes")}), &[]),
        t("escena_crear", "Crea una escena (2D, o 3D con dim3=true).", json!({"nombre": s("Nombre"), "fondo": a("Color [r,g,b]"),
            "dim3": json!({"type": "boolean", "description": "Escena 3D (se juega con nicostation3d)"}),
            "entorno": o("Entorno 3D: cielo, luz ambiente, niebla, glow, ssao, tonemap, exposure (ver catalogo)"),
            "capas": a("Capas [{name, visible, fixed}]; \"\" es la base, fixed=true no sigue a la cámara (HUD)")}), &["nombre"]),
        t("escena_editar", "Cambia nombre, fondo, capas, variables o grupos de una escena. Renombrar actualiza las referencias.",
            json!({"escena": s("Escena"), "nuevo_nombre": s("Nuevo nombre"), "fondo": a("[r,g,b]"), "capas": a("Reemplaza las capas"),
                   "variables": o("Variables de escena a poner (se combinan)"), "grupos": o("Grupos {nombre: [objetos]} a poner (se combinan)"),
                   "dim3": json!({"type": "boolean", "description": "Convertir en escena 3D (true) o 2D (false)"}),
                   "entorno": o("Entorno 3D (reemplaza el actual)")}), &["escena"]),
        t("escena_borrar", "Borra una escena.", json!({"escena": s("Escena")}), &["escena"]),
        t("objeto_poner", "Crea o reemplaza (por nombre) un objeto: sprite, text o box, con comportamientos y variables. Sin escena = objeto global.",
            json!({"escena": s("Escena (omitir = global)"), "objeto": o("Objeto, p. ej. {\"name\":\"Heroe\",\"type\":\"sprite\",\"animations\":[{\"name\":\"quieto\",\"frames\":[\"img/heroe.png\"]}],\"behaviors\":[{\"type\":\"platformer\"}]}")}), &["objeto"]),
        t("objeto_borrar", "Borra un objeto, sus instancias y su presencia en grupos.", json!({"escena": s("Escena (omitir = global)"), "nombre": s("Objeto")}), &["nombre"]),
        t("mapa_pintar", "Pinta un mapa de baldosas (objeto type=tilemap) con filas de texto: cada carácter es una celda según la leyenda; '.' o ' ' = vacía.",
            json!({"escena": s("Escena del objeto (omitir = global)"), "objeto": s("Nombre del objeto tilemap"),
                   "filas": a("Filas de texto, p. ej. [\"..........\", \"####..####\"]"),
                   "leyenda": o("Carácter -> índice de baldosa en el tileset, p. ej. {\"#\": 0, \"=\": 4}"),
                   "columna": n("Columna donde empieza (0)"), "fila": n("Fila donde empieza (0)"),
                   "agrandar": json!({"type": "boolean", "description": "Agrandar el mapa si las filas no caben (por defecto true)"})}), &["objeto", "filas", "leyenda"]),
        t("instancias_agregar", "Coloca instancias de objetos en una escena.",
            json!({"escena": s("Escena"), "instancias": a("[{obj, x, y, layer?, angle?, z?, w?, h?, vars?}]. En 2D x,y = esquina superior izquierda; en escenas 3D x,y,z = centro (Y arriba, metros) y se agregan pitch?, roll?, d?")}), &["escena", "instancias"]),
        t("instancias_quitar", "Quita instancias de una escena: las de un objeto, o todas.",
            json!({"escena": s("Escena"), "objeto": s("Solo las de este objeto (omitir = todas)")}), &["escena"]),
        t("eventos_poner", "Agrega eventos a la hoja de una escena o función (o la reemplaza).", ev_props, &["eventos"]),
        t("eventos_ver", "Muestra la hoja de eventos numerada y en lenguaje natural.", target, &[]),
        t("eventos_borrar", "Borra un evento (con sus sub-eventos) por su número.", ev_del, &["ruta"]),
        t("grafo_poner", "Crea o reemplaza (por nombre) un grafo de nodos en una escena, o el grafo de una función. Formato en catalogo (sección Grafos). Devuelve los eventos que genera.",
            json!({"escena": s("Escena"), "funcion": s("Función (en vez de escena)"), "grafo": o("{name, nodes: [...], flow: [...], wires: [...]}"),
                   "desde_hoja": json!({"type": "boolean", "description": "En vez de «grafo», convertir la hoja de eventos actual en un grafo (la hoja queda vacía)"})}), &[]),
        t("grafo_ver", "Muestra un grafo de nodos (nodos, flechas y cables) y lo que compila: eventos generados o errores por nodo.",
            json!({"escena": s("Escena"), "funcion": s("Función (en vez de escena)"), "nombre": s("Nombre del grafo (omitir = todos los de la escena)")}), &[]),
        t("recetas", "Lista de recetas: trozos de juego listos (moverse con flechas, recoger y sumar puntos, perder al tocar un enemigo, disparar...) con los huecos que piden.", json!({}), &[]),
        t("receta_usar", "Agrega a una escena los eventos de una receta, completando sus huecos en orden. Crea las variables que la receta necesita.",
            json!({"escena": s("Escena"), "receta": s("Nombre exacto de la receta"), "respuestas": a("Respuestas a los huecos, en orden (nombres de objetos, escena, tecla o números)")}), &["escena", "receta"]),
        t("funcion_poner", "Crea o reemplaza una función propia hecha con eventos (acción, o condición con is_condition=true y la acción return).",
            json!({"funcion": o("{name, is_condition, params:[nombres], events:[...]}")}), &["funcion"]),
        t("imagen_crear", "Dibuja una imagen PNG dentro de la carpeta del proyecto, con pixel art o formas.",
            json!({"ruta": s("Ruta relativa, p. ej. img/heroe.png"), "ancho": n("Ancho px (con formas)"), "alto": n("Alto px (con formas)"),
                   "fondo": a("Color de fondo [r,g,b,a] (transparente por defecto)"),
                   "pixel_art": o("{paleta: {\"X\": [r,g,b] o [r,g,b,a]}, filas: [\"..XX..\", ...], escala: 4}. Caracteres sin color = transparente"),
                   "formas": a("[{tipo: rect|elipse|linea, x, y, w, h, color: [r,g,b,a?], grosor?}] (linea va de x,y a x+w,y+h)")}), &["ruta"]),
        t("sonido_crear", "Genera un efecto de sonido WAV dentro de la carpeta del proyecto.",
            json!({"ruta": s("Ruta relativa, p. ej. snd/salto.wav"), "notas": a("[{hz, seg}] una tras otra (hz 0 = silencio)"),
                   "onda": json!({"type": "string", "enum": ["seno", "cuadrada", "triangulo", "sierra", "ruido"]}), "volumen": n("0 a 1 (0.5)")}), &["ruta", "notas"]),
        t("validar", "Revisa el proyecto: referencias rotas, instrucciones mal formadas, expresiones con error, archivos faltantes.", json!({}), &[]),
        t("simular", "Ejecuta el juego sin ventana con teclas programadas y devuelve el estado: escena, variables, instancias, posiciones a lo largo del tiempo, sonidos y avisos del motor.",
            json!({"segundos": n("Duración simulada (máx. 600)"), "escena": s("Escena inicial (por defecto la del proyecto)"),
                   "entradas": a("[{desde, hasta, teclas: [\"Right\",\"Space\"], mouse?: [x,y], clic?: true}] en segundos"),
                   "seguir": a("Objetos cuya posición se muestrea"), "muestreo": n("Cada cuántos segundos muestrear (0.5)")}), &["segundos"]),
        t("jugar", "Abre el juego en una ventana para que la persona lo pruebe.", json!({}), &[]),
        t("exportar_web", "Exporta el juego como página web (WebAssembly) lista para subir a un hosting estático.",
            json!({"carpeta": s("Carpeta de salida (por defecto <carpeta del proyecto>/web)")}), &[]),
    ]
}

// ---------- ejecución ----------

struct Srv {
    cur: Option<PathBuf>,
}

fn arg<T: DeserializeOwned>(a: &J, k: &str) -> Result<T, String> {
    serde_json::from_value(a.get(k).cloned().unwrap_or(J::Null)).map_err(|e| format!("«{k}»: {e}"))
}

fn opt<T: DeserializeOwned>(a: &J, k: &str) -> Result<Option<T>, String> {
    match a.get(k) {
        None | Some(J::Null) => Ok(None),
        Some(v) => serde_json::from_value(v.clone()).map(Some).map_err(|e| format!("«{k}»: {e}")),
    }
}

fn load(path: &Path) -> Result<Project, String> {
    let s = std::fs::read_to_string(path).map_err(|e| format!("no pude leer {}: {e}", path.display()))?;
    serde_json::from_str(&s).map_err(|e| format!("{}: {e}", path.display()))
}

fn save(path: &Path, p: &Project) -> Result<(), String> {
    let s = serde_json::to_string_pretty(p).map_err(|e| e.to_string())?;
    std::fs::write(path, s + "\n").map_err(|e| format!("no pude guardar {}: {e}", path.display()))
}

/// Ruta relativa segura dentro de la carpeta del proyecto.
fn inside(base: &Path, rel: &str) -> Result<PathBuf, String> {
    if rel.is_empty() || Path::new(rel).is_absolute() || rel.split(['/', '\\']).any(|c| c == "..") {
        return Err(format!("«{rel}» debe ser una ruta relativa dentro de la carpeta del proyecto (sin .. ni ruta absoluta)"));
    }
    Ok(base.join(rel))
}

fn scene_mut<'a>(p: &'a mut Project, name: &str) -> Result<&'a mut Scene, String> {
    let names: Vec<String> = p.scenes.iter().map(|s| s.name.clone()).collect();
    p.scenes.iter_mut().find(|s| s.name == name).ok_or(format!("no existe la escena «{name}»; hay: {names:?}"))
}

fn events_mut<'a>(p: &'a mut Project, a: &J) -> Result<&'a mut Vec<Event>, String> {
    if let Some(f) = a["funcion"].as_str() {
        let names: Vec<String> = p.functions.iter().map(|f| f.name.clone()).collect();
        return p.functions.iter_mut().find(|x| x.name == f).map(|x| &mut x.events).ok_or(format!("no existe la función «{f}»; hay: {names:?}"));
    }
    let s = a["escena"].as_str().ok_or("indica «escena» o «funcion»")?;
    Ok(&mut scene_mut(p, s)?.events)
}

impl Srv {
    fn path(&self, a: &J) -> Result<PathBuf, String> {
        match a["proyecto"].as_str() {
            Some(p) => Ok(PathBuf::from(p)),
            None => self.cur.clone().ok_or("no hay proyecto abierto: usa proyecto_crear o proyecto_abrir".into()),
        }
    }

    /// Carga, aplica un cambio, guarda y responde con el resultado más los problemas de validación.
    fn edit(&mut self, a: &J, f: impl FnOnce(&mut Project, &Path) -> Result<String, String>) -> Result<String, String> {
        let path = self.path(a)?;
        let mut p = load(&path)?;
        let base = base_dir(&path);
        let msg = f(&mut p, &base)?;
        save(&path, &p)?;
        self.cur = Some(path);
        Ok(format!("{msg}{}", problems(&p, &base)))
    }

    fn call(&mut self, name: &str, a: &J) -> Result<String, String> {
        match name {
            "catalogo" => Ok(catalogo()),
            "plantillas" => Ok(nicostation::plantillas::lista().iter().map(|t| format!("- {}: «{}»", t.carpeta, t.nombre)).collect::<Vec<_>>().join("\n")),
            "proyecto_crear" if a.get("plantilla").is_some_and(|v| v.is_string()) => {
                let path = PathBuf::from(arg::<String>(a, "ruta")?);
                let json = nicostation::plantillas::crear(&arg::<String>(a, "plantilla")?, &base_dir(&path))?;
                if json != path {
                    std::fs::rename(&json, &path).map_err(|e| e.to_string())?;
                }
                let mut p = load(&path)?;
                p.name = arg(a, "nombre")?;
                save(&path, &p)?;
                self.cur = Some(path.clone());
                Ok(format!("Proyecto creado desde la plantilla.\n{}", resumen(&p, &path)))
            }
            "proyecto_crear" => {
                let path = PathBuf::from(arg::<String>(a, "ruta")?);
                if path.exists() && a["sobrescribir"].as_bool() != Some(true) {
                    return Err(format!("{} ya existe; usa proyecto_abrir o sobrescribir=true", path.display()));
                }
                if let Some(d) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
                    std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
                }
                let scene = opt::<String>(a, "escena")?.unwrap_or("Principal".into());
                let p = Project {
                    name: arg(a, "nombre")?,
                    width: opt(a, "ancho")?.unwrap_or(800.),
                    height: opt(a, "alto")?.unwrap_or(600.),
                    first_scene: scene.clone(),
                    globals: Vars::new(),
                    objects: vec![],
                    scenes: vec![serde_json::from_value(json!({"name": scene})).map_err(|e| e.to_string())?],
                    functions: vec![],
                    plantilla: String::new(),
                };
                save(&path, &p)?;
                self.cur = Some(path.clone());
                Ok(format!("Proyecto creado en {} con la escena «{scene}». Las rutas de recursos son relativas a {}.", path.display(), base_dir(&path).display()))
            }
            "proyecto_abrir" => {
                let path = PathBuf::from(arg::<String>(a, "ruta")?);
                load(&path)?;
                self.cur = Some(path);
                self.call("proyecto_resumen", &json!({}))
            }
            "proyecto_resumen" => {
                let path = self.path(a)?;
                let p = load(&path)?;
                Ok(format!("{}{}", resumen(&p, &path), problems(&p, &base_dir(&path))))
            }
            "proyecto_json" => {
                let p = load(&self.path(a)?)?;
                let v = match a["escena"].as_str() {
                    Some(s) => serde_json::to_value(p.scene(s).ok_or(format!("no existe la escena «{s}»"))?),
                    None => serde_json::to_value(&p),
                };
                serde_json::to_string_pretty(&v.map_err(|e| e.to_string())?).map_err(|e| e.to_string())
            }
            "proyecto_configurar" => self.edit(a, |p, _| {
                if let Some(v) = opt(a, "nombre")? { p.name = v }
                if let Some(v) = opt(a, "ancho")? { p.width = v }
                if let Some(v) = opt(a, "alto")? { p.height = v }
                if let Some(v) = opt::<String>(a, "escena_inicial")? {
                    if p.scene(&v).is_none() {
                        return Err(format!("no existe la escena «{v}»"));
                    }
                    p.first_scene = v;
                }
                if let Some(g) = opt::<Vars>(a, "globales")? { p.globals.extend(g) }
                Ok("Proyecto actualizado.".into())
            }),
            "escena_crear" => self.edit(a, |p, _| {
                let name: String = arg(a, "nombre")?;
                if p.scene(&name).is_some() {
                    return Err(format!("ya existe la escena «{name}»"));
                }
                let mut s: Scene = serde_json::from_value(json!({"name": name})).map_err(|e| e.to_string())?;
                if let Some(c) = opt(a, "fondo")? { s.background = c }
                if let Some(l) = opt(a, "capas")? { s.layers = l }
                if let Some(d) = opt(a, "dim3")? { s.dim3 = d }
                if let Some(e) = opt(a, "entorno")? { s.environment = Some(e) }
                if s.dim3 && s.environment.is_none() { s.environment = Some(Environment::default()) }
                p.scenes.push(s);
                Ok(format!("Escena «{name}» creada."))
            }),
            "escena_editar" => self.edit(a, |p, _| {
                let name: String = arg(a, "escena")?;
                let new = opt::<String>(a, "nuevo_nombre")?;
                if new.as_ref().is_some_and(|n| n != &name && p.scene(n).is_some()) {
                    return Err("ya existe una escena con ese nombre".into());
                }
                let s = scene_mut(p, &name)?;
                if let Some(c) = opt(a, "fondo")? { s.background = c }
                if let Some(l) = opt(a, "capas")? { s.layers = l }
                if let Some(v) = opt::<Vars>(a, "variables")? { s.vars.extend(v) }
                if let Some(g) = opt::<BTreeMap<String, Vec<String>>>(a, "grupos")? { s.groups.extend(g) }
                if let Some(d) = opt(a, "dim3")? { s.dim3 = d }
                if let Some(e) = opt(a, "entorno")? { s.environment = Some(e) }
                if s.dim3 && s.environment.is_none() { s.environment = Some(Environment::default()) }
                if let Some(n) = new {
                    s.name = n.clone();
                    if p.first_scene == name { p.first_scene = n.clone() }
                    rename_scene_refs(p, &name, &n);
                }
                Ok(format!("Escena «{name}» actualizada."))
            }),
            "escena_borrar" => self.edit(a, |p, _| {
                let name: String = arg(a, "escena")?;
                if p.scenes.len() == 1 {
                    return Err("no puedes borrar la única escena".into());
                }
                let k = p.scenes.iter().position(|s| s.name == name).ok_or(format!("no existe la escena «{name}»"))?;
                p.scenes.remove(k);
                if p.first_scene == name { p.first_scene = p.scenes[0].name.clone() }
                Ok(format!("Escena «{name}» borrada."))
            }),
            "objeto_poner" => self.edit(a, |p, _| {
                let o: ObjectDef = arg(a, "objeto")?;
                let name = o.name.clone();
                let list = match a["escena"].as_str() {
                    Some(s) => &mut scene_mut(p, s)?.objects,
                    None => &mut p.objects,
                };
                let msg = match list.iter_mut().find(|x| x.name == name) {
                    Some(x) => { *x = o; format!("Objeto «{name}» reemplazado.") }
                    None => { list.push(o); format!("Objeto «{name}» creado.") }
                };
                Ok(msg)
            }),
            "objeto_borrar" => self.edit(a, |p, _| {
                let name: String = arg(a, "nombre")?;
                let list = match a["escena"].as_str() {
                    Some(s) => &mut scene_mut(p, s)?.objects,
                    None => &mut p.objects,
                };
                let before = list.len();
                list.retain(|o| o.name != name);
                if list.len() == before {
                    return Err(format!("no existe el objeto «{name}» ahí"));
                }
                for s in &mut p.scenes {
                    s.instances.retain(|i| i.obj != name);
                    s.groups.values_mut().for_each(|g| g.retain(|m| m != &name));
                }
                Ok(format!("Objeto «{name}» borrado con sus instancias. Revisa los eventos que lo usaban."))
            }),
            "mapa_pintar" => self.edit(a, |p, _| {
                let name: String = arg(a, "objeto")?;
                let filas: Vec<String> = arg(a, "filas")?;
                let leyenda: BTreeMap<String, i32> = arg(a, "leyenda")?;
                let (c0, r0): (usize, usize) = (opt(a, "columna")?.unwrap_or(0), opt(a, "fila")?.unwrap_or(0));
                let grow = a["agrandar"].as_bool() != Some(false);
                let list = match a["escena"].as_str() {
                    Some(s) => &mut scene_mut(p, s)?.objects,
                    None => &mut p.objects,
                };
                let o = list.iter_mut().find(|o| o.name == name).ok_or(format!("no existe el objeto «{name}»"))?;
                let Kind::Tilemap { cols, rows, data, .. } = &mut o.kind else {
                    return Err(format!("«{name}» no es un mapa de baldosas (type=tilemap)"));
                };
                let need_c = c0 + filas.iter().map(|f| f.chars().count()).max().unwrap_or(0);
                let need_r = r0 + filas.len();
                let (old_c, old_r) = (*cols, *rows);
                data.resize(old_c * old_r, -1);
                if grow && (need_c > old_c || need_r > old_r) {
                    let (nc, nr) = (need_c.max(old_c), need_r.max(old_r));
                    let mut nd = vec![-1; nc * nr];
                    for r in 0..old_r {
                        nd[r * nc..r * nc + old_c].copy_from_slice(&data[r * old_c..(r + 1) * old_c]);
                    }
                    (*cols, *rows, *data) = (nc, nr, nd);
                }
                let mut unknown = BTreeSet::new();
                let mut painted = 0;
                for (dr, f) in filas.iter().enumerate() {
                    for (dc, ch) in f.chars().enumerate() {
                        let (c, r) = (c0 + dc, r0 + dr);
                        if c >= *cols || r >= *rows {
                            continue;
                        }
                        let v = match leyenda.get(&ch.to_string()) {
                            Some(v) => *v,
                            None if ch == '.' || ch == ' ' => -1,
                            None => {
                                unknown.insert(ch);
                                continue;
                            }
                        };
                        data[r * *cols + c] = v;
                        painted += 1;
                    }
                }
                let mut msg = format!("Mapa «{name}» pintado: {painted} celdas; ahora mide {}x{} baldosas.", cols, rows);
                if !unknown.is_empty() {
                    msg += &format!(" Caracteres sin leyenda (ignorados): {unknown:?}.");
                }
                Ok(msg)
            }),
            "instancias_agregar" => self.edit(a, |p, _| {
                let list: Vec<Instance> = arg(a, "instancias")?;
                let s = scene_mut(p, &arg::<String>(a, "escena")?)?;
                let n = list.len();
                s.instances.extend(list);
                Ok(format!("{n} instancias agregadas ({} en total en la escena).", s.instances.len()))
            }),
            "instancias_quitar" => self.edit(a, |p, _| {
                let s = scene_mut(p, &arg::<String>(a, "escena")?)?;
                let before = s.instances.len();
                match a["objeto"].as_str() {
                    Some(o) => s.instances.retain(|i| i.obj != o),
                    None => s.instances.clear(),
                }
                Ok(format!("{} instancias quitadas.", before - s.instances.len()))
            }),
            "eventos_poner" => self.edit(a, |p, _| {
                let evs: Vec<Event> = arg(a, "eventos")?;
                let n = evs.len();
                let list = events_mut(p, a)?;
                if a["modo"].as_str() == Some("reemplazar") {
                    *list = evs;
                } else {
                    list.extend(evs);
                }
                Ok(format!("{n} eventos puestos; la hoja tiene {} eventos de primer nivel.", list.len()))
            }),
            "eventos_ver" => {
                let mut p = load(&self.path(a)?)?;
                let list = events_mut(&mut p, a)?;
                let mut out = String::new();
                sheet(list, "", 0, &mut out);
                Ok(if out.is_empty() { "La hoja está vacía.".into() } else { out })
            }
            "grafo_poner" => self.edit(a, |p, _| {
                let desde = a["desde_hoja"].as_bool() == Some(true);
                let mut g: Option<Graph> = opt(a, "grafo")?;
                let list = events_mut(p, a)?;
                if desde {
                    let name = g.as_ref().map_or("Hoja convertida".to_string(), |g| g.name.clone());
                    g = Some(nicostation::nodos::from_events(&name, list));
                    list.clear();
                }
                let g = g.ok_or("falta «grafo» (o desde_hoja=true)")?;
                let res = grafo_texto(&g, false);
                let name = g.name.clone();
                match a["funcion"].as_str() {
                    Some(f) => p.functions.iter_mut().find(|x| x.name == f).ok_or(format!("no existe la función «{f}»"))?.graph = Some(g),
                    None => {
                        let s = scene_mut(p, a["escena"].as_str().ok_or("indica «escena» o «funcion»")?)?;
                        match s.graphs.iter_mut().find(|x| x.name == name) {
                            Some(old) => *old = g,
                            None => s.graphs.push(g),
                        }
                    }
                }
                Ok(format!("Grafo «{name}» puesto.\n{res}"))
            }),
            "grafo_ver" => {
                let p = load(&self.path(a)?)?;
                let gs: Vec<&Graph> = match a["funcion"].as_str() {
                    Some(f) => p.functions.iter().find(|x| x.name == f).ok_or(format!("no existe la función «{f}»"))?.graph.iter().collect(),
                    None => {
                        let name = a["escena"].as_str().ok_or("indica «escena» o «funcion»")?;
                        let s = p.scene(name).ok_or(format!("no existe la escena «{name}»"))?;
                        s.graphs.iter().filter(|g| a["nombre"].as_str().is_none_or(|n| n == g.name)).collect()
                    }
                };
                Ok(if gs.is_empty() { "No hay grafos ahí.".into() } else { gs.iter().map(|g| grafo_texto(g, true)).collect::<Vec<_>>().join("\n") })
            }
            "eventos_borrar" => self.edit(a, |p, _| {
                let ruta: String = arg(a, "ruta")?;
                let idx: Vec<usize> = ruta.split('.').map(|x| x.trim().parse::<usize>().ok().filter(|&n| n > 0).map(|n| n - 1)).collect::<Option<_>>().ok_or("ruta inválida, usa p. ej. \"2.1\"")?;
                let mut list = events_mut(p, a)?;
                for &k in &idx[..idx.len() - 1] {
                    list = &mut list.get_mut(k).ok_or(format!("no existe el evento {ruta}"))?.sub;
                }
                let last = idx[idx.len() - 1];
                if last >= list.len() {
                    return Err(format!("no existe el evento {ruta}"));
                }
                list.remove(last);
                Ok(format!("Evento {ruta} borrado."))
            }),
            "recetas" => Ok(nicostation::recetas::RECETAS.iter().map(|r| {
                let huecos: Vec<String> = r.huecos.iter().map(|(q, t, d)| format!("{q} ({t:?}{})", if d.is_empty() { String::new() } else { format!(", por defecto {d}") })).collect();
                format!("- {}: {}\n  huecos: [{}]{}{}", r.nombre, r.descripcion, huecos.join("; "),
                    if r.variables.is_empty() { String::new() } else { format!("\n  crea variables: {:?}", r.variables.iter().map(|v| v.0).collect::<Vec<_>>()) },
                    if r.nota.is_empty() { String::new() } else { format!("\n  nota: {}", r.nota) })
            }).collect::<Vec<_>>().join("\n")),
            "receta_usar" => self.edit(a, |p, _| {
                let name: String = arg(a, "receta")?;
                let r = nicostation::recetas::RECETAS.iter().find(|r| r.nombre == name)
                    .ok_or(format!("no existe la receta «{name}»; usa la herramienta recetas para ver la lista"))?;
                let mut resp: Vec<String> = opt(a, "respuestas")?.unwrap_or_default();
                // Los huecos que faltan al final toman su valor por defecto, si tienen.
                while let Some((_, _, d)) = r.huecos.get(resp.len()).filter(|h| !h.2.is_empty()) {
                    resp.push(d.to_string());
                }
                let evs = r.aplicar(&resp)?;
                let s = scene_mut(p, &arg::<String>(a, "escena")?)?;
                for (k, v) in r.variables_iniciales() {
                    s.vars.entry(k).or_insert(v);
                }
                let n = evs.len();
                s.events.extend(evs);
                Ok(format!("Receta «{name}» agregada ({n} eventos).{}", if r.nota.is_empty() { String::new() } else { format!(" Nota: {}", r.nota) }))
            }),
            "funcion_poner" => self.edit(a, |p, _| {
                let f: Function = arg(a, "funcion")?;
                let name = f.name.clone();
                match p.functions.iter_mut().find(|x| x.name == name) {
                    Some(x) => *x = f,
                    None => p.functions.push(f),
                }
                Ok(format!("Función «{name}» guardada."))
            }),
            "imagen_crear" => {
                let path = self.path(a)?;
                let dst = inside(&base_dir(&path), &arg::<String>(a, "ruta")?)?;
                let img = draw_image(a)?;
                if let Some(d) = dst.parent() {
                    std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
                }
                img.save(&dst).map_err(|e| format!("{}: {e}", dst.display()))?;
                Ok(format!("Imagen {}x{} guardada en {}.", img.width(), img.height(), dst.display()))
            }
            "sonido_crear" => {
                let path = self.path(a)?;
                let dst = inside(&base_dir(&path), &arg::<String>(a, "ruta")?)?;
                let secs = write_wav(a, &dst)?;
                Ok(format!("Sonido de {secs:.2} s guardado en {}.", dst.display()))
            }
            "validar" => {
                let path = self.path(a)?;
                let p = load(&path)?;
                let v = validate::validate(&p, Some(&base_dir(&path)));
                Ok(if v.is_empty() { "Sin problemas.".into() } else { format!("{} problemas:\n- {}", v.len(), v.join("\n- ")) })
            }
            "simular" => simulate(&self.path(a)?, a),
            "jugar" => {
                let path = self.path(a)?;
                let exe = std::env::current_exe().map_err(|e| e.to_string())?.with_file_name("nicostation");
                let child = std::process::Command::new(&exe)
                    .arg(&path)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                    .map_err(|e| format!("no pude abrir {}: {e} (compila con `cargo build --release --bin nicostation`)", exe.display()))?;
                Ok(format!("Juego abierto en una ventana (pid {}).", child.id()))
            }
            "exportar_web" => {
                let path = self.path(a)?;
                let out = opt::<String>(a, "carpeta")?.map(PathBuf::from).unwrap_or(base_dir(&path).join("web"));
                let files = export::export_web(&path, &out)?;
                Ok(format!("Exportado a {} ({} archivos). Para probarlo: python3 -m http.server -d {} 8000", out.display(), files.len(), out.display()))
            }
            _ => Err(format!("herramienta desconocida: {name}")),
        }
    }
}

fn base_dir(path: &Path) -> PathBuf {
    path.parent().filter(|d| !d.as_os_str().is_empty()).map(Path::to_path_buf).unwrap_or(PathBuf::from("."))
}

fn problems(p: &Project, base: &Path) -> String {
    let v = validate::validate(p, Some(base));
    match v.len() {
        0 => "\nValidación: sin problemas.".into(),
        n => format!("\nValidación: {n} problemas (puede ser normal mientras armas el juego):\n- {}{}",
            v.iter().take(12).cloned().collect::<Vec<_>>().join("\n- "), if n > 12 { "\n- ..." } else { "" }),
    }
}

fn rename_scene_refs(p: &mut Project, old: &str, new: &str) {
    fn evs(l: &mut [Event], old: &str, new: &str) {
        for e in l {
            for i in e.acts.iter_mut().filter(|i| i.op == "change_scene") {
                if i.args.first().is_some_and(|a| a == old) {
                    i.args[0] = new.into();
                }
            }
            evs(&mut e.sub, old, new);
        }
    }
    for s in &mut p.scenes {
        evs(&mut s.events, old, new);
    }
    for f in &mut p.functions {
        evs(&mut f.events, old, new);
    }
}

fn resumen(p: &Project, path: &Path) -> String {
    let mut s = format!("Proyecto «{}» ({}x{}) en {}\nEscena inicial: {}\n", p.name, p.width, p.height, path.display(), p.first_scene);
    if !p.globals.is_empty() {
        s += &format!("Globales: {}\n", serde_json::to_string(&p.globals).unwrap_or_default());
    }
    let obj_line = |o: &ObjectDef| {
        let kind = match &o.kind {
            Kind::Sprite { animations, .. } => format!("sprite, animaciones {:?}", animations.iter().map(|a| &a.name).collect::<Vec<_>>()),
            Kind::Text { .. } => "text".into(),
            Kind::Box { .. } => "box".into(),
            Kind::Tilemap { tileset, cols, rows, tile, .. } => format!("tilemap {cols}x{rows} baldosas de {tile}px, tileset {tileset}"),
            Kind::Mesh { shape, model, size, .. } => format!("mesh 3D {shape:?} {size:?}{}", if model.is_empty() { String::new() } else { format!(" modelo {model}") }),
            Kind::Light { light, .. } => format!("luz 3D {light:?}"),
            Kind::Camera3d { fov, .. } => format!("cámara 3D fov {fov}"),
        };
        let behs: Vec<String> = o.behaviors.iter().filter_map(|b| serde_json::to_value(b).ok()?.get("type")?.as_str().map(String::from)).collect();
        format!("  - {} ({kind}){}\n", o.name, if behs.is_empty() { String::new() } else { format!(" comportamientos {behs:?}") })
    };
    if !p.objects.is_empty() {
        s += "Objetos globales:\n";
        p.objects.iter().for_each(|o| s += &obj_line(o));
    }
    for sc in &p.scenes {
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        sc.instances.iter().for_each(|i| *counts.entry(&i.obj).or_default() += 1);
        s += &format!("\nEscena «{}»: fondo {:?}, capas {:?}, {} instancias {:?}, {} eventos de primer nivel\n",
            sc.name, sc.background, sc.layers.iter().map(|l| &l.name).collect::<Vec<_>>(), sc.instances.len(), counts, sc.events.len());
        if !sc.vars.is_empty() {
            s += &format!("  variables: {}\n", serde_json::to_string(&sc.vars).unwrap_or_default());
        }
        if !sc.groups.is_empty() {
            s += &format!("  grupos: {:?}\n", sc.groups);
        }
        sc.objects.iter().for_each(|o| s += &obj_line(o));
    }
    for f in &p.functions {
        s += &format!("\nFunción {}({}) {}\n", f.name, f.params.join(", "), if f.is_condition { "[condición]" } else { "[acción]" });
    }
    s
}

/// Texto de un grafo (si `nodos`) y lo que compila.
fn grafo_texto(g: &Graph, nodos: bool) -> String {
    let mut s = if nodos { nicostation::nodos::dump(g) } else { String::new() };
    let c = nicostation::nodos::compile_mapped(g);
    if c.errors.is_empty() {
        s += &format!("Compila: {} eventos de primer nivel.\n", c.events.len());
    } else {
        s += &format!("{} errores (los eventos con errores no se ejecutan):\n", c.errors.len());
        for e in &c.errors {
            s += &format!("- {}{}\n", e.node.map_or(String::new(), |n| format!("nodo {n}: ")), e.msg);
        }
    }
    sheet(&c.events, "", 0, &mut s);
    s
}

fn sheet(list: &[Event], prefix: &str, depth: usize, out: &mut String) {
    let pad = "  ".repeat(depth);
    for (k, e) in list.iter().enumerate() {
        let n = format!("{prefix}{}", k + 1);
        let off = if e.disabled { " [deshabilitado]" } else { "" };
        let head = match e.kind {
            EventKind::Comment => format!("{pad}{n}. // {}{off}\n", e.text),
            EventKind::Group => format!("{pad}{n}. Grupo «{}»{off}\n", e.text),
            EventKind::Repeat => format!("{pad}{n}. Repetir {} veces{off}\n", e.text),
            EventKind::ForEach => format!("{pad}{n}. Para cada {}{off}\n", e.text),
            EventKind::While => format!("{pad}{n}. Mientras{off}\n", ),
            EventKind::Standard => format!("{pad}{n}.{off}\n"),
            EventKind::Else => format!("{pad}{n}. Si no{off}\n"),
        };
        *out += &head;
        let sub = |i: &Instr| -> String {
            let mut s = catalog::sentence(i);
            for x in &i.sub {
                s += &format!(" [{}]", catalog::sentence(x));
            }
            s
        };
        for c in &e.conds {
            *out += &format!("{pad}   si   {}\n", sub(c));
        }
        for a in &e.acts {
            *out += &format!("{pad}   hacer {}\n", sub(a));
        }
        sheet(&e.sub, &format!("{n}."), depth + 1, out);
    }
}

// ---------- recursos ----------

fn rgba(v: &[u8]) -> image::Rgba<u8> {
    image::Rgba([v.first().copied().unwrap_or(0), v.get(1).copied().unwrap_or(0), v.get(2).copied().unwrap_or(0), v.get(3).copied().unwrap_or(255)])
}

fn draw_image(a: &J) -> Result<image::RgbaImage, String> {
    let fondo: Vec<u8> = opt(a, "fondo")?.unwrap_or(vec![0, 0, 0, 0]);
    if let Some(pa) = a.get("pixel_art").filter(|v| !v.is_null()) {
        let paleta: BTreeMap<String, Vec<u8>> = arg(pa, "paleta")?;
        let filas: Vec<String> = arg(pa, "filas")?;
        let k = opt::<u32>(pa, "escala")?.unwrap_or(1).clamp(1, 64);
        let cols = filas.iter().map(|f| f.chars().count()).max().unwrap_or(0) as u32;
        if cols == 0 || cols * k > 4096 || filas.len() as u32 * k > 4096 {
            return Err("pixel_art: filas vacías o imagen mayor a 4096 px".into());
        }
        let mut img = image::RgbaImage::from_pixel(cols * k, filas.len() as u32 * k, rgba(&fondo));
        for (y, fila) in filas.iter().enumerate() {
            for (x, ch) in fila.chars().enumerate() {
                if let Some(c) = paleta.get(&ch.to_string()) {
                    for dy in 0..k {
                        for dx in 0..k {
                            img.put_pixel(x as u32 * k + dx, y as u32 * k + dy, rgba(c));
                        }
                    }
                }
            }
        }
        return Ok(img);
    }
    let (w, h): (u32, u32) = (arg(a, "ancho")?, arg(a, "alto")?);
    if w == 0 || h == 0 || w > 4096 || h > 4096 {
        return Err("ancho y alto deben estar entre 1 y 4096".into());
    }
    let mut img = image::RgbaImage::from_pixel(w, h, rgba(&fondo));
    let formas: Vec<J> = opt(a, "formas")?.unwrap_or_default();
    for f in &formas {
        let g = |k: &str| f[k].as_f64().unwrap_or(0.) as f32;
        let color = rgba(&opt::<Vec<u8>>(f, "color")?.unwrap_or(vec![255, 255, 255, 255]));
        let (x, y, fw, fh) = (g("x"), g("y"), g("w"), g("h"));
        let mut put = |px: i64, py: i64| {
            if px >= 0 && py >= 0 && (px as u32) < w && (py as u32) < h {
                img.put_pixel(px as u32, py as u32, color);
            }
        };
        match f["tipo"].as_str().unwrap_or("rect") {
            "rect" => {
                for py in y.floor() as i64..(y + fh).ceil() as i64 {
                    for px in x.floor() as i64..(x + fw).ceil() as i64 {
                        put(px, py);
                    }
                }
            }
            "elipse" | "circulo" => {
                let (cx, cy, rx, ry) = (x + fw / 2., y + fh / 2., fw / 2., fh / 2.);
                for py in y.floor() as i64..(y + fh).ceil() as i64 {
                    for px in x.floor() as i64..(x + fw).ceil() as i64 {
                        let (dx, dy) = ((px as f32 + 0.5 - cx) / rx, (py as f32 + 0.5 - cy) / ry);
                        if dx * dx + dy * dy <= 1. {
                            put(px, py);
                        }
                    }
                }
            }
            "linea" => {
                let r = (g("grosor").max(1.) / 2.).max(0.5);
                let steps = fw.abs().max(fh.abs()).ceil().max(1.) as i64;
                for s in 0..=steps {
                    let (px, py) = (x + fw * s as f32 / steps as f32, y + fh * s as f32 / steps as f32);
                    for oy in (-r).floor() as i64..=r.ceil() as i64 {
                        for ox in (-r).floor() as i64..=r.ceil() as i64 {
                            if ((ox * ox + oy * oy) as f32) <= r * r {
                                put(px.round() as i64 + ox, py.round() as i64 + oy);
                            }
                        }
                    }
                }
            }
            o => return Err(format!("forma desconocida «{o}»: usa rect, elipse o linea")),
        }
    }
    Ok(img)
}

fn write_wav(a: &J, dst: &Path) -> Result<f32, String> {
    #[derive(serde::Deserialize)]
    struct Nota {
        hz: f32,
        seg: f32,
    }
    let notas: Vec<Nota> = arg(a, "notas")?;
    let onda = opt::<String>(a, "onda")?.unwrap_or("cuadrada".into());
    let vol = opt::<f32>(a, "volumen")?.unwrap_or(0.5).clamp(0., 1.);
    let rate = 22050.;
    let total: f32 = notas.iter().map(|n| n.seg.max(0.)).sum();
    if total <= 0. || total > 30. {
        return Err("la duración total debe estar entre 0 y 30 segundos".into());
    }
    let mut samples: Vec<i16> = vec![];
    let mut seed: u32 = 0x1234_5678;
    for n in &notas {
        let len = (n.seg.max(0.) * rate) as usize;
        for i in 0..len {
            let t = i as f32 / rate;
            let ph = (t * n.hz).fract();
            let v = if n.hz <= 0. {
                0.
            } else {
                match onda.as_str() {
                    "seno" => (ph * std::f32::consts::TAU).sin(),
                    "triangulo" => 4. * (ph - 0.5).abs() - 1.,
                    "sierra" => 2. * ph - 1.,
                    "ruido" => {
                        seed ^= seed << 13;
                        seed ^= seed >> 17;
                        seed ^= seed << 5;
                        seed as f32 / u32::MAX as f32 * 2. - 1.
                    }
                    _ => if ph < 0.5 { 1. } else { -1. },
                }
            };
            // Ataque de 5 ms y caída lineal para que no haga clic.
            let env = (t / 0.005).min(1.) * (1. - i as f32 / len as f32);
            samples.push((v * env * vol * 32000.) as i16);
        }
    }
    let data_len = (samples.len() * 2) as u32;
    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend(b"RIFF");
    bytes.extend((36 + data_len).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes()); // PCM
    bytes.extend(1u16.to_le_bytes()); // mono
    bytes.extend((rate as u32).to_le_bytes());
    bytes.extend((rate as u32 * 2).to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend(data_len.to_le_bytes());
    samples.iter().for_each(|s| bytes.extend(s.to_le_bytes()));
    if let Some(d) = dst.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::write(dst, bytes).map_err(|e| format!("{}: {e}", dst.display()))?;
    Ok(total)
}

// ---------- simulación ----------

fn simulate(path: &Path, a: &J) -> Result<String, String> {
    #[derive(serde::Deserialize)]
    struct Entrada {
        desde: f32,
        hasta: f32,
        #[serde(default)]
        teclas: Vec<String>,
        mouse: Option<(f32, f32)>,
        #[serde(default)]
        clic: bool,
    }
    let mut p = load(path)?;
    let base = base_dir(path);
    // Los sprites sin tamaño toman el de su primera imagen, como en el reproductor.
    for o in p.objects.iter_mut().chain(p.scenes.iter_mut().flat_map(|s| s.objects.iter_mut())) {
        if let Kind::Sprite { w, h, animations } = &mut o.kind {
            if let Some(f) = animations.first().and_then(|a| a.frames.first()) {
                if let Ok((iw, ih)) = image::image_dimensions(base.join(f)) {
                    if *w == 0. { *w = iw as f32 }
                    if *h == 0. { *h = ih as f32 }
                }
            }
        }
    }
    let secs: f32 = arg::<f32>(a, "segundos")?.clamp(0., 600.);
    let entradas: Vec<Entrada> = opt(a, "entradas")?.unwrap_or_default();
    let seguir: BTreeSet<String> = opt(a, "seguir")?.unwrap_or_default();
    let cada = opt::<f32>(a, "muestreo")?.unwrap_or(0.5).max(1. / 60.);
    let mut w = World::new(p);
    if let Some(s) = a["escena"].as_str() {
        if w.project.scene(s).is_none() {
            return Err(format!("no existe la escena «{s}»"));
        }
        w.load_scene(s);
    }
    let frames = (secs * 60.).round() as usize;
    let mut prev: BTreeSet<String> = BTreeSet::new();
    let mut prev_click = false;
    let mut eventos: Vec<String> = vec![];
    let mut muestras: Vec<J> = vec![];
    let mut sonidos: BTreeMap<String, usize> = BTreeMap::new();
    let mut escena = w.scene_def().name.clone();
    let mut next_sample = 0.;
    for f in 0..frames {
        let t = f as f32 / 60.;
        let activas: Vec<&Entrada> = entradas.iter().filter(|e| t >= e.desde && t < e.hasta).collect();
        let down: BTreeSet<String> = activas.iter().flat_map(|e| e.teclas.iter().cloned()).collect();
        let click = activas.iter().any(|e| e.clic);
        let mouse = activas.iter().rev().find_map(|e| e.mouse).unwrap_or((0., 0.));
        let inp = Input {
            pressed: down.difference(&prev).cloned().collect(),
            released: prev.difference(&down).cloned().collect(),
            down: down.iter().cloned().collect(),
            mouse,
            mouse_down: [click, false, false],
            mouse_pressed: [click && !prev_click, false, false],
        };
        prev = down;
        prev_click = click;
        if t + 1e-4 >= next_sample && !seguir.is_empty() {
            next_sample += cada;
            let pos: Vec<J> = w.insts.iter().filter(|i| seguir.contains(&i.obj))
                .map(|i| {
                    let mut j = if w.dim3 {
                        let r2 = |v: f32| (v as f64 * 100.).round() / 100.;
                        json!({"obj": i.obj, "x": r2(i.x), "y": r2(i.y), "z": r2(i.z), "angle": r2(i.angle), "pitch": r2(i.pitch)})
                    } else {
                        json!({"obj": i.obj, "x": i.x.round(), "y": i.y.round()})
                    };
                    if i.on_floor { j["en_suelo"] = json!(true) }
                    j
                }).collect();
            muestras.push(json!({"t": (t * 100.).round() / 100., "escena": w.scene_def().name, "pos": pos}));
        }
        w.step(1. / 60., inp);
        for c in std::mem::take(&mut w.cmds) {
            let k = match c {
                nicostation::runtime::Cmd::PlaySound { file, .. } => format!("sonido {file}"),
                nicostation::runtime::Cmd::PlayMusic { file, .. } => format!("música {file}"),
                nicostation::runtime::Cmd::StopMusic => "detener música".into(),
                nicostation::runtime::Cmd::StopSound { file } => format!("detener {file}"),
                nicostation::runtime::Cmd::Persist(_) => "guardar partida".into(),
                nicostation::runtime::Cmd::Quit => {
                    eventos.push(format!("t={t:.2}s: el juego pidió salir"));
                    continue;
                }
            };
            *sonidos.entry(k).or_default() += 1;
        }
        let now = w.scene_def().name.clone();
        if now != escena {
            eventos.push(format!("t={:.2}s: escena «{escena}» -> «{now}»", t + 1. / 60.));
            escena = now;
        }
    }
    let mut por_objeto: BTreeMap<&str, usize> = BTreeMap::new();
    w.insts.iter().for_each(|i| *por_objeto.entry(&i.obj).or_default() += 1);
    let inst: Vec<J> = (0..w.insts.len()).take(60).map(|k| {
        let i = &w.insts[k];
        let mut j = if w.dim3 {
            let r2 = |v: f32| (v as f64 * 100.).round() / 100.;
            json!({"obj": i.obj, "x": r2(i.x), "y": r2(i.y), "z": r2(i.z), "angle": r2(i.angle), "pitch": r2(i.pitch), "w": r2(i.w), "h": r2(i.h), "d": r2(i.d)})
        } else {
            json!({"obj": i.obj, "x": i.x.round(), "y": i.y.round(), "w": i.w.round(), "h": i.h.round()})
        };
        if let nicostation::model::Value::Str(an) = w.get(k, "anim") {
            if !an.is_empty() { j["anim"] = json!(an) }
        }
        if i.on_floor { j["en_suelo"] = json!(true) }
        if !i.vars.is_empty() { j["vars"] = serde_json::to_value(&i.vars).unwrap_or_default() }
        j
    }).collect();
    let r = json!({
        "segundos": secs,
        "escena_final": escena,
        "cambios": eventos,
        "variables_escena": w.vars,
        "globales": w.globals,
        "instancias_por_objeto": por_objeto,
        "instancias": inst,
        "muestras": muestras,
        "sonidos": sonidos,
        "avisos_del_motor": *w.log.borrow(),
    });
    serde_json::to_string_pretty(&r).map_err(|e| e.to_string())
}

// ---------- catálogo ----------

fn catalogo() -> String {
    let mut s = String::from(r#"# NicoStation: formato y catálogo

## Proyecto (lo crean las herramientas; aquí el formato por si usas proyecto_json)
{ "name", "width", "height", "first_scene", "globals": {var: valor}, "objects": [objetos globales],
  "scenes": [escena], "functions": [función] }
Escena: { "name", "background": [r,g,b], "layers": [{ "name": "", "visible": true, "fixed": false }],
  "objects": [...], "groups": { "Enemigos": ["Murcielago", "Slime"] }, "instances": [...], "vars": {...}, "events": [...] }
Capa "" = base. fixed=true no sigue a la cámara (HUD).

## Objetos
- sprite: { "name", "type": "sprite", "w"?, "h"?, "animations": [{ "name", "frames": ["img/a.png", ...], "fps": 10, "looping": true }] }
  w/h = 0 o ausentes: tamaño de la primera imagen. La primera animación es la inicial.
- text: { "name", "type": "text", "text": "Puntos: {puntos}", "size": 24, "color": [r,g,b] }   ({expresión} se actualiza sola)
- box: { "name", "type": "box", "w", "h", "color": [r,g,b] }   (rectángulo de color, ideal para prototipos)
- tilemap: { "name", "type": "tilemap", "tileset": "img/baldosas.png", "tile": 32, "cols", "rows", "data": [índices, -1 = vacía], "pass": [baldosas que no chocan] }
  Mapa de baldosas para armar niveles; con el comportamiento platform sus baldosas son suelo. Píntalo con mapa_pintar.
  El tileset es una imagen con baldosas en grilla (índice 0 arriba a la izquierda, de izquierda a derecha). Expresión tile(Mapa, x, y).
Todos aceptan "behaviors": [...] y "vars": { "vida": 3 }.
Instancia: { "obj", "x", "y", "layer"?, "angle"?, "z"?, "w"?, "h"?, "vars"? }  (x,y = esquina superior izquierda; rota alrededor del centro)

## Escenas 3D
Una escena con "dim3": true es 3D (se juega con el reproductor 3D). Allí x, y, z son la posición del CENTRO en metros,
Y hacia arriba, y adelante es -Z. angle gira alrededor del eje vertical (grados), pitch inclina, roll de costado.
"environment": { "sky_top", "sky_horizon", "sky_ground": [r,g,b], "sky_image": "hdr opcional", "ambient": 1,
  "fog_color", "fog_start", "fog_end", "glow": true, "ssao": true, "tonemap": "filmic|aces|agx|linear", "exposure": 1 }
Objetos 3D:
- mesh: { "type": "mesh", "shape": "cube|sphere|plane|capsule|cylinder|cone|model", "model": "modelos/x.glb", "size": [w,h,d],
  "color": [r,g,b], "metallic": 0..1, "roughness": 0..1, "texture": "img/...", "emission": [r,g,b] }
- light: { "type": "light", "light": "sun|point|spot", "color", "energy": 1, "range": 10, "spot_angle": 45, "shadows": true }
  (apunta según angle/pitch; un sol con pitch -50 ilumina en diagonal)
- camera3d: { "type": "camera3d", "fov": 70, "near": 0.05, "far": 4000 } (la primera instancia es la cámara del juego)
Instancias 3D: { "obj", "x", "y", "z", "angle", "pitch", "roll", "w"?, "h"?, "d"? }. Propiedades: d, pitch, roll, scale_z.
Bloques 3D: move_forward(obj, metros), move_right(obj, metros), look_at3(obj, x, y, z). collides usa cajas 3D.

## Comportamientos (con sus valores por defecto)
"#);
    for t in ["platformer", "platform", "top_down", "bullet", "rotate", "destroy_outside", "bound_to_layout", "screen_wrap", "draggable"] {
        let v = serde_json::from_value::<Behavior>(json!({"type": t}))
            .map(|b| serde_json::to_string(&b).unwrap_or_default())
            .unwrap_or_else(|_| format!("{{\"type\":\"{t}\",\"speed\":<obligatorio>}}"));
        s += &format!("- {v}\n");
    }
    s += r#"Notas: platformer usa flechas Izq/Der, salta con Space, LeftShift o Up (Up/Down suben/bajan escaleras).
platform: suelo/pared; rotada = rampa; jumpthru = se atraviesa desde abajo (Down para bajar); ladder = escalera; grabbable = borde agarrable.
top_down: flechas en 8 direcciones (no choca solo: usa la acción separate contra las paredes).
bullet: avanza según su ángulo (grados, 0 = derecha, 90 = abajo). destroy_outside: se borra al salir de la cámara.

## Eventos
{ "kind"?: "standard"|"else"|"repeat"|"for_each"|"while"|"group"|"comment", "text"?: "...", "conds": [instr], "acts": [instr], "sub": [eventos], "disabled"? }
- standard: si se cumplen TODAS las conds, ejecuta acts y luego sub (cada frame, 60 por segundo).
- else: «si no»; va justo después de un standard (o de otro else): se ejecuta solo si el anterior no se cumplió y sus conds sí.
- repeat: text = cantidad (expresión). for_each: text = objeto; cada vuelta elige una instancia. while: repite mientras conds.
- group: text = título, solo agrupa sub. comment: text = nota.
Instrucción: { "op": "...", "args": ["...", ...], "not"?: true, "sub"?: [instr] (para or/and) }. TODOS los args son texto.
Selección de objetos (como GDevelop): una condición sobre un objeto se queda solo con las instancias que la cumplen;
las acciones del evento y sus sub-eventos afectan solo a esas. Sin condición sobre el objeto = todas sus instancias.
"create" deja elegida solo la instancia nueva (útil: create + set sobre ella). En expresiones, Objeto.x dentro de una
acción sobre ese objeto es la instancia actual; si no, la primera elegida.
Para algo que ocurre una vez al entrar a la escena usa la condición "start". Para no repetir cada frame, "trigger_once".
"wait" (esperar N segundos) deja las acciones siguientes y los sub-eventos para después, con las mismas instancias.
Alarmas por instancia: set_alarm(obj, nombre, seg) y la condición alarm(obj, nombre).
Tweens: tween(obj, prop, valor_final, seg, curva) con curva linear|ease_in|ease_out|ease_in_out|back|elastic|bounce.
Cámara: camera_follow(obj, suavidad), camera_bounds(x0,y0,x1,y1), camera_shake(px, seg), camera_zoom_to(zoom, seg).

## Scripts (Rhai) para lógica compleja
Acción script(código) y condición script_if(código, el último valor es verdadero/falso). Funciones disponibles:
get_var/set_var, get_global/set_global, saved(k, defecto)/save(k, v), picked(obj) y all(obj) (arrays de ids), count(obj),
prop(id, p)/set_prop(id, p, v), object(id), exists(id), create(obj, x, y) -> id, destroy(id), key_down(k), key_pressed(k),
mouse_x(), mouse_y(), random(a, b), dt(), time(), play_sound(archivo), change_scene(nombre), log(texto),
dsin/dcos (grados), point_angle(x1,y1,x2,y2), point_distance(...). Números con decimales: escribe 1.0, no 1.
Ejemplo: for id in picked("Enemigo") { set_prop(id, "x", prop(id, "x") + 60.0 * dt()); }

## Condiciones
"#;
    let line = |m: &catalog::Meta| {
        let ps: Vec<String> = m.params.iter().map(|(n, t)| format!("{n}: {t:?}")).collect();
        let extra = if m.params.last().is_some_and(|p| p.1 == catalog::P::Func) { ", ...parámetros de la función" } else { "" };
        format!("- {}({}{extra}): {}\n", m.op, ps.join(", "), m.sentence)
    };
    catalog::CATALOG.iter().filter(|m| m.cond).for_each(|m| s += &line(m));
    s += "\n## Acciones\n";
    catalog::CATALOG.iter().filter(|m| !m.cond).for_each(|m| s += &line(m));
    s += &format!(r#"
Tipos de parámetro: Object = nombre de objeto o grupo; Expr = expresión; Text = texto literal; Key = tecla; Mouse = Left|Right|Middle;
Op = {:?}; Scene = escena; Prop = propiedad o variable de objeto; Anim = animación; File = ruta relativa; Layer = capa ("" = base);
Var = nombre de variable; Func = función propia (los argumentos que siguen son sus parámetros, como expresiones).

## Expresiones
Números, "texto", + - * / % ( ). Texto + algo = concatenación. Variables de escena, globales o parámetros por nombre.
Comparaciones == != < > <= >= (también =) y lógica and, or, not: verdadero = 1, falso = 0. if(cond, a, b).
Objeto.prop: x, y, cx, cy (centro), w, h, angle, opacity (0-255), z, speed, anim, text, o cualquier variable del objeto.
Funciones: random(a,b) randint(a,b) abs min max clamp(v,a,b) floor ceil round sqrt pow sin cos (grados) lerp(a,b,t)
angle(x1,y1,x2,y2) distance(x1,y1,x2,y2) atan2(y,x) lengthdir_x(largo,dir) lengthdir_y(largo,dir) sign tan
str num len upper lower substr(t,desde,largo?) contains(t,parte) replace(t,a,b) fixed(n,decimales) pad(n,ancho)
count(Objeto) picked(Objeto) timer("nombre") saved("clave", por_defecto) tile(Mapa, x, y).
Variables especiales: dt (segundos del frame), time, mousex, mousey, screen_w, screen_h. true = 1, false = 0.
Propiedades conocidas: {:?}

## Teclas
{}
"#, catalog::OPS, catalog::PROPS, catalog::KEYS.join(" "));
    s += r#"
## Grafos (nodos, como Blueprints)
Otra forma de escribir la lógica: escena.graphs = [grafo] (y función.graph). Se compilan a eventos que van después de la hoja.
{ "name": "...", "nodes": [nodo], "flow": [{"from": id, "out": 0, "to": id}], "wires": [{"from": id, "to": id, "arg": k}] }
Nodo: { "id": n, "k": tipo, "x": px, "y": px, "args": ["..."], "not"?: true, "sub"?: [instr] (or/and), "off"?: true }
- "event" + "op": condición que dispara (start, every, key_pressed, key_down, collides, mouse_clicked, alarm, timer...); "op": "" = cada cuadro.
- "cond" + "op": «Si» con salidas out 0 = sí, out 1 = no (el «no» se compila como evento else). "act" + "op": acción.
- "seq" + "n": salidas 0..n-1 en orden. "repeat" (args[0] = veces; out 0 = cada vez, out 1 = al terminar),
  "for_each" (args[0] = objeto), "while" (out 0 empieza con los «Si» que se repiten). "comment" + "w","h" (args[0] = texto).
- "data" + "f": nodo de datos puro con una salida que va por "wires" al parámetro "arg" de otro nodo (se compila a la expresión):
  num text bool obj var global prop count dt time mousex mousey expr add sub mul div mod min max abs round random randint clamp lerp
  sin cos sqrt distance angle eq ne lt le gt ge and or not if join len upper lower str tonum. Sus entradas sin cable usan sus args.
Los args de event/cond/act son los del catálogo; un cable a un parámetro reemplaza a lo escrito. Orden de los eventos: por "y", luego "x".
Ejemplo: {"name":"Mover","nodes":[{"id":1,"k":"event","op":"key_down","args":["Right"]},{"id":2,"k":"act","op":"add","args":["Heroe","x",""]},
 {"id":3,"k":"data","f":"mul","args":["200","1"]},{"id":4,"k":"data","f":"dt"}],"flow":[{"from":1,"to":2}],
 "wires":[{"from":3,"to":2,"arg":2},{"from":4,"to":3,"arg":1}]}  →  si tecla Right: sumar 200 * dt a x de Heroe.

## Ejemplo mínimo de evento
{ "conds": [{ "op": "collides", "args": ["Heroe", "Moneda"] }],
  "acts": [{ "op": "delete", "args": ["Moneda"] }, { "op": "add_var", "args": ["puntos", "1"] },
           { "op": "play_sound", "args": ["snd/moneda.wav", "80"] }] }
"#;
    s
}
