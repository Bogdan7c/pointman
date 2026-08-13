# Pointman — ядро проекта

Нативный движок для порта F.E.A.R. на Linux (Android позже). Цель: 1:1 кампания + XP/XP2, Vulkan-рендер, геймпад по схеме Xbox 360.

Это не слив LithTech и не пиратский клиент. Форматы — публичный SDK 1.08 и community-спеки. Ассеты в репозиторий не входят: нужна легальная F.E.A.R. Ultimate Shooter Edition (Steam `21090`).

Контракт для агентов: `AGENTS.md` (не rustiplayer). Порядок фаз и критерии 1:1: `ROADMAP.md`. Стиль memories: `mem:memory_maintenance`.

## Текущая фаза

Сейчас фаза 1 (картинка Intro 1:1). Код 1.1 (SkyPointer/SkyCamera, `DdsCubemap`, lighting cubemap) есть; фаза не закрыта, пока двор на скрине без серой дыры. Дальше 1.2 болванки на стенах. Slow-mo / оружие / HUD не начинать, пока фаза 1 не закрыта сверкой скринов.

Lighting: Jupiter EX Blinn-Phong, не clustered/PBR. Геймпад: Xbox 360 SKU. PhysicsBSP — коллизия, не вторая стена в кадре. World00p индексы как в файле (совпадают с vertex normal); не менять i1/i2 — иначе кулится пол двора. G-buffer мира: CullMode::NONE, потому что цоколь Intro (Concrete_Wall / Brick_Red под black.Mat00) смотрит внутрь здания; BACK cull давал щель с небом под домом.

## Workspace crates

Контракт кадра: `Simulation::tick` → `draw_list()` → `Renderer::draw`.

- `apps/pointman` — игровой бинарь (winit, ввод, загрузка Intro). Не раздувать `main.rs` новым load-path.
- `apps/reztool` — CLI для Arch00/REZ (probe/list/world/extract)
- `crates/assets` — Arch00 (`LTAR`), REZ, World00p, WorldObjects, WorldModels, DDS/Mat00. Данных игры не возит.
- `crates/render` — Vulkan 1.1+ `ash`, deferred G-buffer. `backend.rs` уже сильно больше 700–800 строк — новые Vulkan-фичи только в отдельные модули. Тонкий контракт: `DrawList`, `MeshInstance`, `PointLight`, `TextureId`, `CubemapId` (небо, не стена). `DrawList.sky: Option<CubemapId>` — пустые пиксели lighting pass семплят cubemap. `DdsCubemap` отдельно от `DdsImage` (куб не грузится как 2D). `WorldSky` из SkyPointer/SkyCamera. Vulkan cubemap только в `crates/render/src/cubemap.rs`.
- `crates/engine` — `Simulation`, игрок, `ClipMesh`, `Input`
- `crates/game` — `Config`, `GameMount`, catalog/index, Steam-пути
- `crates/ai` — GOAP (`Planner`, `replica` demo vocabulary)

## Стек

- окно: `winit` (egui нет)
- графика: Vulkan 1.1+, `ash`, `gpu-allocator`, шейдеры через `glslc`
- геймпад: `gilrs`
- конфиг: `pointman.toml` / `POINTMAN_GAME_ROOT` / автодетект Steam-пути

## Проверки

- запуск: `cargo run -p pointman`
- архивы: `cargo run -p reztool -- list <Arch00>`
- CI: `cargo test -p pointman-ai -p pointman-assets -p pointman-game` (нужен `glslc`; engine/render/app в CI не входят)
- локально: `cargo test` также гоняет engine (synthetic tick/draw_list) и CPU Blinn-тесты render
- розничный Steam-install в тестах по умолчанию не читать
- живая картинка: kwin MCP **live** (`session_connect`, окно `POINTMAN — F.E.A.R. native`). `session_start`/виртуальный KWin запрещён. Skill: `pointman-kwin-live`. Не CI.
