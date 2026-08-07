# Focus Desktop standalone hang detector (v1.10, #31 / #33)
#
# Polls IsHungAppWindow + SendMessageTimeout(WM_NULL) over every visible top-level
# window of the target process, logs OK/HUNG state transitions, and on the first
# HUNG writes minidump(s) plus hung-window titles/handles and thread count for
# cause analysis. Read-only monitor by default (no stress).
#
# Usage:
#   powershell -NoProfile -ExecutionPolicy Bypass -File hang-detector.ps1 `
#     [-ProcessName desktop] [-Seconds 0] [-LogFile path] [-DumpDir path] `
#     [-IntervalMs 200] [-WaitForProcess 30]
#
# Exit codes:
#   0 = no HUNG within the period
#   1 = HUNG detected
#   2 = process exited during monitoring
#   3 = process not found within WaitForProcess seconds
param(
    [string]$ProcessName = 'desktop',
    [int]$Seconds = 0,
    [string]$LogFile = '',
    [string]$DumpDir = '',
    [int]$IntervalMs = 200,
    [int]$WaitForProcess = 30
)

$ErrorActionPreference = 'Stop'

if (-not $DumpDir) { $DumpDir = Join-Path $env:TEMP 'focus-hangs' }
New-Item -ItemType Directory -Force -Path $DumpDir | Out-Null

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class FocusHang {
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr lParam);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool IsHungAppWindow(IntPtr hWnd);
    [DllImport("user32.dll", EntryPoint = "SendMessageTimeoutW")] public static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam, uint flags, uint timeout, out IntPtr result);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowTextLength(IntPtr hWnd);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int maxCount);
    [DllImport("kernel32.dll")] public static extern IntPtr OpenProcess(uint access, bool inherit, uint pid);
    [DllImport("kernel32.dll")] public static extern bool CloseHandle(IntPtr h);
    [DllImport("dbghelp.dll")] public static extern bool MiniDumpWriteDump(IntPtr hProcess, uint pid, IntPtr hFile, uint type, IntPtr ex, IntPtr us, IntPtr cb);
}
"@

function Get-TopWindows([uint32]$TargetPid) {
    $wins = [System.Collections.Generic.List[IntPtr]]::new()
    $cb = [FocusHang+EnumWindowsProc]{ param($h, $l)
        $pid2 = [uint32]0
        [FocusHang]::GetWindowThreadProcessId($h, [ref]$pid2) | Out-Null
        if ($pid2 -eq $TargetPid -and [FocusHang]::IsWindowVisible($h)) { $wins.Add($h) }
        return $true
    }
    [FocusHang]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
    return $wins
}

function Get-WindowTitle([IntPtr]$h) {
    $len = [FocusHang]::GetWindowTextLength($h)
    if ($len -le 0) { return '' }
    $cap = [Math]::Min($len + 1, 256)
    $sb = [System.Text.StringBuilder]::new($cap)
    [FocusHang]::GetWindowText($h, $sb, $cap) | Out-Null
    return $sb.ToString()
}

function Test-WindowHung([IntPtr]$h) {
    if ([FocusHang]::IsHungAppWindow($h)) { return $true }
    $res = [IntPtr]::Zero
    $r = [FocusHang]::SendMessageTimeout($h, 0, [IntPtr]::Zero, [IntPtr]::Zero, 0x2, 800, [ref]$res)
    return $r -eq [IntPtr]::Zero
}

function Write-Dump([uint32]$procId, [string]$path) {
    $h = [FocusHang]::OpenProcess(0x1F0FFF, $false, $procId)
    if ($h -eq [IntPtr]::Zero) { return $false }
    try {
        $fs = [System.IO.File]::Open($path, [System.IO.FileMode]::Create, [System.IO.FileAccess]::Write)
        try {
            return [FocusHang]::MiniDumpWriteDump($h, $procId, $fs.SafeFileHandle.DangerousGetHandle(), 2, [IntPtr]::Zero, [IntPtr]::Zero, [IntPtr]::Zero)
        } finally { $fs.Dispose() }
    } finally { [FocusHang]::CloseHandle($h) | Out-Null }
}

function Describe-Windows($wins) {
    if (-not $wins) { return '' }
    ($wins | ForEach-Object {
        $t = Get-WindowTitle $_
        $h = $_.ToString('X')
        if ($t) { $h + '(' + $t + ')' } else { $h }
    }) -join ';'
}

