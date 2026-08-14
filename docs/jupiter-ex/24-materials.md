# Материалы и шейдерные семейства (инвентарь)

## 1. Scope

Все `.fx`/`.fxo` в розничном `Shaders/` extract. Не полная VS/PS математика каждого family — только то, что доказано. `dx9lights.fxh` в Arch00 **нет**.

## 2. Ownership

Arch00 `Shaders/` (и `shaders/`). D3DX effects грузит `FEAR.exe`. Mat00 (`LTMI`) задаёт путь шейдера + defs.

## 3. Inputs

Mat00 layer: shader path + typed defs (string/vec/int/float). Technique names — таблица exe `0x0056dab8` (**Ghidra**), 14 слотов.

Extract (**archive**): 96 `.fx`, 36 `.fxi`, 132 `.fxo`, 14 уникальных technique names.

## 4. Алгоритм выбора technique

Имя → id `0x00503cb0`. Кадр без override гоняет набор в `0x00510680`. Материал без техники в bitmask пропускается (`0x00503820`).

## 5. Constants

Technique id 0..13: Ambient, Translucent, Point, PointFill, SpotProjector, CubeProjector, Directional, BlackLight, ShadowVolume, ShadowVolumeDebug, DirectionalShadowVolume, DirectionalShadowVolumeDebug, FogVolume_Depth, FogVolume_Blend.

`NUM_POINT_FILL_LIGHTS=3` (по массивам в `.fxo`; `dx9lights.fxh` в extract нет). `fMaxSpecularPower` default 64.

## 6. State tables (доказанные)

**specular Point/Fill ps_2_0** (**archive** fxo): см. [03-materials.md](03-materials.md), [02-lights.md](02-lights.md).

**shadowvolume.fxi** (**archive**): [04-shadows.md](04-shadows.md).

**skybox.fx**: только Ambient; `texCUBE(-N) * tint * objectColor`; One/Zero; AlphaTest off. См. [05-sky.md](05-sky.md).

**specular_alphatest.fx**: Ambient и FogVolume_Depth — `AlphaTestEnable=True`, `AlphaRef=96`, `AlphaFunc=Greater`. Point/Fill/Spot/Cube/Dir в `.fx` AlphaTest не ставят. Intro Present 10987749 (**capture**): после Ambient (100/110 DIP с test) **Enable остаётся TRUE, AlphaRef=96** на всех 86 volume DIP и на Point. Light pass **наследует** устройство, не выключает test.

**cloth.fx** (**archive** `Shaders/rigid/Solid/cloth.fx`; skeletal = `#define SKELETAL_MATERIAL` + include). DX8 parent `diffuse_dx8.fxi`. Текстуры: diffuse / emissive / **rim** / normal. `fRimScale` default **2.0**, в PS через TEXCOORD (иначе константа клипается в ±1).

Albedo: `lerp(rim, diffuse, sat(N·V * fRimScale))`. Grazing (N·V≈0) → rim; face-on → diffuse.

- Ambient: `lightDiffuse * albedo + emissive` (ps_1_4). AlphaTest **off**.
- Point/Spot/Cube: `GetDiffuseColor(N, L, albedo, light) * atten`; **без spec**. `atten` тот же `(1-sat((d/r)²))²`. Lambert как [03-materials.md](03-materials.md) (`NdotL * albedo * light`).
- Fill: три Lambert по `NUM_POINT_FILL_LIGHTS=3`, тот же rim-albedo.
- Directional: Lambert без atten, rim тот же.

`cloth_detail.fx`: в Arch00 только skeletal-заглушка 73 B (`#include ..\..\rigid\Solid\cloth_detail.fx`). Самого `rigid/Solid/cloth_detail.fx` в Arch00 **нет** — тот же archive gap, что `skin.fx`. Базовый `cloth.fx` для картинки одежды достаточен.

**hair.fx** (**archive** `Shaders/rigid/Solid/hair.fx`; skeletal 65 B = include). DX8 parent `specular_DX8.fxi` (на диске именно так, caps — у соседей lowercase `_dx8`). Описание в файле: anisotropic + вертикальный offset спека в **alpha specular-карты**.

Слоты: diffuse0, emissive1, specular2, normal3, **tAnisotropyMap 4**. Anisotropy sampler **CLAMP**; остальные WRAP. `fMaxSpecularPower` default **64**.

TBN для aniso **не** из вершинного tangent: `B = normalize(cross(N, (0,1,0)))`, `T = cross(N, B)` — «вверх мира».

Point / Spot / Cube (`GetLitPixel` с atten):

1. `N = normalize(normalMap - 0.5)` (как specular).
2. Diffuse: `albedo * lightDiffuse * sat(N·L)`.
3. Aniso RGB: `tex2D(aniso, (T·V, B·V)*0.5 + 0.5).rgb`.
4. Aniso A: `tex2D(aniso, (B·H, T·H)*0.5 + (0.5, specular.a)).a` — **hair offset** = specular alpha.
5. `spec = specular.rgb * lightSpec * aniso.rgb * pow(aniso.a, fMaxSpecularPower)`.
6. `(diffuse+spec) * atten`.

