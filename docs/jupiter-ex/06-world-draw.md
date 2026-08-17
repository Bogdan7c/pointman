# Что оригинал рисует из мира

Bake, WorldModel и OT_MODEL — **три независимых списка** в classify. UV и Mat00 для кистей **не** живут в PhysicsBSP.

## Bake World00p (**архив** + **Ghidra**)

Секция render: один mesh, поверхности + таблица материалов. Версии 113, XOR magic 399. Layout: [22-world00p.md](22-world00p.md).

Vertex: pos, normal, UV, tangent, binormal. Индексы как в файле (не крутить i1/i2 — иначе кулится пол двора).

Цоколь Intro (`Concrete_Wall` / `Brick_Red` под `black.Mat00`) смотрит внутрь здания: BACK cull даёт щель с небом. Pointman: `CullMode::NONE`. Это эмпирика карты, не SDK.

`shadowvolume` поверхности — volumes (technique 8), не стены.

Classify `0x0051f550` (Ambient / Translucent / FogVolume, **Ghidra**): если `include_world` и cvar `DrawWorld`, для каждой **0x20-записи** мира с `record[+7]==0` и ненулевым материалом → `0x0050fa20(..., object*=0, bytes 8/0x19)`.

Те же 0x20-записи в light-classify `0x0051f700` / `0x0051f920` идут с LOD-фильтром `record[+7]-1 <= clamp(DAT_0056da84,0,2)` и байтами `9/0x1a`. Shadow-gather `0x0051fac0` берёт `record[+7]==0 && (record[+6]&1)` → байты `8/0x19` ([04-shadows.md](04-shadows.md)).

Проход кадра задаёт `.fx`-technique отдельно (`0x0050ffc0(..., tech=0)` = Ambient). Байты 8/9/0xc в record — класс источника, не id Ambient/Point.

## WorldModel (**SDK** + **Ghidra**)

Типы: WorldModel, двери, switch, spinning. Именованный BSP (коллизия), **не** Model00p. Кадр — не полигоны PhysicsBSP.

### Bind имени (**Ghidra**)

`se_InitWorldModel` `0x00459200`: lookup по имени (`arg+0x38`) через `DAT_0057237c` vt+0x10. Успех: смещение `*(object+0xf0)+0x1c` (xyz) крутится кватернионом `object+0xa8` (`0x00405da0`) и прибавляется к `object+0x9c/+0xa0/+0xa4`. Провал: строка «brush has at least one **non-renderonly** brush», код `0x17` (`LT_MISSINGWORLDMODEL`).

Серверный близнец: `WorldModelExtraInit` `0x00463e70` (`DAT_00572628` vt+0x10). Обёртка `0x00463ed0` при успехе копирует `*(ushort*)(wmdata+2)` → `object+0x154`.

`object+0xf0` — **не** общий слот: у `OT_CUSTOMRENDER` туда кладут custom-render ptr (`0x0051f550` → `0x0050f9b0`). У WM это ptr на wmdata; первый **ushort** = индекс таблицы кусков.

### Кадр `DrawWorldModels` `0x0051ebf0` (**Ghidra**)

```
table = *(DAT_00576ff4 + 0x18)          # пары {piece*, count}, stride 8
entry  = table[ *(ushort*)*(object+0xf0) ]
piece  stride 0x14
record stride 0x20
```

Кусок 0x14 (тот же layout, что vis world-block `DAT_00576ff4+4`, индекс `node+0x24`, **Ghidra** `0x00520ea0`):

| Off | Поле |
|---|---|
| `+8` | ushort: число «видимых» / начало диапазона |
| `+0xc` | ptr на массив 0x20-записей |
| `+0x10` | uint: полное число записей |

Запись 0x20 (bake и WM — **один** формат):

| Off | Поле |
|---|---|
| `+0` | ptr на material-struct (`[0]` handle/mesh-bind, `[1]` таблица слотов) |
| `+4` | ushort: индекс runtime-поверхности stride **0x34** |
| `+6` | flags; bit0 = volume (shadow gather); bit1 = visited vis |
| `+7` | LOD-байт; bake Ambient требует `==0` |
| `+8`…`+0x1c` | AABB min/max (debug `0x00516330`) |

