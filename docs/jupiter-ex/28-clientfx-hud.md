# ClientFX, SFX, HUD, DrawPrim, render targets

## 1. Scope

Игровой слой поверх `ILTRenderer`. Compose retail **доказан** (RTTI + ndisasm `GameClient.dll`, image `0x10000000`): тот же PreUpdate → Update → PostUpdate, что SDK. Строк `RenderCamera`/`Start3D` нет — vtable `ILTClient`/`ILTRenderer`. Адреса: [21-client-frame.md](21-client-frame.md).

## 2. Ownership

| Система | Модуль |
|---|---|
| Compose / Flip | `CGameClientShell`, `CInterfaceMgr` |
| Networked SFX | `CSFXMgr` + `SFXMsgId` |
| FXEdit groups | `CClientFXMgr` + ClientFxDLL |
| HUD | `CHUDMgr` / `ILTDrawPrim` |
| Mirrors / extra views | `CRenderTargetFX` **до** мира |
| Volumetric | `CVolumetricLightFX` — второй `RenderCamera` |

D3D9 pass list — exe, не DLL.

## 3. Inputs

Первый байт SFX payload = `SFXMsgId` (**SDK** `SFXMsgIds.h`). ClientFX: `SFX_CLIENTFXGROUP=43`, `SFX_CLIENTFXGROUPINSTANT=44`. Transport `MID_SFX_MESSAGE` 231 / override 232.

Object flags: `FLAG_VISIBLE`, `FLAG_NOLIGHT`, `FLAG2_SKYOBJECT`, `FLAG2_FORCETRANSLUCENT`, `FLAG2_SKYOVERLAYOBJECT` (**SDK**). WorldModel: Visible, Translucent/Alpha/TranslucentLight, CastShadow, ShadowLOD, StartHidden.

## 4. Алгоритм

### Кадр игры (**SDK** `GameClientShell.cpp`)

`PreUpdate` → `Update` (`UpdatePlaying` → `RenderCamera`) → `PostUpdate` (`FlipScreen`).

Внутри `RenderCamera` (фокус окна обязателен): SFX/ClientFX tick → `Start3D` → `IncrementCurrentFrame` → **`UpdateRenderTargets`** → **`PlayerCamera::Render`** (clear ALL, color 0, `RenderCamera(NULL override)`) → **`RenderFX`** (в SDK только DebugLine + Character) → **`InterfaceMgr::Draw`** (ClientFX overlays, HUD, menus) → console → `End3D`.

`DontRenderCamera`: те же tick, без 3D (MP меню).

### SFX lifecycle

`SetObjectSFXMessage` → `SpecialEffectNotify` → `HandleSFXMsg` → `CreateSFX` → Update → Destroy `OnObjectRemove`. Caps: PolyGrid 50, RT 30, groups 4, marks 100, volumetric `MAX_VOLUMETRICLIGHTFX_OBJECTS`.

`CreateSFX` **без factory**: `SFX_PARTICLETRAIL_ID`, `SFX_STEAM_ID`, `SFX_SOUND_ID` (слоты enum, не рисовать «как будто есть»). Номера этих слотов (за 44) не сняты.

### ClientFX DLL vs packs

- `ClientFx.fxd` — DLL типов (`fxGetNum`/`fxGetRef`). 18 имён: ParticleSystem … Overlay … Rumble (**SDK** `clientfx.cpp:218-238`).
- Группы эффектов — `ClientFX/*.Fx00p` (`LTFX` v2). Overlay: `RenderOverlays` sort by layer; DrawPrimMaterial (**SDK** `OverlayFX.cpp`). Sky: `EFXSkySetting` → FLAG2 (**SDK** `ClientFXSkyUtils.h`).

### Extra cameras / RT

