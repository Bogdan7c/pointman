---
name: pointman-ghidra
description: Headless Ghidra MCP loop for reverse-engineering retail F.E.A.R. Jupiter EX (FEAR.exe engine + GameClient/GameServer). Use when documenting draw passes, lights, physics, AI, shells, or original engine behavior. Never commit decompilation or game binaries.
---

# Pointman Ghidra (retail Jupiter EX)

Closed engine is retail **PE**, not Fear-SDK-1.08. SDK + `.fx` say the contract. Ghidra + D3D9 capture say how the exe implements it. Game logic is in unpacked `GameClient.dll` / `GameServer.dll`.

## Paths (gitignored)

- Ghidra: `/home/bogdan/src/ghidra_12.1.2_PUBLIC`
- JDK 21: `/home/bogdan/src/jdk-21`
- MCP clone: `/home/bogdan/src/ghidra-mcp`
- Project DB: `local/ghidra/` (never commit)
- Unpacked exe: `local/ghidra/binaries/FEAR.unpacked.exe` (Steamless Variant 2.1)
- Steamless: `local/tools/steamless/` (run CLI via Proton wine if system wine missing)
- Extracts: `local/fear-extract/`
- SDK: `/home/bogdan/src/Fear-SDK-1.08`

Codex MCP: `ghidra` in `.cursor/mcp.json`. Bind `127.0.0.1`. `GHIDRA_MCP_ALLOW_SCRIPTS` off.

Unpacked project (use this): `local/ghidra/PointmanFearUnpacked.gpr` — MCP HTTP **8090** if packed instance still occupies 8089. Load `/FEAR.unpacked.exe` (6131 functions) or `/GameClient.dll`. Packed `PointmanFear.gpr` on 8089 is a dead end.

Frame dispatcher: `0x00510680`. Lights: object `+0x57==3`, type `+0x105` = `EEngineLightType`. Point `0x0051e640` does ShadowVolume (id 8) then Point (id 2). Fill batches of 3 at `0x0051c5b0`. ps_2_0 atten: `(1-sat((d/r)²))²`.

Local capture (no pacman wine): `local/tools/wine/wine` + `local/tools/apitrace/win32/.../d3d9.dll`. `PROTON_USE_WINED3D=1`, `WINEDLLOVERRIDES=d3d9=n,b`. Prefix: `compatdata/21090/pfx`. Do not treat DXVK screenshots as D3D9 stencil truth.

## Never analyze packed FEAR.exe

Retail `FEAR.exe` is **SteamStub 2.1**: `.bind` EP, `.text` entropy 8.0, ~262 fake functions, **zero xrefs**. Unpack first (`Steamless.CLI.exe --recalcchecksum`). Success: `.text` entropy ~6.5, EP in `.text`.

`load_program_from_project` needs JSON body `{"path":"/FEAR.unpacked.exe"}`. Form-urlencoded `path=` is ignored.

HTTP `run_analysis` without a switched current program re-analyzes the packed one — check `health.program_name` and function_count (thousands, not 262).

## Loop

1. Headless HTTP up. `health` first.
2. Load **unpacked** engine, then game DLLs (`GameClient.dll` is HUD/weapons, `GameServer.dll` is AI/world — neither owns D3D9).
3. Strings: `CLTRenderer`, `ShadowVolume`, technique names, `ILTPhysics`, `CAIPlanner` (server), `CHUD`.
4. Record **behavior** in `docs/jupiter-ex/` with tag **Ghidra** (address + name), not decompiled C.
5. Confirm draw order with **D3D9** capture (wined3d + apitrace). Proton+DXVK is not stencil truth. `WINEDLLOVERRIDES=d3d9=b` (builtin wined3d).

## Do not

- Commit `.gpr`, PE, `.fx`, decompiled C.
- Treat `jsj2008/lithtech` (NOLF2) as F.E.A.R. EX.
- Use Pointman kwin screenshots as original-engine evidence.
- Enable Ghidra inline scripts unless asked.
