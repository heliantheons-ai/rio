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


## macOS-style controls and rounded corners

The fork adds two opt-in settings for the Linux/wgpu setup:

    [window]
    decorations = "Disabled"
    corner-radius = 12.0

    [navigation]
    mode = "Tab"
    hide-if-single = false
    window-buttons = true
    window-buttons-style = "TrafficLights"
    drag-window = true
    scroll-tabs = true

TrafficLights places 18px red/yellow/green circular controls on the left.
Red closes the window, yellow minimizes, and green toggles maximize/restore.
The control glyphs appear on hover. The original Standard style remains
the default and keeps the right-hand buttons. Native macOS controls are
unchanged. Circular hit regions exclude the gaps between the lights, and
the tab/title layout reserves their space.

The corner-radius value is in logical pixels, clamped to 0..32. It applies
to undecorated Linux windows with the wgpu renderer. It enables window alpha
without reducing the opacity of the terminal contents. A final GPU pass
clips just four corner quads with antialiasing; it does not paint black
corners or impose a rounded rectangle over the content. Maximized and
fullscreen windows skip the corner pass.

Validation on the WSLg host:

- 46 titlebar layout/hit-region tests, 7 navigation configuration tests,
  and the corner-radius configuration test passed.
- An explicit GPU readback test verified all four corners are transparent,
  center/straight-edge colors remain unchanged, edge pixels are
  premultiplied and antialiased, and radius zero preserves square corners.
- A contrasting temporary background confirmed actual transparent
  corners in the WSLg-composited window, with the three colored lights.
- Startup/clean shell exit was checked in windowed, maximized and
  fullscreen modes with the AMD D3D12 adapter.

Run the graphics test explicitly when a Vulkan fallback adapter is available:

    source ~/.cargo/env
    CARGO_PROFILE_RELEASE_DEBUG=0 CARGO_PROFILE_RELEASE_LTO=thin \
    CARGO_PROFILE_RELEASE_CODEGEN_UNITS=8 \
    cargo test --locked --release -p rioterm -p sugarloaf --no-default-features \
      --features rioterm/wayland,rioterm/wgpu context::corner_mask::tests -- --ignored


## Roomier custom header

On Linux/Windows the custom strip is 56 logical pixels high, with an
18px title and 18px traffic lights. The lights use 28px spacing and reserve
112px at the left. Tab pills use 10px vertical insets and 8px rounding.
The hit areas, title truncation and terminal top inset share these metrics.
Native macOS strip metrics remain as before. Terminal font settings are
independent; the local configuration keeps WenKai Mono size 16 and Medium
500 for bold/italic-bold.
