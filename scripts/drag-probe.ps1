<#
.SYNOPSIS
  Focus Desktop drag regression probe (v1.2.1).

.DESCRIPTION
  Synthesizes a real "press -> stepwise move -> release" drag with
  SetCursorPos + mouse_event and records the target window's GetWindowRect
  trajectory against the cursor. The cursor path is interpolated from the
  window's current position toward a FREE grid cell (other float windows are
  avoided so the anti-overlap snap-back cannot fire), then the window is
  dragged back to its original cell. The probe is DPI-aware so all
  coordinates are physical, matching the app.

  Before the v1.2.1 fix this showed oscillation and a final drop at (0,0);
  after the fix every step tracks ~1:1 (max lag <= 8px), the drop is a
  non-(0,0) on-screen grid cell, and the restore returns to the original cell.

.PARAMETER ProcessName
  Process to probe (default: desktop = the release binary).
.PARAMETER WindowTitle
  Substring of the target window title (default: "对话" = chat float).
.PARAMETER Steps
  Number of synthetic move steps per drag.
.PARAMETER DelayMs
  Settle delay after each cursor move (must exceed the 15ms Rust poller).
#>
param(
    [string]$ProcessName = "desktop",
    [string]$WindowTitle = "对话",
    [int]$Steps = 8,
    [int]$DelayMs = 80
)

$ErrorActionPreference = "Stop"

if (-not ("Win32Probe" -as [type])) {
    Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;

public class Win32Probe {
    [StructLayout(LayoutKind.Sequential)]
    public struct POINT { public int X; public int Y; }
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }

    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);

    [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out POINT lpPoint);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern void mouse_event(uint dwFlags, uint dx, uint dy, uint dwData, UIntPtr dwExtraInfo);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder lpString, int nMaxCount);
    [DllImport("user32.dll")] public static extern int GetSystemMetrics(int nIndex);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
"@
}

# Physical-pixel coordinate space, same as the (DPI-aware) app.
[Win32Probe]::SetProcessDPIAware() | Out-Null

$MOUSEEVENTF_LEFTDOWN = 0x0002
$MOUSEEVENTF_LEFTUP   = 0x0004
$SM_XVIRTUALSCREEN    = 76
$SM_YVIRTUALSCREEN    = 77
$SM_CXVIRTUALSCREEN   = 78
$SM_CYVIRTUALSCREEN   = 79

$vx = [Win32Probe]::GetSystemMetrics($SM_XVIRTUALSCREEN)
$vy = [Win32Probe]::GetSystemMetrics($SM_YVIRTUALSCREEN)
$vw = [Win32Probe]::GetSystemMetrics($SM_CXVIRTUALSCREEN)
$vh = [Win32Probe]::GetSystemMetrics($SM_CYVIRTUALSCREEN)
$GRID_COLS = 12
$GRID_ROWS = 8

$procs = @(Get-Process -Name $ProcessName -ErrorAction SilentlyContinue)
if ($procs.Count -eq 0) {
    Write-Host "[probe] ERROR: no process named '$ProcessName' is running."
    exit 1
}
$procId = $procs[0].Id

function Get-Rect($h) {
    $r = New-Object Win32Probe+RECT
    [Win32Probe]::GetWindowRect($h, [ref]$r) | Out-Null
    return $r
}
function Get-Cursor {
    $p = New-Object Win32Probe+POINT
    [Win32Probe]::GetCursorPos([ref]$p) | Out-Null
    return $p
}

$found = New-Object System.Collections.ArrayList
$cb = {
    param($hWnd, $lParam)
    $wp = 0
    [Win32Probe]::GetWindowThreadProcessId($hWnd, [ref]$wp) | Out-Null
    if ($wp -eq $procId -and [Win32Probe]::IsWindowVisible($hWnd)) {
        $sb = New-Object System.Text.StringBuilder 256
        [Win32Probe]::GetWindowText($hWnd, $sb, 256) | Out-Null
        $r = Get-Rect $hWnd
        [void]$found.Add([pscustomobject]@{
            Hwnd = $hWnd; Title = $sb.ToString()
            Left = $r.Left; Top = $r.Top; Right = $r.Right; Bottom = $r.Bottom
        })
    }
    return $true
}
[Win32Probe]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null

if ($found.Count -eq 0) {
    Write-Host "[probe] ERROR: no visible windows found for process '$ProcessName' (pid $procId)."
    exit 1
}

$target = $found | Where-Object { $_.Title -like "*$WindowTitle*" } | Select-Object -First 1
if (-not $target) {
    Write-Host "[probe] ERROR: window with title containing '$WindowTitle' not found. Visible windows:"
    $found | Format-Table Hwnd, Title, Left, Top, Right, Bottom | Out-String | Write-Host
    exit 2
}

