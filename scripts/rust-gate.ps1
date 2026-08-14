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
} finally {
    Pop-Location -ErrorAction SilentlyContinue
    Remove-Item Env:\RUSTFLAGS -ErrorAction SilentlyContinue
}
exit $code
