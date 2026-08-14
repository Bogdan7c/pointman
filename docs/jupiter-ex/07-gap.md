# Pointman vs оригинал (справочник дыр)

Не список задач «сделать завтра». Карта расхождений Pointman vs уже известный оригинал.

| Тема | Оригинал (метка) | Pointman сейчас |
|---|---|---|
| Пайплайн | Forward: Ambient → (ShadowVolume+light)×N → Translucent → BlackLight (**Ghidra** `0x00510680`) | Vulkan deferred |
| Лампы Intro | 168: 35+80+39+12+2 (**dump-draw**) | Point/Fill, 8 ближайших |
| Fill за проход | **3** лампы (`specular.fxo` + exe `0x0051c5b0`) | до 8, без отдельного fill-pass |
| Attenuation | `(1-sat((d/r)²))²` (**архив** ps_2_0); Dir — attenuation texture | `(1-d/r)²` — **другое уравнение** |
| ИИ / игрок / HUD | сервер GOAP+NavMesh, клиент HUD/slow-mo (**SDK**/DLL) | GOAP-демо в `crates/ai`; HUD нет |
| Геймпад 360 SKU | LB slow-mo, RB следующее оружие (**ROADMAP**/README) | в `gamepad.rs` digital **LT/RT** → slow-mo/next weapon; **LB/RB не замаплены**; analog LT/RT = граната/огонь |
| Звук | `ILTSoundMgr` / listener / occlusion (**SDK** + exe) | нет playback crate; `.wav` только в индексе ассетов |
| FarZ | 100000 (**dump-draw**) | 12000 |
| Ambient | `25/255` на High/Med; Low = `25/255+0.13` (**SDK** + **capture** PS c0=0.098039) | raw 25,25,25 без `/255` и без LOD-add |
| Тени | на каждую лампу: ShadowVolume (id 8) → свет; finite radius + scissor + blur (**Ghidra**/архив) | нет |
| Emissive | `tEmissiveMap` в Ambient pass (**архив**) | нет |
| Translucent | отдельный смысл флага + alpha (**SDK**) | объекты выкинуты |
| Небо | sky **до** мира (FLAG2_SKYOBJECT, viewport AABB, near 0.01, `SkyFarZ` 10000) + overlay после (**Ghidra** `0x00518a70`/`0x00518c70`) | cubemap на empty depth |
| WorldModel | Mat00 + UV | цвет-хеш, UV=0 |
| Fog | cvars, на Intro выкл (**SDK**) | нет |
| Cull | device default **CCW** (**захват**); per-mesh неизвестно | NONE из-за цоколя Intro |

Порядок проходов, falloff и stencil (z-fail two-sided) закрыты статикой. Init D3D9 снят (**захват**, 0 кадров мира). Фазу 1 скрином Pointman не закрываем.
