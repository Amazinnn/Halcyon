# Float Native Acrylic Corners Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Clip each internal float's native acrylic composition to rounded host corners without changing the verified caption or drag behavior.

**Architecture:** During the existing hidden, one-time float-host configuration, apply the Windows 11 DWM `DWMWA_WINDOW_CORNER_PREFERENCE` attribute with the `ROUND` preference. The existing direct window procedure and all show, drag, resize and snap paths remain unchanged.

**Tech Stack:** Rust, Tauri 2, `windows` crate `Win32_Graphics_Dwm`, Windows DWM.

## Global Constraints

- Apply only to `chat`, `stats`, `music`, `pet`, and `workflow` via their existing `configure_float_host()` creation path.
- Do not reintroduce `SetWindowRgn`, `SetWindowSubclass`, repeated style writes, or `SWP_FRAMECHANGED` outside hidden creation.
- The user, not a script or screenshot, performs final visual acceptance.

---

### Task 1: Apply DWM rounded-corner preference once

**Files:**
- Modify: `apps/desktop/src-tauri/Cargo.toml`
- Modify: `apps/desktop/src-tauri/src/lib.rs`
- Modify: `docs/maintenance/float-window-blue-border-repair.md`
- Modify: `docs/evals/2026-08-11-float-window-repair-session.md`

**Interfaces:**
- Produces: `float_corner_preference_attribute() -> u32` and `float_corner_preference_value() -> i32`.
- Consumes: the existing `configure_float_host(&WebviewWindow)` hidden-creation lifecycle.

- [x] **Step 1: Write the failing test**

```rust
#[test]
fn float_hosts_prefer_dwm_rounded_corners_for_native_acrylic() {
    assert_eq!(float_corner_preference_attribute(), 33);
    assert_eq!(float_corner_preference_value(), 2);
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test --lib float_hosts_prefer_dwm_rounded_corners_for_native_acrylic`

Expected: compilation failure because the two helpers do not exist.

- [x] **Step 3: Write minimal implementation**

```rust
pub(crate) const fn float_corner_preference_attribute() -> u32 { 33 }
pub(crate) const fn float_corner_preference_value() -> i32 { 2 }
```

Enable the `Win32_Graphics_Dwm` feature and call `DwmSetWindowAttribute` once
inside `configure_float_host()` after the existing creation-only native setup.

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test --lib float_hosts_prefer_dwm_rounded_corners_for_native_acrylic`

Expected: one passing test.

- [x] **Step 5: Verify and hand off for manual acceptance**

Run:

```powershell
cd apps/desktop; npm run build
cd src-tauri; cargo test --lib
cd ../../packages/event-schema; npm test
cd ../..; .\launch-focus.cmd rebuild
git diff --check
```

Ask the user to inspect every float's four corners and then drag one float to
confirm the verified caption defect does not return.