Directional (перегрузка без atten): aniso A берётся с центра `( *0.5+0.5 )`, **без** hair offset; степень `pow(aniso.a, specular.a * fMaxSpecularPower)` — как обычный gloss, не как offset.

Fill: `GetPointFillPixelColor` — **только Lambert**, без aniso spec.

Ambient: `lightDiffuse * diffuse + emissive`. **AlphaTest TRUE, AlphaRef=96, Greater** (как alphatest). FogVolume_Depth: EncodeDepth + alpha из diffuse, тот же AlphaRef 96.

VS skinning: `SKIN_POINT` / `SKIN_VECTOR` — **3 кости**, `w.xyz` без renormalize / implied w2; `A` не читается. `skeletal.fxh` / rigid `skin.fx` в Arch00 нет; контракт из `.fxo` (сумма весов 3 костей `765.005859` = 3 × 255.001953). Shadow VS — [04-shadows.md](04-shadows.md).

**Post** (**archive** `.fx`; OverlayFX / translucent surfaces, не world Ambient):

- `screeneffect.fx` (только OverlayFX): 16 сэмплов радиального блюра (вес = индекс), `/ (N(N+1)/2 + 1)`; discolor `* Color * ColorIntensity`; sharpen `5*src − 4 соседа` × InvColor; `lerp(sharpen, radial, |uv*2−1| * Gradient * Intensity)`. `RadialScale=(4/800,4/600)*RadialBlur*Intensity` (не от разрешения). SrcAlpha/InvSrcAlpha. Defaults: Intensity/Sharpen/RadialBlur/Gradient/ColorIntensity = 1.
- `screeneffect_bleach.fx`: bleach-bypass `lerp(src, bleach, sat(BleachBypass))`; bloom-порог 4 тапа. Film grain в исходнике закомментирован.
- `motionblur.fx`: `lerp(cur, last, fBlur * diffuse.a) * diffuse.rgb * vertexColor`. Default `fBlur=0.9`. SrcAlpha/InvSrcAlpha. DX8 → refract.
- `blur.fx`: 8 тапов (4 оси + 4 диагонали) `curFrame`; offset = `fBlurScale * diffuse.a * (res.x/800) / res`. Default 1.5. × diffuse. SrcAlpha/InvSrcAlpha.
- `depthoffield.fx` technique **FogVolume_Blend**: `DecodeDepth`; kernel = `fMaxKernelSize/res * sat((d − start/FarZ) * FarZ/(end−start))`; 8 тапов с per-tap depth; × diffuse. AlphaBlend **off**, Z LessEqual, ZWrite off. Defaults: kernel 4, start 100, end 1000.
- `refract.fx`: offset из `(N_ts − (0,0,1))` в clip; `scale = Nclip.z * fRefractScale * nrm.a * vertAlpha`; sample `curFrame`; `* lerp(1, diffuse, diffuse.a * vertAlpha)`. AlphaBlend **off**. Default scale **0.3**.
- `refract_thick.fx`: то же + хроматическое разделение RGB (`vChromaticSeparation`).
- `refract_additive.fx`: вместо lerp добавляет `diffuse.rgb * diffuse.a * vertAlpha`.

Остальные family: только имена файлов и technique set, пока не дизассемблирован ps.

### Family matrix (**archive** `local/fear-extract/Shaders/`)

Пути относительно extract. `Shaders/sdk/*.fxh` (**0 файлов на диске**) — referenced, не extracted. Math Point/Fill/Spot/Cube/Dir — из `specular.fxo` ps_2_0, не из `dx9lights.fxh`.

