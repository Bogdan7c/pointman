# Порядок проходов кадра

Источник закрытого рендера: **Ghidra** (распакованный `FEAR.exe`) + **захват**. Packed SteamStub для проходов бесполезен — см. [08-engine-shell.md](08-engine-shell.md).

## Контракт игры → движок (**SDK**)

Игра не собирает D3D9 pass list. `CGameClientShell::RenderCamera` → `ILTRenderer`:

1. `Start3D`
2. `RenderCamera` (камера или transform+FOV+viewport; опционально `pTechniqueOverride`)
3. `End3D`
4. `FlipScreen`

Между Start/End движок сам гоняет world/models/lights. Счётчики имён (не порядок) — `ERendererFrameStats`: World, WorldShadow, Model, ModelShadow, WorldModel, CustomRender; лампы Point/Fill/Spot/Cube/Dir/BlackLight и их shadow-варианты. (`rendererframestats.h`)

## Факты из шейдеров (**архив**)

- **Forward multi-pass, не deferred.** У `specular.fx` / `.fxo` отдельные technique: `Ambient`, `Point`, `PointFill`, `SpotProjector`, `CubeProjector`, `Directional`.
- Ambient: две выборки + сложение (diffuse + emissive). PointFill — без specular. Остальные — Blinn + затухание, см. [02-lights.md](02-lights.md) и [03-materials.md](03-materials.md).
- **`NUM_POINT_FILL_LIGHTS = 3`**: в `specular.fxo` параметры `vObjectSpaceFillLightPos`, `fInvFillLightRadius`, `vObjectFillLightColor` — массивы длины 3.
- Вектор к лампе уже масштабирован **`fInvLightRadius = 1/radius`**. Кривая в ps_2_0: **`(1 - saturate((d/r)²))²`**, не `(1-d/r)²`.
- Небо: `skybox.fx` technique `Ambient`. Полупрозрачное: `FLAG2_FORCETRANSLUCENT`. FarZ/fog — WorldProperties. Intro: FarZ 100000, fog выкл.

## Ghidra (unpacked `FEAR.unpacked.exe`)

SteamStub 2.1 снят. `.text` entropy 6.51, **6131** функций (packed база: 262 и ноль xrefs — выбросить).

### Цепочка кадра

`ILTRenderer::RenderCamera` живёт в exe (строка `RenderCamera` → `0x004f5020`). Если у объекта байт `+0x57 == 4` (камера), копируются поля камеры и вызывается `0x004f4c80`. Там `Start3D`, сбор пакета, ambient RGB из cvar (`Light_Ambient*` + `Light_AddAmbient`, clamp 0..1), override техники:

- `pTechniqueOverride == NULL` → байт `0xf` («все техники»)
- иначе `0x00503cb0` (имя → id 0..13, неизвестное → `0xf`)

Дальше `0x004ffee0` → мировой кадр `0x00510ad0` → диспетчер `0x00510680`.

`0x00510ad0` перед/после основной воронки:

1. Gather main: `0x00518dd0` + `MainViewFilter` `0x00510020` (reject `FLAG2_SKYOBJECT|SKYOVERLAY`).
2. `0x00518a70` — **sky pass** (**Ghidra**, назначение закрыто): cvar `DrawSky`; SkyCamera `+0x2c4`; экранный AABB; SetViewport (AABB, depth 0..1 — near **0.01** ставится в **PROJECTION** `_43`, не `Viewport.MinZ`; far — cvar `SkyFarZ` default **10000**, см. [05-sky.md](05-sky.md)); gather `SkyObjectFilter` `0x005186a0`; снова `0x00510680`.
3. основной `0x00510680` — мир.
4. если sky success: `0x00518c70` — overlay, filter `0x005186b0` = `FLAG2_SKYOVERLAYOBJECT`.

Не `DrawSkyPortals` (`DAT_0056d2d4` default **0**). Детали: [05-sky.md](05-sky.md), [27-visibility-sort.md](27-visibility-sort.md).

