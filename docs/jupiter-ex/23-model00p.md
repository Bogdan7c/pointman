# Model00p / skeletal rendering

## 1. Scope

Packed model `*.Model00p`. Парсера в Pointman нет. Ниже — loader exe + empirical выборка (cactus, cheezePoofs, medKit, alma / alma_noshadow, deltaForce, delta_mask, Base_Intro_Soldiers). Render VB/IB/группы/веса, **shadow-меш** и **M format 0–6 disk size** закрыты. AnmTree00p — **partial**. Slot types 1–8: **диск-layout закрыт** (census 495×v0x21: type 1/4/8 = 0 штук), семантика float'ов type 3/7 и planes type 4 — **partial**.

## 2. Ownership

| | |
|---|---|
| Расширение | **SDK** `RESEXT_MODEL_PACKED = "Model00p"` |
| Регистрация | `FEAR.exe` `0x0040ee70` / `0x004634c0` → `FUN_0044f900(..., "Model00p", factory)` |
| Чтение файла | `0x004325a0` (вызов `0x004314d0`) |
| Fallback | `0x00459130` → `"default.Model00p"` |
| Runtime API | **SDK** `iltmodel.h` `CLTModel*` |
| Кадр | `OT_MODEL` `0x0051f200`: модель `object+0x110`, materials `object+0x13c` (`0x00435b80` = getter). Поверхности stride 0x34 через `0x00511fb0`. [06-world-draw.md](06-world-draw.md) |

## 3. Inputs / binary layout

Stream: vtable `param_1+0x4` = `Read(dst, nbytes)`. (**Ghidra**)

### Фиксированное начало (**empirical** `attachments/models/cactus.Model00p` 61768 B + **Ghidra** `0x004325a0`)

| Offset | Size | Значение |
|---|---|---|
| 0 | u32 LE | magic `'MODL'` = `0x4C444F4D` |
| 4 | u32 | version **`0x21` (33)** |
| 8 | 21×u32 | заголовок `0x00432ac0` (84 байта) |
| 92 | … | string/node blob, затем pieces |

Проверка: `if (version != 0x21 \|\| magic != 'MODL') return 1`. Порядок полей в декомпиле Ghidra перепутан стеком; **на диске** magic затем version (cactus: `4d4f444c 21000000`).

Cactus header[0..20] at +8 (**empirical**). `0x00432be0` считает размер packed-арены из этих счётчиков (**Ghidra**, align 4).

**Ловушка стека:** `0x00432ac0` `RET 4` снимает arg-stream. После возврата header лежит на `ESP+0x5c`, не `+0x60`. Из-за этого `[ESP+0x80]` в `0x004326b0` = **hdr[9]** (76 B имён), `[ESP+0x7c]` в таблицах = **hdr[8]**, `[ESP+0x64]` = **hdr[2]**, `[ESP+0x60]` в цикле pieces = **hdr[1]**, `[ESP+0xac]` blob = **hdr[20]**. Старый текст «hdr[0] = число таблиц» **неверен**.

| hdr[i] | this+ | cactus | смысл / вклад в размер |
|---|---|---|---|
| 0 | +0 | 2 | **Σ piece.N** — суммарное число reloc-пар после всех 0x28. size-fn `n×8`. cactus 2; deltaForce 6×2=12; soldiers **2652**. **Не** число клипов `GetAnim*` и **не** hdr[1] |
| 1 | +4 | 1 | **число pieces**; n×**0x28** |
| 2 | +8 | 2 | **skeleton-ноды** (`0x00431d30`); n×4; **(n−1)×0x6c**; **внутренний цикл u16-таблиц** |
| 3 | +0xc | 2 | n×0xc |
| 4 | +0x10 | 1 | **не** в size-fn |
| 5 | +0x14 | 2 | n×0x10 |
| 6 | +0x18 | 0 | n×0x28 |
| 7 | +0x1c | 2 | n×0xc; × hdr[2]×4 |
| 8 | +0x20 | 1 | **число таблиц треков** (`0x00431be0`): на каждую 2×u16 + **hdr[2]×(u16 pos, u16 rot)**; `0xFFFF` = bind pose |
| 9 | +0x24 | **0x4C** | байтовый блоб имён (`base`, `null3`, `joint2`, `group12_Group`, `group13_Group`, `Null`, `Twitch`) |
| 10 | +0x28 | 0 | **K** именованных биндов `0x00431e80` (внутрь × N). medKit=1; deltaForce=2 |
| 11 | +0x2c | 0 | **N** объектов `0x004320b0` (запись 0x50). medKit=1; deltaForce=**11** (stream N, не hdr[10]) |
| 12 | +0x30 | 2 | **число `*_Group`** = `0x00432520` = nGroups render-меша |
| 13 | +0x34 | 0 | n×**0x24** — vertex-format type 0 (`0x00431b40`) |
| 14 | +0x38 | 0 | n×**0x28** — type 1 |
| 15 | +0x3c | 0 | n×**0x3c** — type 2 |
| 16 | +0x40 | 0 | n×**0x60** — type 3 |
| 17 | +0x44 | 0 | n×**0x70** — type 4 |
| 18 | +0x48 | 0 | n×**0x58** — type 5 |
| 19 | +0x4c | 0 | n×**0x60** — type 6 (alloc 0x60) |
| 20 | +0x50 | 0 | **packed keyframe bytes** (`Read` в `0x0043285f`); cactus 0, cheeze 20, alma 892, soldiers **518 506** |