| FX | Extra RenderCamera | Technique |
|---|---|---|
| `CRenderTargetFX` | mirror / refraction / cube | NULL (полный пайплайн) |
| `CVolumetricLightFX` | depth + slices; временно `DrawSky=0`, NearZ/FarZ | `"FogVolume_Depth"`, `"Ambient"` |
| Pixel-double | half-res → StretchRect | NULL; выкл если AA oversample |
| OverlayFX | нет; DrawPrim в текущий RT | — |
| PolyGrid | объект мира, `FLAG2_FORCETRANSLUCENT` | движок |

RT group default **128×128**, mip on, cube off (**SDK** `DEFAULT_WIDTH`). Три слота LOD: Low / Med / High (`m_nDimensions[3]`). Retail `GameClient.dll`: cvar **`RenderTargetLOD`** default **2.0** (high), clamp 0..2 (`FUN_100cedd0`). Create: `FUN_100cece0` берёт `this+0x50 + lod*8` = (width, height).

Intro (**dump-draw**): 2 группы + 7 `RenderTarget`.

| Group | Low | Med | High | LastFrame | CurrFrame | Mip |
|---|---|---|---|---|---|---|
| `ReflectGroup` | 128 | **256** | 512 | 0 | 1 | 0 |
| `eye_fx.Vision_RTGroup` | **256** | 512 | 1024 | 1 | 1 | 0 |

Семь RT: `Car_RT` (`tMirrorMap`), `Floor_RT` / `AlmaFloor01_RT01` / `RenderTarget01` (`tReflectionMap`) — `ReflectGroup`, `Mirror=1`; `09_RT01` — тот же group, не зеркало, `tDiffuseMap`; `AlmaFloor01_RT00` → **`Test_RTG`** (группы в мире нет); `eye_fx.Vision_RenderTarget` → vision group, mat `FX\Visions\Vision_Additive.Mat00`, FOV 115×85.

Этот Intro-capture: `RenderTargetLOD` = **Med**, не retail-default High (default 2.0 — cvar `FUN_100cedd0`, см. выше; нет 1024, нет 128²). Thin 256² = **`ReflectGroup` Med** (depth-backed mirror, player 16:9 FOV + clip plane). Vision = отдельный **512** pass, FOV **115×85**. `AlmaFloor01_RT00`/`Test_RTG` не создаётся. Четыре зеркала делят одну 256-текстуру — last writer wins; какой объект остаётся в текстуре **не закрыто**. 256-работа = `UpdateRenderTargets` **до** player view, не 256 Present. Fat Present — backbuffer 1280×720.

Bloom/DOF/motion blur: **нет** классов в SDK; шейдеры `screeneffect`/`motionblur`/`depthoffield` в Arch00 + флаги `eRTO_CurrentFrameEffect` / `PreviousFrameEffect` — связь **hypothesis**.

### Intro SpecialFX (**dump-draw**, объект `SpecialFX` **SDK** `ServerSpecialFX.cpp`, `OT_NORMAL`)

106 объектов, проп `FxName` = имя группы в packed `ClientFX/*.Fx00p` (**не** `ClientFx.fxd`). `ClientFx.fxd` — DLL фабрик типов (`fxGetRef`), грузится отдельно (**SDK** `LoadFxDll`). `StartOn`/`Loop` — булы. Сообщения `ON`/`OFF`/`TOGGLE`/`EFFECT`.

**58** с `StartOn=1` = **19** `FxName` (имя объекта ≠ группа: `Fire_Small01` → `Fire_sm_200cm`). Packs: `World` / `Lighting` / `Weapons` / `TestFile`. Overlay=15 в этих группах **нет**. DynaLight `Type`: 0=Point, 3=Cubic. Emission: 0=Sphere, 1=Point, 3=Cylinder, 4=Box. InSky: 0=No, 1=Yes, 2=Overlay.

