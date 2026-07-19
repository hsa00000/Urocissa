param(
  [Parameter(Mandatory = $true)]
  [string]$WindowTitleToken
)

$ErrorActionPreference = 'Stop'

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

public static class UrocissaNativeWheel
{
    private const uint InputMouse = 0;
    private const uint MouseEventWheel = 0x0800;

    [StructLayout(LayoutKind.Sequential)]
    private struct MouseInput
    {
        public int dx;
        public int dy;
        public uint mouseData;
        public uint dwFlags;
        public uint time;
        public UIntPtr dwExtraInfo;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct Input
    {
        public uint type;
        public MouseInput mouseInput;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct Rect
    {
        public int left;
        public int top;
        public int right;
        public int bottom;
    }

    [DllImport("user32.dll", SetLastError = true)]
    private static extern uint SendInput(
        uint numberOfInputs,
        Input[] inputs,
        int inputSize
    );

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool SetForegroundWindow(IntPtr windowHandle);

    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr windowHandle, out uint processId);

    [DllImport("kernel32.dll")]
    private static extern uint GetCurrentThreadId();

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool AttachThreadInput(uint attachThread, uint attachToThread, bool attach);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool BringWindowToTop(IntPtr windowHandle);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool ShowWindowAsync(IntPtr windowHandle, int command);

    [DllImport("user32.dll")]
    private static extern IntPtr SetFocus(IntPtr windowHandle);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool GetWindowRect(IntPtr windowHandle, out Rect rect);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool SetCursorPos(int x, int y);

    public static uint SendWheel(int delta)
    {
        Input[] inputs = new Input[1];
        inputs[0].type = InputMouse;
        inputs[0].mouseInput.mouseData = unchecked((uint)delta);
        inputs[0].mouseInput.dwFlags = MouseEventWheel;
        return SendInput(1, inputs, Marshal.SizeOf(typeof(Input)));
    }

    public static bool ActivateWindow(IntPtr windowHandle)
    {
        IntPtr foregroundWindow = GetForegroundWindow();
        if (foregroundWindow == windowHandle)
        {
            return true;
        }

        uint ignoredProcessId;
        uint currentThread = GetCurrentThreadId();
        uint foregroundThread = GetWindowThreadProcessId(foregroundWindow, out ignoredProcessId);
        uint targetThread = GetWindowThreadProcessId(windowHandle, out ignoredProcessId);
        bool attachedForeground = false;
        bool attachedTarget = false;

        try
        {
            if (foregroundThread != 0 && foregroundThread != currentThread)
            {
                attachedForeground = AttachThreadInput(currentThread, foregroundThread, true);
            }
            if (targetThread != 0 && targetThread != currentThread)
            {
                attachedTarget = AttachThreadInput(currentThread, targetThread, true);
            }

            ShowWindowAsync(windowHandle, 9);
            BringWindowToTop(windowHandle);
            SetForegroundWindow(windowHandle);
            SetFocus(windowHandle);
        }
        finally
        {
            if (attachedTarget)
            {
                AttachThreadInput(currentThread, targetThread, false);
            }
            if (attachedForeground)
            {
                AttachThreadInput(currentThread, foregroundThread, false);
            }
        }

        return GetForegroundWindow() == windowHandle;
    }
}
'@

$deadline = [DateTime]::UtcNow.AddSeconds(15)
$chromeWindow = $null
do {
  $chromeWindow = Get-Process -Name chrome -ErrorAction SilentlyContinue |
    Where-Object {
      $_.MainWindowHandle -ne [IntPtr]::Zero -and
      $_.MainWindowTitle.Contains($WindowTitleToken)
    } |
    Select-Object -First 1

  if ($null -eq $chromeWindow) {
    Start-Sleep -Milliseconds 100
  }
} while ($null -eq $chromeWindow -and [DateTime]::UtcNow -lt $deadline)

if ($null -eq $chromeWindow) {
  throw "Could not find the isolated Chrome window '$WindowTitleToken'."
}

$windowHandle = $chromeWindow.MainWindowHandle
$windowActivator = New-Object -ComObject WScript.Shell
$windowRect = New-Object UrocissaNativeWheel+Rect
if (-not [UrocissaNativeWheel]::GetWindowRect($windowHandle, [ref]$windowRect)) {
  throw "GetWindowRect failed for Chrome PID $($chromeWindow.Id)."
}

$cursorX = [Math]::Floor(($windowRect.left + $windowRect.right) / 2)
$cursorY = [Math]::Floor(($windowRect.top + $windowRect.bottom) / 2)

$ready = [ordered]@{
  type = 'ready'
  chromePid = $chromeWindow.Id
  windowTitle = $chromeWindow.MainWindowTitle
  cursorX = $cursorX
  cursorY = $cursorY
}
[Console]::Out.WriteLine(($ready | ConvertTo-Json -Compress))
[Console]::Out.Flush()

while ($null -ne ($line = [Console]::In.ReadLine())) {
  $parts = $line.Trim().Split(' ', [System.StringSplitOptions]::RemoveEmptyEntries)
  if ($parts.Count -eq 0) {
    continue
  }
  if ($parts[0] -eq 'quit') {
    break
  }
  if ($parts[0] -ne 'wheel' -or $parts.Count -ne 2) {
    throw "Unknown native input command: $line"
  }

  $delta = [int]$parts[1]
  $alreadyForeground = [UrocissaNativeWheel]::GetForegroundWindow() -eq $windowHandle
  if ($alreadyForeground) {
    $activated = $true
  } else {
    $null = $windowActivator.AppActivate($chromeWindow.Id)
    Start-Sleep -Milliseconds 50
    $activated = [UrocissaNativeWheel]::ActivateWindow($windowHandle)
    Start-Sleep -Milliseconds 20
  }
  $foreground = [UrocissaNativeWheel]::GetForegroundWindow() -eq $windowHandle
  if (-not $foreground) {
    $result = [ordered]@{
      type = 'wheel'
      delta = $delta
      sent = 0
      activated = $activated
      foreground = $false
      timestampUtc = [DateTime]::UtcNow.ToString('O')
    }
    [Console]::Out.WriteLine(($result | ConvertTo-Json -Compress))
    [Console]::Out.Flush()
    continue
  }
  if (-not [UrocissaNativeWheel]::SetCursorPos($cursorX, $cursorY)) {
    throw 'SetCursorPos failed.'
  }
  $sent = [UrocissaNativeWheel]::SendWheel($delta)

  $result = [ordered]@{
    type = 'wheel'
    delta = $delta
    sent = $sent
    activated = $activated
    foreground = $foreground
    timestampUtc = [DateTime]::UtcNow.ToString('O')
  }
  [Console]::Out.WriteLine(($result | ConvertTo-Json -Compress))
  [Console]::Out.Flush()
}
