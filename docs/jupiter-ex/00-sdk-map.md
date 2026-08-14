# Карта публичного SDK 1.08

Источник: **SDK** `/home/bogdan/src/Fear-SDK-1.08`. В репозиторий Pointman SDK не кладём.

`engine/` в SDK — **только** `sdk/inc/` (126 заголовков). Реализации рендера, загрузчика World00p и D3D9-устройства нет. Их даёт розничный `FEAR.exe` (**Ghidra**).

## Фасады

| Интерфейс | Заголовок | Кто реализует | Смысл |
|---|---|---|---|
| `ILTCSBase` | `iltcsbase.h` | `FEAR.exe` (`CLTClient`/`CLTServer`) | Общее: объекты, свет, `IntersectSegment`, таймеры, под-API |
| `ILTClient` | `iltclient.h` | exe | Клиент: renderer, drawprim, текстуры, listener |
| `ILTServer` | `iltserver.h` | exe | Сервер: `LoadWorld`, create/remove, сообщения, `SetSkyCamera` |
| `IClientShell` | `iclientshell.h` | **GameClient.dll** | Движок зовёт игру: init, update, render hooks |
| `IServerShell` | `iservershell.h` | **GameServer.dll** | Движок зовёт игру: мир, клиенты, симуляция |

Игра не рисует кадр сама: `CGameClientShell::RenderCamera` зовёт `ILTRenderer::Start3D` / `RenderCamera` / `End3D` / `FlipScreen`. (**SDK**)

## Подсистемы на `ILTCSBase`

| Интерфейс | Заголовок | Владеет |
|---|---|---|
| `ILTRenderer` | `iltrenderer.h` | 3D кадр, материалы `HMATERIAL`, RT, `RenderCamera` |
| `ILTDrawPrim` | `iltdrawprim.h` | 2D HUD/debug |
| `ILTCustomRender` | `iltcustomrender.h` | Своя геометрия (FX, небо) |
| `ILTTextureMgr` | `ilttexturemgr.h` | Текстуры |
| `ILTPhysics` | `iltphysics.h` | Legacy: dims, `MoveObject`, stairs |
| `ILTClientPhysics` | `iltphysics.h` | `UpdateMovement` на клиенте |
| `ILTPhysicsSim` | `iltphysicssim.h` | Rigid body / WorldModel shapes (Havok-era API) |
| `ILTModel` | `iltmodel.h` | Сокеты, ноды, трекеры анимации |
| `ILTSoundMgr` / `ILTClientSoundMgr` | `iltsoundmgr.h` | Звук, listener, occlusion |
| `ILTCommon` | `iltcommon.h` | Аттачи, флаги, `CreateMessage` |
| `ILTInput` | `iltinput.h` | Устройства (не висит на ILTClient, `define_holder`) |
| `ILTTimer` | `ilttimer.h` | Таймеры и slow-mo scale |
| `ILTFileMgr` / `ILTResourceMgr` | `iltfilemgr.h`, `iltresourcemgr.h` | Файлы и ресурсы |

Типы объектов и флаги: `ltbasedefs.h` (`OT_*`, `FLAG_*`, `EEngineLightType`, `EEngineLOD`).

Счётчики кадра (имена проходов, не порядок): `rendererframestats.h` — отдельно World / Model / WorldModel / Shadow, и по типам ламп Point / Fill / Spot / Cube / Dir / BlackLight. (**SDK**)

## Игровой код `Game/`

```
Game/ClientShellDLL/   IClientShell, HUD, SFX, PlayerMgr, PlayerCamera
Game/ObjectDLL/        IServerShell, AI*, WorldModel, Light*, миссии
Game/Shared/           БД, movement, физика персонажа, MsgIDs
Game/ClientFxDLL/      частицы / спрайты
```

Это **исходники игры**, не движка. Розница компилирует их в `GameClient.dll` / `GameServer.dll`.

## Чего в SDK нет

- Реализация D3D9, порядок light pass, stencil volumes.
- Парсер World00p / BSP.
- Тела `GetLitPixelColor` (`dx9lights.fxh` нет и в Arch00).
- Заголовки `ILTLightAnim`, `ILTTexInterface`, `ILTFontManager`, `ILTWidgetManager` (только forward declare).
- Ассеты и шейдеры.

Порядок проходов — **Ghidra** + **захват**. Формулы света — **архив** `.fxo` (см. [02-lights.md](02-lights.md)).