| FxName (n) | pack | types | Mat00 / Texture | `.fx` |
|---|---|---|---|---|
| `Fire_sm_200cm` ×6 | World | ParticleSystem | `FX\FIRE\Fire01_Strip.Mat00` | `additive.fx` |
| `Fire_Lg` ×2 | World | ParticleSystem×8 | `Fire01_Strip` + `Cloud05.Mat00` | `additive.fx` + `translucent.fx` |
| `Fire_barrel` ×1 | World | Sprite + ParticleSystem | `HeatShimmer.Mat00` + `Fire01_Strip` | `refract_dx9.fx` + `additive.fx` |
| `Fire_barrel_light_500` ×1 | Lighting | DynaLight Cubic | `FireCaster1.TxAnm00` (не Mat00) | движок |
| `FireTest_LightCube` ×2 | World | DynaLight Cubic r=800 | тот же TxAnm00 | движок |
| `Steam_lg_sm` ×2 | World | ParticleSystem | `WSmoke_01.Mat00` | `translucent.fx` |
| `BH_FlareFX01` ×6 / `02` ×9 | World | Sprite | `LFlare_01.Mat00` | `additive.fx` |
| `Sky_SunGlow` ×1 | World | Sprite InSky + LensFlare Overlay | `Flare_Day` + `LensFX_Warm1` | `additive.fx` + `additive_noz.fx` |
| `LensFlare` ×1 | TestFile | LensFlare | `Engineering\Flare.Mat00` | `additive_noz.fx` |
| `LensFX_Heli_Good` ×6 | Weapons | **0 keys** | нет | нет (сосед `Good_1` не ссылается) |
| `Light_Hazard_SP_02` ×3 | Lighting | DynaLight Cubic r=1000 | `Light_2way_C.dds` | движок |
| `Helicopter_Exhaust` ×4 | World | Sprite | `Heli_Refract.Mat00` | `refract_dx9.fx` |
| `Helicopter_Cockpit` / `_Interior` / `_Runners` | World | DynaLight Point | нет Texture | движок |
| `HelicopterRide` ×2 | World | CameraShake | — | — |
| `Vision_Vortex` ×2 | World | ParticleSystem + Sprite×8 | Iris → `multiply.fx` / `translucent.fx` | не Overlay=15 |
| `FEAR_Briefing_Pause` ×1 | World | VideoController | `videos\FEAR_Fettel.bik` Pause | не картинка мира |

Сумма ×n по таблице = 49; три строки `Helicopter_Cockpit` / `_Interior` / `_Runners` без счётчиков добирают до 58 — точные n не сняты (**dump-draw** gap).

`Helicopter_RotorWash` в мире есть, **StartOn=0**. Отдельного `particle.fx` в Arch00 нет.

**48** с `StartOn=0` (скрипт/сообщение). HUD/overlay имена (все off на спавне, кроме `Vision_Vortex` выше):

| FxName | n | Loop | объект-пример |
|---|---|---|---|
| `Hud_Flash` (+ Jankowski/Disler/Jack) | 7+3 | 0 | `Rem_Flash*`, `DoorFlashFX*`, `SecretFlashFX` |
| `Hud_BlurVisionFX` / `_OUT` | 2+3 | 0 | `134_fade*`, `100_VisionBlur*`, `trans_blur` |
| `Hud_Vision_Bleach_Loop` | 2 | 1 | `Rem_Bleach`, `100_VisionBleach` |
| `Hud_SloMo_Loop` / `_bleach` | 1+1 | 1 | `Rem_Blur`, `SecretBleachFX` |
| `Hud_Cin_Loop` | 2 | 1 | `134_cinFx`, `100_VisionBlurLoop` |
| `Hud_Signal_Loop` / `_Out` | 1+1 | 1/0 | `131_visionfx_*` |
| `Hud_DeathFX` | 1 | 0 | `134_attackFX01` |
| `Overlay_Vision_Additive` | 1 | 1 | `eye_fx.Vision_OverlayFX` |

Двор `GameStartPoint00`: HUD-группы **не** стартуют. Overlay/bleach/slo-mo — отдельные камеры/скрипты (`100_Vision*`, `intro_cam01`, `SecretCam`).