`0x00517ff0` в диспетчере — **FogVolumes**, гейт `DrawFogVolumes` (`DAT_0056d91c`). Translucent — `DrawTranslucent` (`DAT_0056d214`). Таблица cvar: [20-evidence.md](20-evidence.md). (**Ghidra**)

### Слоты technique (таблица `.data` `0x0056dab8`, шаг 8)

Это **id**, которые `0x0050ffc0` передаёт в D3DX effect. Порядок слотов ≠ порядок в кадре.

| id | Имя |
|---|---|
| 0 | `Ambient` |
| 1 | `Translucent` |
| 2 | `Point` |
| 3 | `PointFill` |
| 4 | `SpotProjector` |
| 5 | `CubeProjector` |
| 6 | `Directional` |
| 7 | `BlackLight` |
| 8 | `ShadowVolume` |
| 9 | `ShadowVolumeDebug` |
| 10 | `DirectionalShadowVolume` |
| 11 | `DirectionalShadowVolumeDebug` |
| 12 | `FogVolume_Depth` |
| 13 | `FogVolume_Blend` |

14 техник × **3 LOD** кэшируются в `0x00503950`. Неизвестное имя → `0xf`.

Рядом cvar-таблица света `0x0056d490` (шаг `0x18`): `Light_Point` … `Light_BlackLight`, плюс `Light_DrawPointFillBox`, `Light_EnableFillToPoint`, `Light_EnablePointToFill`, `Light_ShadowVolume`, `Light_ShadowBlur`. (**Ghidra**)

### Draw order при override `0xf` (`0x00510680`)

Обёртка устройства `DAT_00576ff0`: vtable `+0xe8` = GetRenderState, `+0xe4` = SetRenderState. Номера — D3D9 `D3DRS_*`.

1. **Ambient** `0x005182e0` — technique **0**. `D3DRS_ZWRITEENABLE` включён, `D3DRS_ZENABLE` вкл. Опционально FogVolume_Depth (id **12**), если у камеры флаг и `0x0050e670` находит технику 13/12 (порядок приоритета 13 vs 12 не закрыт).
2. **Лампы** `0x00517700` — `ZWRITEENABLE=0`, `ZFUNC=D3DCMP_EQUAL` (3), `COLORWRITEENABLE=0xF`, `ALPHABLENDENABLE=1`, `SRCBLEND=SRCALPHA` (5), `DESTBLEND=ONE` (2), `FOGCOLOR=0`. Свет аддитивный поверх уже записанного depth/ambient. У лампы альфа в ps = 1, поэтому SrcAlpha,One при α=1 совпадает с One,One.
3. Опциональный **FogVolumes** `0x00517ff0` — **только если** Ambient вернул 1 (нашёл tech 13 в opaque-батче) **и** `DrawFogVolumes` (`DAT_0056d91c`) **и** `camera+0x314 & 0x20` (`eRTO_FogVolume`). Не «всегда после ламп». Intro fat: Ambient вернул 0 → skip. Volumetric FX — **не** этот слот: второй `RenderCamera` до мира (`CVolumetricLightFX`).
4. **Translucent** `0x00517e70` — technique **1**, гейт `DAT_0056d214` (`DrawTranslucent`). `ZWRITE=0`, `ZENABLE=1`, `ALPHABLEND=1`. Sort: back-to-front `dist²`, tie ключи батча ([27-visibility-sort.md](27-visibility-sort.md)).
5. **BlackLight** `0x00517100` — объекты `+0x57==3` и `+0x105==6`, cvar `Light_BlackLight` (default 0). Drawer `0x0051bab0` = клон Spot. Тени `0x0051fac0` tech **8**, затем lit `0x0050ffc0` tech **7**. Blend/z как у ламп. **В розничном контенте нет ни одного `.fx`/Mat00/света с техникой BlackLight → проход всегда пустой, реализовывать нечего** ([02-lights.md](02-lights.md)).
6. Дебаг/оверлеи `0x005164b0`.

