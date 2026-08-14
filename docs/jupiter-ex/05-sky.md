# Небо и sky overlay

## 1. Scope

Отдельный view неба **до** мира и overlay **после** мира. Не Pointman cubemap-on-empty-depth. Не `DrawSkyPortals` (default 0).

## 2. Ownership

| Слой | Модуль |
|---|---|
| Объекты / флаги | ObjectDLL SkyPointer/SkyCamera; ClientFX `EFXSkySetting` |
| Pass | `FEAR.exe` `0x00510ad0` → `0x00518a70` / `0x00518c70` |
| Камера неба | `0x00518860` |
| Viewport AABB | `0x005187d0` + `0x00508420` |
| Draw | тот же `0x00510680` |

## 3. Inputs

- Cvar `DrawSky` value `DAT_0056d8d4` (default 1). (**Ghidra**)
- У пакета основной камеры `+0x2c4` — объект SkyCamera; `==0` → skip. (**Ghidra**)
- `FLAG2` dword объекта `+0x5c`: bit 1 = `FLAG2_SKYOBJECT`, bit 8 = `FLAG2_SKYOVERLAYOBJECT`. (**Ghidra** + **SDK** `ltbasedefs.h`)
- Cvar `SkyFarZ` value `DAT_0056d9ac`, default **10000.0**. Имя `"SkyFarZ"` at `0x0055ece4`. (**Ghidra**)
- Near для sky viewport: литерал `0.01` (`0x3c23d70a`). (**Ghidra**)
- Sky ambient: `Light_SkyAmbientR/G/B` + `Light_AddAmbient`, clamp 0..1, в пакете `0x00518860`. **Без** Low/Med/High. Не смешивается в world Ambient. (**Ghidra**)
- WorldProperties: `SkyAmbientLight`, `SkyFogEnable/NearZ/FarZ` (**SDK**). Intro SkyAmbient **0,0,0**. Skybox DIP: `skybox.fx` = `texCUBE(-N) * vTint * vObjectColor`, **не** SkyAmbient. Связь WorldProperties→cvar в exe: **partial**.

## 4. Алгоритм

Мировой кадр `0x00510ad0`:

1. Gather основного вида: `0x00518dd0(..., MainViewFilter @ 0x00510020)`.
2. `0x00518a70` — sky. Возврат «успех» (1) или skip (0).
3. Основной `0x00510680` (мир).
4. Если шаг 2 успешен: `0x00518c70` — overlay.

### `0x00518a70` (sky)

1. Если `DrawSky==0` → return 0.
2. Если `camera+0x2c4==0` → return 0.
3. `0x005187d0`: экранный AABB sky-объектов (min/max, старт `FLT_MAX` / `-FLT_MAX`).
4. Если ширина `< 1` или высота `< 1` (пиксели) — не рисовать; overlay тоже не вызывается.
5. `0x00508420(mainCamera, outPacket, 0.01, SkyFarZ, aabb)` — viewport из AABB + near/far.
6. Device wrapper `DAT_00576ff0+0xc0` GetViewport, `+0xbc` SetViewport на sky rect.
7. Gather `0x00518dd0(..., SkyObjectFilter @ 0x005186a0)` в бакеты `this+0x30/40/50`.
8. `0x00510680` на sky-камере (`0x00518860`).
9. Restore viewport; очистить бакеты; return 1.

### `0x00518c70` (overlay)

Тот же builder `0x00518860`, **без** смены viewport в этой функции. Gather с `SkyOverlayFilter @ 0x005186b0`. Снова `0x00510680`.

### Фильтры (`FLAG2` at `object+0x5c`)

| Функция | Адрес | Правило |
|---|---|---|
| `SkyObjectFilter` | `0x005186a0` | `(flags >> 1) & 1` → `FLAG2_SKYOBJECT` |
| `SkyOverlayFilter` | `0x005186b0` | `(flags >> 8) & 1` → `FLAG2_SKYOVERLAYOBJECT` |
| `MainViewFilter` | `0x00510020` | reject если `flags & 0x102` (sky **или** overlay) |

## 5. Constants

| Имя | Значение | Тег |
|---|---|---|
| `DrawSky` default | 1 | **Ghidra** |
| `DrawSkyPortals` default | 0 | **Ghidra** — не этот pass |
| Sky near | 0.01 | **Ghidra** |
| `SkyFarZ` default | 10000.0 | **Ghidra** |
| Min AABB | 1×1 | **Ghidra** |
| SkyObject slots | 0..7 indexed order | **SDK** |

## 6. State tables

Sky pass наследует dispatcher `0x00510680` (Ambient→lights→…). Viewport другой; depth/blend — как у мира, пока capture не покажет иное.

Device: Get/SetViewport через vtable `+0xc0` / `+0xbc` обёртки `DAT_00576ff0`. (**Ghidra**; имена Get/SetViewport — **hypothesis** по роли, не строка экспорта.)

## 7. Псевдокод

