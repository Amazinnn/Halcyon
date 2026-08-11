param(
    [int]$ProcessId = 0,
    [switch]$AsJson,
    [ValidateRange(0, 300)]
    [int]$WatchSeconds = 0,
    [ValidateRange(10, 5000)]
    [int]$IntervalMs = 100,
    [string]$OutputPath = ""
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
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr hwnd, StringBuilder text, int max);
  [DllImport("user32.dll")] public static extern IntPtr GetWindowLongPtr(IntPtr hwnd, int index);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern IntPtr GetParent(IntPtr hwnd);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hwnd);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hwnd, out RECT rect);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr hwnd, ref POINT point);
  [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr hwnd, int attribute, out RECT value, int size);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
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

function Get-RectText([FocusWindowProbe+RECT]$Rect) {
    "$($Rect.Left),$($Rect.Top) $($Rect.Right - $Rect.Left)x$($Rect.Bottom - $Rect.Top)"
}

function Get-FocusWindowRows {
    $foreground = [FocusWindowProbe]::GetForegroundWindow()
    $rows = [System.Collections.Generic.List[object]]::new()
    $capturedAt = [DateTime]::UtcNow.ToString("o")
    $addRow = {
    param([IntPtr]$hwnd, [string]$kind)
    [uint32]$windowPid = 0
    [void][FocusWindowProbe]::GetWindowThreadProcessId($hwnd, [ref]$windowPid)
    $title = [System.Text.StringBuilder]::new(256)
    $className = [System.Text.StringBuilder]::new(256)
    [void][FocusWindowProbe]::GetWindowText($hwnd, $title, $title.Capacity)
    [void][FocusWindowProbe]::GetClassName($hwnd, $className, $className.Capacity)
    $rect = [FocusWindowProbe+RECT]::new()
    $client = [FocusWindowProbe+RECT]::new()
    $clientOrigin = [FocusWindowProbe+POINT]::new()
    $dwmFrame = [FocusWindowProbe+RECT]::new()
    [void][FocusWindowProbe]::GetWindowRect($hwnd, [ref]$rect)
    [void][FocusWindowProbe]::GetClientRect($hwnd, [ref]$client)
    [void][FocusWindowProbe]::ClientToScreen($hwnd, [ref]$clientOrigin)
    $dwmResult = [FocusWindowProbe]::DwmGetWindowAttribute(
        $hwnd,
        9,
        [ref]$dwmFrame,
        [Runtime.InteropServices.Marshal]::SizeOf([type][FocusWindowProbe+RECT])
    )
    $rows.Add([pscustomobject]@{
        CapturedAt = $capturedAt
        Kind = $kind; Pid = $windowPid; Hwnd = ('0x{0:X}' -f $hwnd.ToInt64())
        Parent = ('0x{0:X}' -f [FocusWindowProbe]::GetParent($hwnd).ToInt64()); Title = $title.ToString(); Class = $className.ToString()
        Style = ('0x{0:X8}' -f [uint32][FocusWindowProbe]::GetWindowLongPtr($hwnd, -16).ToInt64())
        ExStyle = ('0x{0:X8}' -f [uint32][FocusWindowProbe]::GetWindowLongPtr($hwnd, -20).ToInt64())
        Visible = [FocusWindowProbe]::IsWindowVisible($hwnd); Foreground = ($hwnd -eq $foreground)
        OuterRect = Get-RectText $rect
        ClientRect = "$($clientOrigin.X),$($clientOrigin.Y) $($client.Right - $client.Left)x$($client.Bottom - $client.Top)"
        DwmFrame = if ($dwmResult -eq 0) { Get-RectText $dwmFrame } else { $null }
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
    return @($rows | Sort-Object Kind, Title, Hwnd)
}

$samples = [System.Collections.Generic.List[object]]::new()
$deadline = [DateTime]::UtcNow.AddSeconds($WatchSeconds)
do {
    foreach ($row in Get-FocusWindowRows) {
        [void]$samples.Add($row)
    }
    if ($WatchSeconds -gt 0 -and [DateTime]::UtcNow -lt $deadline) {
        Start-Sleep -Milliseconds $IntervalMs
    }
} while ($WatchSeconds -gt 0 -and [DateTime]::UtcNow -lt $deadline)

if ($AsJson -or $OutputPath) {
    # Serialize each row independently. Windows PowerShell can corrupt a
    # multi-line object array (notably with non-ASCII window titles) even when
    # ConvertTo-Json receives it as one input object.
    $items = foreach ($sample in $samples.ToArray()) {
        (@(ConvertTo-Json -InputObject $sample -Compress -Depth 3) -join "")
    }
    $json = "[`n" + ($items -join ",`n") + "`n]"
    if ($OutputPath) {
        [System.IO.File]::WriteAllText(
            [System.IO.Path]::GetFullPath($OutputPath),
            $json,
            [System.Text.UTF8Encoding]::new($false)
        )
    }
    if ($AsJson) { $json }
} else {
    $samples | Format-Table CapturedAt, Kind, Title, Class, Hwnd, Style, ExStyle, Visible, Foreground, OuterRect, ClientRect, DwmFrame -AutoSize
}
