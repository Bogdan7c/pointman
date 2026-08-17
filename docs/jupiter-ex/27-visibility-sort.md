# Visibility, gather, sorting

## 1. Scope

Как exe собирает списки объектов кадра и сортирует draw-records до `DrawIndexed`. On-disk sector section — [22-world00p.md](22-world00p.md) (`0x00458610`).

## 2. Ownership

`FEAR.exe`: gather `0x00518dd0`, tree walk `0x00521370` / `0x00521440`, classify `0x0051f550`, sort+draw `0x0050fa70`.

## 3. Inputs / layout

Пакет камеры (смещения **Ghidra**):

| Offset | Поле |
|---|---|
| `+0x14` | FOV-related, в vis helper |
| `+0x18` | byte: если 0 и cvar — путь `0x00521370` |
| `+0x1fc`… | 6 плоскостей frustum (копия в `DAT_00578440`) |
| `+0x27c`… / `+0x288` | позиция камеры (sort dist² отсюда) |
| `+0x2e1` | `==1` → явный список, не дерево |
| `+0x30c/+0x310` | ptr/count явного списка |
| `+0x2c4` | SkyCamera object |

Объект:

| Offset | Поле | Тег |
|---|---|---|
| `+0x57` | `EObjectType` | **SDK** + **Ghidra** |
| `+0x58` | FLAG_ byte; bit 0 visible-in-classify; bit 2 `FLAG_NOLIGHT` | **SDK**/Ghidra |
| `+0x5c` | FLAG2 dword | **SDK**/Ghidra |
| `+0x9c,+0xa0,+0xa4` | позиция XYZ для dist² | **Ghidra** |
| `+0xd4` | материал/цвет блок; `+8` depth-bias index; `+9..+11` extra RGB | **Ghidra** |
| `+0xe4/+0xe8/+0xec` | CustomRender vis/render; `OT_MODEL` `+0xe4` int LOD; WM `+0xe8` ushort LOD (`0x0051f700` / `0x0051f920`) | **Ghidra** |
| `+0xf0` | **тип-зависимо**: WM = wmdata* (первый ushort = индекс `DAT_00576ff4+0x18`); CustomRender = ptr в `0x0050f9b0` | **Ghidra** |
| `+0x110` | `OT_MODEL`: runtime Model00p* | **Ghidra** `0x0051f200` |
| `+0x13c` | `OT_MODEL`: material-binding* (`0x00435b80`) | **Ghidra** |
| `+0x154` | WM: ushort из `wmdata+2` после ExtraInit | **Ghidra** `0x00463ed0` |

Draw record **0x28 байт** (10×u32), вектор `begin/end` как STL:

| Dword | Смысл |
|---|---|
| `[0]` | **material\*** = `0x00503a30(mat, technique, lod)` — слот `mat+4 + tech×12 + lod×4` |
| `[1]` | mesh/device ptr |
| `[2]` | object* (0 = world baked path) |
| `[3],[4]` | дальнейший tie-break |
| `[6]` | `dist²(cam.pos, obj.pos)`; 0 если object* null |

`0x00503a30`: `return *(mat + 4 + technique×12 + lod×4)`. Таблица 14×3 material* @ `+0x04`, D3DXHANDLE @ `+0xB0` (annotations `Low`/`Medium`/`High` / `Fallback`, `0x00505130`). Opaque cmp — unsigned lex `[0]..[4]`.

Shader LOD кадра (`0x004ffee0`): `clamp(LODMaterials, 0, 2)` → `0x00510ad0`. **Не** формула дистанции. Default dword `DAT_0056dab4` = **2** (High). Лампы: параллельный clamp `LODLights` (`DAT_0056da9c`) vs `object+0x102`.

Mesh LOD (`OT_MODEL` `0x0051f200`): `d' = max(0, (length(cam−obj) + *(obj+0x140) + ModelLODDistanceBias) × ModelLODDistanceScale)`; `0x0042ee70` идёт по piece+4 stride `0x10`, берёт первый `i` с `d' < threshold[i+1]`, затем `+ ModelLODOffset`. Count может быть >3. Cvars default: Offset 0, Scale 1, Bias 0.

## 4. Алгоритм gather `0x00518dd0`

Параметры: frustum query, camera packet, три бакета, **filter callback**.

Если `camera+0x2e1==1`: цикл явного списка; callback; `OT_CUSTOMRENDER` может отклонить через `object+0xe4`. Push в бакет.

Иначе:

1. Скопировать frustum/позицию из пакета, если **`VisLock`** `DAT_0056d304==0` (default 0 = каждый кадр).
2. `0x00518d50` — подготовить vis query.
3. Если **`VisDrawFrustum`** `DAT_0056d31c==0` (default 0) и `camera+0x18==0`: world-tree `0x00521370` (`DAT_00579934+0x34` → `0x00458420` lookup по `camera+8` float*). Нода: vis-byte **`+0x38`**, skip-early **`+0x39`**, visited **`+0x3a`**. Рекурсия `0x00521080`. Глубина: **`VisMaxSectorDepth`** `DAT_0056d334` (default **-1** = без лимита).
4. Fallback `0x00521440` только если tree не взяли. Если tree ok **или** **`VisDisableWhenOutside`** `DAT_0056d34c!=0` — fallback пропускается.
5. Профильтровать вектор callback-ом; compact; `0x00510a40` обрезать.

