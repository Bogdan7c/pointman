# Клиентский кадр (GameClient → ILTRenderer)

## 1. Scope

Сборка кадра **игрой**, не D3D9 pass list. Retail `GameClient.dll` (image `0x10000000`, sha256 `38604fbf…7db8`): RTTI + ndisasm. Строк `RenderCamera`/`Start3D`/`FlipScreen` нет — vtable. `g_pLTClient = 0x101a5418`, `g_pLTRenderer = 0x101a541c`. `GetRenderer` = `ILTClient+0x16c`.

## 2. Ownership

| Слой | Модуль |
|---|---|
| Compose | `CGameClientShell` (`GameClient.dll`) |
| Камера / clear / RenderCamera | `CPlayerCamera` |
| RT до мира | `CSFXMgr::UpdateRenderTargets` |
| FX после мира | `CSFXMgr::RenderFX` |
| HUD | `CInterfaceMgr::Draw` → `ILTDrawPrim` |
| D3D9 passes | `FEAR.exe` `ILTRenderer` |

## 3. Inputs

`HLOCALOBJ` камеры игрока, FOV (`GetCameraFOV`, default π/2 **SDK**; Intro capture: Y=**45°** — [13-input-camera-audio.md](13-input-camera-audio.md)), transform, focus окна (`m_bMainWindowFocus`).

## 4. Алгоритм (**SDK** `GameClientShell.cpp:4108-4149`, `PlayerCamera.cpp:401-428`)

1. Если нет фокуса окна — не рендерить.
2. `ILTRenderer::Start3D()`.
3. `CRenderTargetFX::IncrementCurrentFrame()`.
4. `m_sfxMgr.UpdateRenderTargets(tCamera, vCameraFOV)` — зеркала/RT **до** мира.
5. `CPlayerCamera::Render()`:
   - first person: depth-bias слой оружия/тела, затем общий путь;
   - `ClearRenderTarget(CLEARRTARGET_ALL, 0)` — color+z+stencil, цвет **0**;
   - опционально внутренний RT + `StretchRect` на экран;
   - `RenderCamera(m_hCamera)` без technique override (NULL → exe id `0xf`).
6. `m_sfxMgr.RenderFX(hCamera)` — SFX поверх мира, ещё внутри Start3D/End3D.
7. `GetInterfaceMgr()->Draw()` — HUD/меню DrawPrim.
8. notifiers / streaming HUD / frame-stat console.
9. `RenderConsoleToRenderTarget()`.
10. `End3D()`.
11. `FlipScreen` — **не** в `RenderCamera`; `CInterfaceMgr::PostUpdate` если не `GS_LOADINGLEVEL` (**SDK** `InterfaceMgr.cpp:770-781`).
12. Меню/не-playing: `PreUpdate` может `ClearRenderTarget(ALL, 0)` без мира (**SDK** `InterfaceMgr.cpp:746-757`).

Параметр `RenderCamera(bool bDrawInterface)` в SDK **и** retail **не используется** — `Draw()` всегда (`0x10056e50` push 1). Volumetric: отдельный `RenderCamera(..., "FogVolume_Depth")` / `"Ambient"` (`VolumetricLightFX.cpp`). Pixel-double: half-res RT, выкл если `AntiAliasFSOverSample` (**SDK** `PlayerCamera.cpp:2977-3013`).

Retail `ILTRenderer` слоты этого пути: Clear `+0x10`, Start3D `+0x18`, End3D `+0x1c`, FlipScreen `+0x20`, `RenderCamera(h)` `+0x44`, SetRT `+0x6c`, SetRTScreen `+0x74`, StretchRect `+0x78`.

## 5. Constants

`CLEARRTARGET_ALL = COLOR|ZBUFFER|STENCIL`. Clear color `0`.

## 6. State tables

Не задаёт D3DRS — это exe. Игра задаёт RT binding и clear.

## 7. Псевдокод

```text
if not window_focus: return
renderer.Start3D()
sfx.update_render_targets(camera)
renderer.ClearRenderTarget(ALL, 0)
renderer.RenderCamera(camera, technique=None)
sfx.render_fx(camera)
hud.draw()
renderer.End3D()
# FlipScreen elsewhere (UI)
```

## 8. Edge cases

Нет фокуса — skip. Zoom/ironsight прячет тело. Ladder: не переключать player render layer (**SDK**).

## 9. Evidence

| Claim | Source |
|---|---|
| Compose order | **SDK** + **ndisasm** retail: `PreUpdate 0x10052030` → `Update 0x100590a0` → `UpdatePlaying 0x10058a10` → `RenderCamera 0x10056e50` → `PostUpdate 0x10052060` |
| `CGameClientShell` vftable | **RTTI** `0x10176554` / name `0x1019f6a4` |
| Clear ALL, color 0 | **SDK** + retail `CPlayerCamera::Render` inner `0x100a69b0` |
| `RenderCamera(h)` 1-arg, NULL override | retail `ILTRenderer+0x44` |
| FlipScreen unless `GS_LOADINGLEVEL` | retail `CInterfaceMgr::PostUpdate 0x1007f5b0` |
| exe id 0xf | **Ghidra** 0x004f4c80 |

## 10. Known unknowns

Retail `FUN_10050fe0` = `ConsoleRunWorld`: пустое имя → ошибка; иначе лог, `Database\FEAR.Gamdb00p`, `FUN_1001a000` (SetupServerSinglePlayer), `FUN_10092420` (StartGameFromLevel). Строка cvar `runworld` `0x10175efc`. Слив меню: `+runworld Worlds/Release/Intro`.

RTTI: `CGameClientShell` `0x1019f6a4`, `CInterfaceMgr`, `CHUDMgr` `0x1019fedc`, `CClientFXMgr`. Cvar init `FUN_100828dc` регистрирует `DrawInterface`, `FovYInterface` default **75.0**, `FovAspectRatioScaleInterface` **1.0**, `NoMovies`, LetterBox*. Это **не** compose кадра.

Capture first Present = 2 HUD quads **без** world Draw (меню, `GS_SCREEN`). Intro fat: мир → 2 tech-1 Translucent → HUD DrawPrim → Present. `FovAspectRatioScaleInterface` крутит **только** interface camera X FOV **один раз** в Init, не player 45°. Skip splash movies: `DisableMovies` / `NoMovies`; menu `ScreenMovie` этими cvars **не** гейтится. Bink = exe + `Binkw32`, не GameClient.

## 11. Acceptance

Capture одного кадра Intro: `Clear` (color=0, flags ALL) → world draws → DrawPrim HUD → `EndScene`/`Present`. Synthetic: не применимо без exe.

## 12. Status

`verified-static` (retail CF = SDK compose + мелкие дельты: PreUpdate skip clear ещё при load-state `+0x3800`∈{2,3}). `bDrawInterface` unused. Closure: HTTP 8091 decompile тех же VA — не обязателен для порядка.