```text
fn world_frame(cam):
    lists = gather(cam, MainViewFilter)          # reject FLAG2_SKYOBJECT|SKYOVERLAY
    sky_ok = false
    if DrawSky and cam.sky_camera != 0:
        aabb = screen_aabb(sky_objects)
        if aabb.w >= 1 and aabb.h >= 1:
            vp = map_aabb_to_viewport(aabb, near=0.01, far=SkyFarZ)
            old = GetViewport()
            SetViewport(vp)
            sky_lists = gather(sky_cam, SkyObjectFilter)
            dispatch_00510680(sky_cam, sky_lists)
            SetViewport(old)
            sky_ok = true
    dispatch_00510680(cam, lists)
    if sky_ok:
        overlay_lists = gather(sky_cam, SkyOverlayFilter)
        dispatch_00510680(sky_cam, overlay_lists)
```

### `skybox.fx` cubemap (**archive** `Shaders/rigid/Solid/skybox.fx`)

Только technique `Ambient`. VS: clip = transform(skinned pos); `TexCoord = -Normal`. PS: `texCUBE(diffuse, dir) * vTintColor * vObjectColor`. Sampler WRAP. Pass: `AlphaTestEnable=False`, `SrcBlend=One`, `DestBlend=Zero` (replace). vs_1_1 / ps_1_1. Диск: `Sky_Day_C.dds` 1024² DXT1 cube Levels=2 → `CreateCubeTexture` (call 19745). Cloudplane = 2D DXT5. Это **не** empty-depth cubemap Pointman.

### `sky.fx` overlay (**archive** `Shaders/rigid/Solid/sky.fx`)

Technique **Translucent** (не Ambient). AlphaTest ref **96**, SrcBlend One / DestBlend Zero. PS: `tex2D * GetLightDiffuseColor()`. Когда движок биндит overlay-объекты на `sky.fx` vs геометрию — **partial** (фильтр FLAG2 доказан, материал — нет).

## 8. Edge cases

- Нет SkyCamera / `DrawSky=0` → нет неба и **нет overlay**.
- AABB < 1 px → skip sky+overlay.
- Sky objects: `FLAG_NOTINWORLDTREE | FLAG2_SKYOBJECT` после SkyPointer (**SDK**).
- Pointman cubemap-on-empty-depth **не** этот контракт.

## 9. Evidence

| Claim | Source |
|---|---|
| Порядок sky → world → overlay | **Ghidra** `0x00510ad0` |
| `0x00518a70` = sky + viewport | **Ghidra** control flow, filters, Get/SetViewport |
| Filters = FLAG2 bits | **Ghidra** `0x005186a0/b0/0x00510020` + **SDK** FLAG2 |
| `SkyFarZ=10000`, near=0.01 | **Ghidra** `DAT_0056d9ac`, литерал |
| Sky ambient cvars | **Ghidra** `0x00518860` |
| Indexed SkyObject0..7 | **SDK** `ltengineobjects.cpp` |
| ClientFX sky/overlay | **SDK** `ClientFXSkyUtils.h` |
| Viewport/sky depth в кадре | **capture** Present 10987749: AABB viewport 410×0 298×346; near **0.01 в PROJECTION `_43`**, не `Viewport.MinZ` (тот 0..1); затем `Clear Z\|STENCIL` без TARGET |

## 10. Known unknowns

- Точная формула `0x00508420` / `0x00513ac0` (world→screen AABB) — есть декомпил, нужна сверка единиц.
- Parallax SkyCamera vs main: `0x00518860` копирует `+0x298/+0x2b0/+0x2f0`; полная матрица **partial**.
- Sky fog vs world fog в этом pass.
- Геометрия SkyCube vs `skybox.fx`: cubemap по **-Normal**, только Ambient, opaque blend One/Zero (**archive**). Parallax камеры всё ещё **partial**.
- Capture: viewport/stencil/clear вокруг sky — **закрыто**: AABB-viewport **перед** sky, Clear Z/Stencil **после** sky, до world Ambient. Overlay-viewport после мира в Present 10987749 **нет**: хвост — 2 world Translucent additive + HUD на 1280×720, не AABB 298×346.

## 11. Acceptance

- Объект только с `FLAG2_SKYOBJECT` не в основном виде.
- Overlay только после мира и только если sky pass не skip.
- Sky viewport ≥1 px. Near 0.01 — в **проекции** (`_43`), viewport depth 0..1. Far в этом кадре бесконечный (`_33=_34=1`), не обязательно `Viewport.MaxZ=SkyFarZ`.
- Golden: двор Intro, небо не серое; облака — sky objects, не G-buffer empty-depth.
- Capture: SetViewport до world draws; второй gather FLAG2 overlay после мира.

## 12. Status

`verified-capture` для sky-before-world (AABB viewport + proj near 0.01 + Clear Z/Stencil). Overlay `0x00518c70` после мира — всё ещё **verified-static** (в этом Present нет второго AABB).
