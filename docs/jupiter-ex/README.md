# Jupiter EX — контракт оригинального движка

Это не «как нарисовать Intro похоже». Это карта **всего** розничного Jupiter EX: кадр, свет, физика, объекты, ИИ, ввод, звук, клиент/сервер. Pointman потом совпадает **по поведению**, не копируя D3D9 pass-for-pass.

Закрытый код живёт в розничном `FEAR.exe` (SteamStub). Игровая логика — в `GameClient.dll` / `GameServer.dll`. Публичный контракт — Fear-SDK-1.08 (`engine/sdk/inc` + `Game/`). Это не слив LithTech и не NOLF2 (`jsj2008/lithtech`).

## Метки источников

| Метка | Значит |
|---|---|
| **SDK** | Fear-SDK-1.08 |
| **archive** / **архив** | Arch00 `.fx`/`.fxo`/Mat00/World00p |
| **Ghidra** | unpacked PE, адрес+имя |
| **capture** / **захват** | D3D9 apitrace до Present |
| **empirical** / **dump-draw** | parse Intro / SHA / reztool |
| **hypothesis** / **догадка** | не факт |

## Три слоя бинарника

```
FEAR.exe          закрытый движок: CLTRenderer, CLTPhysics*, D3D9, ILT*
                  SteamStub Variant 2.1; для Ghidra нужен unpacked (.text entropy ~6.5)
GameClient.dll    IClientShell: HUD, оружие, SFX, ввод, камера игрока
GameServer.dll    IServerShell: мир, ИИ/NavMesh, CommandMgr, миссии
```

Распаковка: Steamless CLI, локально `local/tools/steamless/` (gitignore). В git не кладём PE, `.fx`, декомпил, `.gpr`.

## Как читать

### Кадр и свет

1. [01-frame.md](01-frame.md) — проходы кадра (`0x00510680`: Ambient → лампы+тени → Translucent → BlackLight), `ILTRenderer`, SteamStub.
2. [02-lights.md](02-lights.md) — типы ламп, `NUM_POINT_FILL_LIGHTS=3`.
3. [03-materials.md](03-materials.md) — Mat00 и `.fx`.
4. [04-shadows.md](04-shadows.md) — stencil volumes + blur.
5. [05-sky.md](05-sky.md) — `0x00518a70` sky + viewport; `0x00518c70` overlay.
6. [06-world-draw.md](06-world-draw.md) — bake vs WorldModel vs PhysicsBSP; WM/model bind (`0x0051ebf0` / `0x0051f200`).
7. [07-gap.md](07-gap.md) — Pointman сейчас vs оригинал.

### Движок целиком

8. [00-sdk-map.md](00-sdk-map.md) — индекс `ILT*` / `I*` и чего в SDK нет.
9. [08-engine-shell.md](08-engine-shell.md) — кто в каком PE, SteamStub, Ghidra.
10. [09-physics.md](09-physics.md) — `ILTPhysics` / Sim, BSP, персонаж.
11. [10-game-objects.md](10-game-objects.md) — WorldProperties, спавн, WorldModel, лампы как объекты.
12. [11-animation.md](11-animation.md) — `ILTModel`, деревья анимации.
13. [12-messaging.md](12-messaging.md) — `ObjectCreateStruct`, сообщения, CommandMgr.
14. [13-input-camera-audio.md](13-input-camera-audio.md) — ввод, камера, звук.
15. [14-client-server.md](14-client-server.md) — шеллы, тик, SFX.
16. [15-ai.md](15-ai.md) — GOAP-планировщик в SDK, NavMesh/цели в сервере.
17. [16-weapons-player.md](16-weapons-player.md) — оружие, slow-mo, HUD, story mode.

Наблюдение карты: `cargo run -p reztool -- dump-draw <game-root>`.

Ghidra: skill `pointman-ghidra`. Headless только на **распакованный** `FEAR.unpacked.exe`.

### Coverage и evidence

18. [render-coverage.md](render-coverage.md) — матрица статуса всего рендера.
19. [20-evidence.md](20-evidence.md) — SHA, image base, Arch00, capture recipe, cvar gates.
20. [21-client-frame.md](21-client-frame.md) — Start3D…End3D compose, clear.
21. [22-world00p.md](22-world00p.md) — layout baked render.
22. [23-model00p.md](23-model00p.md) — packed model (`MODL`/0x21); verts/VB/IB/shadow-меш закрыты.
23. [24-materials.md](24-materials.md) — инвентарь shader families.
24. [25-capture-manifest.md](25-capture-manifest.md) — как снимать traces.
25. [26-acceptance.md](26-acceptance.md) — golden/fixtures plan.
26. [27-visibility-sort.md](27-visibility-sort.md) — gather, FLAG2-фильтры, opaque/translucent sort keys.
27. [28-clientfx-hud.md](28-clientfx-hud.md) — SFX IDs, ClientFX, HUD/DrawPrim, RT (**SDK**).
