---
name: pointman-kwin-live
description: Live KDE/Wayland screenshot and keyboard loop for the Pointman Vulkan game window via kwin-mcp. Use when verifying Intro 1:1, capturing GameStartPoint00, walking the courtyard, checking sky/lights/walls, or iterating render fixes. Never start a virtual KWin session.
---

# Pointman live window (kwin-mcp)

Drive the **already-running** Pointman window on the real Plasma desktop. Virtual `session_start` has no usable GPU — 1:1 lighting/sky checks would lie.

## Loop

1. If the game is not up: `cargo run -p pointman`, wait until the window exists.
2. `session_connect()` — live KWin only.
3. `list_windows` → `focus_window` on title `POINTMAN — F.E.A.R. native`.
4. `screenshot` (and keyboard WASD only **after** focus). EIS keys go to whatever is focused; do not type into Cursor/chat.
5. Compare against a FEAR reference shot from the same pose if one exists. Otherwise report obvious holes (grey sky, colored collision stubs on walls, black distance).
6. `session_stop` when done — disconnects MCP, does **not** kill the game.

## Do not

- Call `session_start` / isolated `kwin_wayland --virtual`.
- Trust `accessibility_tree` / `find_ui_elements` — no widgets, raw Vulkan.
- Treat this as Xbox 360 gamepad coverage — MCP is keyboard/mouse only.
- Put this in CI. No GPU + no Steam install there.
- Inject input without `focus_window` first.

Rebuild → focus → screenshot again after a render fix. That is the feedback loop; kwin does not write code.
