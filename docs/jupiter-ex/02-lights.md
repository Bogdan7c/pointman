# Лампы

Источник: **SDK** `Game/ObjectDLL/Light*.cpp`, `engine/sdk/inc/ltbasedefs.h`, `iltcsbase.h`.

Иерархия: `LightBase` → Point / PointFill / Spot / Cube / Directional. Отдельного `DirLight` нет.

## Типы `EEngineLightType`

| Тип | Класс | Тени | Specular | Смысл |
|---|---|---|---|---|
| `Point` | `LightPoint` | да (LOD) | да | равно во все стороны |
| `PointFill` | `LightPointFill` | нет | нет | дешёвая заливка |
| `SpotProjector` | `LightSpot` | да | да | усечённый конус + gobo |
| `CubeProjector` | `LightCube` | да | да | cubemap во все стороны |
| `Directional` | `LightDirectional` | да | да | ortho OBB + текстура |
| `BlackLight` | нет WorldEdit-класса | да (Spot-клон) | tech 7 | фонарик. `0x0051bab0` = Spot setup; тени tech 8; lit `0x0050ffc0` id 7. В `.fx` technique нет |

## Общие поля (`LightBase`)

| Поле | Default | Примечание |
|---|---|---|
| `LightColor` | 255,255,255 | основное |
| `TranslucentColor` | 255,255,255 | модуляция для translucent |
| `SpecularColor` | 255,255,255 | нет у PointFill |
| `LightRadius` | 300 | см, falloff |
| `IntensityScale` | 1.0 | 0..1 |
| `LightLOD` | Low | когда лампа жива |
| `WorldShadowsLOD` / `ObjectShadowsLOD` | Low | нет у PointFill |
| `Texture` | "" | gobo / cube / directional |
| `AttenuationTexture` | "" | только Directional |

Spot: `FovX`/`FovY` в **градусах полного конуса** в WorldEdit; в движок уходит **половина в радианах**. `NearZ`. Volumetric* — отдельный SFX, не основной свет.

Directional: `Dims` (ortho box), `FLAG_FULLPOSITIONRES` чтобы не квантовали поворот.

## Контракт API

`SetLightType`, `SetLightRadius`, `SetLightIntensityScale`, `SetLightTexture`, `SetLightAttenuationTexture`, `SetLightTranslucentColor`, `SetLightSpecularColor`, `SetLightDirectionalDims`, `SetLightSpotInfo(fovX, fovY, nearClip)`, `SetLightDetailSettings(enableLOD, worldShadowLOD, objectShadowLOD)`.

Кривая затухания по радиусу в SDK **не задана** — только радиус и опциональная attenuation-текстура у Directional.

Из розничного `specular.fxo` (**архив**, D3DX-параметры):

- одна лампа Point/Spot/Cube: `vObjectSpaceLightPos`, `fInvLightRadius`, `vObjectLightColor`
- Fill: массивы **`vObjectSpaceFillLightPos[3]`**, **`fInvFillLightRadius[3]`**, **`vObjectFillLightColor[3]`** → **`NUM_POINT_FILL_LIGHTS = 3`**
- Directional: `tDirectional_Projection`, `tDirectional_Attenuation`, `tDirectional_ClipMap`, `vDirectional_Dir`, `fDirectional_FarPlane`

Вектор к лампе масштабируется **1/радиус** (exe: `1.0 / radius` в `0x0051e640`). Intro Present 10987749: VS register **c8.x = 0.0004** = `1/2500` (**capture**). Object-space pos лампы в том же cbuffer: VS **c9 = (151.69, 23.46, 258.59, 0)**.

Кривая из ps_2_0 `specular.fxo` (**архив**, шейдеры Point `[10]` и PointFill `[8]`):

`atten = (1 - saturate(dot(L, L)))^2`, где `L` — интерполятор уже умноженный на `fInvLightRadius`, то есть `dot(L,L) = (d/r)²`. За пределами радиуса `sat((d/r)²)=1` → atten 0.

Это **не** `(1 - d/r)²` (так сейчас в Pointman — расхождение, не догадка про оригинал).

