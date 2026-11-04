param(
    [Parameter(Mandatory)] [ValidateSet('start','shot','key','click','close','wait')] [string]$Do,
    [string]$Arg = '',
    [string]$Proto = '',
    [int]$X = 0, [int]$Y = 0
)
# #1085 T5 real-window driver: disposable APPDATA, DX12 + dxc so GDI capture works.
$ErrorActionPreference = 'Stop'
$W = Split-Path -Parent $MyInvocation.MyCommand.Path
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public static class U {
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref POINT p);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, UIntPtr e);
  [DllImport("user32.dll")] public static extern void keybd_event(byte k, byte s, uint f, UIntPtr e);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
}
"@
function Get-App { Get-Process universe-app -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1 }
function Origin($p) { $pt = New-Object U+POINT; [U]::ClientToScreen($p.MainWindowHandle, [ref]$pt) | Out-Null; $pt }
switch ($Do) {
  'start' {
    $env:APPDATA = "$W\profile"; $env:WGPU_BACKEND = 'dx12'; $env:WGPU_DX12_COMPILER = 'dxc'
    if ($Proto) { $env:ASTROLITH_PROTOTYPE = $Proto } else { Remove-Item Env:ASTROLITH_PROTOTYPE -ErrorAction SilentlyContinue }
    $p = Start-Process "$W\bin\universe-app.exe" -WorkingDirectory "$W\bin" -PassThru -RedirectStandardError "$W\$Arg-stderr.txt" -RedirectStandardOutput "$W\$Arg-stdout.txt"
    for ($i = 0; $i -lt 60 -and -not (Get-App); $i++) { Start-Sleep 1 }
    "started pid=$($p.Id) window=$([bool](Get-App))"
  }
  'wait' { Start-Sleep ([int]$Arg); 'waited' }
  'shot' {
    $p = Get-App; [U]::SetForegroundWindow($p.MainWindowHandle) | Out-Null; Start-Sleep -Milliseconds 400
    $r = New-Object U+RECT; [U]::GetClientRect($p.MainWindowHandle, [ref]$r) | Out-Null; $o = Origin $p
    $bmp = New-Object System.Drawing.Bitmap ($r.R), ($r.B)
    $g = [System.Drawing.Graphics]::FromImage($bmp); $g.CopyFromScreen($o.X, $o.Y, 0, 0, $bmp.Size)
    $bmp.Save("$W\$Arg.png"); "shot $Arg $($r.R)x$($r.B)"
  }
  'key' {
    $p = Get-App; [U]::SetForegroundWindow($p.MainWindowHandle) | Out-Null; Start-Sleep -Milliseconds 200
    foreach ($k in $Arg.Split(',')) { $vk = [byte][int]$k; [U]::keybd_event($vk, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 120; [U]::keybd_event($vk, 0, 2, [UIntPtr]::Zero); Start-Sleep -Milliseconds 400 }
    "keys $Arg"
  }
  'click' {
    $p = Get-App; [U]::SetForegroundWindow($p.MainWindowHandle) | Out-Null; $o = Origin $p
    [U]::SetCursorPos($o.X + $X, $o.Y + $Y) | Out-Null; Start-Sleep -Milliseconds 300
    [U]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 150; [U]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
    "click $X,$Y"
  }
  'close' {
    $p = Get-App; $p.CloseMainWindow() | Out-Null
    for ($i = 0; $i -lt 30 -and (Get-Process -Id $p.Id -ErrorAction SilentlyContinue); $i++) { Start-Sleep 1 }
    "closed exited=$(-not (Get-Process -Id $p.Id -ErrorAction SilentlyContinue))"
  }
}