Debug-draw cvars (default 0, не алгоритм отбора): `VisDrawPortals`, `VisDrawSectorDims`, `VisDrawWorldBlocks`, `VisDrawWorldModelDims`, `VisDrawModelDims`, `VisDrawLightDims`, `VisDrawCustomRenderDims`. `DrawWorldTree` — отдельный debug.

### Runtime-нода / портал (`0x00521080`, **Ghidra**)

Runtime vis-нода stride **0x40** (`0x00458c40` / `0x00458e80`). Не 12-байтные PhysicsBSP-ноды.

| Off | Смысл |
|---|---|
| `+0x00` | ptr имени (арена); `+0x04` хеш `0x00475360` |
| `+0x0c` / `+0x18` | AABB min / max (6 float) — тест `0x00412680` из `0x00458a60` |
| `+0x28` / `+0x2c` | ptr плоскостей / count; plane stride **0x14** (4 float + октант + 3 B pad). Intro **count=0** |
| `+0x30` | ptr массив порталов |
| `+0x34` | count порталов |
| `+0x38` | vis enabled (byte); 0 → skip в `0x00521370` |
| `+0x39` | если 0 и `param_4!=0` — early out, снять `param_3` с счётчика |
| `+0x3a` | visited this walk (ставится 1) |

Портал runtime stride **0x18** (`0x00458cf0` / `0x00458cc0`): индекс-список hull → плоскость + два i32 связанных портала (`-1` = нет).

**Нет PVS-битового массива в файле.** Intro sector section заканчивается точно на порталах (18 747 B). `count[5]=1192` — размер **runtime-арены clip-плоскостей**, не хвост World00p (1192 не кратен stride 0x14 — арена с заголовком/выравниванием; точный layout **open**).

Flood `0x00521080` (**Ghidra** asm, не stack-имена):

1. `plane·camera − dist`; порог `|f| >= 0.1`. Знак выбирает front-child; clip портала `0x00520310`.
2. `param_3` = **число уже накопленных clip-плоскостей** (stride 0x14), не биты.
3. `param_3==0`: `0x00520690` строит новые плоскости из полигона портала (cross рёбер с камерой).
4. иначе `0x00520070` — clip полигона портала о текущий frustum (Sutherland–Hodgman, vec3).
5. Рекурсия в соседнюю vis-ноду. `0x00520fe0` помечает сектор видимым (`+0x3c`) и собирает объекты.

На Intro у всех 119 нод `nPlanes=0` — extra clip-плоскости vis-ноды с диска не едут. Карты с `nPlanes>0` не снимали.

On-disk порядок: 8×u32 counts → 128 hull-quads → 119 именованных vis-нод → 57 порталов. Разбор: [22-world00p.md](22-world00p.md).

## 5. Constants

| | |
|---|---|
| Record size | `0x28` |
| Opaque sort | lexicographic `[0]..[4]` unsigned (`0x0050e030`) |
| Translucent sort | **back-to-front** по `[6]` (больший dist² первый), tie `[0]..[4]` (`0x0050e070`) |
| Sort impl | introsort, порог 33, `0x0050f7a0` |
| Fill batch | 1..3, не этот sort |
| `OT_*` | NORMAL=0 MODEL=1 WORLDMODEL=2 LIGHT=3 CAMERA=4 CONTAINER=5 CUSTOMRENDER=6 |

## 6. State tables

После sort, `0x0050fa70` батчит одинаковый object*, меняет `D3DRS_SLOPESCALEDEPTHBIAS` (0xAF) и `D3DRS_DEPTHBIAS` (0xC3) по байту `*(obj+0xd4)+8` — индекс в таблицу `0x004fce20`. (**Ghidra**; имена RS — **hypothesis** по номерам D3D9.)

Classify `0x0051f550` (**Ghidra**; translucent: `include_world=0`):

- **Bake** (список поверхностей мира, не объекты): если `include_world` и **`DrawWorld`** `DAT_0056d8bc` → `0x0050fa20` (draw record object*=0).
- Объекты: `FLAG_VISIBLE` (`+0x58` bit 0). Skip `(+0x65)&1`.
- `OT_MODEL` (1) только если **`DrawModels`** `DAT_0056d8a4` → `0x0051f200`.
- `OT_WORLDMODEL` (2) только если **`DrawWorldModels`** `DAT_0056d8ec` → `0x0051ebf0` (ещё: `+0x3b!=0xFF` или `FLAG2_FORCETRANSLUCENT` `+0x5c&4`).
- `OT_CUSTOMRENDER` (6) только если **`DrawCustomRender`** `DAT_0056d904` → `0x0050f9b0`.

Это **два независимых списка**, не overlap-тест. Pointman `BakedOverlapIndex` 0.6 **не** оригинал.

