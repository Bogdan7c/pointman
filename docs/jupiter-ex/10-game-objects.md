# Игровые объекты мира

Движок создаёт объекты по `ObjectCreateStruct`. Игровые классы живут в ObjectDLL / `GameServer.dll`. Рендер видит `OT_*` и флаги, не C++-имена. (**SDK**)

## WorldProperties

Уровень, не лампа. Пропы → cvars:

| Проп | Cvar | Intro (dump-draw) |
|---|---|---|
| `FarZ` | `FarZ` | 100000 |
| `ClampFarZ` | — | 1 |
| `AmbientLight` | `Light_AmbientR/G/B` = **value/255** | 25,25,25 → 0.098039 |
| `AddAmbientLightLow/Med/High` | добавка **после** `/255`, только выбранный LOD | Low=0.13; Med/High=0 |
| `SkyAmbientLight` | `Light_SkyAmbientR/G/B` | 0,0,0 |
| `FogEnable` | `Fog*` | 0 |

Класс: `Game/ObjectDLL/WorldProperties.*`. (**SDK** + **dump-draw**)

## GameStartPoint

Маркер спавна. Intro: `GameStartPoint00` — контрольная точка сверки кадра, не «фаза игрока закрыта». Класс: `GameStartPoint.h`, список через `GameStartPointMgr`. (**SDK**)

## Лампы как объекты

`LightBase::SetupEngineLight()` → `ILTCSBase::SetLightType` и поля радиуса/LOD/текстур. Типы: Point / PointFill / Spot / Cube / Directional. `OT_LIGHT`. Подробно [02-lights.md](02-lights.md).

Intro: 35 Point + 80 Fill + 39 Cube + 12 Spot + 2 Dir = **168**. (**dump-draw**)

## WorldModel

Именованный BSP-браш, не Model00p. Класс `WorldModel.h`: Visible, StartHidden, Translucent/Alpha, CastShadow, ShadowLOD. Retail GameServer: строки `StartHidden`, `CastShadow`, `CASTSHADOW 0`, `Translucent`, `TranslucentLight`. Движок: `OT_WORLDMODEL`; bake — отдельный список `DrawWorld`. `GetMainWorldModel` — exe, не DLL.

Фильтр «рисовать ли болванку» — решение Pointman (`world_model_in_frame`), не SDK.

WorldModel с shatter: `BlindObjectIndex` ≥ 0 → `GetBlindObjectData(index, 0xa85)` ([22-world00p.md](22-world00p.md)). На Intro индексы **27..92** (глобальные, не 0..65). Остальные WM = −1.

## BlindObjectData (не кадр)

Секция World00p после objects. Каталог `(size, typeId, offset)` в хвосте; offset от арены после `u32 count` + `u32 arenaBytes`. `GetBlindObjectData(nNum, nId)` — **глобальный** `nNum`. Intro: 26 KeyFramer (0..25) + 1 NavMesh (26) + 66 Shatter (27..92). Не albedo и не vis.

## Небо и кисти движка

`SkyPointer`, `SkyCamera`, `Decal`, `Brush` регистрируются в `ltengineobjects.cpp` (SDK engine objects, не Game/). `Decal` и `Brush` = `CF_NORUNTIME`: в Intro **0** штук; packer запекает. См. [05-sky.md](05-sky.md), [22-world00p.md](22-world00p.md).

## Модели персонажей / пропов

`OT_MODEL` + `ILTModel`. Формат Model00p в Pointman ещё не парсится — дыра фазы 1.3, не закрываем «на глаз».
