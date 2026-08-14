# Что оригинал рисует из мира

## Bake World00p (**архив**)

Секция render: один mesh, поверхности + таблица материалов. Версии 113, XOR magic 399.

Vertex: pos, normal, UV, tangent, binormal. Индексы как в файле (не крутить i1/i2 — иначе кулится пол двора).

Цоколь Intro (`Concrete_Wall` / `Brick_Red` под `black.Mat00`) смотрит внутрь здания: BACK cull даёт щель с небом. Pointman: `CullMode::NONE`. Это эмпирика карты, не SDK.

`shadowvolume` поверхности скипаем как стены.

## WorldModel (**SDK** + эмпирика)

Типы: WorldModel, двери, switch, spinning. Геометрия — именованный BSP, не Model00p.

В кадр: Visible, не StartHidden, не PhysicsBSP, не sky object, не translucent (пока нет alpha pass), не `*shadow*`, не дубль запечённой стены.

Порог дубля `BakedOverlapIndex` (ячейка 16 см, доля 0.6) — **догадка Pointman**, не оригинал. Exe рисует bake и WorldModel **независимо** (`DrawWorld` / `DrawWorldModels`).

`Translucent`, `Alpha`, `TranslucentLight`, `CastShadow`, `ShadowLOD` — в SDK есть; Pointman берёт только Translucent как фильтр.

## PhysicsBSP

Коллизия ног, не вторая стена. Blockers — тоже клип.

## Model00p

Формат разобран **partial** (парсера в Pointman нет): render FVF 64, группы, pack весов, **shadow-меш** — [23-model00p.md](23-model00p.md). Pointman пропы без UV — цвет-хеш болванки.