Дальше тот же `0x0050fa20` → `0x0050f8f0`, что bake:

1. `0x00511fb0(*material)`: кэш `+8`, иначе handle `+0` → `0x0044fdf0` → кусок `ushort(+4)×0x20`.
2. Поверхность = `*mesh + index×0x34`. Skip если `surf+0x10==0`.
3. Слот текстуры/шейдера: `material[1][ *(ushort*)(surf+0x14) ]`.

Opaque classify зовёт `0x0051ebf0(..., param_3=0)`: цикл `ushort(piece+8) .. uint(piece+0x10)`, байты **0xc/0x1b**. Light-classify (`0x0051f700` Point/Spot/Cube, `0x0051f920` Dir) зовёт `param_3=1`: цикл `0 .. ushort(piece+8)`, фильтр `record[+7]-1 <= clamp(DAT_0056da84,0,2)` (default **2** = High), байты **0xd/0x1c**.

### Что это значит для картинки

UV и Mat00 WM — в **render-меше** (те же 0x34-поверхности / VB, что bake), не в clip-полигонах PhysicsBSP. Render-only кисть без collision-браша объекта не создаёт (`LT_MISSINGWORLDMODEL`). Exe рисует bake и WM **независимо**, если оба visible. Pointman `BakedOverlapIndex` 16 см / 0.6 — **не** оригинал.

В кадр (политика порта, не exe): Visible, не StartHidden, не PhysicsBSP, не sky, не translucent (пока нет alpha), не `*shadow*`. `Translucent` / `Alpha` / `TranslucentLight` / `CastShadow` / `ShadowLOD` — в SDK; Pointman сейчас режет только Translucent.

## PhysicsBSP

Коллизия ног, не вторая стена. Blockers — тоже клип. 12 B нода = `i32` poly + 2 child. UV там нет.

## Model00p (`OT_MODEL`)

Формат диска: [23-model00p.md](23-model00p.md). Кадр `0x0051f200` (**Ghidra**):

| Off объекта | Смысл |
|---|---|
| `+0x110` | runtime Model00p* |
| `+0x13c` | material-binding* (`0x00435b80` просто читает это поле) |
| `+0x140` | bias к mesh-LOD |
| `+0x12c` | bitset скрытых pieces (`local_28>>5`) |

Mesh-LOD: `0x0042ee70` vs piece stride `0x0c` (`model+0x34`, count `model+0x38`). Поверхности снова stride **0x34** через `0x00511fb0`. Байты record: `0xa/0x17` (или `+1`, если `param_5` — light/shadow path). UV — FVF64 в файле модели. Pointman пропы без парсера = цвет-хеш, UV=0.

## Evidence

| Claim | Source |
|---|---|
| Bake и WM — два списка, overlap 0.6 не оригинал | **Ghidra** `0x0051f550` / `0x0051ebf0` |
| 0x20 record + 0x14 piece + таблица `DAT_00576ff4+0x18` | **Ghidra** `0x0051ebf0` / `0x00520ea0` |
| `se_InitWorldModel` / ExtraInit / `LT_MISSINGWORLDMODEL` | **Ghidra** `0x00459200` / `0x00463e70` + строки |
| Поверхность 0x34, mesh через `0x00511fb0` | **Ghidra** `0x0050f8f0` |
| OT_MODEL `+0x110/+0x13c` | **Ghidra** `0x0051f200` / `0x00435b80` |

## Known unknowns

- `DAT_00576ff4+0x18` = TOC[0] записей, грузит `0x0050d0a0`. Entry0: `nPieces=nSectors`; прочие по 1 куску. Кто пишет `object+0xf0` (индекс в эту таблицу) — open.
- Хвост World00p закрыт: [22-world00p.md](22-world00p.md). Extra-полигоны обычно квады; в кадр не тащить, пока нет роли.
- Имена байт 8/9/0xc в 0x28 record vs слот `0x00503a30`.
- Один мировой VB на все WM или per-WM mesh (handle в material-struct).
