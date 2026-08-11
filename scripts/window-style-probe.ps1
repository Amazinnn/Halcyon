param(
    [int]$ProcessId = 0,
    [switch]$AsJson
)

if (-not ("FocusWindowProbe" -as [type])) {
    Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class FocusWindowProbe {
  public delegate bool EnumWindowsProc(IntPtr hwnd, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr lp);
  [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr parent, EnumWindowsProc cb, IntPtr lp);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint pid);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr hwnd, StringBuilder text, int max);
  [DllImport("user32.dll")] public static extern IntPtr GetWindowLongPtr(IntPtr hwnd, int index);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern IntPtr GetParent(IntPtr hwnd);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hwnd);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hwnd, out RECT rect);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
}
'@
}

[void][FocusWindowProbe]::SetProcessDPIAware()

$targetPids = if ($ProcessId) { @($ProcessId) } else {
    @(Get-Process -ErrorAction SilentlyContinue |
        Where-Object { $_.ProcessName -in @('desktop', 'focus-desktop', 'Focus Desktop') } |
        Select-Object -ExpandProperty Id)
}
if (-not $targetPids) { throw 'Focus process not found. Pass -ProcessId <pid>.' }

$foreground = [FocusWindowProbe]::GetForegroundWindow()
$rows = [System.Collections.Generic.List[object]]::new()
$addRow = {
    param([IntPtr]$hwnd, [string]$kind)
    [uint32]$windowPid = 0
    [void][FocusWindowProbe]::GetWindowThreadProcessId($hwnd, [ref]$windowPid)
    $title = [System.Text.StringBuilder]::new(256)
    [void][FocusWindowProbe]::GetWindowText($hwnd, $title, $title.Capacity)
    $rect = [FocusWindowProbe+RECT]::new()
    $client = [FocusWindowProbe+RECT]::new()
    [void][FocusWindowProbe]::GetWindowRect($hwnd, [ref]$rect)
    [void][FocusWindowProbe]::GetClientRect($hwnd, [ref]$client)
    $rows.Add([pscustomobject]@{
        Kind = $kind; Pid = $windowPid; Hwnd = ('0x{0:X}' -f $hwnd.ToInt64())
        Parent = ('0x{0:X}' -f [FocusWindowProbe]::GetParent($hwnd).ToInt64()); Title = $title.ToString()
        Style = ('0x{0:X8}' -f [uint32][FocusWindowProbe]::GetWindowLongPtr($hwnd, -16).ToInt64())
        ExStyle = ('0x{0:X8}' -f [uint32][FocusWindowProbe]::GetWindowLongPtr($hwnd, -20).ToInt64())
        Visible = [FocusWindowProbe]::IsWindowVisible($hwnd); Foreground = ($hwnd -eq $foreground)
        Rect = "$($rect.Left),$($rect.Top) $($rect.Right - $rect.Left)x$($rect.Bottom - $rect.Top)"
        Client = "$($client.Right - $client.Left)x$($client.Bottom - $client.Top)"
    })
}
$childCallback = [FocusWindowProbe+EnumWindowsProc]{
    param([IntPtr]$hwnd, [IntPtr]$ignored)
    & $addRow $hwnd 'child'
    return $true
}
$callback = [FocusWindowProbe+EnumWindowsProc]{
    param([IntPtr]$hwnd, [IntPtr]$ignored)
    [uint32]$windowPid = 0
    [void][FocusWindowProbe]::GetWindowThreadProcessId($hwnd, [ref]$windowPid)
    if ($targetPids -notcontains [int]$windowPid) { return $true }
    & $addRow $hwnd 'host'
    [void][FocusWindowProbe]::EnumChildWindows($hwnd, $childCallback, [IntPtr]::Zero)
    return $true
}
[void][FocusWindowProbe]::EnumWindows($callback, [IntPtr]::Zero)
if ($AsJson) {
    $rows | Sort-Object Kind, Title, Hwnd | ConvertTo-Json -Depth 3
} else {
    $rows | Sort-Object Kind, Title, Hwnd | Format-Table -AutoSize
}