### Packed ClientFX (`*.Fx00p`) (**SDK** `ClientFXDB::LoadFxGroups` + **archive**)

`ClientFX/` в Arch00: 13× `Fx00p`. FourCC **`LTFX`**, `m_nFileVersion=2`, `m_nEffectVersion=5`, **18** типов — тот же порядок, что SDK `g_EffectTable` (`ParticleSystem`=0 … `Overlay`=**15** … `Rumble`=17). Retail packs: Overlay keys почти все в `interface.Fx00p` (119 шт.).

После header 40 B: `ntypes`×u32 (счётчики ключей по типу, для аллокатора) → string table → curve blob → группы. Группа: `{nameIndex, lengthMS, nKeys}` + ключи `{type, t0, t1, id, linkedId, nProps}` + свойства `{nameIndex, postPos}` + payload. Строка = индекс в table (`CFxProp_String`: u32). Не класть `.Fx00p` в git.

Overlay key (**SDK** `OverlayFX.cpp`): `DrawPrimMaterial` квада на экран, **не** world technique. Проп `OverlayMaterial` → Mat00 → `.fx`.

Intro HUD `FxName` → материал → шейдер (**archive** Mat00, формулы уже в [24-materials.md](24-materials.md) / post):

| FxName | OverlayMaterial | `.fx` |
|---|---|---|
| `Hud_Flash` | `FX\Hud\OV_Flash.Mat00` | `Translucent\additive.fx` |
| `Hud_SloMo_Loop` | `Hud_SloMo_Loop.Mat00` + `OV_Blurr_Clear.Mat00` | `Effect\blur.fx` (не `motionblur.fx`) |
| `Hud_SloMo_Loop_bleach` | + `OV_Vision_Bleach_2.Mat00` | `blur.fx` + `screeneffect_bleach.fx` |
| `Hud_Vision_Bleach_Loop` | `OV_Blurr_Clear` + `OV_Vision_Bleach_2` | `blur.fx` + `screeneffect_bleach.fx` |
| `Hud_BlurVisionFX` / `_OUT` / `Hud_DeathFX` | `OV_DeathFX_Red` + 4× `OV_DeathFX_blur` | `multiply.fx` + `blur.fx` |
| `Hud_Signal_Loop` / `_Out` | `FX\Special\ScreenEffect_SloMo.Mat00` | `Effect\screeneffect.fx` |
| `Hud_Cin_Loop` | blur + `Hud_SloMo_Loop` + `OV_Scope_Circle` | `blur.fx` |
| `Overlay_Vision_Additive` | `FX\Visions\Vision_Additive.Mat00` | `additive.fx` (тот же Mat00, что vision RT) |

`Hud_Blur_Loop` в packs: `Hud_MotionBlur_Loop.Mat00` — файла нет в Arch00 (**archive gap**, как `skin.fx`). `motionblur.fx` в шейдерах есть; Intro HUD его **не** биндит этими группами.

### HUD / DrawPrim / compose states (**SDK**)

Playing HUD и splash/menu — **разные** пути. `CInterfaceMgr::Update()` либо владеет кадром (`bHandled`), либо падает в `UpdatePlaying` → `RenderCamera`.

| State | Кто рисует | Мир? |
|---|---|---|
| `GS_PLAYING` | `RenderCamera` → мир → `InterfaceMgr::Draw()` | да |
| `GS_MENU` | тот же world path; Back HUD skip | да |
| `GS_SCREEN` / `GS_SPLASHSCREEN` / `GS_MOVIE` / `GS_LOADINGLEVEL` | свой `Start3D` | **нет** |

`+runworld` ставит cvar → **splash skip**. Capture4 (без `runworld`) = меню, не Intro.

`CHUDMgr::Render(layer)`: `BeginDrawPrimBlock` → items с `item.level <= mgr.level` → `EndDrawPrimBlock`. Default `Modulate_Translucent`. **Back** (playing + `DrawInterface`): crosshair, overlay mgr, ammo, health, armor, slow-mo. **Front**: chat. `DrawInterface 0` убивает Back + letterbox + fade; Front и ClientFX overlays **рисуются**. Acceptance «убирает HUD» верно только для Back.

