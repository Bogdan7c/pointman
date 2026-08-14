# Оболочка движка: кто в каком PE

## Слои (**Ghidra** строки + размеры PE)

| Файл | Размер | `.text` entropy | Роль |
|---|---|---|---|
| `FEAR.exe` packed | 1.9 МБ | **8.0** + секция `.bind` | SteamStub 2.1 поверх движка |
| `FEAR.exe` unpacked | 1.6 МБ | **6.51** | `CLT*` реализации, D3D9, физика, модели, звук, сеть |
| `GameClient.dll` | 1.8 МБ | 6.53 | Игровой клиент, unpacked |
| `GameServer.dll` | 2.4 МБ | 6.56 | Игровой сервер, unpacked |
| `EngineServer.dll` | ~1 МБ | — | retail dedicated-server шелл; в `local/` нет, не разобран (SHA — [20-evidence.md](20-evidence.md)) |

EP packed: `.bind` `0x0059d2ed`. EP unpacked: `.text` `0x0053e428`. (**Ghidra** / PE)

Импорты exe: `Direct3DCreate9`, `D3DXCreateEffect`, `d3dx9_27.dll`, `DirectInput8Create`. DrawIndexedPrimitive в IAT нет — это COM vtable устройства. (**Ghidra**)

## SteamStub

Розничный Steam `FEAR.exe` (app `21090`) упакован **SteamStub Variant 2.1**. Статика по packed `.text` врёт: 262 «функции», ноль xrefs на `ShadowVolume`.

Распаковка только локально: Steamless 3.1.0.5 CLI → `FEAR.exe.unpacked.exe`. Не кряк SteamAPI, только снятие стаба для анализа. PE в git не кладём.

Ghidra-проект: `local/ghidra/`. Headless MCP обязан грузить **unpacked**, не `.bind`.

## Имена в exe (**Ghidra** строки)

Движок регистрирует интерфейсы строками `ILTRenderer.Default`, `ILTDrawPrim.Default`, `ILTInput.Default`. Реализации — `CLTClient`, `CLTServer`, `CLTRenderer`, `CLTPhysicsClient` / `CLTPhysicsServer`, `CLTModel*`, `CLTSoundMgr*`, `CLTFileMgr`, `CLTResourceMgr`, `CLTCustomRender`.

Консольные имена света: `Light_Point`, `Light_PointFill`, `Light_SpotProjector`, `Light_CubeProjector`, `Light_Directional`, `Light_BlackLight`, `Light_ShadowVolume`, `Light_ShadowBlur`, `Light_AmbientR/G/B`.

Техники материалов как строки: `Ambient`, `Point`, `PointFill`, `SpotProjector`, `CubeProjector`, `Directional`, `ShadowVolume`, `DirectionalShadowVolume`.

## Игра vs движок

Клиент/сервер DLLs **не** создают D3D9 device. Они зовут `ILT*` и реализуют шеллы. Строки HUD/оружия — клиент; NavMesh/AINode/`CCommandMgr` — сервер. Подробности: [14-client-server.md](14-client-server.md).

## Инструменты (gitignore `local/`)

- Ghidra 12.1.2 + JDK 21: `/home/bogdan/src/`
- MCP: `.cursor/mcp.json` → `bridge-mcp-ghidra`. Unpacked проект HTTP **`127.0.0.1:8090`** (packed 8089 — мёртвый: 262 функции). `load_program_from_project` — JSON `{"path":"/FEAR.unpacked.exe"}`
- Steamless: `local/tools/steamless/` через Proton wine (`local/tools/wine/wine` → Proton Experimental `wine-11.0`)
- Apitrace 14: Linux `local/tools/apitrace/apitrace-14.0-Linux/bin/apitrace`, Win32 wrapper `.../win32/apitrace-14.0-win32/lib/wrappers/d3d9.dll`. Системный `pacman -S wine apitrace` не нужен. Захват: `WINEDLLOVERRIDES=d3d9=n,b` + `PROTON_USE_WINED3D=1`, не DXVK как истина картинки.