Если override ≠ `0xf`, `0x00518520`: id 0 → Ambient, id 1 → Translucent, иначе один `0x0050ffc0` с этим id (`FogVolume_Depth` id 12 ставит `ZFUNC=LESSEQUAL` (4) вместо EQUAL).

### Цикл ламп (`0x00517700`)

Объект света: байт `+0x57 == 3`. Тип — `EEngineLightType` в `+0x105` (**SDK** `ltbasedefs.h`, совпадает с switch):

| `+0x105` | Тип | Рисовальщик | technique |
|---|---|---|---|
| 1 | Point | `0x0051e640` (cvar `DAT_0056d49c`) | 2 |
| 2 | PointFill | очередь → пачки по 1..3 в `0x0051c5b0` (`DAT_0056d4b4`) | 3 |
| 3 | Directional | `0x0051cfb0` (`DAT_0056d4fc`) | 6 |
| 4 | SpotProjector | `0x0051ddc0` (`DAT_0056d4cc`) | 4 |
| 5 | CubeProjector | `0x0051d990` (`DAT_0056d4e4`) | 5 |
| 6 | BlackLight | не здесь, а в шаге 5 кадра | 7 |

`Light_EnablePointToFill` / `Light_EnableFillToPoint` перекидывают Point↔Fill (Fill без теней). Fill **никогда** не идёт в shadow volume.

**На одну Point-лампу** (`0x0051e640`): сначала stencil volume (technique **8** через `0x0051fac0` → `0x0050fa20(..., 8, ...)`), scissor по сфере (`0x00521c30` / `0x00521940` = scissor+NVDB, **не** blur), опционально soft `0x005166f0` — гейт вызова `0x517c20` (непустой список кастеров лампы `this+0x70`); внутри blur-подблок — по `Light_ShadowBlur` ([04-shadows.md](04-shadows.md)), **потом** technique **2** Point. Тени — **до** аддитивного света этой лампы, не отдельным глобальным проходом после всех ламп.

`fInvLightRadius` в движке: `1.0 / *(float*)(light+300)` (`0x0051e640`).

`Translucent` — отдельный technique слот, не только флаг объекта. Fog volume — два внутренних pass (depth + blend).

## Захват устройства (**захват**, apitrace 14, обёртка `d3d9.dll`)

Канон мира: `local/ghidra/traces/fear-intro-20260813-224237.trace` (symlink `fear-intro.trace`). `+runworld Worlds\Release\Intro`, 1280×720 windowed, D24S8, MIXED VP, IMMEDIATE. После брифинга (клавиша в окне игры) кадры до **517** `DrawIndexed` / ~555k треугольников.

Меню-only: `fear-frame.trace` (не dispatcher мира).

Состояние RS на входе в **world** `BeginScene` (Present **10987749**, calls 10982854–10982881) — leftover от init/меню, **не** дефолты D3D9 (у D3D9 `STENCILFUNC=ALWAYS`): Cull **CCW**, Z on/write, `ZFUNC=LESSEQUAL`, blend off, stencil off / `STENCILFUNC=EQUAL`, specular off, fog table LINEAR / vertex NONE, dither on. Сэмплеры **0..15**: **WRAP + LINEAR** min/mag/mip, bias 0. **0** sRGB / AF в этом кадре. Init/меню до reset мог держать `MIPFILTER=POINT`.

### Проекция Intro (**capture** call 10983269)

`D3DTS_PROJECTION` основного вида:

`{{1.357995,0,0,0},{0,2.414213,0,0},{0,0,1,1},{0,0,-4.3,0}}`

Это D3D perspective, **бесконечный far** (`_33=_34=1`), near **4.3** (`_43=−zn`). `2.414213 = cot(22.5°)` → **FOV Y = 45°**. `1.357995 = 2.414213 × (720/1280)` — аспект 16:9. VIEW translation в см LithTech (тысячи единиц). Не путать с cvar `FovYInterface=75` (HUD).

