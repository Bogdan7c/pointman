# Evidence pack — бинарники, архивы, воспроизводимость

Теги: **SDK** / **archive** / **Ghidra** / **capture** / **empirical** / **hypothesis**.

## Scope

Идентификация розничного Steam `21090` (F.E.A.R. Ultimate Shooter Edition), соответствие адресов unpacked exe, порядок Arch00, рецепт capture. Не содержит PE.

## Ownership

Steam install: `~/.local/share/Steam/steamapps/common/FEAR Ultimate Shooter Edition`.  
Локальные копии для Ghidra: `local/ghidra/binaries/` (gitignore).

## Binaries (SHA-256, **empirical** 2026-08-13)

| File | SHA-256 | Size | Image base | EP RVA |
|---|---|---|---|---|
| Steam `FEAR.exe` packed | `d5ebc38a4f12b772c9112a2811c290adb6c5052d3bc2f817302d38cf55bb2cbe` | 1978368 | `0x00400000` | `0x0019d2ed` (`.bind` SteamStub 2.1) |
| `FEAR.unpacked.exe` | `7d571b8496e52a80a8e37201a3a7a5bfd77d8acc713d2b7d9ffe9004f01ca411` | 1626112 | `0x00400000` | `.text` `0x0013e428` → VA `0x0053e428` |
| `EngineServer.dll` | `a4bdf165636cb11e5a8375f7eaf7a2920603e3be41576726b5d04ffd415b9325` | 1077248 | `0x10000000` | `0x000da91e` |
| `local/.../GameClient.dll` | `38604fbfbc91d27427368ff93739816b85365eb6d3666990d16ab9a876237db8` | 1851392 | `0x10000000` | `0x0016c084` |
| `local/.../GameServer.dll` | `a5856334d5013bb6d8d49eb40717a799d327e30e1f242346d178c86eb9649c7d` | 2506752 | `0x10000000` | `0x001a49aa` |

Packed Steam `FEAR.exe` ≡ копия в `local/ghidra/binaries/FEAR.exe` (одинаковый SHA).

`GameClient.dll` / `GameServer.dll` **нет в корне Steam-папки** (подгружаются из Arch00). Локальные копии `local/fear-extract/` ≡ `local/ghidra/binaries/` (SHA совпал **empirical**).

Ghidra-адреса в спеке — **только** unpacked `FEAR.unpacked.exe`, image base `0x00400000`. Packed `.bind` не цитировать.

## Archive overlay (**empirical**)

`Default.archcfg` порядок (позже перекрывает раньше):

```
FEAR.Arch00
FEARA.Arch00
FEARL.Arch00
FEARE.Arch00
FEAR_1 … FEAR_8
FEARA_1 … FEARA_8
FEARL_1 … FEARL_8
FEARE_1 … FEARE_8
```

SHA каждого Arch00: `local/ghidra/traces/binary-hashes.txt` (gitignore). В git — этот список имён и правило overlay, не байты.

