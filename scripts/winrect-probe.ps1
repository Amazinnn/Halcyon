# winrect-probe.ps1 - objective grid-alignment probe for Focus Desktop (v1.10.3.1, #48).
# Prints each float window's actual rect vs expected rect (from settings.json)
# and checks the window center against the grid-cell center.
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File scripts\winrect-probe.ps1
# Exit code: 0 = all aligned, 1 = at least one mismatch.
param(
  [string]$ProcessName = 'desktop',
  [string]$SettingsPath = ''
)
$ErrorActionPreference = 'Stop'

$code = @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class WinRectProbe {
  public delegate bool EnumProc(IntPtr hWnd, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr lParam);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT r);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder sb, int max);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  public class WinInfo { public int L, T, R, B; public string Title; public int W { get { return R - L; } } public int H { get { return B - T; } } }
  public static List<WinInfo> List(uint pid) {
    var res = new List<WinInfo>();
    EnumWindows((h, l) => {
      uint p; GetWindowThreadProcessId(h, out p);
      if (p == pid && IsWindowVisible(h)) {
        RECT r; GetWindowRect(h, out r);
        var sb = new StringBuilder(256);
        GetWindowText(h, sb, 256);
        res.Add(new WinInfo { L = r.Left, T = r.Top, R = r.Right, B = r.Bottom, Title = sb.ToString() });
      }
      return true;
    }, IntPtr.Zero);
    return res;
  }
}
"@
Add-Type -TypeDefinition $code -Language CSharp

$proc = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $proc) { Write-Output "ERROR: no process named $ProcessName"; exit 2 }

if (-not $SettingsPath) { $SettingsPath = Join-Path $env:APPDATA 'com.focusdesktop.app\settings.json' }
$settings = Get-Content -LiteralPath $SettingsPath -Raw -Encoding UTF8 | ConvertFrom-Json

$wins = [WinRectProbe]::List([uint32]$proc.Id)
$desk = $wins | Where-Object { $_.Title -eq 'Focus Desktop' } | Select-Object -First 1
if (-not $desk) { Write-Output 'ERROR: Focus Desktop fullscreen window not found'; exit 3 }

$cellW = ($desk.R - $desk.L) / 12.0
$cellH = ($desk.B - $desk.T) / 8.0
Write-Output ("screen(basis)={0}x{1} cell={2:N2}x{3:N2}" -f ($desk.R-$desk.L), ($desk.B-$desk.T), $cellW, $cellH)

$labels = @('chat','stats','music','pet','workflow')
$fail = 0
foreach ($label in $labels) {
  $g = $settings.grid.$label
  if (-not $g) { Write-Output ("{0}: no grid entry" -f $label); continue }
  $ex = $g.col * $cellW
  $ey = $g.row * $cellH
  $ew = $g.cols * $cellW
  $eh = $g.rows * $cellH
  $ecx = $ex + $ew / 2
  $ecy = $ey + $eh / 2
  $collapsed = @($settings.collapsed) -contains $label
  $best = $null
  $bestD = [double]::MaxValue
  foreach ($w in $wins) {
    if ($w.Title -eq 'Focus Desktop' -or $w.W -lt 50 -or $w.H -lt 50) { continue }
    $d = [Math]::Abs($w.L - $ex) + [Math]::Abs($w.T - $ey)
    if ($d -lt $bestD) { $bestD = $d; $best = $w }
  }
  if ($collapsed) {
    if ($best -and $bestD -lt 5) {
      Write-Output ("{0}: COLLAPSED but visible at ({1},{2}) -> FAIL" -f $label, $best.L, $best.T)
      $fail = 1
    } else {
      Write-Output ("{0}: hidden (collapsed) -> OK" -f $label)
    }
    continue
  }
  if (-not $best) {
    Write-Output ("{0}: expected visible but no window -> FAIL" -f $label)
    $fail = 1
    continue
  }
  $dx = $best.L - $ex
  $dy = $best.T - $ey
  $dw = $best.W - $ew
  $dh = $best.H - $eh
  $acx = $best.L + $best.W / 2.0
  $acy = $best.T + $best.H / 2.0
  $cdx = $acx - $ecx
  $cdy = $acy - $ecy
  $ok = ([Math]::Abs($dx) -le 1) -and ([Math]::Abs($dy) -le 1) -and ([Math]::Abs($dw) -le 1) -and ([Math]::Abs($dh) -le 1) -and ([Math]::Abs($cdx) -le 1) -and ([Math]::Abs($cdy) -le 1)
  if (-not $ok) { $fail = 1 }
  $verdict = $(if ($ok) { 'OK' } else { 'FAIL' })
  Write-Output ("{0}: {1}  actual=({2},{3} {4}x{5}) expected=({6:N1},{7:N1} {8:N1}x{9:N1}) diff=(x{10:+0;-0} y{11:+0;-0} w{12:+0;-0} h{13:+0;-0}) center=(dx{14:+0.0;-0.0} dy{15:+0.0;-0.0})" -f $label, $verdict, $best.L, $best.T, $best.W, $best.H, $ex, $ey, $ew, $eh, $dx, $dy, $dw, $dh, $cdx, $cdy)
}
Write-Output ("RESULT: " + $(if ($fail -eq 0) { 'PASS' } else { 'FAIL' }))
exit $fail
