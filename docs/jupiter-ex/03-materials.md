# Материалы и шейдеры

## Mat00 (**архив**)

Magic `LTMI`. Слои FX: путь шейдера + defs. Pointman читает только первый слой и слоты `tDiffuseMap`, `tNormalMap`, `tSpecularMap`, `fMaxSpecularPower`.

Остальные defs (emissive, env, alpha, векторы) в файле есть — accessor'ы не используются.

`specular.fx` слоты: `tDiffuseMap`, `tEmissiveMap`, `tSpecularMap` (alpha = gloss 0..fMaxSpecularPower), `tNormalMap`. Default power 64. (**архив**)

## Шейдеры в рознице (**архив**)

Каталог `Shaders/` в `FEAR.Arch00`, патчи в `FEAR_1` / `FEAR_3` / `FEAR_7`.

Известные пути из Mat00/тестов:

- `shaders/rigid/Solid/specular.fx` — основной Blinn мир
- `shaders/rigid/Solid/specular_bump.fx`
- `shaders/rigid/Solid/skybox.fx` — cubemap по нормали куба, не 2D-стена
- `shaders/model.fx`
- `shaders/rigid/shadowvolume.fxo` — compiled, исходника `.fx` в паке может не быть
- `shaders/rigid/Translucent/...` включая volumetric
- `shaders/skeletal/Solid/skin.fx` — 65 B заглушка `#include` rigid-target; самого rigid `skin.fx` в extract **нет** (archive gap)
- `Shaders/Internal/DrawPrimBaseDefs.fxh`

Выжимка формул из `.fx` пишется сюда после локального extract (`local/fear-extract/Shaders/`). Сами `.fx` в git не кладём.

## Текстуры / DDS (**Ghidra** + **archive** + **capture**)

Розница — **`.dds`**, не DTX. `Texture00p` в `FEAR.Arch00` = **0**. Overlay: later Arch00 wins (`FUN_00401d50` читает `default.archcfg`). Доказательство: `Interface/menu/frame_h.dds` в `FEAR` = DXT3 64²; в `FEAR_3` = uncompressed RGB. `Sky_Day_C.dds` в обоих **идентичен**.

Create `0x0050a210`: type `+0x38` **1=2D**, **2=cube** (квадрат), **3=volume** здесь не создаётся. Pool `MANAGED`, Usage 0. Cube mips: если нет `D3DPTEXTURECAPS_MIPCUBEMAP` → Levels=1. DDS→`D3DFORMAT`: таблица 19 записей `0x00546180` / match `0x00525220`.

Живые форматы Intro/меню (остальные только в таблице):

| D3DFMT | Диск | Vulkan 1:1 |
|---|---|---|
| DXT1 | albedo, emissive, **sky cube** | `BC1_UNORM_BLOCK` |
| DXT3 | spec, UI | `BC2_UNORM_BLOCK` |
| DXT5 | cloudplane 2D | `BC3_UNORM_BLOCK` |
| A8R8G8B8 | normals, clip, часть cube/RT | `B8G8R8A8_UNORM` |

**Не** `*_SRGB`. Intro: **0** `D3DSAMP_SRGBTEXTURE` / `SRGBWRITEENABLE`. Свет — gamma-space Blinn. `GammaR/G/B` = 1.0 на capture4 → identity ramp, не decode текстур. Нормали: `normalize(tex-0.5)`, BGRA UNORM.

Куб ≠ 2D: `DDSCAPS2_CUBEMAP|ALL_FACES`, порядок +X−X+Y−Y+Z−Z. `Sky_Day_C.dds` 1024² DXT1 Levels=2 → `CreateCubeTexture`. `engine/normalize.dds` 128² A8R8G8B8 cube. `Day_Cloudplane1_D.dds` 1024² DXT5 — **2D**. `skybox.fx` = `texCUBE(-N)`. Не биндить куб как стену.

