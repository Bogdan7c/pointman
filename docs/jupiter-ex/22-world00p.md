# World00p — render layout

## 1. Scope

Бинарный layout секции render розничного World00p v113. Не PhysicsBSP. Bake vs WorldModel: **два списка** в `0x0051f550` ([27-visibility-sort.md](27-visibility-sort.md)); Pointman overlap 0.6 **не** оригинал.

## 2. Ownership

Файл мира в Arch00. Парсер Pointman: `crates/assets/src/world00p.rs` — **empirical** на Intro, не закрытый loader exe.

## 3. Inputs / layout (**archive** + **empirical** Intro)

Header 56 bytes:

| Off | Field |
|---|---|
| 0 | version u32 = **113** |
| 4 | render_section_offset |
| 8 | sector_section_offset — vis WorldTree, см. ниже |
| 12 | object_section_offset |
| 16 | blinddata_section_offset (**не кадр**; layout ниже) |
| 20 | AABB min xyz |
| 32 | AABB max xyz |
| 44 | offset xyz (в кадре Pointman не применяется) |

Magic XOR для counts в BSP: `raw ^ 399`. Exe считает так же: старт `0x71`, плюс байты строки `"FEAR"` в `DAT_0055446c` (`0x00479390`, **Ghidra**). `113+'F'+'E'+'A'+'R'=399`.

Render section: **10×u32 TOC** @ `render_section_offset`, затем mesh body (counts **повторяются**). SharedBSP `0x00479c50` эту секцию **не** читает.

`CWorldClientBSP` vtable `0x00554380` — **14** слотов (`+0x00`…`+0x34`), потом строка имени. `LoadFromStream` `0x004781b0` зовёт **интерфейсные** `[vt+0x40]` / `[vt+0x44]` @ `0x00478209`/`0x00478223` — на этой таблице их нет. Импл LoadSector = `0x00478140` (**vt+0x34**): `this+=0x40; jmp 0x00458610`. **vt+0x30** `0x00478150` = хук `DAT_00575d74` (в PE ноль, заполняется в runtime). Публичный Load `0x004782c0`: `IWorldSharedBSP` `DAT_005753e8` vt+0x0c → член `this+4`, затем vt+0x2c, `0x00478170`. Layout объекта: `+4` shared, `+0x40` WorldTree, `+0x60` resource, `+0x64` loaded, `+0x66` имя. (**Ghidra**)

Preamble на трёх v113 (**empirical** Intro / Factory / WTF_Entry):

| Slot | Intro | Factory | WTF_Entry | Смысл |
|---|---|---|---|---|
| [0] | 347 | 715 | 338 | число **записей** хвоста = PhysicsBSP `bsp_count`. Таблица `DAT_00576ff4+0x18` |
| [1] | 465 | 954 | 458 | число **кусков** (0x14): `[0] + nSectors − 1`. Entry0 даёт `nSectors` кусков, остальные по 1 |
| [2] / [6] | 2268 | 5013 | 2475 | `surface_count`; столько же 36 B AABB в хвосте |
| [3] | 1 | 1 | 1 | `render_mesh_count`; цикл `0x0050d0a0`; Pointman error если ≠1 |
| [4] | 508 | 698 | 521 | `material_count` |
| [5] | 1 | 1 | 1 | = [3] на всех трёх; exe ≠1 **unknown** |
| [7] | 108 | 182 | 80 | число extra-полигонов (`0x00513f70`) |
| [8] | 432 | 726 | 320 | Σ вершин extra. Обычно 4 (квад); Factory 180×4+2×3=726 |
| [9] | 0 | 0 | 0 | читается (`0x0050d0a0` 10-й u32); мир где ≠0 не найден |

Сразу после 40 B: mesh `unknown_id=0`, те же surface/material counts, VB/IB, 11 pack defs, 2268×36 B surfaces, 508 имён Mat00. Pointman читает 10 u32, требует `[3]=1`, остальное игнор.

### Хвост render → sector (**Ghidra** `0x0050d0a0` + **empirical** 3 карты)

После имён Mat00 stream стоит на хвосте. Reader: `0x0050ce70` (TOC[0..8]) + ещё один Read (TOC[9]) → один mesh `0x00512000` (цикл TOC[3]) → **TOC[0]** раз `0x0050cf40`.

