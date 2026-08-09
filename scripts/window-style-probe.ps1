param(
    [int]$ProcessId = 0
)

if (-not ("FocusWindowProbe" -as [type])) {
    Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class FocusWindowProbe {
  public delegate bool EnumWindowsProc(IntPtr hwnd, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr lp);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr hwnd, StringBuilder text, int max);
  [DllImport("user32.dll")] public static extern IntPtr GetWindowLongPtr(IntPtr hwnd, int index);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
'@
}

$targetPids = if ($ProcessId) { @($ProcessId) } else {
    @(Get-Process -ErrorAction SilentlyContinue |
        Where-Object { $_.ProcessName -in @('desktop', 'focus-desktop', 'Focus Desktop') } |
        Select-Object -ExpandProperty Id)
}
if (-not $targetPids) { throw 'Focus process not found. Pass -ProcessId <pid>.' }

$foreground = [FocusWindowProbe]::GetForegroundWindow()
$rows = [System.Collections.Generic.List[object]]::new()
$callback = [FocusWindowProbe+EnumWindowsProc]{
    param([IntPtr]$hwnd, [IntPtr]$ignored)
    [uint32]$pid = 0
    [void][FocusWindowProbe]::GetWindowThreadProcessId($hwnd, [ref]$pid)
    if ($targetPids -notcontains [int]$pid) { return $true }
    $title = [System.Text.StringBuilder]::new(256)
    [void][FocusWindowProbe]::GetWindowText($hwnd, $title, $title.Capacity)
    $rect = [FocusWindowProbe+RECT]::new()
    [void][FocusWindowProbe]::GetWindowRect($hwnd, [ref]$rect)
    $rows.Add([pscustomobject]@{
        Pid = $pid; Hwnd = ('0x{0:X}' -f $hwnd.ToInt64()); Title = $title.ToString()
        Style = ('0x{0:X8}' -f [uint32][FocusWindowProbe]::GetWindowLongPtr($hwnd, -16).ToInt64())
        ExStyle = ('0x{0:X8}' -f [uint32][FocusWindowProbe]::GetWindowLongPtr($hwnd, -20).ToInt64())
        Foreground = ($hwnd -eq $foreground)
        Rect = "$($rect.Left),$($rect.Top) $($rect.Right - $rect.Left)x$($rect.Bottom - $rect.Top)"
    })
    return $true
}
[void][FocusWindowProbe]::EnumWindows($callback, [IntPtr]::Zero)
$rows | Sort-Object Title | Format-Table -AutoSize
