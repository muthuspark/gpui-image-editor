# GPUI Image Editor

Phase 1 of [the PRD](PRD.md): a minimal, near-black desktop window that imports and displays one image. Built with Rust and GPUI, entirely offline.

## Run

Initial platform: **Ubuntu 24.04 LTS, x86-64**, with X11 or Wayland and a Vulkan-capable graphics driver. Other operating systems are not tested for this phase.

Install Rust using rustup, then install the Linux build and file-picker dependencies:

```sh
sudo apt install build-essential pkg-config cmake clang libfontconfig1-dev libfreetype6-dev libxkbcommon-dev libxkbcommon-x11-dev libxcb1-dev libwayland-dev libvulkan1 xdg-desktop-portal xdg-desktop-portal-gtk
cargo run --locked
```

The file picker uses the desktop's XDG portal. Run within a desktop session, and restart the session after installing a portal backend if necessary. Drag and drop is also available.

## Use

- Click **Import image**, press **Ctrl+O**, or drop a single PNG or JPEG onto the window.
- The image is centered and fitted to the available canvas, without enlarging small images. Resize the window to refit it.
- The bottom bar shows the filename and oriented pixel dimensions. Transparency appears over a checkerboard.
- Canceling the picker or failing an import preserves the current image. Loading and errors appear inline.
- Press **Ctrl+Q** or close the window to quit.

Imports decode on a background worker. Source files are opened read-only and never changed. Limits: 100 MiB encoded size, 24 megapixels, and 12,000 pixels per side. JPEG EXIF orientation is applied on import. Animated PNG and precision above 8 bits per channel are rejected.

Phase 1 accepts untagged images and PNG's standard sRGB declaration. Embedded ICC profiles, including embedded sRGB profiles, are conservatively rejected with instructions to re-export without the profile. Full embedded sRGB profile validation remains a later milestone. Zoom, pan, editing, history, and export are not implemented yet.

## Verify

```sh
cargo fmt --check
cargo test --locked -j 2
cargo clippy --locked --all-targets -j 2 -- -D warnings
```

GPUI and the image decoder are pinned in `Cargo.toml`; the complete dependency graph is in `Cargo.lock`. Use `--locked`: GPUI's transitive `xattr 0.2.3` currently requires the locked `libc 0.2.186` to build on Linux.

Manual checks: open a transparent PNG and an oriented JPEG, cancel a replacement, try a corrupt file, drop multiple files, and resize the window. Check that failures retain the displayed image and that filename and dimensions update only after a successful open.

Verified during implementation: all seven decoder tests, formatting, and Clippy passed. Native X11 smoke checks passed for PNG import, correct color channels and transparency, canceled replacement, corrupt replacement preserving the image, and Tab/Enter access to the picker. The development host had the XKB runtime but lacked its development linker symlink, so build/test verification used a temporary `LIBRARY_PATH` link to the installed runtime. Installing `libxkbcommon-x11-dev` above supplies that link for normal builds.

## Reference environment

Initial development machine: Ubuntu 24.04.3 LTS, Linux 6.17, NVIDIA GeForce RTX 3060, two 1920 × 1080 displays at 60 Hz, Rust 1.98.0. Minimum window size: 960 × 640 logical pixels.

Release measurement configuration: `cargo build --release --locked`. The PRD's 20-run launch/import latency, GPU frame-time, and peak memory benchmarks have not yet been collected. This implementation is the initial import spike, not completion of the PRD's full platform/workflow release gate.