```text
# 0x0050cf40 × TOC[0]
u32 nPieces
# 0x005144c0 × nPieces
u32 nRec
u32 extra          # используются младшие 16 бит
nRec × 36 B        # runtime record 0x20
# 0x00513f70 × extra
u8 n
n × 12 B           # vec3
```

Entry **0**: `nPieces == nSectors` (119 / 240 / 121). Остальные TOC[0]−1 записей: `nPieces == 1`. Отсюда TOC[1] = TOC[0] + nSectors − 1.

Σ `nRec` = `surface_count` (каждая bake-поверхность ровно один раз). Intro «заголовок» `119,66,12` — это `nPieces, nRec, extra` **первого куска**, не глобальный hdr.

Запись 36 B → runtime 0x20 (`0x005144c0`, 9×Read 4):

| Off disk | Поле | Runtime 0x20 |
|---|---|---|
| 0 | AABB min xyz | `+8` |
| 12 | AABB max xyz | `+0x14` |
| 24 | u32 flags (часто `0`; `1`/`3` = shadowvolume) | `+6/+7` LOD/flags |
| 28 | u32 `0` | — |
| 32 | u32 индекс bake-поверхности | `+4` ushort + material* |

Extra: почти всегда `n=4` (квад). Factory: 180×4 + 2×3 = 726 = TOC[8]. **Семантика: sky-порталы** (`0x00513ac0` клипит полигон плоскостью `param+0x1ec/+0x1f0/+0x1f4/+0x1f8` и расширяет AABB; `0x005187d0` собирает AABB **мира из extra** для sky-pass `0x00518a70`; `0x00515af0` рисует extra цветом `0x50ffff00` под cvar **`DrawSkyPortals`**). В боевой кадр (Ambient/light/translucent/shadow) extra **не** входят — не рисовать как стены.

Кусок runtime 0x14: `+0` = ptr массива extra-записей (8 B: `{ptr vec3[], n}`), `+4` = extra count (ushort, из `u32 extra`), `+8` = visible ushort (vis), `+0xc` = ptr записей 0x20, `+0x10` = count записей (`0x005144c0` / `0x0051ebf0` / `0x00520ea0`).

TOC[9]: читается в `0x0050d0a0` отдельным Read **после** `0x0050ce70`, в расчёт размера арены `sVar7` не входит, значение никуда не пишется — **зарезервирован** (0 на всех трёх картах).

`347×208` и `89660÷20` отвергнуты: хвост — переменные куски, не фиксированный stride. Три карты: consume 100%, 0 leftover.

Vertex pack: location 0 pos, 3 normal, 5 UV, 6 tangent, 7 binormal. Indices u16, winding **as-file**.

### BSP / WorldModel секция @ byte 56 (**empirical** Pointman `world_models.rs`, не SDK struct)

После header сразу BSP-блок (Intro; «всегда 56» для всех миров — **hypothesis**). AABB min/max, count, flags bitset, 8×u32 XOR `399` → name_count, names_len, plane_count, bsp_count (назначение остальных 4 из 8 — **open**). Имена null-terminated. Per-BSP: polys (`12+nverts×4`), затем **node_count × 12 B** clip-ноды (`i32` poly + 2 child, −1/−2 лист), затем points. Pointman skip нод — для кадра верно. Хедер: последний u32 = **root**, не unused. Blockers: fan-tris. Подробно [09-physics.md](09-physics.md).

Имя в exe **`PhysicsBsp`** `0x0054df64`; Pointman `"PhysicsBSP"` — тот же brush (**SDK** `PhysicsWorldBsp`), case-insensitive.

Load `0x00479c50`: vis kd-tree `0x0047abb0` (stride 0x2c) → WorldBsp+clip `0x004797d0` → sim shapes `0x00425650` (7×f32, **не** ноды) → Asset00 → blinddata.

### Objects @ `object_section_offset` (**empirical** `world_objects.rs`)

u32 count; per object: u16-len type + property bag (name_off, kind, 4 data). Kinds: STRING/VECTOR/COLOUR/FLOAT/INT/FLAGS/QUAT. Pointman держит подмножество типов; полный дамп — `world_object_dump.rs`.

