# Acceptance: fixtures, golden, visual comparison

## 1. Scope

Как проверять реализацию рендера по спеке **без** Ghidra. Menu Present есть (`fear-frame.trace`); **Intro world Present есть** (`fear-intro-20260813-224237.trace`, Present 10987749 — [20-evidence.md](20-evidence.md)); полного synthetic mesh World00p — нет.

## 2. Ownership

Тесты: `crates/assets` (форматы), будущие `crates/render` контракты. Golden shots: `local/` gitignore. Машиночитаемые таблицы: этот каталог `docs/jupiter-ex/`.

## 3. Inputs

Coverage matrix. Capture dump. World00p: in-memory header/UV tests в `world00p.rs` — **не** полный mesh-файл. Model00p fixture — нет.

## 4. Алгоритм сравнения

1. Unit: формулы CPU (atten, Blinn, winding) на синтетике.
2. Integration: `Simulation::draw_list` отдаёт типы объектов по контракту (не «хелпер парсера»).
3. Golden: скрин оригинала vs порта с той же позы; SSIM/pixel diff с маской HUD.
4. Capture replay checklist: порядок RS vs [01-frame.md](01-frame.md).

## 5. Constants (предложение, не подпись)

| Metric | Start |
|---|---|
| Opaque albedo (no HUD) | ΔE / pixel fail TBD after first paired shots |
| Sky pixels | mask depth≈far |
| HUD | exclude from 1:1 world |

Не фиксировать порог, пока нет пары скринов.

## 6. State tables

N/A.

## 7. Псевдокод

```text
for contract in coverage:
  require unit or capture checklist or golden
  fail if status is unknown and contract affects pixels
```

## 8. Edge cases

DXVK screenshot ≠ D3D9. kwin Pointman ≠ оригинал.

## 9. Evidence

Intro winding tests **empirical**. Atten formula **archive**. Present golden — нет.

## 10. Known unknowns

Позы двора/туалета/лестницы. Толерансы. Feature-сцены XP/XP2.

## 11. Acceptance

Этот документ становится исполняемым, когда есть хотя бы intro-gsp00 shot pair (capture Present мира Intro уже есть — R00).

## 12. Status

`partial`.
