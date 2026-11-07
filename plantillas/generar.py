#!/usr/bin/env python3
"""Genera las plantillas de juegos usando el conector MCP de NicoStation (la misma vía que un agente).

Uso: python3 plantillas/generar.py [ruta/a/nicostation-mcp]
Cada plantilla queda en plantillas/<carpeta>/juego.json con sus imágenes y sonidos, validada y simulada.
"""
import json, os, shutil, subprocess, sys

RAIZ = os.path.dirname(os.path.abspath(__file__))
MCP = sys.argv[1] if len(sys.argv) > 1 else os.path.join(RAIZ, "..", "target", "release", "nicostation-mcp")
srv = subprocess.Popen([MCP], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
_n = 0


def call(name, **args):
    global _n
    _n += 1
    srv.stdin.write(json.dumps({"jsonrpc": "2.0", "id": _n, "method": "tools/call", "params": {"name": name, "arguments": args}}) + "\n")
    srv.stdin.flush()
    r = json.loads(srv.stdout.readline())["result"]
    txt = r["content"][0]["text"]
    if r.get("isError"):
        sys.exit(f"ERROR en {name}: {txt}")
    if "Validación: sin problemas" not in txt and "problemas" in txt and name not in ("validar", "simular"):
        print(f"  aviso {name}: {txt.splitlines()[-1]}")
    return txt


def nuevo(carpeta, nombre, ancho, alto, escena):
    d = os.path.join(RAIZ, carpeta)
    shutil.rmtree(d, ignore_errors=True)
    call("proyecto_crear", ruta=os.path.join(d, "juego.json"), nombre=nombre, ancho=ancho, alto=alto, escena=escena)
    return d


def pixel(ruta, filas, paleta, escala=4):
    call("imagen_crear", ruta=ruta, pixel_art={"filas": filas, "paleta": paleta, "escala": escala})


def sonido(ruta, notas, onda="cuadrada", volumen=0.4):
    call("sonido_crear", ruta=ruta, notas=[{"hz": h, "seg": s} for h, s in notas], onda=onda, volumen=volumen)


def sprite(nombre, frames, comportamientos=(), fps=8, **extra):
    o = {"name": nombre, "type": "sprite", "animations": [{"name": "a", "frames": frames, "fps": fps}], "behaviors": list(comportamientos)}
    o.update(extra)
    return o


def texto(nombre, t, size=24, color=(255, 255, 255)):
    return {"name": nombre, "type": "text", "text": t, "size": size, "color": list(color)}


def ev(conds=(), acts=(), sub=(), **extra):
    e = {"conds": [i(*c) for c in conds], "acts": [i(*a) for a in acts], "sub": list(sub)}
    e.update(extra)
    return e


def i(op, *args, no=False):
    d = {"op": op, "args": [str(a) for a in args]}
    if no:
        d["not"] = True
    return d


def comentario(t):
    return {"kind": "comment", "text": t}


def escena_fin(titulo, color, siguiente):
    """Escena final con un mensaje y Enter o clic para volver a jugar."""
    call("escena_crear", nombre="Fin", fondo=list(color))
    call("objeto_poner", escena="Fin", objeto=texto("Titulo", titulo, 56, (255, 220, 90)))
    call("objeto_poner", escena="Fin", objeto=texto("Info", "Puntos: {puntos_final}   ·   Enter para jugar otra vez", 24))
    call("instancias_agregar", escena="Fin", instancias=[{"obj": "Titulo", "x": 60, "y": 150}, {"obj": "Info", "x": 60, "y": 260}])
    call("eventos_poner", escena="Fin", eventos=[
        comentario("Pantalla final: muestra los puntos y espera Enter o un clic."),
        {"conds": [{"op": "or", "sub": [i("key_pressed", "Enter"), i("mouse_clicked", "Left")]}], "acts": [i("change_scene", siguiente)]},
    ])


def comprobar(nombre, **sim):
    v = call("validar")
    if v != "Sin problemas.":
        sys.exit(f"{nombre}: {v}")
    r = json.loads(call("simular", **sim))
    if r["avisos_del_motor"]:
        sys.exit(f"{nombre}: avisos del motor {r['avisos_del_motor']}")
    return r


# Colores compartidos
K, W = [25, 25, 30], [250, 250, 250]

# ---------------------------------------------------------------- Plataformas
d = nuevo("plataformas", "Aventura en plataformas", 800, 480, "Nivel")
heroe = {"o": [70, 190, 110], "O": [40, 140, 80], "k": K, "w": W, "s": [250, 200, 150], "z": [60, 80, 160]}
pixel("img/heroe_quieto.png", ["..oooo..", ".oooooo.", ".owkowk.", ".oooooo.", "..oooo..", ".zzzzzz.", ".zzzzzz.", ".s....s."], heroe)
pixel("img/heroe_corre1.png", ["..oooo..", ".oooooo.", ".owkowk.", ".oooooo.", "..oooo..", ".zzzzzz.", "zzzzzzz.", "s.....s."], heroe)
pixel("img/heroe_corre2.png", ["..oooo..", ".oooooo.", ".owkowk.", ".oooooo.", "..oooo..", ".zzzzzz.", ".zzzzzzz", ".s...s.."], heroe)
pixel("img/moneda.png", ["..yyyy..", ".yYYYYy.", "yYyyyyYy", "yYyYYyYy", "yYyYYyYy", "yYyyyyYy", ".yYYYYy.", "..yyyy.."], {"y": [250, 200, 40], "Y": [255, 235, 120]}, 3)
pixel("img/enemigo.png", ["..rrrr..", ".rrrrrr.", "rrwkrwkr", "rrrrrrrr", "rr.rr.rr", "rrrrrrrr", ".r.rr.r.", "r..rr..r"], {"r": [210, 60, 80], "w": W, "k": K})
pixel("img/bandera.png", ["pyy.....", "pyyyy...", "pyyyyyy.", "pyyyy...", "pyy.....", "p.......", "p.......", "p......."], {"p": [200, 200, 210], "y": [240, 80, 80]})
g, t_, b_, w_, n_ = [90, 180, 70], [125, 85, 55], [150, 150, 160], [175, 115, 60], [255, 255, 255]
pixel("img/baldosas.png", [
    "gggggggg" "tttttttt" "bbbbbbbb" "wwwwwwww",
    "gGgggGgg" "tTttttTt" "bBbbbBbb" "wWwwwwWw",
    "tttttttt" "tttttttt" "bbbbbbbb" "........",
    "tTttttTt" "ttTttttt" "BbbbbbBb" "........",
    "tttttttt" "tttttTtt" "bbbbbbbb" "........",
    "ttTttttt" "tTtttttt" "bbBbbbbb" "........",
    "tttttTtt" "tttttttt" "bbbbbbbb" "........",
    "tTtttttt" "ttttTttt" "bBbbbbbB" "........",
], {"g": g, "G": [60, 150, 50], "t": t_, "T": [100, 65, 40], "b": b_, "B": [120, 120, 130], "w": w_, "W": [140, 90, 45]})
sonido("snd/moneda.wav", [(988, 0.06), (1319, 0.1)])
sonido("snd/salto.wav", [(440, 0.05), (660, 0.07)], "triangulo")
sonido("snd/golpe.wav", [(220, 0.06), (110, 0.12)], "ruido", 0.3)
sonido("snd/ganar.wav", [(523, 0.1), (659, 0.1), (784, 0.1), (1047, 0.25)])
call("escena_editar", escena="Nivel", fondo=[120, 185, 235], capas=[{"name": ""}, {"name": "Guia", "visible": False}, {"name": "HUD", "fixed": True}], variables={"puntos": 0})
call("objeto_poner", escena="Nivel", objeto={"name": "Heroe", "type": "sprite", "animations": [
    {"name": "quieto", "frames": ["img/heroe_quieto.png"]}, {"name": "corre", "frames": ["img/heroe_corre1.png", "img/heroe_corre2.png"], "fps": 8}],
    "behaviors": [{"type": "platformer", "max_speed": 260, "jump_speed": 620}]})
call("objeto_poner", escena="Nivel", objeto={"name": "Mapa", "type": "tilemap", "tileset": "img/baldosas.png", "tile": 32, "cols": 1, "rows": 1,
                                            "behaviors": [{"type": "platform"}]})
call("objeto_poner", escena="Nivel", objeto={"name": "Puente", "type": "tilemap", "tileset": "img/baldosas.png", "tile": 32, "cols": 1, "rows": 1,
                                            "behaviors": [{"type": "platform", "jumpthru": True}]})
call("objeto_poner", escena="Nivel", objeto=sprite("Moneda", ["img/moneda.png"]))
call("objeto_poner", escena="Nivel", objeto=sprite("Enemigo", ["img/enemigo.png"], vars={"dir": 1}))
call("objeto_poner", escena="Nivel", objeto=sprite("Meta", ["img/bandera.png"]))
call("objeto_poner", escena="Nivel", objeto={"name": "Borde", "type": "box", "w": 16, "h": 64, "color": [255, 0, 0]})
call("objeto_poner", escena="Nivel", objeto=texto("Marcador", "Puntos: {puntos}", 26))
call("mapa_pintar", escena="Nivel", objeto="Mapa", leyenda={"#": 0, "t": 1, "b": 2}, filas=[
    "....................................................",
    "....................................................",
    "....................................................",
    "....................................................",
    "..................................bbbb..............",
    "....................................................",
    "............bbb.....................................",
    "....................................................",
    "..........................bbb.......................",
    "...................................................b",
    "...................................................b",
    "####################....###########....############",
    "tttttttttttttttttttt....ttttttttttt....tttttttttttt",
    "tttttttttttttttttttt....ttttttttttt....tttttttttttt",
    "tttttttttttttttttttt....ttttttttttt....tttttttttttt",
])
call("mapa_pintar", escena="Nivel", objeto="Puente", leyenda={"=": 3}, filas=["....................................................",
    "....................................................", "....................................................",
    "....................................................", "....................................................",
    "....................................................", "....................................................",
    "....................................................", "....................................................",
    "....................====....................====...."])
call("instancias_agregar", escena="Nivel", instancias=[
    {"obj": "Mapa", "x": 0, "y": 0}, {"obj": "Puente", "x": 0, "y": 0}, {"obj": "Heroe", "x": 60, "y": 300},
    {"obj": "Moneda", "x": 420, "y": 160}, {"obj": "Moneda", "x": 870, "y": 220}, {"obj": "Moneda", "x": 1130, "y": 100},
    {"obj": "Moneda", "x": 680, "y": 280}, {"obj": "Moneda", "x": 1480, "y": 280},
    {"obj": "Enemigo", "x": 900, "y": 320}, {"obj": "Enemigo", "x": 1350, "y": 320},
    *[{"obj": "Borde", "x": x, "y": 288, "layer": "Guia"} for x in (790, 1105, 1250, 1620)],
    {"obj": "Meta", "x": 1580, "y": 320}, {"obj": "Marcador", "x": 16, "y": 12, "layer": "HUD"}])
call("eventos_poner", escena="Nivel", eventos=[
    comentario("Aventura en plataformas: recoge monedas, salta sobre los enemigos y llega a la bandera."),
    ev([("key_down", "Left")], [("flip_x", "Heroe", 1)]),
    ev([("key_down", "Right")], [("flip_x", "Heroe", 0)]),
    ev([("is_moving", "Heroe")], [("set_animation", "Heroe", "corre")]),
    {"conds": [i("is_moving", "Heroe", no=True)], "acts": [i("set_animation", "Heroe", "quieto")]},
    ev([("jumping", "Heroe"), ("trigger_once",)], [("play_sound", "snd/salto.wav", 50)]),
    ev([("collides", "Heroe", "Moneda")], [("delete", "Moneda"), ("add_var", "puntos", 1), ("play_sound", "snd/moneda.wav", 70)]),
    {"kind": "group", "text": "Enemigos", "sub": [
        ev([], [("add", "Enemigo", "x", "Enemigo.dir * 70 * dt"), ("flip_x", "Enemigo", "0.5 - Enemigo.dir * 0.5")]),
        ev([("collides", "Enemigo", "Borde")], [("separate", "Enemigo", "Borde"), ("set", "Enemigo", "dir", "0 - Enemigo.dir")]),
        ev([("collides", "Heroe", "Enemigo")], sub=[
            ev([("falling", "Heroe")], [("delete", "Enemigo"), ("jump", "Heroe"), ("add_var", "puntos", 5), ("play_sound", "snd/golpe.wav", 80)]),
            {"conds": [i("falling", "Heroe", no=True)], "acts": [i("restart_scene")]},
        ]),
    ]},
    ev([("collides", "Heroe", "Meta")], [("play_sound", "snd/ganar.wav", 80), ("set_global", "puntos_final", "puntos"), ("change_scene", "Fin")]),
    ev([], [("camera", "clamp(Heroe.cx, 400, 1664 - 400)", "clamp(Heroe.cy, 240, 480 - 240)")]),
])
call("receta_usar", escena="Nivel", receta="Perder al caer al vacío", respuestas=["Heroe"])
escena_fin("¡Llegaste a la meta!", [30, 40, 70], "Nivel")
call("proyecto_configurar", globales={"puntos_final": 0})
r = comprobar("plataformas", segundos=3, entradas=[{"desde": 0, "hasta": 3, "teclas": ["Right"]}, {"desde": 1.0, "hasta": 1.3, "teclas": ["Right", "Space"]}], seguir=["Heroe"])
print("plataformas ok; héroe en", r["muestras"][-1]["pos"])

# ---------------------------------------------------------------- Recolector (vista de arriba)
d = nuevo("recolector", "Gato goloso", 800, 600, "Juego")
gato = {"o": [240, 150, 60], "O": [200, 110, 40], "k": K, "w": W, "p": [250, 150, 170]}
pixel("img/gato.png", ["o......o", "oo....oo", "oooooooo", "owkoowko", "oooppooo", "oOooooOo", ".oooooo.", ".o.oo.o."], gato)
pixel("img/pez.png", ["........", "..bbb..b", ".bbbbbbb", "bwkbbbbb", "bbbbbbbb", ".bbbbbbb", "..bbb..b", "........"], {"b": [90, 180, 255], "w": W, "k": K}, 3)
pixel("img/perro.png", ["mm....mm", "mmmmmmmm", "mwkmmwkm", "mmmmmmmm", "mmmkkmmm", "mmmmmmmm", ".mmmmmm.", ".m.mm.m."], {"m": [150, 110, 80], "w": W, "k": K})
pixel("img/arbusto.png", ["..gggg..", ".gGgggg.", "gggggGgg", "gGgggggg", "ggggGggg", "gggggggg", ".gGgggg.", "..gggg.."], {"g": [60, 150, 70], "G": [40, 110, 50]})
sonido("snd/pez.wav", [(880, 0.06), (1320, 0.08)])
sonido("snd/perder.wav", [(400, 0.1), (300, 0.1), (200, 0.25)], "sierra", 0.3)
call("escena_editar", escena="Juego", fondo=[70, 140, 70], capas=[{"name": ""}, {"name": "HUD", "fixed": True}], variables={"puntos": 0, "tiempo": 30})
call("objeto_poner", escena="Juego", objeto=sprite("Gato", ["img/gato.png"], [{"type": "top_down", "max_speed": 240}, {"type": "bound_to_layout"}]))
call("objeto_poner", escena="Juego", objeto=sprite("Pez", ["img/pez.png"]))
call("objeto_poner", escena="Juego", objeto=sprite("Perro", ["img/perro.png"]))
call("objeto_poner", escena="Juego", objeto=sprite("Arbusto", ["img/arbusto.png"]))
call("objeto_poner", escena="Juego", objeto=texto("Marcador", "Peces: {puntos}     Tiempo: {tiempo}", 26))
call("instancias_agregar", escena="Juego", instancias=[
    {"obj": "Gato", "x": 380, "y": 280}, {"obj": "Perro", "x": 700, "y": 80}, {"obj": "Marcador", "x": 16, "y": 12, "layer": "HUD"},
    *[{"obj": "Arbusto", "x": x, "y": y} for x, y in [(200, 160), (232, 160), (264, 160), (520, 400), (520, 432), (520, 464), (120, 460), (650, 250)]]])
call("eventos_poner", escena="Juego", eventos=[
    comentario("Gato goloso: come todos los peces que puedas en 30 segundos sin que te atrape el perro."),
    ev([("collides", "Gato", "Arbusto")], [("separate", "Gato", "Arbusto")]),
    ev([("start",)], [("create", "Pez", "random(40, 740)", "random(60, 540)", "")]),
    ev([("every", "1.2"), ("compare", "count(Pez)", "<", "4")], [("create", "Pez", "random(40, 740)", "random(60, 540)", "")]),
    ev([("collides", "Gato", "Pez")], [("delete", "Pez"), ("add_var", "puntos", 1), ("play_sound", "snd/pez.wav", 70)]),
    ev([("every", "1")], [("add_var", "tiempo", -1)]),
    ev([("compare", "tiempo", "<=", "0")], [("set_global", "puntos_final", "puntos"), ("change_scene", "Fin")]),
])
call("receta_usar", escena="Juego", receta="Enemigo que persigue", respuestas=["Perro", "Gato", "70 + puntos * 5"])
call("eventos_poner", escena="Juego", eventos=[
    ev([("collides", "Gato", "Perro")], [("play_sound", "snd/perder.wav", 80), ("set_global", "puntos_final", "puntos"), ("change_scene", "Fin")]),
])
escena_fin("¡Se acabó!", [40, 60, 40], "Juego")
call("proyecto_configurar", globales={"puntos_final": 0})
r = comprobar("recolector", segundos=4, entradas=[{"desde": 0, "hasta": 2, "teclas": ["Left"]}, {"desde": 2, "hasta": 4, "teclas": ["Down"]}])
print("recolector ok; escena", r["escena_final"], r["variables_escena"])

# ---------------------------------------------------------------- Naves
d = nuevo("naves", "Lluvia de asteroides", 600, 800, "Juego")
pixel("img/nave.png", ["...ww...", "...ww...", "..wbbw..", "..wbbw..", ".wwwwww.", "wwrwwrww", "ww.ww.ww", "r......r"], {"w": [220, 225, 240], "b": [90, 170, 255], "r": [255, 120, 60]}, 5)
pixel("img/laser.png", ["y", "y", "Y", "Y"], {"y": [255, 240, 120], "Y": [255, 180, 60]}, 4)
pixel("img/roca.png", ["..gggg..", ".gGggGg.", "gggGgggg", "gGgggGgg", "ggggggGg", "gGggGggg", ".ggggGg.", "..gggg.."], {"g": [140, 120, 105], "G": [100, 85, 75]}, 5)
pixel("img/estrella.png", ["w"], {"w": [255, 255, 255]}, 3)
sonido("snd/laser.wav", [(1200, 0.03), (900, 0.05)], "cuadrada", 0.25)
sonido("snd/boom.wav", [(150, 0.15)], "ruido", 0.4)
call("escena_editar", escena="Juego", fondo=[10, 10, 30], capas=[{"name": "Fondo"}, {"name": ""}, {"name": "HUD", "fixed": True}], variables={"puntos": 0})
call("objeto_poner", escena="Juego", objeto=sprite("Nave", ["img/nave.png"], [{"type": "bound_to_layout"}]))
call("objeto_poner", escena="Juego", objeto=sprite("Laser", ["img/laser.png"], [{"type": "bullet", "speed": 600}, {"type": "destroy_outside"}]))
call("objeto_poner", escena="Juego", objeto=sprite("Roca", ["img/roca.png"], [{"type": "bullet", "speed": 160}, {"type": "rotate", "speed": 60}, {"type": "destroy_outside"}]))
call("objeto_poner", escena="Juego", objeto=sprite("Estrella", ["img/estrella.png"], [{"type": "bullet", "speed": 80}, {"type": "screen_wrap"}]))
call("objeto_poner", escena="Juego", objeto=texto("Marcador", "Puntos: {puntos}", 28))
call("instancias_agregar", escena="Juego", instancias=[{"obj": "Nave", "x": 280, "y": 700}, {"obj": "Marcador", "x": 16, "y": 12, "layer": "HUD"}])
call("eventos_poner", escena="Juego", eventos=[
    comentario("Lluvia de asteroides: muévete con las flechas y dispara con Espacio."),
    ev([("start",)], [("set", "Nave", "speed", 0)], sub=[{"kind": "repeat", "text": "40", "acts": [
        i("create", "Estrella", "random(0, 600)", "random(0, 800)", "Fondo"), i("set", "Estrella", "angle", "90")]}]),
    ev([("key_down", "Left")], [("add", "Nave", "x", "-320 * dt")]),
    ev([("key_down", "Right")], [("add", "Nave", "x", "320 * dt")]),
    ev([("key_pressed", "Space")], [("create", "Laser", "Nave.cx - 2", "Nave.y", ""), ("set", "Laser", "angle", "-90"), ("play_sound", "snd/laser.wav", 50)]),
    ev([("every", "max(0.35, 1.2 - puntos * 0.03)")], [("create", "Roca", "random(0, 560)", "-40", ""), ("set", "Roca", "angle", "random(80, 100)")]),
    ev([("collides", "Laser", "Roca")], [("delete", "Laser"), ("delete", "Roca"), ("add_var", "puntos", 1), ("play_sound", "snd/boom.wav", 60)]),
    ev([("collides", "Nave", "Roca")], [("play_sound", "snd/boom.wav", 100), ("set_global", "puntos_final", "puntos"), ("change_scene", "Fin")]),
])
escena_fin("¡Boom!", [20, 20, 45], "Juego")
call("proyecto_configurar", globales={"puntos_final": 0})
r = comprobar("naves", segundos=6, entradas=[{"desde": t, "hasta": t + 0.1, "teclas": ["Space"]} for t in [0.5 * k for k in range(1, 12)]])
print("naves ok;", r["instancias_por_objeto"], r["variables_escena"])

# ---------------------------------------------------------------- Atrapa la fruta
d = nuevo("atrapa", "Atrapa la fruta", 800, 600, "Juego")
pixel("img/canasta.png", ["w..........w", "wwwwwwwwwwww", "wWwWwWwWwWww", "wwwwwwwwwwww", ".wWwWwWwWww.", "..wwwwwwww.."], {"w": [190, 130, 70], "W": [150, 95, 50]}, 6)
pixel("img/manzana.png", ["...g....", "..rgr...", ".rrrrrr.", "rrwrrrrr", "rrrrrrrr", "rrrrrrrr", ".rrrrrr.", "..r..r.."], {"r": [220, 40, 50], "g": [70, 150, 60], "w": [255, 200, 200]}, 4)
pixel("img/banana.png", ["......y.", ".....yy.", "....yyy.", "...yyy..", "..yyyy..", "yyyyy...", ".yyy....", "........"], {"y": [250, 220, 60]}, 4)
pixel("img/corazon.png", [".rr.rr.", "rrrrrrr", "rrrrrrr", ".rrrrr.", "..rrr..", "...r..."], {"r": [235, 60, 90]}, 4)
sonido("snd/atrapar.wav", [(660, 0.05), (990, 0.08)], "seno", 0.5)
sonido("snd/caer.wav", [(300, 0.08), (180, 0.15)], "triangulo", 0.4)
call("escena_editar", escena="Juego", fondo=[150, 210, 250], capas=[{"name": ""}, {"name": "HUD", "fixed": True}], variables={"puntos": 0, "vidas": 3})
call("objeto_poner", escena="Juego", objeto=sprite("Canasta", ["img/canasta.png"], [{"type": "bound_to_layout"}]))
call("objeto_poner", escena="Juego", objeto=sprite("Manzana", ["img/manzana.png"], [{"type": "bullet", "speed": 180}]))
call("objeto_poner", escena="Juego", objeto=sprite("Banana", ["img/banana.png"], [{"type": "bullet", "speed": 240}]))
call("objeto_poner", escena="Juego", objeto={"name": "Suelo", "type": "box", "w": 800, "h": 30, "color": [90, 160, 70]})
call("objeto_poner", escena="Juego", objeto=texto("Marcador", "Puntos: {puntos}     Vidas: {vidas}", 26, (30, 40, 60)))
call("escena_editar", escena="Juego", grupos={"Fruta": ["Manzana", "Banana"]})
call("instancias_agregar", escena="Juego", instancias=[{"obj": "Canasta", "x": 360, "y": 520}, {"obj": "Suelo", "x": 0, "y": 570}, {"obj": "Marcador", "x": 16, "y": 12, "layer": "HUD"}])
call("eventos_poner", escena="Juego", eventos=[
    comentario("Atrapa la fruta: mueve la canasta con las flechas o con el mouse. Si se te caen 3, pierdes."),
    ev([("key_down", "Left")], [("add", "Canasta", "x", "-420 * dt")]),
    ev([("key_down", "Right")], [("add", "Canasta", "x", "420 * dt")]),
    ev([("mouse_down", "Left")], [("set", "Canasta", "cx", "mousex")]),
    ev([("every", "max(0.5, 1.4 - puntos * 0.04)")], [("create", "Manzana", "random(20, 760)", "-30", ""), ("set", "Manzana", "angle", 90)]),
    ev([("every", "3.1")], [("create", "Banana", "random(20, 760)", "-30", ""), ("set", "Banana", "angle", 90)]),
    ev([("collides", "Canasta", "Fruta")], [("delete", "Fruta"), ("add_var", "puntos", 1), ("play_sound", "snd/atrapar.wav", 70)]),
    ev([("collides", "Fruta", "Suelo")], [("delete", "Fruta"), ("add_var", "vidas", -1), ("play_sound", "snd/caer.wav", 70)]),
    ev([("compare", "vidas", "<=", "0")], [("set_global", "puntos_final", "puntos"), ("change_scene", "Fin")]),
])
escena_fin("¡Se te cayó la fruta!", [60, 90, 130], "Juego")
call("proyecto_configurar", globales={"puntos_final": 0})
r = comprobar("atrapa", segundos=12)
print("atrapa ok; escena", r["escena_final"], r["cambios"])

srv.stdin.close()
srv.wait()
print("Plantillas listas en", RAIZ)
