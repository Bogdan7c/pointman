# Физика и коллизия

Две разные вещи: **ноги игрока** и **картинка**. PhysicsBSP не рисуется как вторая стена. (**инвариант Pointman**, подтверждается SDK-флагами)

## Два API (**SDK**)

### Legacy `ILTPhysics` (`iltphysics.h`)

Движение объекта как «коробки» в мире:

- `SetObjectDims` / `GetObjectDims`
- `MoveObject`
- `GetStandingOn`
- stair height, global force
- клиент: `ILTClientPhysics::UpdateMovement(MoveInfo*)`, `MovePushObjects` (строка `CLTPhysicsClient::MovePushObjects` в exe)

Сегмент vs мир: `ILTCSBase::IntersectSegment` / `IntersectSegmentAgainst`. **`FLAG2_PLAYERCOLLIDE`** — цилиндр по Y vs BSP; **`FLAG2_PLAYERSTAIRSTEP`** (40 cm, **SDK** `DEFAULT_STAIRSTEP_HEIGHT`) только вместе с ним. **`FLAG_POINTCOLLIDE`** — точка dims 0, ~10× быстрее, **без** standing-on. Не путать. (**SDK** `ltbasedefs.h`)

Главный мир: `GetMainWorldModel` (**SDK**; строки в PE **нет** — метод движка, не строковая регистрация). Пересечения с WorldModel: `FindWorldModelObjectIntersections` (строка в exe **есть**).

### `ILTPhysicsSim` (`iltphysicssim.h`)

Rigid body: `CreateRigidBody`, `TeleportRigidBody`, `GetWorldModelShape`. Группы: `ltphysicsgroup.h` — `ePhysicsGroup_WorldModels`, `ePhysicsGroup_MOPPWorldModels`.

Это не «современная физика вместо BSP». World00p PhysicsBSP остаётся для уровня; rigid body — для обломков/ограничений/трупов.

## Игровой слой (**SDK** `Game/Shared`)

| Файл | Смысл |
|---|---|
| `CharacterPhysics.h` | ragdoll, wall-stick |
| `PhysicsCollisionMgr.h` | цепочка столкновений server↔client |
| `PlayerRigidBody.h` | обёртка RB игрока |
| `SharedMovement.h` | `PlayerPhysicsModel` |

Розница: `ClientPhysicsCollisionMgr` / `ServerPhysicsCollisionMgr`, `PHYSICSCONSTRAINTCREATESTRUCT`, `KeyframeToRigidBody`. (**строки DLL**)

Консоль exe: `DrawWorldModelPhysicsDims`, `PhysicsBsp`. Строка exe **`PhysicsBsp`** `0x0054df64` (Pointman/packer: `PhysicsBSP`, case-insensitive). (**Ghidra**)

Гравитация игрока **−2000** cm/s² (**SDK** `DEFAULT_PLAYER_GRAVITY`). Кадр ног: `CMoveMgr::MoveLocalSolidObject` → `ILTClientPhysics::UpdateMovement` → exe `0x00407f60` / интегратор `0x0043cb30`.

## Clip-ноды PhysicsBSP 12 B (**Ghidra** `0x0047b280`)

Не vis. Не Havok. На диске после полигонов, **до** точек (Pointman `read_bsp` skip `node_count×12` — верный порядок).

| Off | Тип | Смысл |
|---|---|---|
| 0 | i32 | индекс полигона |
| 4 | i32 | child, отрицательное полупространство (`n·p − d < −r`) |
| 8 | i32 | child, положительное (`n·p − d > +r`) |

Child `≥ 0` = индекс ноды (runtime `base + i×0x10`). **−1** / **−2** = листья, обход стоп (`DAT_0056c260` / `DAT_0056c264`). Коллизия с полигоном **на внутренней ноде**, не «солидный лист».

Runtime нода **16 B**: poly*, 2 child ptr, byte visited, axis 0–5 или 6 (`0x0047aee0`). Последний u32 хедера WorldBsp = **root index** (−1/−2/≥0), не «всегда 0». Pointman читает его как `_zero` и дерево не строит — для Intro walk достаточно треугольников пола.

Sphere-walk `0x0040b880`; вход `0x0040c7a0` если `object+0x57 == 2` (`OT_WORLDMODEL`).

## `0x00425650` — не эти ноды

После WorldBsp-load (`0x004797d0`), до Asset00. Count × записи: **7 float** (`0x0041e1a0`) + stride `0x38`. Это `ILTPhysicsSim` shapes (коробки/MOPP), не ноги по BSP.

Три разных «12 байт» в мире: `vec3` плоскость; clip-нода; каталог blinddata `{size, typeId, offset}`. Vis kd-tree `0x0047abb0` — runtime stride **`0x2c`**, не 12.

## Что нужно для Intro

**Картинка:** PhysicsBSP не рисовать. Havok/`0x00425650`/ragdoll не нужны.

**Ходьба для сверки двора:** полигоны PhysicsBSP + blockers, капсула по Y (упрощение Pointman; у retail — **цилиндр** `FLAG2_PLAYERCOLLIDE`, см. выше), гравитация. Разбор 12 B нод — ускорение, не семантика пола. Pointman `ClipMesh` (фан → треугольники) для «двор узнаётся» достаточен. Stair 40 cm 1:1 — фаза игрока.

## Что Pointman уже знает

`ClipMesh` / PhysicsBSP — ходьба. WorldModel в кадр только если его нет в запечённом меше. Не путать `OT_WORLDMODEL` для рисования и physics shape для ног.