Blinn: `H = normalize(normalize(L) + normalize(V))`, spec = `spec.rgb * vSpecularColor * (N·H)^(spec.a * fMaxSpecularPower)`, diffuse = `diffuse * vObjectLightColor * sat(N·L)`, затем `* atten`. PointFill то же atten и `N·L` по **трём** лампам, без spec и без shadow. Directional atten берёт текстуры, не эту полиномную кривую.

## Projector-матрицы (**Ghidra** drawers + **archive** `specular.fxo`)

HLSL row-vector: `mul(float4(objPos,1), M)`. Per-mesh: `M_object = World * M_world` (`0x00507f60`, object `+0x44`).

Движковые поля (после SDK-конверта): pos `+0x9C`, quat `+0xA8`, cookie `+0xE8`, atten tex `+0xEC`, Spot half-FOV `+0xF0/+0xF4`, NearZ `+0xF8`, Dir dims `+0x108` (уже WorldEdit×`(0.5,0.5,1)`), radius `+0x12C`.

### Spot `0x0051ddc0` / BlackLight `0x0051bab0` (тот же setup)

```
fovX,fovY = half-radians; near = max(NearZ, 0.01); far = LightRadius
sx=1/tan(fovX); sy=1/tan(fovY); Q=1/(far-near); qn=-near/(far-near)
P = diag-persp(sx,sy,Q,qn)  // w = z_view
B = | 0.5  0   0  0.5 |     // D3D clip→UV, Y flip
    |  0 -0.5  0  0.5 |
    |  0   0   1   0  |
    |  0   0   0   1  |
R = quat_to_mat3(light)     // 0x00414dc0
V = [Rᵀ | Rᵀ*(-pos)]        // 3×4
M_world = B * P * V
```

Q/qn — **неканонический** z-ряд (far→1/far, не 1; **Ghidra**, с каноном D3D не сверян). Для projective UV безразличен; near-clip идёт отдельной константой `vSpotProjector_ClipNear`.

Те же Point-константы: `fInvLightRadius=1/r`, `vObjectSpaceLightPos`. `vSpotProjector_ClipNear` = **row 2** object-space `mSpotProjector_LightTransform` (`LAB_0051b8e0`). Cookie `tSpotProjector_LightMap` ← `+0xE8` (capture **s4**). `tClipMap` в Spot PS есть, drawer **не** биндит.

### Cube `0x0051d990`

Invert `pos+quat[+scale]` (`0x00436350` / `0x00435db0`). `mCubeProjector_LightTransform` = 3×4 `scale * R_inv | t_inv` (`0x0051d710`). Cubemap dir = эта 3×4 × `float4(pos,1)`. `tCubeProjector_LightMap` ← `+0xE8` (capture **s3**).

### Directional `0x0051cfb0`

WorldEdit `Dims=(Dx,Dy,Dz)` → engine `(hx,hy,Dz)=(Dx/2,Dy/2,Dz)`.

```
origin = pos - X*hx + Y*hy          // угол near-plane, не центр
inv = inverse(origin, quat)
S = diag(0.5/hx, -0.5/hy, 1/Dz)     // = diag(1/Dx, -1/Dy, 1/Dz)
mDirectional_ObjectToTex = S * inv  // 3×4 → 4×4, last row 0001 (`0x004f6380`)
vDirectional_Dir = quat column 2
fDirectional_FarPlane = dot(forward, pos) + Dz + 10   // +10 константа
```

`tDirectional_Projection` ← `+0xE8`, `tDirectional_Attenuation` ← `+0xEC`. `tDirectional_ClipMap` сэмплится, Dir drawer **не** биндит. Capture Dir: VS **13** regs, stages **s3+s4+s5**.

BlackLight: строка `"BlackLight"` `0x0055f4e0`, id **7**, cvar `DAT_0056d514` (`Light_BlackLight`, default **0**). Тот же setup+проход, что Spot (`0x0051bab0`: проектор, frustum `0x0051b620`, vis-gather, shadow-caster `0x0051fac0`, blur-гейт, lit `0x0050ffc0` tech **7**). **В розничном контенте нет ни одного `.fx` с technique BlackLight, ни одного Mat00/света этого типа** (grep по всем 36 Arch00: только `WEAP_Blacklight`/`CA_BlackLight` — оружие-спрей UV-чернил в GameClient.dll, не свет) → при `Light_BlackLight=1` pass всё равно пустой (D3DX handle tech 7 = 0). Для 1:1 реализовывать нечего.

