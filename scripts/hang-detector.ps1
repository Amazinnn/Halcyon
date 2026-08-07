# Focus Desktop 独立卡死检测（v1.10, #31）
# 用 IsHungAppWindow + SendMessageTimeout(WM_NULL) 轮询目标进程所有可见顶层窗口，
# 记录 OK/HUNG 状态迁移；首次 HUNG 时抓取 minidump 供原因分析。
# 默认只监测不压测。用法：
#   powershell -File hang-detector.ps1 [-ProcessName desktop] [-Seconds 0] [-LogFile path] [-DumpDir path] [-IntervalMs 200]
# 退出码：0 = 监测期内未检测到 HUNG；1 = 检测到 HUNG；3 = 进程未找到。

param(
    [string]$ProcessName = 'desktop',
    [int]$Seconds = 0,
    [string]$LogFile = '',
    [string]$DumpDir = '',
    [int]$IntervalMs = 200
)

$ErrorActionPreference = 'Stop'

if (-not $DumpDir) { $DumpDir = Join-Path $env:TEMP 'focus-hangs' }
New-Item -ItemType Directory -Force -Path $DumpDir | Out-Null

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class FocusHang {
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr lParam);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint pid);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool IsHungAppWindow(IntPtr hWnd);
    [DllImport("user32.dll", EntryPoint = "SendMessageTimeoutW")] public static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint msg, IntPtr wParam, IntPtr lParam, uint flags, uint timeout, out IntPtr result);
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

function Test-WindowHung([IntPtr]$h) {
    if ([FocusHang]::IsHungAppWindow($h)) { return $true }
    $res = [IntPtr]::Zero
    $r = [FocusHang]::SendMessageTimeout($h, 0, [IntPtr]::Zero, [IntPtr]::Zero, 0x2, 800, [ref]$res)
    return $r -eq [IntPtr]::Zero
}

function Write-Dump([uint32]$pid, [string]$path) {
    $h = [FocusHang]::OpenProcess(0x1F0FFF, $false, $pid)
    if ($h -eq [IntPtr]::Zero) { return $false }
    try {
        $fs = [System.IO.File]::Open($path, [System.IO.FileMode]::Create, [System.IO.FileAccess]::Write)
        try {
            return [FocusHang]::MiniDumpWriteDump($h, $pid, $fs.SafeFileHandle.DangerousGetHandle(), 2, [IntPtr]::Zero, [IntPtr]::Zero, [IntPtr]::Zero)
        } finally { $fs.Dispose() }
    } finally { [FocusHang]::CloseHandle($h) | Out-Null }
}

$proc = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $proc) {
    Write-Host "[hang-detector] 进程 $ProcessName 未找到"
    exit 3
}
$targetPid = [uint32]$proc.Id

if ($LogFile) {
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $LogFile) | Out-Null
    Set-Content -LiteralPath $LogFile -Value "time,state,windows,dump" -Encoding UTF8
}
function Write-Log($line) {
    Write-Host $line
    if ($LogFile) { Add-Content -LiteralPath $LogFile -Value $line -Encoding UTF8 }
}

$start = Get-Date
$last = ''
$dumped = $false
Write-Log ("{0},MONITOR,pid={1}" -f (Get-Date -Format 'HH:mm:ss'), $targetPid)
while ($true) {
    $wins = Get-TopWindows $targetPid
    if ($wins.Count -eq 0) {
        # process may have exited; check once more
        if (-not (Get-Process -Id $targetPid -ErrorAction SilentlyContinue)) {
            Write-Log ("{0},EXITED" -f (Get-Date -Format 'HH:mm:ss'))
            exit 2
        }
        $state = 'NO_WINDOWS'
    } else {
        $hung = @($wins | Where-Object { Test-WindowHung $_ })
        $state = if ($hung.Count -gt 0) { "HUNG($($hung.Count)/$($wins.Count))" } else { 'OK' }
    }
    if ($state -ne $last) {
        $dump = ''
        if ($state -like 'HUNG*' -and -not $dumped) {
            $dumped = $true
            $dumpPath = Join-Path $DumpDir ("{0}-{1}.dmp" -f $ProcessName, (Get-Date -Format 'yyyyMMdd-HHmmss'))
            $ok = Write-Dump $targetPid $dumpPath
            $dump = if ($ok) { $dumpPath } else { 'dump-failed' }
            Write-Log ("{0},{1},{2},DUMP:{3}" -f (Get-Date -Format 'HH:mm:ss'), $state, (($wins | ForEach-Object { $_.ToString('X') }) -join ','), $dump)
        } else {
            Write-Log ("{0},{1},{2}" -f (Get-Date -Format 'HH:mm:ss'), $state, (($wins | ForEach-Object { $_.ToString('X') }) -join ','))
        }
        $last = $state
    }
    if ($Seconds -gt 0 -and ((Get-Date) - $start).TotalSeconds -ge $Seconds) {
        if ($last -like 'HUNG*') { exit 1 } else { exit 0 }
    }
    Start-Sleep -Milliseconds $IntervalMs
}