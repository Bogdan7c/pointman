# Анимация и модели

## Движок `ILTModel` (**SDK** `iltmodel.h`)

- трекеры: `AddTracker`, `UpdateMainTracker`, `SetCurAnim`
- скелет: `GetNodeTransform`, `GetSocketTransform`
- веса: `SetWeightSet`, `FindPhysicsWeightSet`
- материалы на кусках модели: `SetMaterial`
- клиентские запросы: `ILTModelClient`

Строки exe: `CLTModelClient`, `CLTModelServer`, `ILTModel::GetAnimKeyFrames`. (**Ghidra**)

On-disk Model00p: hdr[11] слотов `0x004320b0` (type 1–8 = physics-shape грамматика `0x0044ccc0`), hdr[10] биндов. Keyframes **в том же Model00p** (не отдельный Anim00p в Arch00). AnmTree только выбирает имя клипа.

## Keyframes (`ILTModel`, **Ghidra** asm)

Таблица анимаций модели: begin `model+0x64`, end `+0x68`, stride **8** (`sar 3`). Это тот же custom-vector `0x004302f0(model+0x60)` (элемент 8 B). Запись: `[0]` = shared header (`model+0x44`, `0x0042ef90`), `[4]` = объект клипа.

`GetAnimKeyFrames` `0x00439eb0`: `*out = clip+0x14` (число кадров). `GetCurAnimLength` `0x0043a2c0` → `0x0042e3e0`: время последнего ключа `clip+0x10[(n-1)]`, массив `{u32 time_ms, u32}`. `GetAnimName` `0x0043a1c0`: `clip+0x24` строка. `GetAnimIndex` `0x00438d60` → bsearch `0x0042f410` по `model+0x74` (отсортированные индексы, `_stricmp`).

`GetAnimNodeTransform` `0x00439cd0` (object-space, parent chain):

1. clip = table[hAnim][4]; node = `model+0x2c`[hNode]; `hKey < clip+0x14`.
2. parent = `*(u16*)(node+8)`; `0xFFFF` = корень. Идём к корню.
3. Трек ноды: `header+8` → per-node index; `0xFFFFFFFF` → bind pose `node+0x4c` (xyz) / `+0x58` (quat).
4. Иначе `0x0042ebf0(clip, key, nodeIndex, model)`:
   - таблица `clip+0x1c`: 2×u16 шапка + на ноду `(u16 pos, u16 rot)` = **hdr[8]** on-disk (`0x004325a0`).
   - `0xFFFF` pos/rot = bind.
   - packed bytes `clip+0x20` ← **hdr[20]**.
   - **pos**: если `clip+0x1a` ≠ 0 → три **i16 × 1/64** (`0x550ff8` = 0.015625); иначе три f32. Знаковый индекс с битом 15 → `0x0042e2e0` (`& 0x7fff`).
   - **quat**: четыре **i16 × 1/32767** (`0x551010`).
5. Локальный pose × parent (`0x0042f070` / `0x0040b0f0`).

hdr[0] в size-fn — это **Σ piece.N** (reloc-пары), не число клипов. cactus hdr[0]=2 = N куска; soldiers 2652 пар на 43 pieces. `GetAnim*` vector: `0x004306f0` пушит **по одному 8 B на piece** (hdr[1]), не hdr[0]. Имена `Null`/`Twitch` живут в name blob; это не 1:1 с hdr[0].

Отдельного `*.Anim00p` в FEAR.Arch00 нет (`RESEXT_KEY_*` в SDK есть, в рознице не лежит).

## AnmTree00p (packed search tree)

Не скелетные ключи. Это **выбор анимации по props** (Action/Body/Weapon/…). Кадры живут в Model00p / anim resources. Грузит игровой слой, не `FEAR.exe` (строк `ANMT` в exe нет).

Формат (**SDK** `Game/Shared/AnimationTreePacked*.h` + **empirical** EOF на Bird/Alma/Soldier):

```text
u32 FourCC 'ANMT'          # LE 0x544D4E41
u32 version = 3
# дальше один data-block (без повторного header):
u32 nStringTable           # байты, 4-aligned
char stringTable[n]        # [0] = имя дерева (Bird/Alma/Soldier)
u32 cAnimPropGroups        # Intro-выборки: 10; запись 12 B (строковый offset группы, count, index в props)
u32 cAnimDescGroups        # 2; запись 12 B
u32 cAnimProps             # 150; u32 = offset в string table, FixUp → enum
u32 cAnimDescs             # 21
u32 animDescData[cAnimDescs]           # по 4 B каждый
u32 cTransitions
  EnumDesc[cTransitions * cAnimDescGroups]   # EnumDesc = 4 B
  AT_TRANSITION[cTransitions]          # 32 B; имена/blend — индексы, FixUp → ptr
u32 cTransitionSetTransitions
  u32 indices[]
u32 cTransitionSets
  AT_TRANSITION_SET[]                  # 8 B
u32 cAnimations
  EnumDesc[cAnimations * cAnimDescGroups]
  AT_ANIMATION[cAnimations]            # 52 B
u32 cPatterns
  EnumProp[cPatterns * cAnimPropGroups]      # EnumProp = 4 B
  AT_PATTERN[cPatterns]                # 12 B
u32 cTreeNodes
  AT_TREE_NODE[cTreeNodes]             # 20 B; root = [0]
```

FixUp (**SDK** `CAnimationTreePackedLoader::FixUpAnimTreeData`): индексы в string table / массивы → указатели и `AnimPropUtils::Enum`. Не копировать .cpp в git.

| файл | nStr | trans | anims | patterns | nodes | remain |
|---|---|---|---|---|---|---|
| Bird 4828 | 2712 | 1 | 8 | 8 | 15 | **0** |
| Alma 6376 | 2732 | 0 | 17 | 17 | 43 | **0** |
| Soldier 120924 | 7596 | 75 | 649 | 542 | 2047 | **0** |

Ресурс: `AnimationDatabase/AnimTrees/*.AnmTree00p` (`RESEXT_ANIMTREE_PACKED`).

## Status

`verified-static` (SDK structs + three files EOF). Keyframes Model00p: pos i16/64, quat i16/32767, bind `0xFFFF` — **Ghidra** `0x0042ebf0`. hdr[0]=Σ reloc-пар (**empirical** 6 файлов). Cloth/hair lighting — [24-materials.md](24-materials.md).

## Игровой слой (**SDK** `Game/Shared`)

| Модуль | Смысл |
|---|---|
| `ModelsDB` | скелеты / ноды из игровой БД |
| `Animator` | высокоуровневые состояния |
| `AnimationTreePackedMgr` | упакованные деревья (строка есть в **обеих** DLL) |
| `AnimatorPlayer` | игрок |

Серверная строка: `DisableServerAnimationBlending`. Клиент: `CNodeController`, `CLeanNodeController`.

## Шейдеры моделей (**архив**)

Каталог `Shaders/skeletal/` зеркалит `rigid/`: `skin.fx`, cloth, specular, translucent FX. Тень: `skeletal/shadowvolume*.fxi` и `rigidshadowvolume*.fxi`.

Rigid и skeletal — **один** technique-набор: skeletal-файлы это `#define SKELETAL_MATERIAL` + include того же rigid `.fx`; разница только в skinning VS. Lighting include `dx9lights.fxh` общий (в extract нет).