### Intro capture (не только fat)

| Present | Что |
|---|---|
| **10987749** fat | 2 Point; **0** Spot/Cube/Dir (VS 10, s0–s3) |
| ~160407 | Spot-like: VS 12, `1/r=0.00125` (r=800), cookie **s4** |
| **3432322** | Dir: VS 13, `1/600`, s3+s4+s5, RGB `0.42,0.67,0.75` spec 64; Cube: VS 14, `1/r=0.003333` (r=300), cubemap **s3** |

Точные индексы регистров Spot12 / Cube14 / Dir13 кроме Point c8/c9 — **не закрыты**. BlackLight-фонарика в контенте нет — не блокер.

## WorldProperties (не лампа, но свет кадра)

| Свойство | Default | Cvar |
|---|---|---|
| `FarZ` | 100000 | `FarZ` |
| `ClampFarZ` | true | режет по fog/миру |
| `AmbientLight` | 0,0,0 | `Light_AmbientR/G/B` |
| `SkyAmbientLight` | 0,0,0 | `Light_SkyAmbientR/G/B` |
| `FogEnable` / `FogColor` / `FogNearZ` / `FogFarZ` | выкл / 127 / 1 / 5000 | `Fog*` (color **/255**) |
| `SkyFog*` | выкл | отдельный fog неба; xrefs на value **нет** |
| `AddAmbientLightLow/Med/High` | 0 | добавка по `LODLights` |

Три fog-пути (**не** один проход):

1. **Table fog** — D3D9 `FUN_005009c0` (из Start3D): `FOGCOLOR/START/END/ENABLE` из cvars. `TableFog` default 1. Intro `FogEnable=0`. VolumeBrush — локальный override тех же cvars, не 4-й путь.
2. **FogVolume tech 12/13** — материалы (вода/alphatest/DOF), слот `0x00517ff0` после ламп **если** Ambient нашёл tech 13. Intro groups `FogVolumes=0`; fat Present skip.
3. **Volumetric FX** — `CVolumetricLightFX` до мира: `RenderCamera(light, "FogVolume_Depth")` + `RenderCamera(player, "Ambient")` + slices. Intro: одна лампа `LightDirectional00` `VolumetricLOD=High`; остальные 13 — `VolumetricLOD=Never`. Не слот `0x00517ff0`.

Intro: FarZ 100000, AmbientLight **25,25,25** как 0..255. SDK `WorldProperties` пишет `Light_Ambient* = color/255` → **`25/255 ≈ 0.098039`**. Затем `CPerformanceMgr::ApplyAmbientLOD`: `Light_Ambient += AddAmbient{Low|Med|High}` **уже в 0..1**, выбор по `LODLights` (0=Low, 1=Med, **2=High default**). Intro Low=**0.13**, Med/High=**0**. Этот fat Present = High/Med → PS `c0` **0.098039**, не `(25+0.13)/255` и не raw 25. `Light_AddAmbient` (отдельный cvar) default 0. Fog выключен. (**эмпирика карты** + **SDK** + **capture** + **Ghidra** `0x004f4c80`)

Консоль exe (таблица `0x0056d490`, **Ghidra**): `Light_Point`, `Light_PointFill`, `Light_SpotProjector`, `Light_CubeProjector`, `Light_Directional`, `Light_BlackLight`, плюс дебаг `Light_DrawPointFillBox`, `Light_EnableFillToPoint`, `Light_EnablePointToFill` (качество: Fill↔Point), `Light_ShadowVolume`, `Light_ShadowBlur`.

Intro dump-draw (1988 объектов): **35 Point + 80 PointFill + 39 Cube + 12 Spot + 2 Directional = 168 ламп**. Не «~110 point/fill». Yard_Directional01 — ortho 6500×4500×4500 на двор. PointFill двора — радиус 1500–2474, Medium LOD. Многие Cube с `Prefabs\Test\light_02a_C.dds`. (**dump-draw**)

**Тот же Intro, Present 10987749** (**capture**): на экране только **две Point с volumes**. Fill/Spot/Cube/Dir в **этом** кадре нет (карта их содержит — другой ракурс/LOD). Projectors в том же `.trace`: Present **3432322** (Dir+Cube), ~160407 (Spot-like).