Layout overlays (`Client/Overlay`: Binoculars, Zoom, Damage, SignalStatic) стартуют hidden — `DrawPrimMaterial` layout `Mask`, не ClientFX Overlay keys.

Fat Present 10987749: 2 world Translucent additive (`0x00517e70`) + **7** HUD additive + 2 Translucent quads + `StretchRect`. `CHUDMgr` Additive не ставит — dest-One = DrawPrimMaterial (ClientFX Overlay / HUDAnimation / weapon RT). Какие именно 7+2 — **не закрыто**. `CameraPixelDouble` default **0** — не объясняет StretchRect.

Шрифты: `LayoutDB` `Interface/Fonts` Face + `Interface/Shared.FontFile[i]` → `ILTTextureString::RegisterCustomFontFile` (**exe**, не GameClient). HUD Face = layout `Font` или `Shared.HUDFont`. Glyphs = dynamic atlas → DrawPrim. Scale `width/640`, `height/480`; widescreen если aspect ≥ 14/9. Конкретные пути `FontFile` живут в `FEAR.Gamdb00p` (в дереве нет).

### Video / Bink

Bink = **`FEAR.exe` `CLTVideoTexture` + `Binkw32.dll`**. `GameClient.dll` Bink-строк **нет**. Нет/битый `Binkw32` → «video playback disabled». Расширение `.bik`.

1. Splash movies — `DisableMovies` **и** `NoMovies`.
2. Menu `ScreenMovie` — **не** гейтится этими cvars (движок no-op без Binkw32).
3. In-world: `VideoController` (type 13) `FindVideoTexture` — **не создаёт**. Overlay (15) может сэмплить video texture.

`World.Fx00p`: `GameIntroPause`/`Restart` → `videos\GameIntro.bik`. Intro dump имеет `FEAR_Briefing_Pause`, не `GameIntroPause`. `interface.Fx00p`: имена `Hud_Zoom_*_bink` **без** `.bik` path.

### `FovAspectRatioScaleInterface`

**Не** player FOV. Fat Intro = FOV Y **45°** (`FovY` / `FovYWidescreen` + `FovAspectRatioScale`). Interface camera **один раз** в `CInterfaceMgr::Init`: Y = clamp(`FovYInterface=75`), X = `2*atan(tan(Y/2)*W/H) * FovAspectRatioScaleInterface` (PC default **1.0**). `ScreenDimsChanged` обновляет rect + HUD scale, **не** `SetCameraFOV`. Playing OverlayFX сидит на player cam — Interface scale HUD-оверлеи мира не крутит. HUD спрайты — 640×480, не этот FOV.

## 5. Constants

`CLEARRTARGET_ALL`. FOV player: `FovY` / `FovYWidescreen`. Interface FOV: `FovYInterface`. Gamma RGB в profile. `ClientFXDetailLevel` 0–2. `UpdateClientFX` default 1.

## 6. State tables

DrawPrim (**archive** Internal): NoBlend α-off; Additive SrcAlpha/One; Translucent SrcAlpha/InvSrcAlpha. `tex2D * color`, VS `mDrawPrimToClip`.

HUD не выставляет D3DRS мира.

## 7. Псевдокод

```text
if not window_focus: return
sfx.update(); clientfx.update()          # gated UpdateClientFX
renderer.Start3D()
sfx.update_render_targets(camera)        # mirrors before world
clear(ALL, 0); renderer.RenderCamera(cam, override=None)
sfx.render_fx(cam)                       # SDK: debug lines + characters
clientfx.render_overlays()
hud.draw()                               # DrawPrim block
renderer.End3D()
# later PostUpdate:
if state != LOADING: FlipScreen()
```

## 8. Edge cases