Сэмплеры world BeginScene (Present 10987749, **0..15**): WRAP + LINEAR min/mag/mip, bias 0. AF в этом capture **нет** (строки Anisotropic есть; `D3DTEXF_ANISOTROPIC` = 0). Hair «aniso» = 2D lookup. DrawPrim: WRAP + `MipMapLODBias=-1`. CLAMP только где `.fx` ставит (hair slot 4, water/mirror).

Missing bitmap: **unknown**. Нет pink/checker/`white.dds` в exe. `Fallback` = D3DX technique, не текстура. Load miss → handle 0 / `LT_MISSINGFILE`. Pointman white/flat-normal — **не** оригинал.

## Lighting math

`specular.fx`: Point/Spot/Cube → `GetLitPixelColor(...)`. Fill → `GetPointFillPixelColor` без spec, **3 лампы за проход** (массивы в `.fxo`). Directional → `GetDirectionalLitPixelColor`. Исходник `dx9lights.fxh` в Arch00 **нет**; тела восстановлены из ps_2_0 в том же `specular.fxo` (**архив**).

Point / Spot / Cube (ps_2_0, DEF `c3 = (-0.5, 1, 0, 0)`):

- нормаль: `normalize(normalMap - 0.5)` — то же направление, что `(2tex-1)` после normalize
- `NdotL = saturate(N·normalize(L))`
- `H = normalize(normalize(L) + normalize(V))`
- spec: `spec.rgb * vSpecularColor * pow(sat(N·H), spec.a * fMaxSpecularPower)`
- diffuse: `diffuse * vObjectLightColor * NdotL`
- `atten = (1 - saturate(dot(L,L)))^2` затем `(diffuse+spec) * atten`
- Spot дополнительно делит projector UV на `w` и множит на clip/projection-текстуру

PointFill: три таких диффузных члена (интерполяторы трёх `L`, константы трёх цветов), без POW/spec.

Pointman `spec.rgb * pow(N·H, spec.a * fMaxSpecularPower)` по spec **совпадает**; falloff `(1-d/r)²` — **нет**, оригинал `(1-(d/r)²)²`.

Normal maps: `normalize(tex - 0.5)`. D3D9 BGRA.

**Cloth** (`cloth.fx`, **archive**): не Blinn. Albedo = `lerp(rimMap, diffuse, sat(N·V * fRimScale))`, default scale 2. Лампы: Lambert `NdotL * albedo * light * atten`, без spec. Ambient: `light * albedo + emissive`.

**Hair** (`hair.fx`, **archive**): Lambert + anisotropic lookup. TBN = `N × (0,1,0)`. Aniso RGB от `(T·V, B·V)`; aniso A на Point/Spot/Cube от `(B·H, T·H)` со сдвигом `(0.5, spec.a)`; `pow(A, fMaxSpecularPower)`. Directional: A без сдвига, степень `spec.a * fMaxSpecularPower`. Fill — только Lambert. Ambient AlphaRef 96. Полный контракт: [24-materials.md](24-materials.md).

**Glass** (`glass.fx`, **archive**): Ambient-technique **пустой** (в Ambients pass стекло не рисуется). Лампы = тот же `GetLitPixelColor`, что specular, но `L.z = abs(L.z)` (подсветка с обеих сторон), ZFunc **LessEqual**, Stencil off, SrcBlend **One**. DestBlend в pass не ставится → наследует light `One`. **Нет** technique Translucent — стекло **внутри** цикла ламп, не в `0x00517e70`. Fat Intro Present 10987749 glass **не** рисует; в том же `.trace` — 647 более ранних Present (брифинг). `glass_flat`: N=`(0,0,1)`. `glass_env`: albedo = `lerp(diffuse, texCUBE(reflect(V,N))*mask.rgb, mask.a)`.

**Post** (**archive**, [24-materials.md](24-materials.md)): OverlayFX `screeneffect` = 16-tap radial + 4-tap sharpen; `motionblur` = lerp(cur,last); `blur`/`DOF` = 8-tap; `refract` = N-offset curFrame. DOF живёт в technique FogVolume_Blend.
