#!/usr/bin/env python3
"""v1.5 code-compare check: verify the working tree actually contains the
v1.5 changes on top of the v1.4.1 baseline (HEAD = 4af7430).

Usage:  python scripts/v15-check.py
Prints per-key-file: expected markers present/missing, plus git diff stat.
"""
import subprocess, sys, os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# key marker -> (relative path, substring expected)
MARKERS = [
    ("settings.rs 新增 Url/Internal", "apps/desktop/src-tauri/src/settings.rs", "ShortcutType::Url"),
    ("storage.rs 新表迁移 0003", "apps/desktop/src-tauri/src/storage.rs", "0003_shortcuts_layouts"),
    ("storage.rs list_shortcuts", "apps/desktop/src-tauri/src/storage.rs", "pub fn list_shortcuts"),
    ("storage.rs week_focus_summary", "apps/desktop/src-tauri/src/storage.rs", "pub fn week_focus_summary"),
    ("lib.rs mod cli/launch", "apps/desktop/src-tauri/src/lib.rs", "mod cli;"),
    ("lib.rs add_url_shortcut", "apps/desktop/src-tauri/src/lib.rs", "fn add_url_shortcut"),
    ("lib.rs launch_shortcut cmd", "apps/desktop/src-tauri/src/lib.rs", "fn launch_shortcut"),
    ("lib.rs cli::spawn", "apps/desktop/src-tauri/src/lib.rs", "cli::spawn"),
    ("lib.rs move_shortcut", "apps/desktop/src-tauri/src/lib.rs", "fn move_shortcut"),
    ("cli.rs 存在", "apps/desktop/src-tauri/src/cli.rs", "pub fn spawn"),
    ("launch.rs 存在", "apps/desktop/src-tauri/src/launch.rs", "pub fn launch_shortcut"),
    ("focus-cli bin 存在", "apps/desktop/src-tauri/src/bin/focus-cli.rs", "fn main"),
    ("Cargo.toml focus-cli bin", "apps/desktop/src-tauri/Cargo.toml", 'name = "focus-cli"'),
    ("DesktopView centered-grid (2x5)", "apps/desktop/src/views/desktop/DesktopView.vue", "shortcut-grid"),
    ("DesktopView views-tray", "apps/desktop/src/views/desktop/DesktopView.vue", "views-tray"),
    ("DesktopView Dock 开始专注 v-if", "apps/desktop/src/views/desktop/DesktopView.vue", "ui.focusState !== 'focus'"),
    ("shortcuts store addUrl", "apps/desktop/src/stores/shortcuts.ts", "add_url_shortcut"),
    ("shortcuts store move", "apps/desktop/src/stores/shortcuts.ts", "async move("),
    ("ui store cli:timer", "apps/desktop/src/stores/ui.ts", "cli:timer"),
    ("lib/shortcuts.ts v2 类型", "apps/desktop/src/lib/shortcuts.ts", '"internal"'),
    ("ADR-0006 存在", "docs/decisions/ADR-0006-agent-cli-control-plane.md", "ADR-0006"),
    ("设计稿 §28 存在", "local-focus-desktop-agent-design-v0.2.md", "# 28. Agent 本地控制面"),
]

def main():
    missing = []
    for label, rel, needle in MARKERS:
        path = os.path.join(ROOT, rel)
        if not os.path.exists(path):
            missing.append((label, "FILE MISSING"))
            continue
        text = open(path, encoding="utf-8", errors="replace").read()
        if needle not in text:
            missing.append((label, f"marker not found: {needle!r}"))
    print("=== v1.5 marker check (working tree vs v1.4.1 baseline) ===")
    if missing:
        print("MISSING:")
        for label, why in missing:
            print(f"  [x] {label}: {why}")
        print("\nRESULT: INCOMPLETE")
        return 1
    print("  all", len(MARKERS), "markers present.")
    print("\n=== git diff stat vs HEAD (v1.4.1) ===")
    r = subprocess.run(["git", "diff", "--stat", "HEAD"], cwd=ROOT, capture_output=True, text=True)
    print(r.stdout or r.stderr)
    print("RESULT: OK")
    return 0

if __name__ == "__main__":
    sys.exit(main())