- `RenderCamera(false)` не прячет HUD. `DrawInterface 0` прячет только Back HUD.
- Volumetric глушит `DrawSky` на время своего view.
- Intro: 7 RT + 2 group (**dump-draw**). Thin 256² = ReflectGroup Med (**capture**). Какой из 4 зеркал last-writer — **partial**. `Test_RTG` dangling.
- Particles не отдельный `.fx` в Arch00.
- Имена Intro FX + world StartOn props закрыты (**dump-draw** + **archive** Fx00p). HUD OverlayMaterial→`.fx` закрыто. `LensFX_Heli_Good` = 0 keys.

## 9. Evidence

| Claim | Source |
|---|---|
| Compose + Flip в PostUpdate | **SDK** GameClientShell.cpp, InterfaceMgr.cpp |
| SFXMsgId 0..42 + 43/44 | **SDK** SFXMsgIds.h, FxDefs.h |
| ClientFX type names | **SDK** clientfx.cpp:218-238 |
| RT 128² defaults; 3 LOD sizes | **SDK** RenderTargetGroup.cpp `DEFAULT_WIDTH=128` |
| `RenderTargetLOD` 0..2 default 2 | **Ghidra** GameClient `FUN_100cedd0` (`0x40000000`=2.0f) |
| Create w/h from object+0x50[lod] | **Ghidra** `FUN_100cece0` |
| Intro 2 group + 7 RT, ReflectGroup Med=256 | **dump-draw** Worlds/Release/Intro |
| Intro 106 SpecialFX, 58 StartOn; HUD-имена StartOn=0 | **dump-draw** + **SDK** `SpecialFX` |
| Fx00p `LTFX` v2 effectVersion 5, 18 types, Overlay=15 | **SDK** `LoadFxGroups` + **archive** 13 packs |
| Intro HUD OverlayMaterial → blur/screeneffect/bleach/additive | **archive** `interface.Fx00p` + Mat00 |
| Thin 256² = ReflectGroup Med (этот capture LOD=Med) | **capture** CreateRT 125663–125725; Vision 512 FOV 115×85 |
| Overlay = DrawPrimMaterial, не world pass | **SDK** `OverlayFX.cpp` |
| Intro StartOn world FX → Mat00/Texture | **archive** World/Lighting/Weapons/TestFile Fx00p |
| Playing vs splash/menu compose | **SDK** InterfaceMgr / GameClientShell |
| Bink = exe + Binkw32, не GameClient | **SDK** ILTVideoTexture + exe imports |
| Interface FOV scale once at Init | **SDK** `CInterfaceMgr::Init` |
| Volumetric extra camera | **SDK** VolumetricLightFX.cpp:647-711 |
| DrawPrim 3 modes | **SDK** iltdrawprim.h + **archive** Internal/*.fx |
| Retail DLL compose CF | vtable ILTClient, строк RenderCamera нет |

## 10. Known unknowns

Какой из 4 ReflectGroup зеркал last-writer в 256. Пишет ли DrawPrim HUD в RT. Cube RT. Fat 7 additive + 2 Translucent quads — какие группы. Fat StretchRect source. `FontFile`/Face в `FEAR.Gamdb00p`. `Hud_Zoom_*_bink` VideoName. `GameIntro.bik` на `GameStartPoint00`. `Hud_MotionBlur_Loop.Mat00` нет в Arch00. PolyGrid. `Test_RTG` dangling.

## 11. Acceptance

- Capture: SetRenderTarget (если RT) **до** world draws; DrawPrim **после** world, до Present.
- `DrawInterface 0` убирает Back HUD; Front + ClientFX overlays остаются; мир остаётся.
- Synthetic: SFX id без factory не создаёт объект.
- Volumetric: второй RenderCamera с override имени техники.

## 12. Status

`verified-static` (**SDK** + retail compose ndisasm) + Intro FX/HUD Overlay + thin 256 = ReflectGroup Med (**capture**). Fat HUD item IDs `partial`.
