# Игрок, оружие, HUD, slow-mo

Фаза 1 Pointman — **картинка Intro**. Этот файл фиксирует контракт игрока, чтобы не начать HUD «на глаз» раньше ROADMAP.

## Игрок

Сервер: `CPlayerObj : CCharacter` — инвентарь, оружие, slow-mo, physics model, story mode (`StartStoryMode` / `EndStoryMode`). (**SDK** + строки сервера)

Клиент: `CPlayerMgr` — команды, в том числе `COMMAND_ID_SLOWMO`. Камера: [13-input-camera-audio.md](13-input-camera-audio.md).

Таймеры движка: `ILTTimer` умеет slow-mo scale. (**SDK**) Не путать с клиентским UI `CHUDSlowMo`. Экранный FX slow-mo (`Hud_SloMo_Loop` → `blur.fx` / `screeneffect.fx`) закрыт в [28-clientfx-hud.md](28-clientfx-hud.md) — это **не** геймплей reflex.

Unknown (фаза 2, не Intro-кадр): цепочка input → сервер `CPlayerObj` → `ILTTimer` scale → ClientFX `Hud_SloMo_*`; пул reflex; `CHUDSlowMo` update.

## Оружие

Клиент: `CClientWeapon::SetState(W_*)`, `CProjectile::DoVector`, `WeaponDisplay`, `CDamageFXMgr`. (**строки клиента**)

Сервер: `WeaponItem` (gravity/floor), `WeaponType`, пикапы, AI-ноды подбора. Общие CREATESTRUCT: `PROJECTILECREATESTRUCT`.

Retail: строка `Database\FEAR.Gamdb00p` в `FUN_10050fe0` (**Ghidra** GameClient, [21-client-frame.md](21-client-frame.md)). Схема Gamdb00p, `W_*` переходы, ADS/recoil, viewmodel depth-bias — **unknown**. Pointman `gamdb00p` — только kind в индексе, парсера нет.

## HUD / интерфейс

Только клиент: семейство `CHUD*`, `CLTGUIString`, character/weapon displays, loading screen, MP filters. Рисуется через `ILTDrawPrim` / texture strings, не через world forward-pass. (**SDK** + DLL)

RTTI `CHUDMgr` `0x1019fedc`, cvars `FUN_100828dc` — [21-client-frame.md](21-client-frame.md). Fat Intro: HUD DrawPrim **после** мира (7 additive + 2 Translucent quads); какие виджеты — **не закрыто**. Меню-Present = 2 квада без мира.

## Что нельзя закрыть скрином двора

Ходьба/обзор в Pointman — камера для сверки кадра. Slow-mo, оружие, HUD — следующая фаза ROADMAP. Этот документ — карта, не TODO «сделать завтра».