Steam USE: **36** Arch00. World00p (**27** SP-карт, включая `WTF_*` = Extraction Point) лежат **только** в `FEAR.Arch00` (+ патчи `FEAR_1..8`). `FEARE*` (~4.2M) — скрипты / Fog/Glass props / cvars, **без** миров. `FEARL*` — `Level_*` / `Briefing_*` / `Voice\`. `FEARA*` — sfx/music. Later-wins: `Interface/menu/frame_h.dds` в `FEAR_3` перекрывает `FEAR` (DXT3 → RGB). `Sky_Day_C.dds` в обоих одинаковый. SKU-map кампания vs XP vs XP2 **не закрыт** (R27). Perseus Mandate: миров в этом install нет (только `Sign_Perseus_D.dds`). Traces — только Intro + menu.

## Device init (**capture**, trace truncated, 0 Present)

Рецепт:

1. Steam запущен (steam_api).
2. Скопировать `local/tools/apitrace/win32/apitrace-14.0-win32/lib/wrappers/d3d9.dll` → каталог игры как `d3d9.dll`.
3. `WINEPREFIX=~/.steam/steam/steamapps/compatdata/21090/pfx`
4. `local/tools/wine/wine FEAR.exe` (Proton Experimental wrapper).
5. Обёртка пишет `C:\users\steamuser\Desktop\FEAR.trace`.
6. **Обязательно удалить** `d3d9.dll` из каталога игры после съёма.
7. `PROTON_USE_WINED3D=1` на этом префиксе **не победил**: под native apitrace всё равно поднялся DXVK. D3D9 COM-поток игры при этом валиден как **capture API**; картинка DXVK — не истина stencil.

Снято 2026-08-13 (~70 с, убито на PSO compile): `CreateDevice` 640×480 `A8R8G8B8` + `D24S8`, `MIXED_VERTEXPROCESSING`, `PRESENT_INTERVAL_IMMEDIATE`. RS на старте (leftover init, **не** дефолты D3D9 — у D3D9 `STENCILFUNC=ALWAYS`): cull CCW, Z LESSEQUAL, stencil func EQUAL / enable FALSE. **Present не было** — этот trace не закрывает DoD п.3.

Повтор: не убивать, пока dump не содержит `IDirect3DDevice9::Present`. Кэш DXVK может быть тёплым. Второе окно 180 с (2026-08-13 18:12) снова **init-only** (1005K, 0 Present) — timeout на splash/movies.

Третий заход 2026-08-13 18:22–18:34 (`+DisableMovies 1`, timeout 720 с, wine **124**): trace **застыл на 1028098 байт с t=40с**. Dump: **0** `Present`, **0** `BeginScene`. Последний вызов: `CreateVolumeTexture(64×64×64 A8R8G8B8 MANAGED)` → `LockBox` (SlicePitch=16384 → 1 MiB payload); `UnlockBox` нет. `DXVK_state_cache` пустой. Не «мало ждали» и не порог 2 МБ в скрипте — файла нет роста.

Два `CreateDevice` в том же trace: сначала **Windowed=TRUE** 640×480, затем **Windowed=FALSE** 640×480 (оба A8R8G8B8+D24S8, MIXED VP, IMMEDIATE). Семь `Clear` на init (шесть TARGET color=0; один TARGET|Z|STENCIL color=0, **Z=1**, stencil=0). Куб 128³, RT 640×480 × несколько, volume 64³ — init ресурсов, не кадр мира.

Следующий шаг: прогреть DXVK **без** apitrace `d3d9.dll` до реального кадра, потом снова обёртка. Повтор 12 мин с пустым PSO под wrapper — бесполезен.

Прогрев 2026-08-13 18:36–18:39 (**ошибка рецепта**): `WINEDLLOVERRIDES=d3d9=b` форсит **wined3d**, не DXVK. Timeout 180 с, wine 124. `DXVK_state_cache` пустой, `.dxvk.bin` 0 байт — кэш не грелся. В логе только `ntsync`. Нужен `proton run` (Proton Experimental, compatdata 21090) **без** `d3d9=b`, чтобы поднялся prefix `syswow64/d3d9.dll` (DXVK).

Прогрев windowed 18:42–18:44: `d3d9.forceWindowed` + `dialogBoxMode` — оба `ResetSwapChain` **Windowed=true** (больше нет `640x480@0` exclusive). `.dxvk.bin` **0 → 147K**. В логе старта всё ещё `Cache: 0 shaders` (читали пустой файл). Окно по title `F.E.A.R` за 20 с не нашлось; timeout 90 с. Кэш тёплый — следующий шаг apitrace с теми же DXVK_CONFIG/Windowed.

## Cvar draw-gates (**Ghidra**, `.data` records stride `0x18`, value at `+0x0C`)

| Cvar | name VA | record | value VA | default dword | Consumer |
|---|---|---|---|---|---|
| `DrawTranslucent` | `0x0055e728` | `0x0056d208` | `0x0056d214` | 1 | `0x00517e70` |
| `DrawSky` | `0x0055ec54` | `0x0056d8c8` | `0x0056d8d4` | 1 | `0x00518a70` |
| `DrawFogVolumes` | `0x0055ec80` | `0x0056d910` | `0x0056d91c` | 1 | `0x00517ff0` |
| `DrawWorld` | `0x0055ec48` | `0x0056d8b0` | `0x0056d8bc` | 1 | classify bake `0x0051f550` |
| `DrawModels` | `0x0055ec3c` | `0x0056d898` | `0x0056d8a4` | 1 | classify `OT_MODEL` |
| `DrawWorldModels` | `0x0055ec5c` | `0x0056d8e0` | `0x0056d8ec` | 1 | classify `OT_WORLDMODEL` |
| `DrawCustomRender` | `0x0055ec6c` | `0x0056d8f8` | `0x0056d904` | 1 | classify `OT_CUSTOMRENDER` |
| `VisLock` | `0x0055e7f8` | `0x0056d2f8` | `0x0056d304` | 0 | gather: 0 = копировать frustum каждый кадр |
| `VisDrawFrustum` | `0x0055e800` | `0x0056d310` | `0x0056d31c` | 0 | 0 = world-tree `0x00521370` |
| `VisMaxSectorDepth` | `0x0055e810` | `0x0056d328` | `0x0056d334` | **-1** | глубина tree; <0 → -1 |
| `VisDisableWhenOutside` | `0x0055e824` | `0x0056d340` | `0x0056d34c` | 0 | 1 = не fallback `0x00521440` если tree fail |
| `DrawSkyPortals` | `0x0055e7d8` | `0x0056d2c8` | `0x0056d2d4` | 0 | не `0x00518a70` |
| `SkyFarZ` | `0x0055ece4` | `0x0056d9a0` | `0x0056d9ac` | 10000.0f | sky viewport far `0x00518a70` |

## Client clear (**SDK**)

`CPlayerCamera::RenderCamera`: `ClearRenderTarget(CLEARRTARGET_ALL, 0)` затем `RenderCamera(m_hCamera)`. Цвет 0 = чёрный, флаги color+z+stencil. **SDK** `PlayerCamera.cpp:417-419`. Retail: `PointmanFearGameDlls` (HTTP **8091**) — GameClient.dll + GameServer.dll сохранены `analyzeHeadless` 2026-08-13 20:39 exit 0. HTTP `import_file` на MCP exe (**8090**) может ответить «GUI only» — это **не** запрет импорта: headless `-import` работает.

## Capture4 menu Present (**capture**, 2026-08-13 20:50)

Рецепт, который дошёл до Present (меню, не мир):

1. Proton Experimental `proton run`, compatdata `21090`.
2. `+DisableMovies 1 +NoMovies 1 +Windowed 1 +ScreenWidth 1280 +ScreenHeight 720`.
3. `DXVK_CONFIG=d3d9.forceWindowed = True; d3d9.dialogBoxMode = True`.
4. Native apitrace `d3d9.dll` в каталог игры, `WINEDLLOVERRIDES=d3d9=n,b`.
5. Тёплый `DXVK_state_cache` (~147K).

Canonical snapshot: `local/ghidra/traces/fear-frame.trace` (~5.0G, 20:53). First Present = call **3395**, device `0x277d680`. Dump counts at snapshot: 101566 Present, 120806 BeginScene. Это **меню HUD**, не Intro:

- extra RT **256×256** A8R8G8B8 → Clear TARGET 0 → restore BB
- Clear **ALL** color **0**, **Z=1**, stencil 0 (3147/3151)
- BeginScene 3153 → 2× `DrawIndexedPrimitive` TRIANGLELIST quads, Z off, Cull NONE, SrcAlpha/InvSrcAlpha → EndScene/Present
- FogEnable FALSE

User settings этого прогона: [goldens/settings-capture4.cfg](../../local/ghidra/traces/goldens/settings-capture4.cfg) (gitignore) — 1280×720, Gamma 1, VSync 0, DisableMovies 1.

Мир Intro / двор в snapshot **нет**. Desktop leftover ~103G — не canonical.

## Intro world Present (**capture**, 2026-08-13 22:42)

Рецепт: `scripts/fear-capture-intro.sh`, `+runworld Worlds\Release\Intro` (backslash). Слэши → чёрный Clear-only. После загрузки — **Press Any Key** в окне FEAR (не Cursor).

Canonical: `local/ghidra/traces/fear-intro-20260813-224237.trace` (~645M, 5350 Present, 552913 DrawIndexed). Жирный кадр Present **10987749**: 517 DIP / 555402 треугольника. Порядок vs `0x00510680`: [01-frame.md](01-frame.md).

## Known unknowns

- Overlay sky после мира; FogVolumes/BlackLight на Intro fat. Tech-1 Translucent = 2 additive DIP (не `translucent.fx`).
- Позы двора/туалета/лестницы.
- SHA DLL vs байт внутри Arch00 (копии extract ≡ binaries совпали).
