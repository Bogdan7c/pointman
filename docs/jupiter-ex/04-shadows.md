# Тени

Реализация stencil volumes в SDK **нет**. Игровой код только говорит, **кто** кидает тень и на каком LOD.

## LOD (**SDK**)

`EEngineLOD`: Low / Medium / High / Never.

- Лампа: `WorldShadowsLOD`, `ObjectShadowsLOD` (нет у PointFill).
- Brush: `ClipLight` (default true) = pack-time «кидает тень»; `ShadowLOD`. Brush = `CF_NORUNTIME | CF_HIDDEN`. Строки `ClipLight` в exe **нет**. Intro: **0** Brush / `ClipLight` / `ShadowOverride`.
- WorldModel: `CastShadow` false → `CASTSHADOW 0` → `SetObjectShadowLOD(Never)`. Object `+0x3b == 0xFF` = Never; gather `0x0051fac0` skip WM если Never, кроме `FLAG2_FORCETRANSLUCENT`. Intro: 296 WM `CastShadow=0`, 347 `=1`.
- Bake volumes: `surf[+7]==0 && (surf[+6] & 1)` → tech 8. **Hypothesis:** ClipLight=false ⇒ packer не пишет pack-10. Не доказано xref.
- `ShadowOverride` в иерархии — WorldEdit, stripped.

`PointFill` явно: без теней и без specular, зато дешевле.

## Геометрия volumes (**архив** `Shaders/rigid/shadowvolume*.fxi` + `skeletal/*`)

World/prop (pos+nrm): light-facing (`dot(L,N) >= 0`) остаются. Отвёрнутые выталкиваются вдоль −L:

`fLightRadius = 2.0 / fInvLightRadius` (= 2×r). Смещение `max(fLightRadius − |L|, 1)`. Комментарий: chord ≤ **120°** (`1/cos 60° = 2`). `z += 0.01` **выключен**. Directional — тот же stencil, VS `DirectionalShadowVolume_VS`.

Модели, 32 B pos+nrm+index (`skeletal/rigidshadowvolume_base.fxi`): одна кость `mModelObjectNodes[i.x]`, chord ≤ **60°**, `fLightRadius = 1.154700538 / fInvLightRadius` (= `(2/√3)×r`). `z += 0.01` **вкл**.

Модели, 64 B 3×POSITION (`skeletal/shadowvolume_base.fxi`): в каждой вершине **все три угла треугольника**; skin каждый; `N = cross(p1−p0, p2−p0)`; экструдируется только p0. Тот же 60°/z-bias.

Поверхности World00p с материалом `engine\shadowvolume.Mat00` — **prebuilt volumes**, не стены. Mat00 = `LTMI` → `Shaders\rigid\shadowvolume.fxi` (pos+nrm, chord 120°, `2.0/r`, z-bias off). Intro: **289** поверхностей, stride **24** (pos+nrm), pack type **10**, из 2268. Pointman их skip'ает — оригинал рисует technique 8. `Car_Shadows` — декаль. Модели: [23-model00p.md](23-model00p.md).

## Stencil (**архив**, D3DX pass, не C++ `SetRenderState`)

Основной `shadowvolume.fxi`, technique `ShadowVolume` / `DirectionalShadowVolume`, один pass `StencilBoth`:

- `ZFunc = Less`, `ColorWriteEnable = 0`, `CullMode = None`
- **`TwoSidedStencilMode = True`**
- `StencilFunc = Always`, `StencilFail = Keep`, **`StencilZFail = Incr`**, `StencilPass = Keep`
- CCW: Fail Keep, **`CCW_StencilZFail = Decr`**, Pass Keep

Это **Carmack reverse (z-fail)**, two-sided за один pass. Fallback `shadowvolume_safe.fxi`: `TwoSidedStencilMode = False`, два pass (Front Incr / Back Decr на ZFail) — если карта не умеет two-sided.

`Internal/ShadowBlur_CopyStencil.fx` technique `Translucent`: pass White, pass Black с `StencilFunc = NotEqual` → чёрное там, где stencil ≠ 0. Потом `ShadowBlur_BlurBuffer.fx`. Soft-ish stencil, не SSAO.

## Когда volume относительно света (**Ghidra**)

Не глобальный «все тени, потом все лампы». На **каждую** Point/Spot/Cube/Dir с тенями (`0x0051e640` и соседние):