### Порядок проходов того же кадра (**capture**)

1. `Clear TARGET|Z|STENCIL` color **0**, Z=1, stencil 0, viewport 1280×720.
2. **Небо** (воронка `0x00518a70`): второй `SetTransform` PROJECTION с `_43=−0.01` (near **0.01**, не `Viewport.MinZ`) и косыми `_31/_32`; `SetViewport` AABB **410,0,298×346**; 3 DIP (Ambient replace / additive / translucent). Затем `Clear Z|STENCIL` **без TARGET** — цвет неба остаётся, глубина мира с нуля. См. [05-sky.md](05-sky.md).
3. **Ambient** 1280×720: `ZWRITE=1`, `ZFUNC=LESSEQUAL`, 110 DIP / ~60k треугольников. PS `c0` = **`(0.098039, 0.098039, 0.098039, 1)`** = `25/255` (WorldProperties `AmbientLight`; `LODLights` High/Med, `AddAmbientLow` не прибавлен). У части материалов `AlphaRef=96`, `AlphaFunc=GREATER`, `AlphaTestEnable=TRUE` (как `specular_alphatest.fx`).
4. Цикл ламп (`0x00517700`): `ZWRITE=0`, `ZFUNC=EQUAL`, stencil on. На лампу: `Clear STENCIL` only → ShadowVolume (`ZFUNC=LESS`, `COLORWRITE=0`, `Cull=NONE`, **TwoSidedStencilMode=TRUE**, `StencilZFail=INCR`, `CCW_StencilZFail=DECR`) → `IDirect3DStateBlock9::Apply` (39 раз в кадре; часто возвращает ColorWrite **без** нового `SetRenderState`) → Point (`SrcAlpha/One`). **Две** shadowed Point, **ноль** Fill. PS c0 RGB **>1** (`1.57, 1.88, 2`), spec power **64** в PS c2. VS c8.x = **`fInvLightRadius`**: этот кадр **0.0004** → радиус **2500** см. AlphaTest с Ambient (**ref 96**) **не сбрасывается** на volumes и Point. **Glass** (`SrcBlend=One`, Z LESSEQUAL, stencil off) в этом Present **нет**; в том же `.trace` он есть в **647** более ранних Present **внутри** light-loop, не после него.
5. Teardown ламп → `ALPHABLEND=FALSE`, `ZFUNC=LESSEQUAL`, `ZWRITE=TRUE`. Затем **tech 1** `0x00517e70`: **2 DIP** `additive.fx` Translucent (`SrcAlpha/One`, Z on / no-write, PS `{1,1,1,1}` = `FLAG_NOLIGHT`) — stride 24 pc=14 и stride 32 pc=6 @ `(4258.59, 2097.44, -703.96)`. Это **не** `translucent.fx` (`InvSrcAlpha`) и не glass. Дальше HUD: **2 translucent-квада** (`SrcAlpha/InvSrcAlpha`, stride 24, white 4×4, орто-VS `0xbce3358` 1/640+1/360, PS `0xbce3288`; capture 10987639/10987682) + `StretchRect`; 7 additive в этом хвосте не видны ([28-clientfx-hud.md](28-clientfx-hud.md)). BlackLight-setup без DIP. Overlay-неба нет (нет второго AABB).

Тонкие кадры (~23 DIP) в том же trace — 256² RT (зеркала/FX) + HUD, не этот dispatcher. Не брать их за порядок мира.

## Дыры

- Overlay `0x00518c70` после мира в этом Present нет (хвост — 2 tech-1 + HUD 1280, не AABB неба).
- ColorWrite света часто только через `StateBlock::Apply`: в dump `SetRenderState(COLORWRITE=0)` остаётся на EQUAL-проходе, пока не поймаешь Apply. Это **лампы**, не второй shadow.
- Pointman deferred — **не** оригинал.

Спека проходов для порта **пригодна** и подтверждена Present мира. Фазу 1 скрином Pointman не закрываем без эталона FEAR с той же точки.
