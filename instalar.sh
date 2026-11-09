#!/bin/bash
# Instala NicoStation como aplicación de escritorio para este usuario (sin sudo):
#   ~/.local/share/nicostation/   programas, plantillas, reproductores para web/Windows/Android
#   ~/.local/bin/                 accesos directos (nicostation, nicostation-editor, nicostation3d, nicostation-mcp)
#   ~/.local/share/applications/  entrada en el lanzador
# Volver a correrlo después de cada cambio actualiza la instalación.
set -euo pipefail
cd "$(dirname "$0")"
DEST="${XDG_DATA_HOME:-$HOME/.local/share}/nicostation"
BIN="$HOME/.local/bin"
APPS="${XDG_DATA_HOME:-$HOME/.local/share}/applications"

echo "Compilando..."
cargo build --release --workspace
cargo build --release --target wasm32-unknown-unknown --bin nicostation   # para «Exportar web»
if command -v x86_64-w64-mingw32-gcc >/dev/null; then
  cargo build --release --target x86_64-pc-windows-gnu --bin nicostation  # para exportar a Windows
fi

echo "Instalando en $DEST"
mkdir -p "$DEST" "$BIN" "$APPS"
for b in nicostation nicostation-editor nicostation3d nicostation-mcp; do
  install -m 755 "target/release/$b" "$DEST/$b"
done
install -m 644 target/wasm32-unknown-unknown/release/nicostation.wasm "$DEST/nicostation.wasm"
[[ -f target/x86_64-pc-windows-gnu/release/nicostation.exe ]] && install -m 644 target/x86_64-pc-windows-gnu/release/nicostation.exe "$DEST/nicostation.exe"
[[ -f target/android-artifacts/release/apk/nicostation.apk ]] && install -m 644 target/android-artifacts/release/apk/nicostation.apk "$DEST/nicostation.apk"
install -m 644 assets/icono.png "$DEST/icono.png"
rm -rf "$DEST/plantillas"
cp -r plantillas "$DEST/plantillas"
rm -f "$DEST/plantillas/generar.py"

for b in nicostation nicostation-editor nicostation3d nicostation-mcp; do
  ln -sf "$DEST/$b" "$BIN/$b"
done

cat > "$APPS/nicostation.desktop" <<DESKTOP
[Desktop Entry]
Name=NicoStation
GenericName=Creador de juegos
Comment=Crea juegos 2D y 3D con hojas de eventos o nodos
Exec=$DEST/nicostation-editor %f
Icon=$DEST/icono.png
Terminal=false
Type=Application
Categories=Development;IDE;
Keywords=juegos;game;engine;editor;eventos;
StartupWMClass=nicostation
DESKTOP
command -v update-desktop-database >/dev/null && update-desktop-database "$APPS" 2>/dev/null || true
echo "Listo: busca «NicoStation» en el lanzador, o ejecuta nicostation-editor."
