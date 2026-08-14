# Jupiter EX (engine + draw reference)

Spec: `docs/jupiter-ex/`. Skill: `pointman-ghidra`. Unpacked Ghidra MCP HTTP **8090**, program `FEAR.unpacked.exe`. Cursor MCP `ghidra` may be down; HTTP 8090 + ndisasm PE works.

## Binaries

- Retail `FEAR.exe` SteamStub 2.1. Unpacked: `local/ghidra/binaries/FEAR.unpacked.exe`.
- Extracts: `local/ghidra/traces/models`, AnmTrees, `Worlds/Release/Intro.World00p`.
- `reztool`: `target/debug/reztool`.

## Draw (proven)

`0x004f5020` → `0x004f4c80` → `0x004ffee0` → `0x00510ad0` → **`0x00510680`**.
Override `0xf`: Ambient → lights → FogVolumes gated → Translucent → BlackLight.
ps_2_0: `atten=(1-sat((d/r)²))²`. Shadows z-fail; `0x0051fac0` draws meshes, does not build silhouette.
Soft-occlusion `0x005166f0`: call gate = `0x517c20` (empty caster list `this+0x70`, stride 40), NOT Light_ShadowBlur; blur sub-block gated by Light_ShadowBlur (default 0). SRCBLEND after blur = DESTALPHA(7) — matches DX8 parent, no drift. Predicate `0x5169f0` (LOD clamp DAT_0056da84=2 + Light_ShadowVolume=1) is a separate mode-select, not the gate. Model00p `node+8` = parent u16 (0xFFFF=root), not flags.
Intro Ambient PS c0 = 25/255 (0.098039) at High/Med; AddAmbientLow 0.13 only on LODLights=Low, after /255. Fat Present: 2 tech-1 additive Translucent DIP then HUD. World00p only in FEAR*; FEARE* has no worlds. Pack-5 = multiply Zero/SrcColor; Decal/ClipLight are pack-time.

## Sky / sort / vis

Sky before world. No PVS bitmap. Flood = portal plane + clip. Intro sector 18747 B exact. PhysicsBSP 12 B = clip node (i32 poly + 2 children, -1/-2 leaves), not vis. Runtime 16 B. 0x00425650 = 7-float sim shapes, not those nodes. FLAG2_PLAYERCOLLIDE = Y cylinder vs BSP (not FLAG_POINTCOLLIDE).
XOR 399 = 0x71+"FEAR".

## World00p volumes + blinddata

Intro: 289 `engine\\shadowvolume.Mat00` stride 24 pack 10.

Blinddata CLOSED: `u32 nChunks` + `u32 arenaBytes` + arena + dir `(size, typeId, offset)` at end. Offset from arena start (not section+0/+4). `GetBlindObjectData` nNum = global dir index. Intro: KF 0..25, NavMesh 26, Shatter 27..92. Packer float N+0.1. Not renderer.
NavMesh blob: first u32 = processed flag (Intro 1), then version 6. SDK/retail skip +4. Not packer drift.

## Model00p

Slots 1–8 physics `0x0044ccc0`. M format 0–6 disk=alloc. FVF 64 + D3DCOLOR R=w0 G=w1. Shadow 32 B or 3×POSITION.

**Keyframes in Model00p** (no Anim00p in Arch00):
- GetAnim table stride 8 at model+0x64/+0x68 (`0x00439eb0`). nKeyFrames clip+0x14. Length = last time at clip+0x10.
- Name clip+0x24; Find `0x0042f410` bsearch model+0x74.
- Track table hdr[8]: 2×u16 + nNodes×(pos,rot) u16; 0xFFFF = bind pose node+0x4c/+0x58.
- Packed bytes hdr[20]. Pos: clip+0x1a ? i16*(1/64) : f32. Quat: i16*(1/32767) (`0x551010`).
- **hdr[0] = Σ piece.N** (reloc-pairs), not clip count. cactus 2; deltaForce 12; soldiers 2652. GetAnim push `0x004306f0` × hdr[1] pieces.
- Extra-names: count N, loop ebx=1..N-1 (N=8 → 7 child models).

## AnmTree00p

ANMT v3 search tree. Bird/Alma/Soldier EOF. Not in FEAR.exe.

## Cloth / hair (archive .fx)

Cloth: albedo=lerp(rim,diffuse,sat(N·V*fRimScale)), default scale 2; Lambert*atten, no spec.
Hair: TBN from N×Y; aniso map CLAMP; Point offset (0.5, spec.a); Ambient AlphaRef 96. Fill = Lambert only.
rigid/Solid/cloth_detail.fx missing from Arch00 (skeletal stub only).

## Still open

- Overlay/Fog/BlackLight/Fill/`translucent.fx` on a frame that draws them. Fat Intro HAS 2 tech-1 additive Translucent DIP (not glass). Glass is inside light-loop in earlier Presents.
- BlackLight PS (нет .fx); missing dx9lights.fxh/skin.fx/cloth_detail.fx (для всех есть .fxo).
- type 1/4/8 disk layout closed (ndisasm); **0** in 495 v0x21 Arch00 models. Встречаются: 7×809, 2×572, 3×129, 5×87, 6×74.
- Glass + post (screeneffect/blur/motionblur/DOF/refract) closed from .fx.
- BlackLight: Spot clone `0x0051bab0`, shadows tech 8, lit `0x0050ffc0` tech 7; no .fx technique.
- World00p TOC [2]/[3]/[4]/[6] closed on Intro; [0]=347 unused (Pointman `_branch_count`); exe vt+0x40 reader + slots [1]/[5]/[7]/[8]/[9] + 92048 B tail open.
- ClientFX: Fx00p LTFX v2 effectVersion 5, Overlay=15. Intro HUD OverlayMaterial→blur/screeneffect/bleach/additive CLOSED. Compose CF (retail = SDK chain), Bink (exe CLTVideoTexture+Binkw32), thin 256² RT (ReflectGroup Med mirror) CLOSED. Remaining: world particle props.
- Intro SpecialFX names/StartOn CLOSED (106, 58 on; HUD groups off at spawn).
- XP/XP2 (R27). Base_Intro_Soldiers child attach (`0x00430ee0`).
- R15 leftover: BlackLight PS; `skin.fx` rigid missing.

Do not commit PE/.fx/decompile/.gpr. Not NOLF2. Do not implement Pointman renderer in this task.
