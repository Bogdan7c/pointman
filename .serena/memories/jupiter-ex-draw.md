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

WM/model bind: DrawWorldModels `0x0051ebf0` = object+0xf0 ushort → table DAT_00576ff4+0x18 (piece*+count). Piece 0x14, record 0x20 (same as bake), surf 0x34 via `0x00511fb0`/`0x0050f8f0`. se_InitWorldModel `0x00459200` lookup name+0x38; fail LT_MISSINGWORLDMODEL (need non-renderonly brush). ExtraInit `0x00463ed0` copies wmdata+2 → object+0x154. OT_MODEL `0x0051f200`: +0x110 model, +0x13c materials (`0x00435b80`). UV is render mesh / FVF64, not PhysicsBSP. Record source-bytes: bake 8/0x19 (light 9/0x1a), WM 0xc/0x1b (light 0xd/0x1c), model 0xa/0x17. Pass technique is separate (`0x0050ffc0` Ambient=0).

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
- WM/model **draw bind closed**. `vt+0x40` @ `0x00478209` = interface slot (CWorldClientBSP vtable only 14 methods). Impl LoadSector `0x00478140`. Load `0x004782c0` → Shared this+4. World00p tail: header 119,66,12 + **66×36** AABB+surfIndex (empirical Intro); `347×208` rejected; 89660 B open. Still: who fills DAT_00576ff4+0x18 / object+0xf0; exe reader of 66×36/89660; 0x28 record bytes vs `0x00503a30`.
- BlackLight PS (нет .fx); missing dx9lights.fxh/skin.fx/cloth_detail.fx (для всех есть .fxo).
- type 1/4/8 disk layout closed; **0** in 495 v0x21. Counts: 7×809, 2×572, 3×129, 5×87, 6×74. Type 3 = sphere (r_cm×0.01, mass, third); I=0.4mr² (`0x004914d0`). Type 7 = capsule (r, mass, third, pA, pB)×0.01; soldier limbs ±Y (`0x00406320`).
- Glass + post (screeneffect/blur/motionblur/DOF/refract) closed from .fx.
- BlackLight: Spot clone `0x0051bab0`, shadows tech 8, lit `0x0050ffc0` tech 7; no .fx technique.
- World00p tail CLOSED: 0x0050d0a0 reads TOC then TOC[0] entries (0x0050cf40). Entry0 nPieces=nSectors; others 1. TOC[1]=piece count. Each piece: nRec×36 AABB (Σ nRec=surf) + extra (u8 n + n×vec3). TOC[7]=extra polys, TOC[8]=Σ verts (quads; Factory 2 tris). 3 maps consume 100%. TOC[9]=0 unused. Extra role open.
- ClientFX: Fx00p LTFX v2 effectVersion 5, Overlay=15. Intro HUD OverlayMaterial→blur/screeneffect/bleach/additive CLOSED. Compose CF (retail = SDK chain), Bink (exe CLTVideoTexture+Binkw32), thin 256² RT (ReflectGroup Med mirror) CLOSED. Remaining: world particle props.
- Intro SpecialFX names/StartOn CLOSED (106, 58 on; HUD groups off at spawn).
- XP/XP2 (R27). Base_Intro_Soldiers child attach (`0x00430ee0`).
- R14 leftover: type 3/7 third float; skeletal.fxh absent; 199v courtyard model. Type 3/7 r+mass+capsule axis closed.
- R15 leftover: BlackLight PS; `skin.fx` rigid missing.

Do not commit PE/.fx/decompile/.gpr. Not NOLF2. Do not implement Pointman renderer in this task.
