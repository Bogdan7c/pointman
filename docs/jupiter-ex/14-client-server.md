# Клиент и сервер

Jupiter EX — не «один процесс рисует всё». Движок в `FEAR.exe` грузит два шелла.

## Контракт (**SDK**)

| Сторона | Интерфейс | Розничный PE | Игровой вход |
|---|---|---|---|
| Клиент | `IClientShell` | `GameClient.dll` | `CGameClientShell` |
| Сервер | `IServerShell` | `GameServer.dll` | `CGameServerShell` |

Тик: `PreUpdate` / `Update` / `PostUpdate`. Клиент: `OnEngineInitialized`, `OnEnterWorld`. Сервер: `OnClientEnterWorld`, `LoadWorld` зовёт движок. (**SDK**) Retail CF разобран по RTTI/ndisasm: `PreUpdate 0x10052030` → `Update 0x100590a0` → `UpdatePlaying 0x10058a10` → `RenderCamera 0x10056e50` → Flip в `PostUpdate 0x1007f5b0` ([21-client-frame.md](21-client-frame.md)). Строк `RenderCamera`/`Start3D`/`FlipScreen` в DLL нет (vtable `ILTClient`). `FlipScreen` в `PostUpdate` — **SDK**, не capture. Кто в exe зовёт shell tick — **unknown**.

SP-запуск уровня: `ConsoleRunWorld` `FUN_10050fe0` → `SetupServerSinglePlayer` → `StartGameFromLevel`; cvar `runworld` (**Ghidra** GameClient). Intro этим путём заходит (**capture**).

Общее: `Game/Shared` — `MsgIDs.h`, `SFXMsgIds.h`, CREATESTRUCT'ы снарядов/пикапов/взрывов, animation tree.

## Кто что делает (строки DLL)

**Клиент:** весь `CHUD*`, `CPlayerMgr::OnCommandOn(COMMAND_ID_SLOWMO)`, `CHUDSlowMo`, `CClientWeapon`, FX (PolyGrid, RenderTarget, decals), GUI. `SFXMgr` — **SDK** класс (строки в PE нет), не строковая улика.

**Сервер:** `CPlayerObj` (story mode), `CServerMissionMgr`, раунды/карта, `CAI*` / NavMesh, `CCommandMgr`, world plugins (`GameStartPoint`, `KeyFramer`, `ActiveWorldModel`).

**Оба:** CREATESTRUCT'ы, `AnimationTreePackedMgr`, projectile/pickup.

**Только exe:** `CLT*` тела, D3D9, PhysicsBSP, консоль, UDP/`SendToClient`, PunkBuster hooks.

## Сеть (**SDK** + exe)

`ILTServer::SendToClient` / `SendToServer`, unguaranteed data, `ILTClientContentTransfer` / `ILTServerContentTransfer`. Клиент: `ClientConnectionMgr`. Сервер: `ServerVoteMgr`, `KickClient`.

Для фазы 1 кампании сеть не нужна. Для XP/MP — отдельная фаза ROADMAP, не закрываем картинкой Intro.

## SFX

Визуальные эффекты на клиенте по сообщениям сервера (`SendSFXMessage` в exe — **адрес CF unknown**). IDs и caps — [28-clientfx-hud.md](28-clientfx-hud.md). Volumetric light, decals, polygrid — клиентские объекты с CREATESTRUCT, не второй отложенный рендер.