`GetMainWorldModel()` (**SDK** `iltcsbase.h`) — runtime handle главного мира для пересечений, **не** фильтр «не рисуй instance». В GameServer.dll строки `GetMainWorldModel` **нет** (это метод exe `ILTCSBase`).

Поверхности с `"shadowvolume"` в имени материала — volumes, не стены. Intro: **289** шт., `engine\shadowvolume.Mat00`, stride **24**, pack **10**. Pointman skip; оригинал — technique 8.

**Pack 5** (stride 24, pos + D3DCOLOR + UV): Intro **95** поверхностей / 998 tris. Technique Translucent `multiply.fx`: `SrcBlend=Zero`, `DestBlend=SrcColor`, Cull None. Кровь/граффити/drips/`Car_Shadow.Mat00` — и окна/vending с тем же layout. WorldEdit `Decal` = `CF_NORUNTIME`: в exe **0** объектов; packer проецирует в bake. Какие pack-5 строки = Decal vs translucent brush — **не закрыто**. `Car_Shadows` / `blood00` — отдельные WM (`Translucent=1`, `CastShadow=0`), не pack-5.

Другие pack Intro (не albedo-стены): pack 2 `Null.Mat00` ×3 (stride 12) — skip **unknown**; `Invisible.Mat00` ×28 pack 3 — skip **unknown**; pack 6 glow ×19; pack 8 `Sky_Day.Mat00` ×1 в world mesh vs SkyPointer — **unknown**; pack 4 glass/foot ×19 stride 60.

### Sector section @ `sector_section_offset` (**Ghidra** `0x00458610` + **empirical** Intro, 18 747 B точный конец)

`CWorldClientBSP` (`0x004781b0`): Seek(0) → `0x00479570` (version `0x71` + 4 offset) → Seek(render) → vtable load mesh → Seek(sector) → WorldTree `0x00458610` (thunk `this+0x40`).

`ILTInStream` (**SDK** `iltinstream.h`, совпадает с exe): `+4` Read, `+8` ReadString = **uint16 длина без нуля + N символов**, `+0x14` SeekTo(uint64) как два u32 (hi=0).

Заголовок секции — **8×u32** (`0x00458110`):

| Slot | Intro | Смысл |
|---|---|---|
| [0] | 119 | `nSectors` — vis-ноды stride **0x40** |
| [1] | 128 | `nHulls` — полигоны оболочек (на Intro все **quad nv=4**) |
| [2] | 57 | `nPortals` — runtime stride **0x18** |
| [3] | 6 | арена (не отдельный диск-блоб) |
| [4] | 512 | = 128×4 вершин оболочек; арена `0x00458020` |
| [5] | 1192 | размер runtime scratch-арены clip-плоскостей (PVS bitset **нет**, см. §10), **не** хвост файла |
| [6] | 0 | арена |
| [7] | 256 | арена |

Дальше **последовательно, без паддинга между записями**:

1. **nHulls** записей `0x00458de0` (объект 0x30): `u16 nVerts` + `nVerts×vec3` (упакованы сразу после u16, float может быть невыровнен) + 4×f32 плоскость `(n, d)` + vec3 + f32 (на Intro радиус/экстент, напр. 573). Плоскость первой оболочки Intro: `(0,0,-1)` d=`-800` при кваде z=800.
2. **nSectors** vis-нод `0x00458e80` (объект 0x40): ReadString имя → vec3 min → vec3 max → `u32 nPlanes` → `nPlanes×` 16 B (4 float; в рантайме пятый dword — октант) → `u32 nHullIdx` → `nHullIdx×u32` индексы оболочек → `u32 sectorId` (Intro: **0..118 подряд**). Имена Intro: 99×`Brush00`, плюс `Office_Sector*`, `Hall_Sector*`, `*.Sector1` дверей (**SDK** `GetSectorID` / Door). **Все 119 nPlanes=0** на Intro — clip-плоскости `0x00458a60` на этой карте не из файла.
3. **nPortals** записей `0x00458cf0`: `u32 nIdx` + `nIdx×u32` (индексы vis-нод / оболочек) + u32 flags (Intro: 0/2/3) + f32 (Intro: 150, −17500, …) + **два i32** связанных портала (`-1` = нет, иначе `base+i*0x18`).

Intro: 32 + 128×82 + vis-строки/AABB/индексы + 57 порталов = **18 747 = длина секции, остаток 0**.

