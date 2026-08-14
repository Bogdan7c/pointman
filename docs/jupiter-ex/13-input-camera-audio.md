# Ввод, камера, звук

## Ввод (**SDK** + exe)

`ILTInput` не висит на `ILTClient`: игра берёт `define_holder(ILTInput, g_pLTInput)`. Методы: `GetDeviceObjectValue`, `GetDeviceValues`, `FindDeviceByName`. (**SDK** `iltinput.h`)

Exe: `ILTInput.Default`, `ILTCursor.Default`, `CLTCursor`, ошибки буфера DirectInput. Импорт `DirectInput8Create`. (**Ghidra**)

Клиент: бинды в `BindMgr` / `ProfileMgr`, lean, vehicle mouse. Геймпад Pointman — схема Xbox 360 SKU, не «современный шутер».

## Камера

Игра: `CPlayerCamera` (`PlayerCamera.h`) — FP / chase / cinematic, `Render()`. Цикл: `PlayerMgr` + `CGameClientShell::RenderCamera`. (**SDK**)

Движок: `ILTRenderer::RenderCamera(HLOCALOBJ | transform+FOV+viewport)`, `WorldPosToScreenPos`, `GetListener`. Небо: `SetSkyCamera` / `GetSkyCameraTransform`. (**SDK**)

Контрольная точка сверки кадра Intro — `GameStartPoint00`, не закрытая фаза игрока.

Player FOV Intro: Y **45°**; X = `2·atan(tan(Y/2)·1280/720)` ≈ **72.7°** — коэффициент матрицы `_11 = _22 × 720/1280` это cot-скейл, **не** отношение FOV (**capture**). `FovYInterface=75` + `FovAspectRatioScaleInterface=1` — **interface camera**, один раз в `CInterfaceMgr::Init`, не игрок.

Unknown: ADS/recoil; переключение FP/chase/cinematic; связь `GetListener` с audio listener.

## Звук

Движок: `ILTSoundMgr::PlaySound` / `KillSound`; клиент `ILTClientSoundMgr` — 3D init, listener, reverb, occlusion, filter. (**SDK**)

Exe: `CLTSoundMgrServer`, `ILTSoundMgr.Client/Server`, лимиты `SoundMaxPlayerWeaponSoundLimit`. (**Ghidra**)

Игра: `CClientSoundMgr` / `CServerSoundMgr`, `SoundDB`, occlusion/filter/mixer DB, lipsync (`LIPSYNC_FILE_ID`), `PlayBroadcast`. Колбэки шелла: `IClientShell::OnPlaySound`, `OnGetOcclusionFromPoly`. (**SDK** + строки DLL)

Unknown: конкретные mixer/reverb presets; лимиты одновременных звуков кроме weapon cvar; полный lipsync pipeline.
