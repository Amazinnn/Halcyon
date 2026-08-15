# Rust gate wrapper: debug test binaries import comctl32 TaskDialogIndirect
# (via the tao window stack) and need a manifest that activates comctl32 v6;
# without it the loader binds v5 and the process dies with
# STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139). RUSTFLAGS here applies only to
# this cargo invocation, so release builds keep tauri's own manifest.
param([Parameter(ValueFromRemainingArguments = $true)][string[]]$CargoArgs)
$PSNativeCommandUseErrorActionPreference = $false
$root = Split-Path -Parent $PSScriptRoot
$manifest = Join-Path $root "apps\desktop\src-tauri\focus.manifest"
$env:RUSTFLAGS = "-C link-arg=/MANIFEST:EMBED -C link-arg=/MANIFESTINPUT:$manifest"
Push-Location (Join-Path $root "apps\desktop\src-tauri")
try {
    cargo test --lib @CargoArgs
    $code = $LASTEXITCODE
    if ($code -eq 0) {
        # dead-code / unused-import regressions are warnings; any warning fails the gate
        $warns = cargo check --lib --tests 2>&1 | Select-String "^warning"
        if ($warns) { Write-Output "RUST WARNINGS:"; $warns | Select-Object -First 8; $code = 1 }
    }
    if ($code -eq 0) {
        # style lints are out of scope (existing code base is not rustfmt-clean);
        # correctness/suspicious must stay clean
        cargo clippy --lib --tests -- -A clippy::all -W clippy::correctness -W clippy::suspicious -D warnings
        $code = $LASTEXITCODE
    }
} finally {
    Pop-Location -ErrorAction SilentlyContinue
    Remove-Item Env:\RUSTFLAGS -ErrorAction SilentlyContinue
}
exit $code