### Blinddata @ `blinddata_section_offset` (**empirical** Intro + **SDK** `GetBlindObjectData`)

Секция **не** в кадре. Игра читает чанки по индексу из пропов объекта (`KeyDataIndex` / `BlindDataIndex` / `BlindObjectIndex`).

Intro: 126 792 B ровно:

```
u32 nChunks                         // 93
u32 arenaBytes                      // 125668
u8  arena[arenaBytes]               // плотная укладка; старт каждого чанка % 4 == 0
struct {                            // каталог в хвосте, nChunks × 12
    u32 size;
    u32 typeId;
    u32 offset;                     // от начала arena, не от байта 0 секции
} dir[nChunks];
```

Проверка: `8 + arenaBytes + nChunks×12 = длина секции` (Intro: 8+125668+1116=126792). Перед каждым чанком (включая первый) паддинг 0..3 байта до выравнивания 4 — гистограмма Intro по 93 чанкам: 75×0, 9×2, 5×3, 4×1. Каталог **не** после count: байты `[4:4+n×12]` — это уже payload.

`GetBlindObjectData(nNum, nId, pData, nSize)` (**SDK**): `nNum` = **глобальный** индекс в `dir`, не «номер среди своего typeId». Несовпадение `dir[nNum].typeId != nId` → fail. Указатель = `arena + dir[nNum].offset`, размер = `dir[nNum].size`.

WorldPacker пишет индекс в float как `N + 0.1` (чтобы `(uint32)f` отрезал дробь). Intro:

| typeId | SDK `#define` | n | `nNum` | проп |
|---|---|---|---|---|
| `0x6aaf0884` | `KEYFRAMER_BLINDOBJECTID` | 26 | **0..25** | KeyFramer.`KeyDataIndex` |
| `0x83f47c31` | `AINAVMESH_BLINDOBJECTID` | 1 | **26** | только `AINavMesh02`.`BlindDataIndex`; остальные 6 AINavMesh = −1 (packer склеивает в один блоб) |
| `0x00000a85` | `SHATTERINFO_BLINDOBJECTID` | 66 | **27..92** | WorldModel.`BlindObjectIndex` (остальные WM = −1) |

Scatter (`0x73f53a84`) и Stalk (`0x6aaf0885`) на Intro **нет**.

KeyFramer payload: `SKeyDataHeader` {version=**1**, nKeys, keyDataSize} + ключи. Stride Linear = 36 B (`fi7f`), Bezier = 60 B (`fi7f6f`); Intro: 2-key Linear → header+72=84; 4-key Bezier → 252; 5-key Linear → 192. Совпадает со всеми 26 чанками. Команды: `blindData + keyDataSize` (**SDK** `KeyFramer.cpp`), если `keyDataSize < size`.

Shatter: первый u32 = число полигонов (Intro все 66: 1..138, ни одного `>0xFFFF`).

NavMesh: `u32 processed` (Intro **1** = packed) + cooked payload (`version=6`, `fixed_up=0`, edges…). `SetupNavMesh` скидывает первые 4 байта (**SDK** + **ndisasm** GameServer `0x10059e32`). Не drift. Не рендер. [15-ai.md](15-ai.md).

Runtime WorldTree (`0x00458590` destroy): blob `[0]`; vis-ноды `[1]` stride 0x40 count `[2]`; ptr-массив оболочек `[3]` count `[4]`; порталы `[5]` stride 0x18 count `[6]`. Init ноды `0x00458c40`: `+0x38=1` vis, `+0x39=1`, `+0x3a=0` visited. Хеш имени `0x00475360`: `h = toupper(c)-'A' + h*0x1d` (**Ghidra**, для `GetSectorID`).

Kd-tree `0x0047abb0` / lookup `0x00458420` — **другой** блок (после header @56 / SharedBSP), листья указывают на vis-ноды. Не путать с 12 B PhysicsBSP.

## 4. Алгоритм загрузки (наблюдаемый формат)

