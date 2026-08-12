# Roadmap — 1:1 F.E.A.R. native (Linux + Android)

Цель: кампания, Extraction Point и Perseus Mandate **один в один** по геймплею, ассетам и ИИ, на своём движке. Бэкенд современный (Vulkan, нативный геймпад, Linux/Android), картинка и поведение — как у Jupiter EX, не «ремейк с PBR ради PBR».

Данные: розничная **F.E.A.R. Ultimate Shooter Edition** (Steam app `21090`). Ассеты в репозиторий не входят.

Схема геймпада — **Xbox 360 SKU** (не раскладка современного шутера):

| Ввод | Действие |
| --- | --- |
| LS / RS | ходьба / обзор |
| RT / LT | огонь / граната |
| RB / LB | следующее оружие / slow-mo |
| A B X Y | прыжок / мили / перезарядка (удерж. — use) / аптечка |
| D-pad | граната / фонарик / lean |
| LS/RS click | присед / ADS |
| Start / Back | пауза / задачи |

## Фазы

### 0. Каркас — в работе
- [x] Workspace, Vulkan deferred, GOAP, Arch00/REZ, World00p header
- [x] Steam-пути, `Default.archcfg`, probe архивов
- [x] Нативный геймпад, схема Xbox 360
- [x] CI, `main`, remote
- [x] Индекс World00p/Model00p/DDS по архивам без полной распаковки

### 1. Мир на экране
- [ ] Render surfaces из `World00p` (vertex blocks, materials)
- [ ] DDS → Vulkan, `Mat00` (diffuse/spec/normal)
- [ ] Камера/коллизия по BSP, не сквозь стены
- [ ] Загрузка Intro / первой карты кампании

### 2. Игрок 1:1
- [ ] Ходьба, присед, lean, прыжок, фонарик
- [ ] Slow-mo (рефлекс) с ресурсом как в оригинале
- [ ] ADS, recoil, оружие и гранаты из Gamdb00p
- [ ] HUD: патроны, здоровье, рефлексы, подсказки use

### 3. Объекты и скрипты
- [ ] Секция объектов World00p, property bag
- [ ] Клиент/сервер как LithTech (CShell / Object)
- [ ] Двери, лифты, триггеры, keyframer
- [ ] Пикапы, шкафчики, разрушаемые (shatter)

### 4. ИИ Replica
- [ ] Навмеш из blind data
- [ ] Полный GOAP (cover, suppress, flush, investigate)
- [ ] Perception / hearing / альянсы
- [ ] Анимации Model00p + AnmTree00p

### 5. Системы
- [ ] Физика (Rapier, формы из World00p)
- [ ] Звук (OpenAL; XWB/XSB → wav)
- [ ] Частицы / ClientFX
- [ ] Видео Bink → ffmpeg
- [ ] Сейвы, меню, субтитры Strdb00p

### 6. Кампания и DLC
- [ ] Все уровни FEAR
- [ ] FEARXP Extraction Point
- [ ] FEARXP2 Perseus Mandate
- [ ] Мультиплеер — после одиночки, не блокирует 1:1

### 7. Платформы и «современность»
- [ ] Linux: Wayland/X11, HDR optional, без DXVK
- [ ] Android: Vulkan 1.1, тач + геймпад, OBB/ассеты
- [ ] Рендер: clustered/deferred, тени, MSAA/TAA — **без ломки оригинального lighting model**
- [ ] Ребинды, Steam Input как опция, дефолт — 360

Порядок работы: не перескакивать фазы. Следующий пункт после каркаса — **индекс ассетов и первая World00p на экране**.