### Skeleton on-disk (`0x00431d30`, **Ghidra** asm + cactus @ `0xA8`)

Сразу после name blob, **без align** (float'ы могут быть не на 4). Рекурсивная нода:

| Поле | Размер |
|---|---|
| reloc u32 | в `node+0x68` (+ база потока) |
| parent index u16 | `node+8`; `0xFFFF` = корень — runtime parent-chain `0x00439cd0` читает `movzx u16 [node+8]` ([11-animation.md](11-animation.md)). Старое «flags u16» — неверно |
| flags u8 | `node+0xa` |
| 7×f32 | bind pose: **xyz + quaternion** (корень cactus = `0,0,0` + `0,0,0,1`) → `0x0042f2a0` |
| child count u32 | дети stride **0x6c** (`0x00431a00`) |

Cactus: 2 ноды (hdr[2]), ребёнок смещён `(-9.11, 12.44, 11.07)`. Имена в блобе (7 штук) **больше**, чем нод: sockets + **анимации** `Null`/`Twitch`. 7 имён = 58 B с NUL из 76 B (`0x4C`): остаток ~18 B — вероятно ещё 2 коротких имени, на которые указывают material relocs 60/68 (**hypothesis**, полный список не закрыт).

После дерева: `Read` 4 байта в `model+0xf0` (`0x004010f0`).

### Piece on-disk 0x28 (`0x00431c20` asm, не Ghidra stack names)

| Off | Поле | Куда |
|---|---|---|
| +0x00 | u32 | `piece+0` |
| +0x04 | u32 | `piece+4` |
| +0x08 | u32 | `piece+8` |
| +0x0C | u32 | `piece+0xc` |
| +0x10 | u32 reloc | `piece+0x24` (указатель в packed-буфер) |
| +0x14 | u16 | `piece+0x18` |
| +0x16 | pad 2 | |
| +0x18 | u32 index | `piece+0x1c = table[index]` (таблица **hdr[8]**, не hdr[0]) |
| +0x1C | u32 reloc | `piece+0x20` |
| +0x20 | u32 | `piece+0x1a` = nonzero → bool |
| +0x24 | u32 **N** | число следующих пар **не** число вершин |

Дальше **N** пар `(u32, u32)` чанками ≤ `0x80`. Второй u32 += `model+0xf4` (база packed-арены). Арена пар — bump `0x004319b0` (N×8 нулей). `0x00431c20` `RET 0x18` (6 stack args).

Cactus (**empirical**, tables = hdr[8]×(4+hdr[2]×4) = 12 B @ `0xFA`, piece @ `0x106`): `16.0, 16.0, 16.0` (совпадают с дефолтами `0x0042fbe0`), radius **53.0336** @ +0x0C, u16 **200** @ +0x14, **N=2**, пары `(0,0)` и `(6866,0)`. **6866 — поле пары, не vertex count.**

Материалы: `0x00432030` `RET 0xc` — count, затем count записей **0xC** (`0x00431f30`: reloc + count + count×u32). Cactus: 2 группы (reloc 60/68). Дальше `0x00431fc0`: count записей **0x28** через `0x00431910` (10×u32). Cactus count = 0.

Затем u32 «extra names»; если `>1` — `ReadString` × (n−1). Cactus = 1 (skip). Байт-флаг; если set — `0x004320b0` + `0x00432520` + `0x0040df10`.

### Слоты после флага (`0x004320b0` asm, не Ghidra stack names)

Ghidra путает N/M. По байткоду:

```text
u32 nodeIndex          # → model+4, must be < nNodes (model+0x30)
f32 unk                # → model+8; medKit/deltaForce = 100.0, иначе 0
u32 N                  # = **hdr[11]**; alloc N × 0x50
for i in 0..N:
    u8 slot            # индекс ноды, < nNodes
    7×f32              # xyz + quat (`0x0041e1a0`)
    3×u32              # часто 2 float + 0
    u32 type           # 1..8, фабрика `0x0044ccc0`
    type-specific Load # vtable+0x18(stream)
u32 M                  # vertex-format objects `0x00431b40` × Load; deltaForce M=10 (**Load не пропущен**)
u32 K                  # = **hdr[10]**
for i in 0..K:         # `0x00431e80`
    u32 nameReloc
    for j in 0..N:
        u8 flag; u32; u32
```

Ловушка: на medKit hdr[10]=hdr[11]=1, нельзя различить. deltaForce **hdr[10]=2, hdr[11]=11, stream N=11** — N это hdr[11].

Фабрика `0x0044ccc0` (**Ghidra** asm): Read u32 type, `dec` → jumptable `0x0044cde0` на 1..8, alloc, ctor, **vtable+0x18 Load**. Общий базовый vtable `0x0054c720`, потом свой. Это **physics-shape на ноде**, не FVF (ILTModel collision / `FindPhysicsWeightSet`).

| type | alloc | ctor | Load +0x18 | disk | смысл |
|---|---|---|---|---|---|
| 1 | 0x50 | `0x0043c770` vt `0x551934` | `0x0043c820` | `u32+0x38; u32+0x3c; name 0x30; u32+0x40; f32; u32 nVerts; u32 nIdx; n×vec3; indices` (stride **2** если nVerts<65536 иначе **4**). `fld` scale `×0.01` (`0x54c7fc`). cook `0x004ae800` | triangle mesh. **0 штук** в 495× v0x21 FEAR.Arch00 |
| 2 | 0x44 | `0x004482d0` vt `0x551d54` | `0x00448760` | 6×f32 | box / AABB. **572** |
| 3 | 0x44 | `0x00471960` vt `0x5542c0` | `0x00471bd0` | 3×f32: **radius_cm**, **mass**, **density_g** | sphere. Havok `0x10` `0x004afc50`; r′=`max(r×0.01, 0.005)`; inertia `0x004914d0`: V=`4/3π r′³`, I=`0.4 m r′²`. density → `+0x3c` (`0x00471a10`), в cook/inertia не участвует — хранится для физики/баланса. deltaForce: `(13.5, 0.89, 18)` = r 13.5 см, m 0.89 кг, ρ 18 г/см³. **129** |
| 4 | 0x48 | `0x004179b0` vt `0x54e994` | `0x00417a40` | `u32+0x3c; u32+0x40; name 0x30; u32+0x38=k; …; u32 nVerts; n×vec3; k×16 B`. cook `0x004ad5a0` (0xe0, stride 0xc) | convex: 16 B = plane **hypothesis**. **0 штук** в Arch00 |
| 5 | 0x40 | `0x00475cb0` vt `0x554300` | `0x004761d0` | pose `0x0041e1a0` (7×f32) + **вложенный** `0x0044ccc0` | transformed child |
| 6 | 0x44 | `0x004257c0` vt `0x5502a8` | `0x00425b90` | u32 count + N×`0x0044ccc0` | list |
| 7 | 0x48 | `0x00405ca0` vt `0x54c800` | `0x00406320` | 9×f32: **radius_cm**, **mass**, **density_g**, **pA xyz**, **pB xyz** | capsule. Havok `0x30` `0x004abee0`; точки ×0.01; `+0x38`=`\|pA−pB\|` (см, без ×0.01); mass → `+0x3c`, density → `+0x40` (`0x00406040`). deltaForce конечности: pA=`(0,+h,0)` pB=`(0,−h,0)` (ось Y). **809** |
| 8 | 0x44 | `0x00405600` vt `0x54c790` | `0x00405880` | два вложенных `0x0044ccc0` | compound pair |

Выборка: medKit type2; deltaForce 2×type3 + 9×type7. Census **495** MODL v0x21 в FEAR.Arch00 (ещё 2× v0x1F и 13 не-MODL — лоадер `!=0x21` режет): type7 **809**, type2 **572**, type3 **129**, type5 **87**, type6 **74**, type1/4/8 **0**. Mesh-слоты в фабрике есть, в розничных v0x21 моделях не лежат.

deltaForce extra-names (u32 extra=8 → 7 строк): `deathbase`, `miscbase`, `riflebase`, `E3Base`, `E3BaseS`, `soldier_NewMisc`, `soldier_PatSCover` — **child models** (**SDK** `ILTModel` child). В именах ещё `RigidBody`.

`0x00432520` (**asm**): `Read` count, fail если `> 0x40`, затем count × `0x004323c0`. Count = **hdr[12]**. Имя группы — reloc в name blob (`group12_Group`, `body_Group`, `polySurface9`, …). Дальше: `u32`, **3 байта флагов**, `u32 nIds`, `nIds×u32` (обычно `[groupIndex]`).

Третий байт флагов = **CastShadow** (**empirical**). На диске это offset **+10** записи (reloc u32 + u32 + 3 байта флагов); в runtime-записи тот же смысл читается как `rec+7` (`0x0051f200` читает +6/+7) — раскладки записи на диске и в рантайме **разные**, маппинг по смыслу, не по смещению:

| файл | группы (имя, 3 байта, ids) | второй меш |
|---|---|---|
| cheezePoofs | polySurface* `(0,0,0)` | нет |
| alma_noshadow | body/hands `(0,0,0)`, hair `(1,1,0)` | нет |
| alma | body/hands `(0,0,1)`, hair `(1,1,0)` | **есть** |
| cactus | group12 `(1,1,0)`, group13 `(0,0,1)` | **есть** |
| medKit | polySurface9 `(0,0,1)` | **есть** |

Первые два байта (hair/cactus group12 = `1,1`) — не тень (hair не кастует). `alma.Model00p` 490697 B vs `alma_noshadow` 106768 B: structured prefix **байт-в-байт до geom** (`0x12e2`), вся разница — хвост тени.

### Vertex format types (`0x00431b40`, jumptable `0x00431bc4`)

Read u32 type 0..6. Не читают VB из stream: bump packed-арены и ставят vtable. Размеры = слоты hdr[13..18]:

| type | alloc | ctor |
|---|---|---|
| 0 | 0x24 | vtable `0x0055117c` |
| 1 | 0x28 | `0x00431850` → `0x00435540` |
| 2 | 0x3c | `0x00431870` → `0x00435550` |
| 3 | 0x60 | `0x00431890` → `0x00435580` |
| 4 | 0x70 | `0x004318b0` → `0x004355d0` |
| 5 | 0x58 | `0x004318d0` |
| 6 | (jumptable) | `0x00431ba4` |

### Format objects M (`0x00431b40` + vtable+8 Load)

Не FVF вершин (тот в D3DDECL хвоста). On-disk запись = runtime size: первый u32 = type 0..6, дальше Load пишет с `+4`. Общая шапка `0x004356c0` = **8×u32** (поля `+4..+0x20`). Дальше type-specific:

| type | runtime/disk | Load extra после шапки | всего после type |
|---|---|---|---|
| 0 | 0x24 | 0 (Load = `jmp 0x004356c0`) | 32 |
| 1 | 0x28 | 1×u32 @ +0x24 | 36 |
| 2 | 0x3c | 6×u32 (два vec3) | 56 |
| 3 | 0x60 | 15×u32 | 92 |
| 4 | 0x70 | 19×u32 | 108 |
| 5 | 0x58 | 13×u32 | 84 |
| 6 | 0x60 | 15×u32 | 92 |

hdr[13..18] = counts этих types (size-fn). deltaForce: M=**10** = 6×type4 + 4×type3 = hdr[17]+hdr[16]. Первые float'ы похожи на локальные AABB/кости (`0,0,0` затем координаты). Назначение точнее — **partial** (не VB).

Walker с этими размерами доводит **deltaForce 1.49 MB до EOF**: K=2 (`Default` все `(1,0,0)`, `RigidBody` все `(1,0.5,0)`), 7 `*_Group`, render 3561v/4798t + shadow 3×POSITION. `alma` extra-names: `alma_anims` / `Miscbase` / `Deathbase`. `Base_Intro_Soldiers` flag=0, mesh нет (сборка children).

### Геометрия после флага (VB bytes + IB bytes + D3DDECL)

После `0x004320b0` / `0x00432520` stream стоит на сыром хвосте. `0x004325a0` его **не** `Read`'ает по байтам (`0x0042f010` только max u16). `0x0040df10` зовёт runtime `DAT_00575d60(stream)` и пишет объект в `model+0x100`; в unpacked PE слот — мусор, единственный xref — этот `call`. On-disk формат хвоста ниже, без этого ptr.

**8×u32 заголовок render-меша**, поля `[6]` и `[7]` — **байты** VB/IB. `[0]`/`[1]` = 1 без тени, **2 если есть второй меш**. `[4]` = nGroups этого меша.

| файл | 8×u32 | verts | tris | max idx | второй меш |
|---|---|---|---|---|---|
| cactus | `[2,2,4,0,2,2,21504,2064]` | 336 | 344 | 335 | да |
| cheezePoofs | `[1,1,2,0,2,1,2688,252]` | 42 | 42 | 41 | нет |
| alma_noshadow | `[1,1,3,0,3,2,90432,11244]` | 1413 | 1874 | 1412 | нет |
| alma | `[2,2,6,0,3,2,90432,11244]` | 1413 | 1874 | 1412 | да |
| deltaForce | `[2,2,14,0,7,2,227904,28788]` | 3561 | 4798 | — | да (7 групп, 2 empty 32 B) |
| delta_mask | `[2,2,2,0,1,1,12032,1596]` | 188 | 266 | 187 | да 798×32 |

IB = **triangle list** `INDEX16`. Старое `0x4001` на cactus — 2064 прочитали как count, а не байты.

Сразу после IB — **не stride**, а размер таблицы элементов:

```text
u32 nDecls
for each:
    u32 tableBytes          # включая D3DDECL_END; 64 = 7 attrib + END; 40 = 4 attrib + END
    D3DVERTEXELEMENT9...    # Stream u16, Offset u16, Type u8, Method u8, Usage u8, UsageIndex u8
    D3DDECL_END             # Stream = 0xFF
```

Раньше `u32 stride=64` на render-меше совпало с tableBytes (7+END)×8. На shadow 32 B packed tableBytes=**40** ≠ stride. Реальный stride — в группе.

Render FVF (tableBytes 64 во всех файлах выборки):

| Offset | Type | Usage |
|---|---|---|
| 0 | FLOAT3 | POSITION |
| 12 | FLOAT3 | NORMAL (unit) |
| 24 | FLOAT2 | TEXCOORD |
| 32 | FLOAT3 | TANGENT (unit) |
| 44 | FLOAT3 | BINORMAL (unit) |
| 56 | D3DCOLOR | BLENDWEIGHT |
| 60 | D3DCOLOR | BLENDINDICES |

**Pack весов** (D3DCOLOR `0xAARRGGBB`): **R = w0, G = w1, B = w2, A = w3**. Lit VS читает **xyz**, `A`/`w3` **не** использует. Rigid cactus/cheeze: `0x00FF0000` (w0=1). Alma: A=B=0, **R+G=255** (два влияния; третий член VS = 0). Локальный индекс кости в BLENDINDICES → `group.bones[i]` → нода скелета. Cactus: dword индекса `0`, `bones=[1]` → нода 1 (ребёнок), не корень.

Lit skeletal VS (**archive** `.fxo`, не 2-bone): `idx = floor(indices.zyxw * 765.005859)` (localBone×3); `w = weights.xyz` **без** renormalize и **без** `w2=1-w0-w1`; `p/n = Σ_{k=0..2} w[k] * mul(nodes[idx[k]], …)`. Палитра `float3x4[24]` (`mModelObjectNodes`, `0x480`). CPU: собрать 3×4 @ `obj+0x190` (`0x004366d0`), remap `0x0050e6e0` (не-модель = identity), commit `0x00506f60`. `GetWeightSet` — микс клипов, не VS.

Fat Intro Present 10987749: **355** bind FVF 64 (pos/nrm/uv/TBN + WEIGHT + INDICES) — пропы двора, не alma 1413v. Плюс **36× 3pos-shadow** (тоже stride 64: 391 = 355+36, **capture** audit), 43× 32B-shadow.

`hdr[20]` blob: packed keyframe bytes (cactus 0, cheeze 20, alma 892, soldiers 518 506). Первая u16 таблицы треков 0 на cactus = 0 (**empirical**). Декомпресс: [11-animation.md](11-animation.md).

Таблица групп сразу после decl(s):

```text
u32 nGroups
for each:
    u32 vertBase
    u32 nVerts
    u32 stride            # 64 render; 32 или 64 shadow
    u32 indexBase         # в u16
    u32 unk0              # 0
    u32 nTris
    u32 unk1              # 0 или 1
    u32 nBones
    u32 flags             # 0 (на hair-shadow-заглушке alma = 1)
    u8  bones[nBones]     # remap local blend index → skeleton node; без align
```

| файл | nGroups | Σ nVerts | Σ nTris | remain |
|---|---|---|---|---|
| cheezePoofs | 2 (16+26) | **42** | **42** | 0 |
| alma_noshadow | 3 (942+347+124) | **1413** | **1874** | 0 |
| cactus | 2 (64+272) | **336** | **344** | второй меш |
| medKit | 1 (66) | **66** | **76** | второй меш |
| alma | 3 (942+347+124) | **1413** | **1874** | второй меш |

### Второй меш = shadow (**empirical**, почти closed)

После групп, если 8×u32`[0]==2`:

```text
u32×5 = (0, nGroups, unk, vbBytes, ibBytes)
VB; IB; nDecls + table(s); groups  # те же nGroups, 1:1 с render
```

Пустая группа (nVerts=0) = этот `*_Group` **не кастует** (cactus group12; alma hair).

| файл | 5×u32 | layout | группы тени |
|---|---|---|---|
| cactus | `(0,2,2,29952,7488)` | 1 decl, packed **32**: pos+nrm (unit) + 2×BLENDINDICES | empty + 936v/1248t, r=47.45 |
| medKit | `(0,1,2,7296,1824)` | тот же 32 B | 228v/304t, r=25.89 = piece radius |
| alma | `(0,3,2,340992,42624)` | **2 decl**: (1) stride **64** = 3×POSITION + 3×WEIGHT + 4×INDICES; (2) тот же 32 B pos+nrm+indices | 4008+1320+**0** verts, stride 64/64/32; max idx 5327 |

32-байтный меш = **pos+nrm [+ bone index]** для VS, который экструдирует по `dot(L,N)`:

- World/prop: `Shaders/rigid/shadowvolume_base.fxi` — только pos+nrm, chord **120°**, `fLightRadius = 2.0 / fInvLightRadius`, `z+=0.01` **выключен**.
- Skinned «как rigid»: `Shaders/skeletal/rigidshadowvolume_base.fxi` — pos+nrm+`BLENDINDICES0`, одна кость `mModelObjectNodes[index.x]`, chord **60°**, `fLightRadius = 1.154700538 / fInvLightRadius` (= `2/√3 × r`), `z += 0.01` **вкл**. Второй D3DCOLOR indices в файле VS не читает.

64-байтный 3×POSITION = `Shaders/skeletal/shadowvolume_base.fxi` (**archive** комментарий: *«vertex positions for the entire triangle in each vertex»* — совпало с decl). VS:

1. `SkinPoint` каждый угол (pos0/1/2 + свои weights/indices)
2. `N = cross(p1−p0, p2−p0)`, centroid = среднее трёх
3. если `dot(light − centroid, N) < 0` — вытолкнуть **p0** вдоль −L на `max((2/√3)r − |L|, 1)`; иначе p0 на месте
4. clip + `z += 0.01`

Это не CPU-silhouette: `0x0051fac0` только собирает кастеров и зовёт `0x0050fa20` / `0x0051f200`. Adjacency в файле уже «треугольник в каждой вершине».

`DAT_00575d60` в unpacked PE = мусор `0xff4b6740`; единственный xref — `call [DAT]` в `0x0040df10`. Фабрика хвоста — runtime. On-disk контракт выше без этого ptr.

## 4. Алгоритм загрузки (`0x004325a0`)

1. Read 8 байт; fail если не MODL+0x21.
2. `0x00432ac0` — 21 dword.
3. Выделить string blob (align 4); copy names.
4. `0x00431d30`, `0x0042fc70`, `0x0042ef90` — каркас/скелет (**partial**).
5. `Read` u32 → `model+0xf0`. u16-таблицы: count **hdr[8]** (`0x00431be0` только alloc); на каждую: 2×u16, затем hdr[2]×(2×u16). Cactus 1×12 B.
6. `Read` **hdr[20]** байт в packed-арену (`model+0xf4` уже выставлен на курсор перед именами).
7. Pieces: `0x00431c20` × **hdr[1]**, record 0x28 + N пар. `0x004306f0` пушит 8 B в vector `model+0x60` **один раз на piece** (счётчик = hdr[1], не hdr[0]). Затем `0x00430ec0` (in-memory).
8. `0x00432030` — материалы: **сначала Read u32 count** (не hdr[3]); на запись `0x00431f30`: 2×u32 (reloc, nDwords) **плюс nDwords×u32 с потока** в арену. `0x00431fc0` — Read u32 count, затем count×`0x00431910` (**ровно 0x28** с потока: 10×u32).
9. Read u32 extra-count; цикл **ebx=1; ebx < extra** (`0x00432959`): extra=8 → **7** имён. ReadString = **u16 длина + N символов без нуля** (**SDK** `ILTInStream`, max **0x105**). intern `0x00431730`, attach `0x00430ee0`. `HasErrorOccurred` (vtable+0xc). Read **u8 flag**; если set — `0x004320b0` (N×0x50 + M formats + K×`0x00431e80`), `0x00432520` (count ≤0x40 × `0x004323c0`), `0x0040df10` (хвост VB).
10. `HasErrorOccurred` (`vtable+0xc`); `0x0042f010` max bone/slot; успех 0; иначе `"Error loading model: %s"`.

Пустое имя модели → `default.Model00p` (`0x00459130`).

## 5. Constants

| | |
|---|---|
| Magic | `MODL` |
| Version | `0x21` only |
| Header after magic | 21×u32 |
| Piece on-disk | `0x28` |
| Max name read | `0x105` (261) |
| Fallback | `default.Model00p` |

## 6. State tables

Render path моделей: `OT_MODEL` `+0x57==1`. Кадр: `0x0051f550` → `0x0051f200`. Тени моделей: `0x0051fac0` → тот же `0x0051f200` (не строит volume). Группы: байты `0x004323c0` в записи `+6`/`+7`. Шейдеры `skeletal/Solid/{skin,cloth,hair}` + `*shadowvolume*` (**archive**). Lit skinning = **3 кости**, xyz as-is. `skeletal.fxh` в Arch00 нет — макросы из `.fx`+`.fxo`.

## 7. Псевдокод

```text
fn load_model00p(r):
    magic = r.u32(); ver = r.u32()
    if magic != b'MODL' or ver != 0x21: return Error
    hdr = r.read(21 * u32)          # 0x00432ac0
    names = r.read(hdr[9])          # packed arena
    root = read_node(r)             # hdr[2] nodes, xyz+quat
    r.u32()                         # model+0xf0
    read_u16_tables(r, n=hdr[8], inner=hdr[2])
    r.read(hdr[20])                 # pre-piece blob
    for _ in hdr[1]:
        piece_hdr_0x28 = r.read(0x28)
        pairs = r.read(N * 8)       # N at +0x24, NOT vertex count
    materials_0xc(r); records_0x28(r)
    extra = r.u32(); flag = r.u8()
    if flag:
        read_004320b0(r)          # N slots + M formats + K binds
        read_00432520(r)          # hdr[12] named *_Group + CastShadow byte
        bind_tail_ptr(r)          # 0x0040df10 → remainder
    # remainder: 8×u32 + VB + IB + decls + groups [+ 5×u32 shadow mesh]
    return Ok
```

Синтетический fixture: 8 + 84 нулевых **байта** (magic+version + 21×u32 header) + минимальные names **не** достаточен, пока не размечены counts. Не класть cactus в git.

## 8. Edge cases

- Неверный magic/version → код 1, модель не создаётся.
- Нет файла → fallback default.Model00p; если и его нет → `0x16`.
- Cloth/hair — отдельные `.fx`, тот же Model00p-контейнер (FVF 64). Формулы: [24-materials.md](24-materials.md).

## 9. Evidence

| Claim | Source |
|---|---|
| ext Model00p | **SDK** resourceextensions.h |
| CMP magic `0x4C444F4D` | **Ghidra** `0x00432614` in `0x004325a0` |
| version 0x21 | **Ghidra** + **empirical** cactus |
| 21×u32 header | **Ghidra** `0x00432ac0` |
| piece 0x28 field map | **Ghidra** asm `0x00431c20` `RET 0x18` |
| skeleton xyz+quat, hdr[2] nodes | **Ghidra** asm `0x00431d30` + **empirical** cactus @ `0xA8` |
| hdr[8] tables, hdr[20] blob, stack `ESP+0x5c` | **Ghidra** `0x00432ac0` RET 4 + `0x004326b0`/`0x00432773` |
| vertex-format sizes 0x24..0x58 | **Ghidra** `0x00431b40` + `0x00432be0` |
| VB/IB byte sizes + decl tableBytes | **empirical** 5 файлов |
| IB u16 triangle list | **empirical** max=nverts-1, count%3=0 |
| D3DCOLOR R=w0 G=w1, bones[] remap | **empirical** alma R+G=255; cactus local 0 → node 1 |
| hdr[10]=K, hdr[11]=N, hdr[12]=nGroups | **Ghidra** + **empirical** deltaForce 2/11 |
| M format type 0–6 disk = alloc size | **Ghidra** `0x004356c0`+Loads + **empirical** deltaForce EOF |
| deltaForce child models + K Default/RigidBody | **empirical** |
| CastShadow = 3-й байт `0x004323c0` → rec+7 | **empirical** alma vs noshadow; **Ghidra** `0x0051f200` читает +6/+7 |
| второй меш = shadow; 3×POSITION = triangle-in-vert | **empirical** + **archive** `shadowvolume_base.fxi` |
| `0x0051fac0` не строит silhouette | **Ghidra** только Draw technique 8 / `0x0051f200` |
| `GetAnimKeyFrames` / pose i16/64 + quat/32767 | **Ghidra** `0x00439eb0` / `0x0042ebf0` / `0x550ff8` / `0x551010` |
| hdr[0] = Σ piece.N (reloc-пары) | **empirical** cactus/cheeze/medKit/alma/deltaForce/soldiers; size-fn n×8 |
| type 1 nVerts/nIdx порядок полей | **Ghidra** ndisasm `0x0043c820`: `[esp+0x10]`=nVerts, `[esp+0x18]`=nIdx |
| FEAR.Arch00 physics census 495×v0x21 | **empirical** 510 extract; type 1/4/8 = 0 |
| type 3 r/mass + I=`0.4mr²` | **Ghidra** `0x00471bd0`/`0x004914d0` (`4.18879=4/3π`) + **empirical** deltaForce |
| type 3/7 third float = `fDensityG` (г/см³) | **SDK** `iltphysicssim.h` `HandleSphere(vCenter, fRadius, fMassKg, fDensityG)` / `CreateCapsuleShape(vEndPt1, vEndPt2, fRadius, fMassKg, fDensityG)` |
| type 7 capsule r/mass/pA/pB ×0.01 | **Ghidra** `0x00406320`/`0x004abee0` + **empirical** ±Y на конечностях |

## 10. Known unknowns

- Смысл M-блобов (anim/constraint, не VB).
- `DAT_00575d60` кто пишет в runtime.
- байт +6 группы.
- `skeletal.fxh` в Arch00 нет (макросы из `.fx`/`.fxo`). 4-я кость в lit VS нет (`w.xyz`, A не читается). **199v/254t во дворе = скелетные модели со `specular_env`** (capture: stride 64, VS-палитра костей 3×4, 5 сэмплеров: diffuse 512×512/512×256 DXT3, spec 256×128 DXT3, normal 256×256/128 A8R8G8B8, env cube 64² DXT3 общий, env-mask 256² DXT3; 3 инстанса каждая). Имена Model00p не установлены (не критично для рендера).

## 11. Acceptance

- Fixture: magic+version only → loader reject (не 0x21).
- Cactus-эквивалент: 7 имён, 1 piece, radius 53.03, 336 verts / 344 tris, UV≠0, nrm unit, **второй меш 936×32** только у группы с CastShadow.
- Skinned: alma-эквивалент с R+G=255, `bones[]` remap, не отдельный VB. `alma_noshadow` без shadow-хвоста.
- medKit: N=1 type 2; deltaForce: N=11 types 3+7, 7 child-model names.
- Shadow VS: rigid 2.0/r vs skeletal 2/√3 r; 3×POSITION skin+cross+extrude p0.

## 12. Status

`partial`. Закрыты: слоты 1–8 диск, type 3 sphere (r, mass, **density**), type 7 capsule (r, mass, density, pA, pB), M format, groups/shadow, hdr[0], GetAnim*, lit VS 3 кости. Остаток: type 4 без семпла; 199v двор; `skeletal.fxh` нет в архиве.
