# Capture manifest (canonical)

## 1. Scope

Как снимать и именовать D3D9 traces / golden shots оригинала. Сами `.trace` в git не входят.

## 2. Ownership

Инструменты: `local/tools/apitrace`, `local/tools/wine`. Prefix Proton `compatdata/21090`. Skill kwin — только Pointman, не оригинал.

## 3. Inputs

Retail `FEAR.exe` packed (SteamStub). Unpacked exe **не запускать** как игру.

## 4. Алгоритм съёма

**Не ходить по меню клавишами из Cursor.** Потеря фокуса → `Device::Reset` → «Unable to restore video mode». Клавиши EIS/KWin для оригинала не использовать.

Обход меню (**SDK** `GameClientShell.cpp` `ConsoleRunWorld` / cvar `runworld`):

```text
FEAR.exe +DisableMovies 1 +NoMovies 1 +Windowed 1 +runworld "Worlds\Release\Intro"
```

`WorldExists` дописывает `.World00p`. **Только backslash.** Слэши: WorldExists fail, splash не открывается → чёрный Clear+Present. Intro с брифингом ждёт клавишу в окне игры. Скрипт: `scripts/fear-capture-intro.sh`.

1. Steam running. `dxvk.conf` рядом с exe уже должен быть (`forceWindowed`, `countLosableResources=False`, `maxFrameRate=60`).
2. На ~2 минуты **не кликать другие окна**.
3. Запустить скрипт. Он кладёт apitrace `d3d9.dll`, стартует Intro, копирует trace, **снимает** `d3d9.dll`.
4. Канон: `local/ghidra/traces/fear-intro.trace` (symlink на stamped файл).
5. Init-only (0 Present) **не** закрывает DoD. Меню-only Present (2 HUD-квада) тоже нет. Нужны DrawIndexed мира (Ambient + ZFUNC=EQUAL после volumes), не 256² FX-кадры.

Сцены после Intro: тот же `+runworld` на другой World00p, не стрелки в меню. Кандидаты под glass/fog/skeletal (R27): `Worlds\Release\Factory` (офис/стекло/солдаты) или `WTF_Entry` (XP + `WTF_Entry_Yard_Door01.glass`). `+runworld` **скипает splash**. Perseus-миров в Arch00 нет.

## 5. Constants

Min: один полный кадр Intro до Present.

## 6. State tables

N/A.

## 7. Псевдокод dump

```text
apitrace dump scene.trace | extract:
  Clear, BeginScene, SetRenderState*, Draw*, Present
per-frame pass boundaries by ZWRITE/ZFUNC/technique proxy
```

## 8. Edge cases

`+ScreenWindowed 1` игра может проигнорировать. 640×480 exclusive на этом рабочем столе вешает кадр даже без apitrace. Съём: `DXVK_CONFIG=d3d9.forceWindowed=True; d3d9.dialogBoxMode=True` + cvar `Windowed` 1 (**SDK**). `pgrep FEAR` без `-x` ловит буфер обмена.

Init-only hang на `CreateVolumeTexture` 64³ + `LockBox` (capture3): не крутить тот же wrapper + пустой DXVK 12 мин. После forceWindowed: `.dxvk.bin` 147K, затем `futex_wait` без окна — не крутить `proton run` в цикле.

## 9. Evidence

Первый–третий traces: **init-only**. **capture4**: menu `fear-frame.trace`. **Intro world**: `fear-intro-20260813-224237.trace`, Present 10987749 = канон проходов. Leftover Desktop ~103G не canonical.

HTTP `import_file` на 8090 может сказать GUI-only — импорт DLL делать `analyzeHeadless -import` в `PointmanFearGameDlls` (уже сохранено).

Leftover `d3d9.dll` (apitrace) в каталоге игры грузится **каждым** Steam-запуском. На потере фокуса F.E.A.R. зовёт `Device::Reset`; с обёрткой Reset падает: `Device reset failed ... Remaining resources: 2` → диалог **Unable to restore video mode**. После съёма wrapper убирать. Для игры без capture: `dxvk.conf` рядом с `FEAR.exe` (`forceWindowed`, `countLosableResources=False`).

## 10. Known unknowns

Позы двора/туалета/лестницы не зафиксированы. FOV user settings. Non-Intro / XP traces нет.

## 11. Acceptance

`FramesCount >= 1` и ≥1 `Present`. Сопоставить RS sequence с [01-frame.md](01-frame.md).

## 12. Status

`partial` → мир Present есть, позы двора/туалета нет. Closure: DoD п.3.