1. Проверить version==113 (`CWorldSharedBSP` `0x00479c50` / client `0x00479570`).
2. Прочитать 4 section offset + AABB; `1/extent` в объект.
3. SharedBSP: дерево/WM @56 (`0x0047abb0`), XOR-counts (`0x00479390`), physics `0x00425650`, Asset00 `0x00479a00`.
4. Seek `render_section_offset` — mesh (Pointman `WorldRender::parse`).
5. Seek `sector_section_offset` — WorldTree `0x00458610` (hulls → vis → portals).
6. Objects @ `object_section_offset`.
7. Seek `blinddata_section_offset` — `GetBlindObjectData` (**SDK** `ILTCSBase`). Не vis и не albedo.

Exe vis кадр: kd-tree `0x00458420` находит сектор; AABB `0x00458a60` (`this+0xc` min/max vis-ноды); порталы `0x00521080` ([27-visibility-sort.md](27-visibility-sort.md)).

## 5. Constants

`FEAR_WORLD_VERSION=113`, `FEAR_WORLD_MAGIC=399`.

## 6. State tables

N/A (asset).

## 7. Псевдокод

См. `WorldRender::parse` — зеркало формата, не exe. Для порта: тот же layout; vis — только после R06.

## 8. Edge cases

`prop.id != 0` streams пропускаются. Extra UV нет в парсере. Несколько render mesh ≠1 — ошибка в Pointman; поведение exe **unknown**.

## 9. Evidence

| Claim | Source |
|---|---|
| v113, header 56B, winding | **empirical** Intro tests `world00p.rs` |
| XOR 399 = `0x71+"FEAR"` | **Ghidra** `0x00479390` / `DAT_0055446c` |
| Sector 8×u32 + 128 hull + 119 vis + 57 portal, exact size | **Ghidra** `0x00458610` + **empirical** Intro.World00p |
| TOC[0]=bsp entries; [1]=pieces; [7]/[8]=extra count/verts; хвост 100% | **Ghidra** `0x0050d0a0`/`0x005144c0`/`0x00513f70` + **empirical** 3×v113 |
| Extra = sky-порталы; AABB мира из extra; debug `DrawSkyPortals` | **Ghidra** `0x00513ac0`/`0x005187d0`/`0x00515af0` + cvar record `0x0056d2c8` |
| TOC[9] читается (Read после `0x0050ce70`) и игнорируется | **Ghidra** `0x0050d0a0` asm `0050d11b` |
| Кусок 0x14: `+0` extra, `+4` count, `+0xc` records, `+0x10` count | **Ghidra** `0x005144c0`/`0x0051ebf0`/`0x00520ea0` |
| ReadString uint16+chars | **SDK** `iltinstream.h` + vis names Intro |
| Blinddata `count+arena+dir`; offset от arena; nNum глобальный | **empirical** Intro 126792 B + **SDK** IDs/пропы (`KeyDataIndex` 0..25, NavMesh **26**, Shatter **27..92**) |
| KeyFramer header v1 + Linear/Bezier stride | **SDK** `SKeyDataHeader` / `DATATYPE_TO_ENDIANFORMAT` + Intro 26/26 |
| shadowvolume skip | **archive** / **hypothesis** as walls |

## 10. Known unknowns

- PVS bitset **нет**; flood геометрический — [27-visibility-sort.md](27-visibility-sort.md).
- NavMesh: полный layout edges/polys/quadtree после header (не кадр).
- Смысл hull vec3+f32 после плоскости; portal flags 0/2/3 и f32.
- PhysicsBSP 12 B clip-ноды закрыты ([09-physics.md](09-physics.md)); vis kd-tree — другой блок stride 0x2c.
- TOC[0]..[9] закрыты (три v113). TOC[9] читается и игнорируется — зарезервирован.
- Extra-полигоны: обычно квад; роль (портал/clip/shadow) **hypothesis**.
- `vt+0x40` @ `0x00478209` — слот интерфейса. Импл LoadSector = `0x00478140`. Хвост читает `0x0050d0a0` / `0x0050cf40` / `0x005144c0` / `0x00513f70`.
- `header.offset` в transforms.

## 11. Acceptance

Synthetic World00p (без retail) крутит parse+winding test. Intro optional: courtyard floor winding vs normals. Vis culling test — после R06.

## 12. Status

`verified-static` по TOC[0]–[9] и layout хвоста (Ghidra + 3 карты, consume 100%). Extra = sky-порталы (AABB мира + debug DrawSkyPortals), в кадр не входят. TOC[9] = зарезервирован.