Bake и WM толкают **один** формат 0x20-записи (material*, ushort surf, flags+LOD, AABB) в `0x0050fa20`. Куски мира stride 0x14 живут в `DAT_00576ff4+4` (vis/debug) и в таблице WM `DAT_00576ff4+0x18`. Байты, которые `0x0050fa20` кладёт в 0x28 record:

| Источник | param_3 / path | bytes |
|---|---|---|
| Bake Ambient / shadow-gather | `0x0051f550` / `0x0051fac0` | `8/0x19` |
| Bake light | `0x0051f700` / `0x0051f920` | `9/0x1a` |
| WM opaque | `0x0051ebf0` `param_3=0` | `0xc/0x1b` |
| WM light | `0x0051ebf0` `param_3=1` | `0xd/0x1c` |
| Model | `0x0051f200` | `0xa/0x17` (`+1` если `param_5`) |
| CustomRender | `0x0050f9b0` | `0xe/0x1d` |

Проход (`0x0050ffc0`) передаёт свой `.fx`-technique отдельно (Ambient = 0). Байты record — класс источника. Имя слота `0x00503a30` от этих байт — **partial**. Layout 0x20/0x14: [06-world-draw.md](06-world-draw.md).

## 7. Псевдокод

```text
# opaque Ambient / default 0x0050ffc0
sort(records, less=lambda a,b: tuple(a[0:5]) < tuple(b[0:5]))

# translucent 0x0050fff0
for r in records:
    r[6] = 0.0 if r[2]==0 else length_sq(cam_pos - obj_pos(r[2]))
sort(records, less=lambda a,b:
    a[6] > b[6] if a[6]!=b[6] else tuple(a[0:5]) < tuple(b[0:5]))

# translucent color callback 0x00517da0 before draw
if obj is None:
    color = camera_rgb + a=1
elif obj.flags & FLAG_NOLIGHT:
    color = (1,1,1)
else:
    extra = bytes_at(obj+0xd4)[9:12] / 255
    color = camera_rgb + extra
shader_const = obj_rgba_bytes(+0x38) / 255 * color   # 0x00517c60
```

## 8. Edge cases

- Opaque **игнорирует** dist², хотя поле заполняется.
- Baked world record с object*=0 → dist 0; в translucent окажется «ближайшим» — **hypothesis**. Fat Intro tech-1 = 2 **object** additive DIP (`FLAG_NOLIGHT`), не pack-5 multiply. GPU-порядок pack-5 vs WM stains **не снят**.
- Custom vis callback может выкинуть объект после tree walk.
- `FLAG2_SKYOBJECT|SKYOVERLAY` уже отфильтрованы в main gather.

## 9. Evidence

| Claim | Source |
|---|---|
| Tree vs explicit list | **Ghidra** `0x00518dd0` |
| Opaque cmp | **Ghidra** `0x0050e030` |
| Translucent cmp back-to-front dist² | **Ghidra** `0x0050ea20` + `0x0050e070` |
| Ambient/Translucent оба через `0x0050fa70` | **Ghidra** `0x0050ffc0` / `0x0050fff0` |
| OT enum | **SDK** `ltbasedefs.h` |
| Capture порядка draw | **capture** Present 10987749: sky 3 + Ambient 110 + volume 86 + Point (~307 EQUAL) + **2 tech-1 Translucent additive** + HUD |
| Vis node `+0x38/+0x3a`, portal plane | **Ghidra** `0x00521370` / `0x00521080` |
| Sector section exact Intro | **Ghidra** `0x00458610` + **empirical** World00p |

## 10. Known unknowns

- Карты с vis-node `nPlanes>0` (не Intro).
- Shader LOD 0..2 = `clamp(LODMaterials, 0, 2)` (**не** дистанция). Default **2** High. Mesh LOD — отдельный путь: `(dist + obj+0x140 + ModelLODDistanceBias) × ModelLODDistanceScale` vs per-piece thresholds stride `0x10` (`0x0042ee70`).
- D3DX annotations `Low`/`Medium`/`High` на Intro `.fx` (какие child-материалы).
- Piece threshold float в Model00p.
- Per-surface (не per-object) culling.
- Кто на load заполняет `DAT_00576ff4+0x18` / `object+0xf0` (`vt+0x40`).
- Имена байт 8/9/0xc в 0x28 record vs слот `0x00503a30`.

## 11. Acceptance

- Синтетика: два translucent объекта, камера ближе к A → GPU order B затем A (back-to-front).
- Tie: одинаковая позиция → стабильный порядок по ключам `[0]..[4]`.
- Opaque: одинаковый материал батчится (ключи равны), dist не влияет.
- Sky objects отсутствуют в main list.
- Capture: последовательность Draw* translucent vs depth-sorted CPU dump.

## 12. Status

`verified-static` для ключей сортировки (`[0]` = material*), FLAG2-filter, WorldTree disk+walk **и** geometric portal flood. Shader LOD = cvar, не dist. Closure: `nPlanes>0`, capture translucent order, piece LOD thresholds.
