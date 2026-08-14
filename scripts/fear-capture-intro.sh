#!/usr/bin/env bash
# Снять D3D9-trace Intro без меню и без клавиш из Cursor.
# +runworld → SDK ConsoleRunWorld / retail FUN_10050fe0.
# Обёртка apitrace живёт только на время съёма — иначе Steam падает на потере фокуса.
#
# Не править этот файл, пока съём идёт: bash читает скрипт по строкам, сдвиг ломает его.
set -euo pipefail

GAME="${GAME:-$HOME/.local/share/Steam/steamapps/common/FEAR Ultimate Shooter Edition}"
PROTON="${PROTON:-$HOME/.local/share/Steam/steamapps/common/Proton - Experimental/proton}"
PFX="${STEAM_COMPAT_DATA_PATH:-$HOME/.local/share/Steam/steamapps/compatdata/21090}"
OUT="${OUT:-$HOME/projects/pointman/local/ghidra/traces}"
WRAP="${WRAP:-$HOME/projects/pointman/local/tools/apitrace/win32/apitrace-14.0-win32/lib/wrappers/d3d9.dll}"
APIT="${APIT:-$HOME/projects/pointman/local/tools/apitrace/apitrace-14.0-Linux/bin/apitrace}"
DESK="$PFX/pfx/drive_c/users/steamuser/Desktop/FEAR.trace"
SNAP="$OUT/fear-intro-live-snap.trace"
# Windows-путь: FileMgr и GetWorldName(true) ждут backslash.
# Слэши Worlds/Release/Intro на рознице дают чёрный экран: WorldExists падает,
# а splash/меню не открываются, потому что cvar runworld уже задан.
WORLD="${WORLD:-Worlds\\Release\\Intro}"
# после первого DrawIndexed сколько секунд писать мир
RECORD_AFTER_DRAW_S="${RECORD_AFTER_DRAW_S:-25}"
# загрузка мира под apitrace может минуту чистить экран; 200 Present — рано резать
BLACK_AFTER_PRESENT_S="${BLACK_AFTER_PRESENT_S:-90}"
MAX_TRACE_BYTES="${MAX_TRACE_BYTES:-8000000000}"
LOG="$OUT/capture-intro.log"

mkdir -p "$OUT"
exec > >(tee -a "$LOG") 2>&1

cleanup() {
  rm -f "$GAME/d3d9.dll"
  if pgrep -x FEAR.exe >/dev/null; then
    pkill -TERM -x FEAR.exe || true
    sleep 2
    pkill -KILL -x FEAR.exe || true
  fi
}
trap cleanup EXIT

echo "capture-intro start $(date -Iseconds) world=$WORLD"
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
# кавычки обязательны: иначе bash съест \R, а LithTech может разрезать путь по /
"$PROTON" run FEAR.exe \
  +DisableMovies 1 +NoMovies 1 +Windowed 1 \
  +ScreenWidth 1280 +ScreenHeight 720 \
  +runworld "$WORLD" &
WPID=$!
echo "proton pid $WPID"

found_draw=0
black_abort=0
present_since=""
for n in $(seq 1 50); do
  if [ -f "$DESK" ]; then
    SZ=$(stat -c%s "$DESK")
    echo "trace size $SZ t=${n}x3s"
    if [ "$SZ" -gt "$MAX_TRACE_BYTES" ]; then
      echo "trace exceeded cap, stopping"
      break
    fi
    # живой .trace dump'ать нельзя — последний вызов обрезан, Present «не находится».
    if cp -f "$DESK" "$SNAP" 2>/dev/null; then
      DUMP=$(mktemp)
      if "$APIT" dump --calls=0-25000 "$SNAP" >"$DUMP" 2>/dev/null; then
        PRES=$(rg -c 'IDirect3DDevice9::Present' "$DUMP" || true)
        DRAWS=$(rg -c 'DrawIndexedPrimitive' "$DUMP" || true)
        PRES=${PRES:-0}
        DRAWS=${DRAWS:-0}
        echo "snap Present=$PRES DrawIndexed=$DRAWS"
        if [ "$DRAWS" -gt 0 ]; then
          echo "DrawIndexed FOUND size=$SZ t=$n"
          found_draw=1
          sleep "$RECORD_AFTER_DRAW_S"
          rm -f "$DUMP"
          break
        fi
        if [ "$PRES" -gt 0 ]; then
          if [ -z "$present_since" ]; then
            present_since=$(date +%s)
            echo "first Present in snap at t=$n, waiting up to ${BLACK_AFTER_PRESENT_S}s for DrawIndexed"
          else
            ELAPSED=$(( $(date +%s) - present_since ))
            if [ "$ELAPSED" -ge "$BLACK_AFTER_PRESENT_S" ]; then
              echo "BLACK SCREEN: Present=$PRES DrawIndexed=0 after ${ELAPSED}s (Clear+Present, мира нет)"
              black_abort=1
              rm -f "$DUMP"
              break
            fi
          fi
        fi
      else
        echo "snap dump failed (trace still opening)"
      fi
      rm -f "$DUMP"
    fi
  fi
  if ! kill -0 "$WPID" 2>/dev/null; then
    echo "proton exited early"
    break
  fi
  sleep 3
done

if [ -f "$DESK" ]; then
  STAMP=$(date +%Y%m%d-%H%M%S)
  DEST="$OUT/fear-intro-$STAMP.trace"
  cp -f "$DESK" "$DEST"
  echo "copied $DEST $(stat -c%s "$DEST") bytes"
  DUMP=$(mktemp)
  if "$APIT" dump "$DEST" >"$DUMP" 2>/dev/null; then
    PRES=$(rg -c 'IDirect3DDevice9::Present' "$DUMP" || true)
    BEGINS=$(rg -c 'IDirect3DDevice9::BeginScene' "$DUMP" || true)
    DRAWS=$(rg -c 'DrawIndexedPrimitive' "$DUMP" || true)
    echo "Present=${PRES:-0} BeginScene=${BEGINS:-0} DrawIndexed=${DRAWS:-0}"
  else
    echo "final dump failed"
  fi
  rm -f "$DUMP"
  if [ "$found_draw" -eq 1 ]; then
    ln -sfn "$DEST" "$OUT/fear-intro.trace"
  else
    echo "NOT linking fear-intro.trace: no world DrawIndexed (black or init-only)"
  fi
fi

if [ "$black_abort" -eq 1 ]; then
  echo "capture-intro BLACK abort $(date -Iseconds)"
  exit 2
fi
echo "capture-intro done $(date -Iseconds) found_draw=$found_draw"
