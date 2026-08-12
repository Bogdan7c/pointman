# Pointman

Нативный движок для порта **F.E.A.R.** на Linux и Android. Цель — **1:1** кампания + XP/XP2, современный Vulkan-бэкенд, нативный геймпад по схеме **Xbox 360**.

Ассеты не входят в репозиторий. Нужна легальная **F.E.A.R. Ultimate Shooter Edition** (Steam `21090`).

Полный план: [ROADMAP.md](ROADMAP.md).

## Стек

| Слой | Реализация |
| --- | --- |
| Окно | `winit` |
| Графика | Vulkan 1.1+, `ash`, deferred G-buffer |
| Геймпад | `gilrs`, раскладка Xbox 360 SKU |
| Архивы | Arch00 (`LTAR`) по `Default.archcfg`, классический REZ |
| Миры | `World00p` Jupiter EX v113 |
| ИИ | GOAP (Jeff Orkin / GDC) |
| Android | Vulkan 1.1, позже |

Это не слив LithTech и не пиратский клиент. Форматы — публичный SDK 1.08 и community-спеки.

## Пути к игре

Автодетект:

`~/.local/share/Steam/steamapps/common/FEAR Ultimate Shooter Edition`

Иначе `POINTMAN_GAME_ROOT` или `pointman.toml` (см. `pointman.toml.example`).

## Запуск

Нужны `rustc`, `glslc`, Vulkan.

```bash
cargo run -p pointman
```

Клавиатура: WASD, мышь (ЛКМ — захват), Ctrl присед, F фонарик, Q slow-mo, Esc выход.

Геймпад (360): LS/RS ход/обзор, RT огонь, LT граната, LB slow-mo, A прыжок, LS click присед, D-pad down фонарик.

```bash
cargo run -p reztool -- list "$HOME/.local/share/Steam/steamapps/common/FEAR Ultimate Shooter Edition/FEAR_6.Arch00"
```

## Имя

Point Man — позывной протагониста. F.E.A.R. / Monolith / Warner Bros. — чужие торговые марки.