| Family | Файлы | Techniques | Math/RS | Blocking |
|---|---|---|---|---|
| Rigid specular | `rigid/Solid/specular*.fx` | Ambient, Point, PointFill, Spot, Cube, Dir | Point/Fill **closed** (fxo); env blend **partial** | `dx9lights.fxh`; env formula |
| Alphatest specular | `specular_alphatest*.fx` | + FogVolume_Depth | AlphaRef **96** Greater on Ambient+Depth; **capture**: persist на volume+Point | — |
| Diffuse | `rigid/Solid/diffuse.fx` | Ambient + 5 lit, **без** FogVolume_Depth | no spec | normal-mapped PS |
| Skeletal | `skeletal/**/*.fx` (34 wrappers, 32 — чистые include-заглушки) | inherited | `#define SKELETAL_MATERIAL` + include rigid | `skin.fx` / `anisotropic.fx` / `neon_inside.fx` **missing from extract** |
| Emissive | `emissive.fx` | Ambient | `emissive * vertexColor` | — |
| Glass | `glass*.fx` | 5 lit, Ambient **пустой** | тот же Blinn что specular; `L.z=abs(L.z)`; ZFunc=LessEqual; Stencil off; SrcBlend **One**; `diffuse*=vObjectColor`. env: `lerp(diff, cubemap(reflect(V,N))*mask.rgb, mask.a)` | DX8 16-pass; DestBlend наследует light pass |
| Skybox | `skybox.fx` | Ambient (sky pass) | `texCUBE(-N)*tint*object` One/Zero | vs overlay |
| Sky overlay | `sky.fx` | Translucent | AlphaRef 96, One/Zero | when vs skybox |
| Shadow volumes | `*shadowvolume*.fxi` | tech 8/10 (+debug) | z-fail two-sided; extrusion `2/fInvLightRadius` | two-sided vs `_safe` |
| Shadow blur | `Internal/ShadowBlur_*.fx` | Translucent | CopyStencil 2 pass + Blur 1; RT = backbuffer; после blur `SRCBLEND=DESTALPHA` (7) — **бинарь** `0x51694f`; default **off** | kernel authored 640×480 |
| Fog / water | FogVolume_* in alphatest/hair/DOF/murky | 12, 13 | Depth: EncodeDepth; Blend **partial** | water composite PS |
| Volumetric SFX | `VolumetricLight/volslice*.fx` | Translucent | cookie proj + `(1-sat(d²))²`; One/One | noise/dir variants |
| Translucent / additive | `translucent.fx`, `additive*.fx` | Translucent | SrcAlpha/InvSrcAlpha vs SrcAlpha/One | falloff/noz |
| Neon | `neon_outside.fx` | Ambient+Translucent | 8-tap cur-frame blur | `neon_inside` missing |
| Decals | bake: `Translucent/multiply.fx`; model: `skeletal/Translucent/Decal/multiply.fx` | Translucent | bake pack-5: Zero/SrcColor, `tex * vertexColor`; model: `tex2Dproj` | WorldEdit `Decal` CF_NORUNTIME; `DrawModelDecals` `0x0051f200` |
| Cloth | `cloth*.fx` | Ambient + 5 lit | rim `lerp(rim,diff,sat(N·V*fRimScale))`; Lambert×atten; **no spec**; `fRimScale` default 2 | — |
| Hair | `hair.fx` | Ambient + 5 lit + FogVolume_Depth | aniso `tAnisotropyMap` CLAMP; TBN from N×Y; Point hair-offset in spec.a; Ambient AlphaRef **96** | Fill без aniso spec; skinning VS |
| Post | OverlayFX `screeneffect*.fx`; surfaces `motionblur`/`blur`/`refract*`/`depthoffield` | Translucent; DOF = **FogVolume_Blend** | см. блок Post ниже | — |
| DrawPrim | `Internal/DrawPrim*.fx` | Translucent | 3 SDK modes | — |
| Null | `rigid/null.fx` | Stub | 0 passes | — |
| **BlackLight** | **нет** `technique BlackLight` в `.fx` | exe id **7** | Spot-клон: те же P/V/bias (`0x0051bab0`); cookie `+0xE8`; lit `0x0050ffc0` tech **7**. Без tech 7 pass пустой. Матрицы — [02-lights.md](02-lights.md) | нет PS в Arch00; Intro flashlight Present нет |
| Particles / PolyGrid / beams | **не** в `Shaders/` | ClientFx `GameClient.dll` | DrawPrim or custom | which material each FX binds |
| `model.fx` | Pointman Mat00 tests | — | — | **not in extract** |

Unique technique names in `.fx`/`.fxi`: 14, совпадают с exe 0–13 **кроме** BlackLight (только движок), **плюс** `Stub` (`rigid/null.fx`, вне exe-таблицы).

## 7. Псевдокод (выбор, не shading)

```text
id = override is None ? 0xF : name_to_id(override)
if id == 0xF:
    draw Ambient, lights, FogVolumes?, Translucent, BlackLight
else:
    draw only that technique
```

## 8. Edge cases

`*_dx8.fxi` — caps fallback. Skeletal wrappers `#include` rigid; часть targets отсутствует в extract (`skin.fx` rigid missing) — **archive** gap, не «их нет в игре».

## 9. Evidence

Инвентарь семейств: `local/fear-extract/Shaders/` (**archive**). Не коммитить `.fx`.

## 10. Known unknowns

VS/PS math: additive world, volumetric, skin, particles. **Cloth + hair + glass + post** закрыты из `.fx`. BlackLight: Spot-клон + tech 7, **нет** PS в Arch00. Missing-texture color **unknown** (не pink/white). Alphatest на lights — **закрыто** для Intro Present (persist). DDS/UNORM/cube — [03-materials.md](03-materials.md).

## 11. Acceptance

Таблица family→techniques: [materials-families.json](materials-families.json) (не markdown-таблица). Golden pixel — только specular Point/Fill/Ambient, пока остальные не закрыты.

## 12. Status

`partial`. Closure: BlackLight PS (нет `.fx`); missing `dx9lights.fxh`/`skin.fx`/`cloth_detail.fx`. Cloth/hair/glass/post — **закрыты** из `.fx`.
