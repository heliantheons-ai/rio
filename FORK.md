# Integrated titlebar fork

This branch rebases upstream PR [#1917](https://github.com/raphamorim/rio/pull/1917)
onto upstream main `10621ddc0` (2026-11-23 checkout, Rio 0.6.0 development).
It preserves the original author and adapts the title-width calculation to the
new bell indicator so that both stay clear of the window controls.

Remotes: `origin` is `heliantheons-ai/rio`; `upstream` is `raphamorim/rio`.
The unmodified upstream main branch remains separate from `integrated-titlebar`.

## Build on Ubuntu / WSL

Use Rust 1.100.0 (pinned by `rust-toolchain.toml`). Build dependencies:

```sh
sudo apt-get install build-essential pkg-config cmake ninja-build clang \
  libfontconfig1-dev libfreetype6-dev libxcb-xfixes0-dev libxcb-render0-dev \
  libxcb-shape0-dev libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev \
  libvulkan-dev libssl-dev glslang-tools libasound2-dev
bash extra/build-titlebar-local.sh
```

The executable is `target/release/rio`. This builds Wayland and the optional
wgpu renderer; X11 is not enabled. The local release profile uses thin LTO,
8 codegen units and no debug symbols to bound build time and memory use.

## Configure

These navigation keys belong to this unmerged PR and are not supported by
the official 0.5.32 release:

```toml
[window]
decorations = "Disabled"

[navigation]
mode = "Tab"
hide-if-single = false
window-buttons = true
drag-window = true
scroll-tabs = true

[renderer]
backend = "Webgpu"
```

Keep the single-tab strip visible, otherwise its window controls are hidden
too. The strip uses the terminal background color. Existing font, palette,
shell and bindings can be retained. `Webgpu` selects wgpu instead of the new
Linux default native Vulkan renderer, preserving the GL fallback on WSLg.

## Update

```sh
git fetch upstream main
git rebase upstream/main
bash extra/build-titlebar-local.sh
```

Resolve conflicts and validate the executable before replacing an installed
version. Reinstalling and restarting are required to use a rebuilt binary.

## WSLg compatibility fix and validation

This branch also passes the Linux display handle to the wgpu 30 instance.
Without it, the GLES adapter cannot present to a Wayland window. The display
handle and surface share the same `Arc<SugarloafWindow>`.

On this WSLg host, Mesa also needs the following launch environment to select
the hardware D3D12 adapter instead of llvmpipe. These are local launcher
settings, not changes to every user's renderer defaults:

```sh
WGPU_BACKEND=gl GALLIUM_DRIVER=d3d12 target/release/rio
```

Validated on WSL 3.0.5 / WSLg 1.0.79 / Ubuntu 26.04, 2026-11-23:

- `cargo fmt --all -- --check` and `git diff --check`.
- Release build with Wayland and wgpu.
- 44 titlebar layout tests and 7 navigation configuration tests passed.
- Windowed, maximized and fullscreen startup/clean shell exit passed,
  with `D3D12 (AMD Radeon(TM) Graphics)` selected and no new Weston crash.
- Visual inspection confirmed the plain background across the titlebar.

The interactive window buttons and pointer dragging still require hands-on
validation in WSLg; this is a development build, not an upstream release.