function Get-ThreadCount([uint32]$procId) {
    $p = Get-Process -Id $procId -ErrorAction SilentlyContinue
    if ($p) { return $p.Threads.Count }
    return -1
}

# Wait for the process to appear (covers launch-focus.cmd monitor right after start).
$proc = $null
$deadline = (Get-Date).AddSeconds($WaitForProcess)
while (-not $proc -and (Get-Date) -lt $deadline) {
    $proc = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $proc) { Start-Sleep -Milliseconds 500 }
}
if (-not $proc) {
    Write-Host "[hang-detector] process '$ProcessName' not found within ${WaitForProcess}s"
    exit 3
}
$targetPid = [uint32]$proc.Id

if ($LogFile) {
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $LogFile) | Out-Null
    Set-Content -LiteralPath $LogFile -Value 'time,state,detail' -Encoding UTF8
}
function Write-Log($line) {
    Write-Host $line
    if ($LogFile) { Add-Content -LiteralPath $LogFile -Value $line -Encoding UTF8 }
}

$start = Get-Date
$last = ''
$dumped = 0
$maxDumps = 2
$hungSince = $null
$secondDumpAt = $null
Write-Log ('{0},MONITOR,pid={1} proc={2}' -f (Get-Date -Format 'HH:mm:ss'), $targetPid, $ProcessName)
while ($true) {
    $wins = @(Get-TopWindows $targetPid)
    if ($wins.Count -eq 0) {
        if (-not (Get-Process -Id $targetPid -ErrorAction SilentlyContinue)) {
            Write-Log ('{0},EXITED' -f (Get-Date -Format 'HH:mm:ss'))
            exit 2
        }
        $state = 'NO_WINDOWS'
    } else {
        $hung = @($wins | Where-Object { Test-WindowHung $_ })
        $state = if ($hung.Count -gt 0) { 'HUNG(' + $hung.Count + '/' + $wins.Count + ')' } else { 'OK' }
    }

    if ($state -ne $last) {
        if ($state -like 'HUNG*') {
            $hungSince = Get-Date
            $secondDumpAt = $hungSince.AddSeconds(3)
            $detail = 'windows=[' + (Describe-Windows $hung) + '] threads=' + (Get-ThreadCount $targetPid)
            if ($dumped -lt $maxDumps) {
                $dumped++
                $dumpPath = Join-Path $DumpDir ($ProcessName + '-' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '-' + $dumped + '.dmp')
                $ok = Write-Dump $targetPid $dumpPath
                $detail += ' dump=' + $(if ($ok) { $dumpPath } else { 'dump-failed' }) + ' dumped=' + $dumped + '/' + $maxDumps
            }
            Write-Log ('{0},{1},{2}' -f (Get-Date -Format 'HH:mm:ss'), $state, $detail)
        } elseif ($last -like 'HUNG*') {
            $dur = [Math]::Round(((Get-Date) - $hungSince).TotalSeconds, 1)
            Write-Log ('{0},RECOVERED,hung_for={1}s threads={2}' -f (Get-Date -Format 'HH:mm:ss'), $dur, (Get-ThreadCount $targetPid))
        } else {
            Write-Log ('{0},{1},windows=[{2}] threads={3}' -f (Get-Date -Format 'HH:mm:ss'), $state, (Describe-Windows $wins), (Get-ThreadCount $targetPid))
        }
        $last = $state
    } elseif ($state -like 'HUNG*' -and $secondDumpAt -and (Get-Date) -ge $secondDumpAt -and $dumped -lt $maxDumps) {
        # Still hung: take one more dump a few seconds later to catch a sustained hang.
        $secondDumpAt = $null
        $dumped++
        $dumpPath = Join-Path $DumpDir ($ProcessName + '-' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '-' + $dumped + '.dmp')
        $ok = Write-Dump $targetPid $dumpPath
        Write-Log ('{0},{1},windows=[{2}] threads={3} dump={4} dumped={5}/{6}' -f (Get-Date -Format 'HH:mm:ss'), $state, (Describe-Windows $hung), (Get-ThreadCount $targetPid), $(if ($ok) { $dumpPath } else { 'dump-failed' }), $dumped, $maxDumps)
    }

    if ($Seconds -gt 0 -and ((Get-Date) - $start).TotalSeconds -ge $Seconds) {
        if ($last -like 'HUNG*') { exit 1 } else { exit 0 }
    }
    Start-Sleep -Milliseconds $IntervalMs
}