function Overlaps($aL, $aT, $aW, $aH, $bL, $bT, $bW, $bH) {
    return (($aL -lt ($bL + $bW)) -and ($bL -lt ($aL + $aW)) -and `
            ($aT -lt ($bT + $bH)) -and ($bT -lt ($aT + $aH)))
}

# other float windows = visible windows of the process, excluding the fullscreen
# desktop canvas and the target itself
$desktopCanvas = $found | Where-Object { $_.Left -le $vx -and $_.Top -le $vy -and `
    ($_.Right - $_.Left) -ge ($vw - 2) -and ($_.Bottom - $_.Top) -ge ($vh - 2) } | Select-Object -First 1
$others = @($found | Where-Object { $_.Hwnd -ne $target.Hwnd -and $_.Hwnd -ne $desktopCanvas.Hwnd })

$r0 = Get-Rect $target.Hwnd
$w0 = $r0.Right - $r0.Left
$h0 = $r0.Bottom - $r0.Top
if ($w0 -le 0 -or $h0 -le 0) {
    Write-Host "[probe] ERROR: target window has invalid size ${w0}x${h0}."
    exit 3
}
$origL = $r0.Left; $origT = $r0.Top
$grabX = [int]($r0.Left + $w0 / 2)
$grabY = $r0.Top + 15
$offX = $grabX - $r0.Left
$offY = $grabY - $r0.Top

# window footprint in grid cells (margin for border)
$cellW = $vw / $GRID_COLS
$cellH = $vh / $GRID_ROWS
$cw = [Math]::Ceiling(($w0 + 4) / $cellW)
$ch = [Math]::Ceiling(($h0 + 4) / $cellH)
$cw = [Math]::Min($cw, $GRID_COLS)
$ch = [Math]::Min($ch, $GRID_ROWS)

# find a free cell: center-preferred, else row-major scan
function Is-CellFree($col, $row) {
    $l = $vx + $col * $cellW
    $t = $vy + $row * $cellH
    $w = $cw * $cellW
    $h = $ch * $cellH
    if (($col + $cw) -gt $GRID_COLS -or ($row + $ch) -gt $GRID_ROWS) { return $false }
    foreach ($o in $others) {
        if (Overlaps $l $t $w $h $o.Left $o.Top ($o.Right - $o.Left) ($o.Bottom - $o.Top)) { return $false }
    }
    return $true
}

$tCol = $null; $tRow = $null
$centerCol = [Math]::Max(0, [Math]::Min(($GRID_COLS - $cw), [int](($GRID_COLS - $cw) / 2)))
$centerRow = [Math]::Max(0, [Math]::Min(($GRID_ROWS - $ch), [int](($GRID_ROWS - $ch) / 2)))
if (Is-CellFree $centerCol $centerRow) { $tCol = $centerCol; $tRow = $centerRow }
if ($null -eq $tCol) {
    for ($r = 0; $r -le ($GRID_ROWS - $ch) -and $null -eq $tCol; $r++) {
        for ($c = 0; $c -le ($GRID_COLS - $cw); $c++) {
            if (Is-CellFree $c $r) { $tCol = $c; $tRow = $r; break }
        }
    }
}
if ($null -eq $tCol) {
    Write-Host "[probe] ERROR: no free grid cell for a ${cw}x${ch} window."
    exit 4
}
$tL = [int]($vx + $tCol * $cellW + 2)
$tT = [int]($vy + $tRow * $cellH + 2)

function Do-Drag($h, $pressX, $pressY, $toX, $toY) {
    [Win32Probe]::SetCursorPos($pressX, $pressY) | Out-Null
    Start-Sleep -Milliseconds 150
    [Win32Probe]::mouse_event($MOUSEEVENTF_LEFTDOWN, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 200
    $rows = New-Object System.Collections.ArrayList
    function Add-Row($step) {
        $cur = Get-Cursor
        $r = Get-Rect $h
        $expL = $cur.X - $offX
        $expT = $cur.Y - $offY
        [void]$rows.Add([pscustomobject]@{
            Step = $step; CurX = $cur.X; CurY = $cur.Y
            WinL = $r.Left; WinT = $r.Top
            ExpL = $expL; ExpT = $expT
            dL = ($r.Left - $expL); dT = ($r.Top - $expT)
        })
    }
    Add-Row 0
    for ($i = 1; $i -le $Steps; $i++) {
        $cx = [int][Math]::Round($pressX + ($toX - $pressX) * $i / $Steps)
        $cy = [int][Math]::Round($pressY + ($toY - $pressY) * $i / $Steps)
        [Win32Probe]::SetCursorPos($cx, $cy) | Out-Null
        Start-Sleep -Milliseconds $DelayMs
        Add-Row $i
    }
    [Win32Probe]::mouse_event($MOUSEEVENTF_LEFTUP, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 300
    Add-Row "FIN"
    return ,$rows
}

Write-Host ""
Write-Host "[probe] target: '$($target.Title)'  hwnd=$($target.Hwnd)  size=${w0}x${h0}"
Write-Host "[probe] start cell: ($($r0.Left), $($r0.Top))  target cell: (${tL}, ${tT})  steps=$Steps delay=${DelayMs}ms"

$rows1 = Do-Drag $target.Hwnd $grabX $grabY ($tL + $offX) ($tT + $offY)
$fin1 = $rows1[$rows1.Count - 1]

# restore: drag back to the original cell
$grabX2 = $fin1.WinL + $offX
$grabY2 = $fin1.WinT + $offY
$rows2 = Do-Drag $target.Hwnd $grabX2 $grabY2 ($origL + $offX) ($origT + $offY)
$fin2 = $rows2[$rows2.Count - 1]

Write-Host ""
Write-Host "== drag out (tracking) =="
Write-Host "Step  CurX  CurY  WinL  WinT  ExpL  ExpT  dL    dT"
foreach ($row in $rows1) {
    Write-Host ("{0,-5} {1,-5} {2,-5} {3,-5} {4,-5} {5,-5} {6,-5} {7,-5} {8,-5}" -f `
        $row.Step, $row.CurX, $row.CurY, $row.WinL, $row.WinT, $row.ExpL, $row.ExpT, $row.dL, $row.dT)
}

$issues = New-Object System.Collections.ArrayList

$maxLag = 0
foreach ($row in $rows1) {
    if ($row.Step -eq "FIN") { continue }
    $lag = [Math]::Max([Math]::Abs($row.dL), [Math]::Abs($row.dT))
    if ($lag -gt $maxLag) { $maxLag = $lag }
}
if ($maxLag -gt 8) { [void]$issues.Add("tracking lag up to ${maxLag}px (expect <= 8px, ~1:1)") }
else { Write-Host "[probe] OK: max tracking lag ${maxLag}px (1:1 follow)." }

$osc = 0
for ($i = 2; $i -lt $rows1.Count; $i++) {
    $a = $rows1[$i - 1]; $b = $rows1[$i]
    if ($b.Step -eq "FIN") { break }
    $cDx = $b.CurX - $a.CurX; $wDx = $b.WinL - $a.WinL
    if ($cDx -ne 0 -and $wDx -ne 0 -and [Math]::Sign($wDx) -ne [Math]::Sign($cDx)) { $osc++ }
}
if ($osc -gt 0) { [void]$issues.Add("trajectory oscillated $osc step(s).") }
else { Write-Host "[probe] OK: no trajectory oscillation." }

$onScreen = ($fin1.WinL -ge $vx -and $fin1.WinT -ge $vy -and `
             ($fin1.WinL + $w0) -le ($vx + $vw + 2) -and ($fin1.WinT + $h0) -le ($vy + $vh + 2))
if (-not $onScreen) { [void]$issues.Add("drop off-screen: L=$($fin1.WinL) T=$($fin1.WinT)") }
else { Write-Host "[probe] OK: drop on screen." }

if ($fin1.WinL -eq 0 -and $fin1.WinT -eq 0) { [void]$issues.Add("drop at (0,0) - regression!") }
else { Write-Host "[probe] OK: drop is not (0,0)." }

$dist = [Math]::Sqrt(($fin1.WinL - $tL) * ($fin1.WinL - $tL) + ($fin1.WinT - $tT) * ($fin1.WinT - $tT))
if ($dist -gt 140) { [void]$issues.Add("drop ${dist}px from target cell - grid snap may be off.") }
else { Write-Host "[probe] OK: drop within ${dist}px of target cell (grid snap)." }

$dist2 = [Math]::Sqrt(($fin2.WinL - $origL) * ($fin2.WinL - $origL) + ($fin2.WinT - $origT) * ($fin2.WinT - $origT))
if ($dist2 -gt 140) { [void]$issues.Add("restore ${dist2}px from original cell - restore drag failed.") }
else { Write-Host "[probe] OK: restored within ${dist2}px of original cell." }

Write-Host ""
if ($issues.Count -eq 0) { Write-Host "[probe] RESULT: PASS"; exit 0 }
else {
    Write-Host "[probe] RESULT: FAIL"
    foreach ($i in $issues) { Write-Host "  - $i" }
    exit 1
}