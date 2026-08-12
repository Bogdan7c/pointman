# Android

Целевой рантайм — тот же сырой Vulkan, что и на Linux. `winit` умеет `android-activity`; отдельного GLES-бэкенда не будет.

Пока десктопный клиент — источник правды. APK собирается следующим шагом:

1. Android NDK + SDK (у тебя уже есть куски toolchain в `~/.local/share/moonlight-android-toolchain`).
2. `cargo-apk` или `xbuild` с `libpointman.so` (`cdylib`).
3. Ассеты: распакованный Arch00 в `assets/` или mmap из OBB. Розничные файлы в APK не коммитим.
4. Ввод: виртуальный стик + look-pad вместо мыши.

Минимальный Vulkan на целевых телефонах — 1.1, поэтому рендер сидит на render pass'ах, не на dynamic rendering 1.3.
