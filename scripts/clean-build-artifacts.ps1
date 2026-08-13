param()

$root = Split-Path -Parent $PSScriptRoot
$targets = @(
  (Join-Path $root 'apps\desktop\dist'),
  (Join-Path $root 'apps\desktop\src-tauri\target')
)

foreach ($target in $targets) {
  if (Test-Path -LiteralPath $target) {
    Remove-Item -LiteralPath $target -Recurse -Force
    Write-Host "[Focus] Removed $target"
  }
}