1. собрать кастеров `0x0051fac0` (**не** строит silhouette: world-list → `0x0050fa20` technique **8**; модели `OT==1` → `0x0051f200`; WorldModel `OT==2` → `0x0051ebf0`)
2. нарисовать technique **8** `ShadowVolume` (`0x0050fa20(..., technique=8, ...)`)
3. scissor по сфере радиуса лампы, если глаз снаружи (`0x00521c30`); если глаз внутри сферы — полный viewport (`0x005216f0`)
4. `0x00521940` — **scissor + NV depth bounds**, не blur. Гейт `DAT_0057702b & 1` и `ScissorTestDisable` (`DAT_0056d784`, default 0). `SetScissorRect` + `SCISSORTESTENABLE=1`. Если `ScissorTestDepth` (`DAT_0056d7e4`, default 1): `ADAPTIVETESS_X = FOURCC 'NVDB'`. `ScissorTestDebug` (`DAT_0056d79c`) рисует AABB-квад через `0x00521820` (DrawPrim), не `ShadowBlur_*.fx`.
5. Soft occlusion — **`0x005166f0`**. Гейт вызова — `0x517c20(this+0x70)`: «список кастеров пуст» (vector begin/end, stride **40**) — пусто ⇒ пропуск (стоит на всех 5 call-site'ах). Отдельный предикат `0x5169f0` = LOD-кламп (`DAT_0056da84` default **2**, clamp 0..2 vs байты лампы `+0x103/+0x104`) **AND** `Light_ShadowVolume` (`DAT_0056d574`, default **1**) — выбирает режим 1/2 для `0x516a50`, это **не** гейт `0x5166f0`. `Light_ShadowBlur` = `DAT_0056d5a4` default **0** — гейт только blur-подблока внутри. (**objdump** unpacked exe)
6. **потом** technique лампы (Point=2, Spot=4, Cube=5, Dir=6)

PointFill и конвертация Point→Fill **без** этого шага.

### Soft blur `0x005166f0` (**Ghidra** + **archive**)

Не `0x00521940`. После volumes (и **Clear STENCIL** — предыдущий stencil с `0x0051fac0` стирается, если идёт object-shadow list `this+0x70`):

1. CopyStencil `ShadowBlur_CopyStencil.Mat00` tech Translucent: **2** pass (White, Black `StencilFunc=NotEqual`) → alpha **1=lit, 0=shadowed**. `COLORWRITE=alpha`. Clip-triangle `DrawPrimitiveUP`.
2. Если `Light_ShadowBlur` (`cmp DAT_0056d5a4,0; je` @ `0x516900`): BlurBuffer 1 pass, затем `SRCBLEND=DESTALPHA` (**7**; `push 7; push 0x13` @ `0x51694f`).
3. Caller рисует свет. `0x005169c0` выкл stencil; если blur был — `SRCBLEND=SRCALPHA`.

RT = **текущий backbuffer** (`0x004f90f0` берёт PP `+0x160/+0x164` и дважды читает `DAT_0056d5a4`). Intro 1280×720, не 256. При `Light_ShadowBlur=0` аллокатор **не** ставит bit `0x200` → blur-буфер не выделяется; «`0x200` нужен и для CopyStencil» — **hypothesis** (ранний выход `test ah,0x2` @ `0x5167a1` может быть связан). 256² thin = RTO/зеркало, не blur-буфер.

Kernel в исходнике `ShadowBlur_BlurBuffer.fx` захардкожен под **640×480** (как `screeneffect.fx` 800×600). DX8 parent хочет `SrcBlend=DestAlpha`; exe после blur ставит именно **DestAlpha** (7) — совпадает, drift **нет**. Старое «DestColor» — ошибка имени enum: **7 = DESTALPHA**, DESTCOLOR = 9.

Cvar-адреса `DAT_0056d7b4` / `DAT_0056d784` как volume/blur — **неверны** (это `ScissorTestAggressive` / `ScissorTestDisable`).

## Intro Present (**capture** `fear-intro-20260813-224237.trace`, Present 10987749)

Не `_safe`. Один pass two-sided:

- `TwoSidedStencilMode = TRUE`
- `StencilZFail = INCR`, `CCW_StencilZFail = DECR`, Fail/Pass = Keep, `StencilFunc = ALWAYS` на volume
- `ZFunc = LESS`, `ColorWriteEnable = 0`, `CullMode = NONE`
- перед volumes: `Clear STENCIL` only (цвет/Z мира уже есть)
- после volumes: `StateBlock::Apply`, затем `ZFUNC=EQUAL` и Point той же лампы

Scissor в этом кадре полный 1280×720. CopyStencil/Blur **нет** — по бинарю вызов `0x5166f0` гейтится `0x517c20` (пустой список кастеров `this+0x70`), а `Light_ShadowBlur=0` снял бы только BlurBuffer, не CopyStencil. Что именно срезало вызов в этом кадре (пустой список vs ранние выходы по флагам записи) — **open**. 256² thin = ReflectGroup Med, не blur RT.

AlphaTest с Ambient (**ref 96**) остаётся включённым на volume DIP — `.fxi` volumes его не выключает. Кастер с дырками в альфе режет stencil так же, как цвет Ambient.

## Меш кастера в Model00p

В `*.Model00p` после render-меша — **второй меш**, если у `*_Group` 3-й байт = 1. VS выше ест этот меш как есть. `0x0051fac0` список кастеров, не adjacency.

## Дыры

- Adjacency runtime **нет**: и модели, и bake-world несут готовый IB. World pack 10 / 24 B закрыт на Intro. Другие карты — тот же Mat00.
- Не тащим shadow maps / SSAO «потому что современнее».
