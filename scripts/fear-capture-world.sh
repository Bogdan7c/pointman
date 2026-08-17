#!/usr/bin/env bash
# Снять D3D9-trace Factory (мир, не меню).
# Ждём DrawIndexed с NumVertices > MIN_VERTS в ХВОСТЕ trace (после загрузки/брифинга).
# Важно: пользователь должен нажать клавишу на "Press Any Key" и ПОИГРАТЬ/постоять в мире,
# чтобы появились большие меши. Скрипт детектит их и дописывает запись.
set -euo pipefail

GAME="${GAME:-$HOME/.local/share/Steam/steamapps/common/FEAR Ultimate Shooter Edition}"
PROTON="${PROTON:-$HOME/.local/share/Steam/steamapps/common/Proton - Experimental/proton}"
PFX="${STEAM_COMPAT_DATA_PATH:-$HOME/.local/share/Steam/steamapps/compatdata/21090}"
OUT="${OUT:-$HOME/projects/pointman/local/ghidra/traces}"
WRAP="${WRAP:-$HOME/projects/pointman/local/tools/apitrace/win32/apitrace-14.0-win32/lib/wrappers/d3d9.dll}"
APIT="${APIT:-$HOME/projects/pointman/local/tools/apitrace/apitrace-14.0-Linux/bin/apitrace}"
DESK="$PFX/pfx/drive_c/users/steamuser/Desktop/FEAR.trace"
SNAP="$OUT/world-live-snap.trace"
WORLD="${WORLD:-Worlds\\Release\\Factory}"
RECORD_AFTER_DRAW_S="${RECORD_AFTER_DRAW_S:-40}"
LOG="$OUT/capture-world.log"
MIN_VERTS="${MIN_VERTS:-5000}"
# Пропускаем первые N вызовов: загрузка/меню/брифинг до мира (Factory ~2-4M).
SKIP_CALLS="${SKIP_CALLS:-2000000}"

mkdir -p "$OUT"
exec > >(tee -a "$LOG") 2>&1

cleanup() {
  rm -f "$GAME/d3d9.dll"
}
trap cleanup EXIT

echo "capture-world start $(date -Iseconds) world=$WORLD min_verts=$MIN_VERTS skip_calls=$SKIP_CALLS"
rm -f "$DESK" "$SNAP"
cp -f "$WRAP" "$GAME/d3d9.dll"

export STEAM_COMPAT_DATA_PATH="$PFX"
export STEAM_COMPAT_CLIENT_INSTALL_PATH="${STEAM_COMPAT_CLIENT_INSTALL_PATH:-$HOME/.local/share/Steam}"
export SteamAppId=21090
export SteamGameId=21090
export DISPLAY="${DISPLAY:-:0}"
export WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-wayland-0}"
export DXVK_CONFIG="${DXVK_CONFIG:-d3d9.forceWindowed = True; d3d9.dialogBoxMode = True; d3d9.countLosableResources = False; d3d9.maxFrameRate = 60}"
export WINEDLLOVERRIDES="d3d9=n,b"
unset PROTON_USE_WINED3D

cd "$GAME"
"$PROTON" run FEAR.exe \
  +DisableMovies 1 +NoMovies 1 +Windowed 1 \
  +ScreenWidth 1280 +ScreenHeight 720 \
  +runworld "$WORLD" &
WPID=$!
echo "proton pid $WPID"
echo ">> Жди 'Press Any Key' в окне игры и нажми пробел/Enter. Потом играй/стой в мире."

found=0
for n in $(seq 1 240); do
  if ! kill -0 "$WPID" 2>/dev/null; then
    echo "proton exited early"; break
  fi
  if [ -f "$DESK" ]; then
    SZ=$(stat -c%s "$DESK")
    if cp -f "$DESK" "$SNAP" 2>/dev/null; then
      DUMP=$(mktemp)
      if "$APIT" dump --calls="$SKIP_CALLS-" "$SNAP" >"$DUMP" 2>/dev/null; then
        BIG=$(grep -oE 'NumVertices = [0-9]+' "$DUMP" | awk -F'= ' -v m="$MIN_VERTS" '$2 > m' | wc -l || true)
        PRES=$(grep -c 'IDirect3DDevice9::Present' "$DUMP" || true)
        echo "t=$n size=$SZ big_dip=$BIG present_after_skip=$PRES"
        if [ "$BIG" -gt 0 ]; then
          echo "WORLD FOUND (big DIP), recording ${RECORD_AFTER_DRAW_S}s"
          found=1
          sleep "$RECORD_AFTER_DRAW_S"
          rm -f "$DUMP"
          break
        fi
      fi
      rm -f "$DUMP"
    fi
  fi
  sleep 3
done

if [ "$found" -eq 1 ] && [ -f "$DESK" ]; then
  STAMP="fear-world-$(basename "$WORLD" | tr '\\' '-')-$(date +%Y%m%d-%H%M%S).trace"
  cp -f "$DESK" "$OUT/$STAMP"
  rm -f "$OUT/fear-world.trace"
  ln -s "$OUT/$STAMP" "$OUT/fear-world.trace"
  echo "copied $OUT/$STAMP"
else
  echo "WORLD NOT FOUND, trace at $DESK"
fi
echo "capture-world done $(date -Iseconds) found=$found"
