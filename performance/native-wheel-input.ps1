param(
  [Parameter(Mandatory = $true)]
  [string]$WindowTitleToken,

  [string]$CaptureDirectory = ''
)

$ErrorActionPreference = 'Stop'

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class UrocissaNativeInput
{
    private const uint InputMouse = 0;
    private const uint MouseEventWheel = 0x0800;
    private const uint TouchFeedbackNone = 0x3;
    private const uint PointerFlagInRange = 0x2;
    private const uint PointerFlagInContact = 0x4;
    private const uint PointerFlagDown = 0x10000;
    private const uint PointerFlagUpdate = 0x20000;
    private const uint PointerFlagUp = 0x40000;
    private const uint TouchMaskContactArea = 0x1;
    private const uint TouchMaskOrientation = 0x2;
    private const uint TouchMaskPressure = 0x4;

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
    public struct Point
    {
        public int x;
        public int y;
    }

    [StructLayout(LayoutKind.Sequential)]
    public struct Rect
    {
        public int left;
        public int top;
        public int right;
        public int bottom;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct PointerInfo
    {
        public uint pointerType;
        public uint pointerId;
        public uint frameId;
        public uint pointerFlags;
        public IntPtr sourceDevice;
        public IntPtr hwndTarget;
        public Point ptPixelLocation;
        public Point ptHimetricLocation;
        public Point ptPixelLocationRaw;
        public Point ptHimetricLocationRaw;
        public uint dwTime;
        public uint historyCount;
        public int inputData;
        public uint dwKeyStates;
        public ulong performanceCount;
        public uint buttonChangeType;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct PointerTouchInfo
    {
        public PointerInfo pointerInfo;
        public uint touchFlags;
        public uint touchMask;
        public Rect rcContact;
        public Rect rcContactRaw;
        public uint orientation;
        public uint pressure;
    }

    private delegate bool EnumWindowsProc(IntPtr windowHandle, IntPtr parameter);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern uint SendInput(uint numberOfInputs, Input[] inputs, int inputSize);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool InitializeTouchInjection(uint maxCount, uint feedbackMode);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool InjectTouchInput(
        uint count,
        [In] ref PointerTouchInfo contact
    );

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool SetForegroundWindow(IntPtr windowHandle);

    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();

    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr windowHandle, out uint processId);

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

    [DllImport("user32.dll")]
    private static extern void keybd_event(byte virtualKey, byte scanCode, uint flags, UIntPtr extraInfo);

    [DllImport("user32.dll")]
    public static extern uint GetClipboardSequenceNumber();

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool EnumChildWindows(
        IntPtr parentWindow,
        EnumWindowsProc callback,
        IntPtr parameter
    );

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool IsWindowVisible(IntPtr windowHandle);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetClassName(
        IntPtr windowHandle,
        StringBuilder className,
        int maximumCount
    );

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool SetProcessDpiAwarenessContext(IntPtr dpiContext);

    public static void EnablePerMonitorDpiAwareness()
    {
        // DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2
        SetProcessDpiAwarenessContext(new IntPtr(-4));
    }

    public static void CaptureForegroundWindowToClipboard()
    {
        const byte VirtualKeyMenu = 0x12;
        const byte VirtualKeySnapshot = 0x2c;
        const uint KeyEventKeyUp = 0x2;
        keybd_event(VirtualKeyMenu, 0, 0, UIntPtr.Zero);
        keybd_event(VirtualKeySnapshot, 0, 0, UIntPtr.Zero);
        keybd_event(VirtualKeySnapshot, 0, KeyEventKeyUp, UIntPtr.Zero);
        keybd_event(VirtualKeyMenu, 0, KeyEventKeyUp, UIntPtr.Zero);
    }

    public static uint SendWheel(int delta)
    {
        Input[] inputs = new Input[1];
        inputs[0].type = InputMouse;
        inputs[0].mouseInput.mouseData = unchecked((uint)delta);
        inputs[0].mouseInput.dwFlags = MouseEventWheel;
        return SendInput(1, inputs, Marshal.SizeOf(typeof(Input)));
    }

    public static bool InitializeTouch()
    {
        return InitializeTouchInjection(1, TouchFeedbackNone);
    }

    public static bool SendTouch(int x, int y, string phase)
    {
        uint flags;
        switch (phase)
        {
            case "down":
                flags = PointerFlagDown | PointerFlagInRange | PointerFlagInContact;
                break;
            case "update":
                flags = PointerFlagUpdate | PointerFlagInRange | PointerFlagInContact;
                break;
            case "up":
                flags = PointerFlagUp;
                break;
            default:
                throw new ArgumentOutOfRangeException("phase");
        }

        PointerTouchInfo contact = new PointerTouchInfo();
        contact.pointerInfo.pointerType = 2; // PT_TOUCH
        // InitializeTouchInjection's single-contact device accepts contact id 0.
        // Using id 1 is rejected by InjectTouchInput with ERROR_INVALID_PARAMETER.
        contact.pointerInfo.pointerId = 0;
        contact.pointerInfo.pointerFlags = flags;
        contact.pointerInfo.ptPixelLocation.x = x;
        contact.pointerInfo.ptPixelLocation.y = y;
        contact.touchMask =
            TouchMaskContactArea | TouchMaskOrientation | TouchMaskPressure;
        contact.rcContact.left = x - 2;
        contact.rcContact.top = y - 2;
        contact.rcContact.right = x + 2;
        contact.rcContact.bottom = y + 2;
        contact.orientation = 0;
        contact.pressure = 0;
        return InjectTouchInput(1, ref contact);
    }

    public static int PointerInfoSize() { return Marshal.SizeOf(typeof(PointerInfo)); }
    public static int PointerTouchInfoSize() { return Marshal.SizeOf(typeof(PointerTouchInfo)); }

    public static bool GetLargestRenderSurfaceRect(IntPtr rootWindow, out Rect renderRect)
    {
        Rect bestRect = new Rect();
        long bestArea = 0;
        EnumChildWindows(rootWindow, delegate(IntPtr childWindow, IntPtr ignored)
        {
            if (!IsWindowVisible(childWindow)) return true;
            StringBuilder className = new StringBuilder(256);
            GetClassName(childWindow, className, className.Capacity);
            if (!className.ToString().Contains("Chrome_RenderWidgetHostHWND")) return true;

            Rect candidate;
            if (!GetWindowRect(childWindow, out candidate)) return true;
            long width = Math.Max(0, candidate.right - candidate.left);
            long height = Math.Max(0, candidate.bottom - candidate.top);
            long area = width * height;
            if (area > bestArea)
            {
                bestArea = area;
                bestRect = candidate;
            }
            return true;
        }, IntPtr.Zero);

        renderRect = bestRect;
        return bestArea > 0;
    }

    public static bool ActivateWindow(IntPtr windowHandle)
    {
        IntPtr foregroundWindow = GetForegroundWindow();
        if (foregroundWindow == windowHandle) return true;

        uint ignoredProcessId;
        uint currentThread = GetCurrentThreadId();
        uint foregroundThread = GetWindowThreadProcessId(foregroundWindow, out ignoredProcessId);
        uint targetThread = GetWindowThreadProcessId(windowHandle, out ignoredProcessId);
        bool attachedForeground = false;
        bool attachedTarget = false;

        try
        {
            if (foregroundThread != 0 && foregroundThread != currentThread)
                attachedForeground = AttachThreadInput(currentThread, foregroundThread, true);
            if (targetThread != 0 && targetThread != currentThread)
                attachedTarget = AttachThreadInput(currentThread, targetThread, true);

            ShowWindowAsync(windowHandle, 9);
            BringWindowToTop(windowHandle);
            SetForegroundWindow(windowHandle);
            SetFocus(windowHandle);
        }
        finally
        {
            if (attachedTarget) AttachThreadInput(currentThread, targetThread, false);
            if (attachedForeground) AttachThreadInput(currentThread, foregroundThread, false);
        }

        return GetForegroundWindow() == windowHandle;
    }
}
'@

[UrocissaNativeInput]::EnablePerMonitorDpiAwareness()

if ($CaptureDirectory -ne '') {
  Add-Type -AssemblyName System.Drawing
  Add-Type -AssemblyName System.Windows.Forms
  Add-Type -ReferencedAssemblies System.Drawing -TypeDefinition @'
using System;
using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;

public static class UrocissaImageMetrics
{
    public sealed class Difference
    {
        public double changedPixelRatio;
        public double meanChannelDifference;
        public int maximumChannelDifference;
    }

    public static Difference Compare(string baselinePath, string candidatePath)
    {
        return Compare(baselinePath, candidatePath, 0);
    }

    public static Difference Compare(string baselinePath, string candidatePath, int topInset)
    {
        using (Bitmap baseline = new Bitmap(baselinePath))
        using (Bitmap candidate = new Bitmap(candidatePath))
        {
            if (baseline.Width != candidate.Width || baseline.Height != candidate.Height)
                throw new ArgumentException("Capture dimensions do not match.");

            Rectangle bounds = new Rectangle(0, 0, baseline.Width, baseline.Height);
            BitmapData baselineData = baseline.LockBits(bounds, ImageLockMode.ReadOnly, PixelFormat.Format32bppArgb);
            BitmapData candidateData = candidate.LockBits(bounds, ImageLockMode.ReadOnly, PixelFormat.Format32bppArgb);
            try
            {
                int byteCount = Math.Abs(baselineData.Stride) * baseline.Height;
                byte[] left = new byte[byteCount];
                byte[] right = new byte[byteCount];
                Marshal.Copy(baselineData.Scan0, left, 0, byteCount);
                Marshal.Copy(candidateData.Scan0, right, 0, byteCount);
                long channelDifference = 0;
                long changedPixels = 0;
                int maximumDifference = 0;
                int firstRow = Math.Max(0, Math.Min(baseline.Height, topInset));
                long pixelCount = (long)baseline.Width * (baseline.Height - firstRow);

                for (int y = firstRow; y < baseline.Height; y++)
                {
                    int rowOffset = y * Math.Abs(baselineData.Stride);
                    for (int x = 0; x < baseline.Width; x++)
                    {
                        int offset = rowOffset + x * 4;
                        int pixelMaximum = 0;
                        for (int channel = 0; channel < 3; channel++)
                        {
                            int difference = Math.Abs(left[offset + channel] - right[offset + channel]);
                            channelDifference += difference;
                            pixelMaximum = Math.Max(pixelMaximum, difference);
                            maximumDifference = Math.Max(maximumDifference, difference);
                        }
                        if (pixelMaximum > 8) changedPixels++;
                    }
                }

                return new Difference
                {
                    changedPixelRatio = pixelCount == 0 ? 0 : (double)changedPixels / pixelCount,
                    meanChannelDifference = pixelCount == 0 ? 0 : (double)channelDifference / (pixelCount * 3),
                    maximumChannelDifference = maximumDifference
                };
            }
            finally
            {
                baseline.UnlockBits(baselineData);
                candidate.UnlockBits(candidateData);
            }
        }
    }
}
'@
  $CaptureDirectory = [IO.Path]::GetFullPath($CaptureDirectory)
  if (-not (Test-Path -LiteralPath $CaptureDirectory -PathType Container)) {
    throw "Capture directory does not exist: $CaptureDirectory"
  }
  $originalClipboard = [Windows.Forms.Clipboard]::GetDataObject()
}

function Write-JsonLine([hashtable]$Value) {
  [Console]::Out.WriteLine(($Value | ConvertTo-Json -Compress -Depth 6))
  [Console]::Out.Flush()
}

$touchGestureIndex = 0
$touchBaselinePath = $null
$authorityBaselinePath = $null
function Save-TouchScreenCapture($renderRect, [string]$phase, [int]$checkpointIndex) {
  if ($CaptureDirectory -eq '') { return $null }

  $width = $renderRect.right - $renderRect.left
  $height = $renderRect.bottom - $renderRect.top
  if ($width -le 0 -or $height -le 0) { throw 'Render surface has invalid dimensions.' }

  $fileName = "$WindowTitleToken-touch-$touchGestureIndex-$phase-$checkpointIndex.png"
  $path = Join-Path $CaptureDirectory $fileName
  $bitmap = New-Object Drawing.Bitmap(
    $width,
    $height,
    [Drawing.Imaging.PixelFormat]::Format32bppRgb
  )
  $graphics = [Drawing.Graphics]::FromImage($bitmap)
  try {
    $graphics.CopyFromScreen(
      $renderRect.left,
      $renderRect.top,
      0,
      0,
      (New-Object Drawing.Size($width, $height)),
      [Drawing.CopyPixelOperation]::SourceCopy
    )
    $bitmap.Save($path, [Drawing.Imaging.ImageFormat]::Png)
  } finally {
    $graphics.Dispose()
    $bitmap.Dispose()
  }
  return $path
}

function Get-VisualDifference([string]$candidatePath) {
  if ($null -eq $touchBaselinePath -or $null -eq $candidatePath) { return $null }
  return [UrocissaImageMetrics]::Compare($touchBaselinePath, $candidatePath)
}

function Save-AuthorityScreenCapture(
  $windowRect,
  $renderRect,
  [string]$phase,
  [int]$checkpointIndex
) {
  if ($CaptureDirectory -eq '') { return $null }

  $beforeSequence = [UrocissaNativeInput]::GetClipboardSequenceNumber()
  [UrocissaNativeInput]::CaptureForegroundWindowToClipboard()
  $deadline = [DateTime]::UtcNow.AddSeconds(2)
  while (
    [DateTime]::UtcNow -lt $deadline -and
    ([UrocissaNativeInput]::GetClipboardSequenceNumber() -eq $beforeSequence -or
      -not [Windows.Forms.Clipboard]::ContainsImage())
  ) {
    Start-Sleep -Milliseconds 25
  }
  if (-not [Windows.Forms.Clipboard]::ContainsImage()) {
    throw 'Alt+PrintScreen did not produce a composed-window image.'
  }

  $windowWidth = $windowRect.right - $windowRect.left
  $windowHeight = $windowRect.bottom - $windowRect.top
  $renderWidth = $renderRect.right - $renderRect.left
  $renderHeight = $renderRect.bottom - $renderRect.top
  $clipboardImage = [Windows.Forms.Clipboard]::GetImage()
  $scaleX = $clipboardImage.Width / $windowWidth
  $scaleY = $clipboardImage.Height / $windowHeight
  $sourceX = [Math]::Max(0, [Math]::Round(($renderRect.left - $windowRect.left) * $scaleX))
  $sourceY = [Math]::Max(0, [Math]::Round(($renderRect.top - $windowRect.top) * $scaleY))
  $sourceWidth = [Math]::Min(
    $clipboardImage.Width - $sourceX,
    [Math]::Round($renderWidth * $scaleX)
  )
  $sourceHeight = [Math]::Min(
    $clipboardImage.Height - $sourceY,
    [Math]::Round($renderHeight * $scaleY)
  )
  if ($sourceWidth -le 0 -or $sourceHeight -le 0) {
    $clipboardImage.Dispose()
    throw 'Composed-window capture does not contain the Chrome render surface.'
  }

  $fileName = "$WindowTitleToken-authority-$touchGestureIndex-$phase-$checkpointIndex.png"
  $path = Join-Path $CaptureDirectory $fileName
  $bitmap = New-Object Drawing.Bitmap($renderWidth, $renderHeight)
  $graphics = [Drawing.Graphics]::FromImage($bitmap)
  try {
    $graphics.DrawImage(
      $clipboardImage,
      (New-Object Drawing.Rectangle(0, 0, $renderWidth, $renderHeight)),
      (New-Object Drawing.Rectangle($sourceX, $sourceY, $sourceWidth, $sourceHeight)),
      [Drawing.GraphicsUnit]::Pixel
    )
    $bitmap.Save($path, [Drawing.Imaging.ImageFormat]::Png)
  } finally {
    $graphics.Dispose()
    $bitmap.Dispose()
    $clipboardImage.Dispose()
  }
  return $path
}

function Get-AuthorityDifference([string]$candidatePath) {
  if ($null -eq $authorityBaselinePath -or $null -eq $candidatePath) { return $null }
  return [UrocissaImageMetrics]::Compare($authorityBaselinePath, $candidatePath, 80)
}

function Get-TargetGeometry {
  $process = Get-Process -Id $chromeWindow.Id -ErrorAction Stop
  if (
    $process.MainWindowHandle -eq [IntPtr]::Zero -or
    $process.MainWindowHandle -ne $windowHandle -or
    -not $process.MainWindowTitle.Contains($WindowTitleToken)
  ) {
    throw "Chrome target identity changed for PID $($chromeWindow.Id)."
  }

  [uint32]$windowPid = 0
  $null = [UrocissaNativeInput]::GetWindowThreadProcessId($windowHandle, [ref]$windowPid)
  if ($windowPid -ne $chromeWindow.Id) {
    throw "Chrome window PID changed from $($chromeWindow.Id) to $windowPid."
  }

  $windowRect = New-Object UrocissaNativeInput+Rect
  if (-not [UrocissaNativeInput]::GetWindowRect($windowHandle, [ref]$windowRect)) {
    throw "GetWindowRect failed for Chrome PID $($chromeWindow.Id)."
  }
  $renderRect = New-Object UrocissaNativeInput+Rect
  if (-not [UrocissaNativeInput]::GetLargestRenderSurfaceRect($windowHandle, [ref]$renderRect)) {
    $renderRect = $windowRect
  }

  return [ordered]@{
    process = $process
    windowRect = $windowRect
    renderRect = $renderRect
  }
}

function Activate-Target {
  $alreadyForeground = [UrocissaNativeInput]::GetForegroundWindow() -eq $windowHandle
  if ($alreadyForeground) { return $true }

  $null = $windowActivator.AppActivate($chromeWindow.Id)
  Start-Sleep -Milliseconds 50
  $activated = [UrocissaNativeInput]::ActivateWindow($windowHandle)
  Start-Sleep -Milliseconds 20
  return $activated
}

function Get-ScreenPoint($renderRect, [double]$xRatio, [double]$yRatio) {
  if ($xRatio -lt 0 -or $xRatio -gt 1 -or $yRatio -lt 0 -or $yRatio -gt 1) {
    throw 'Input coordinate ratios must be between 0 and 1.'
  }
  return [ordered]@{
    x = [Math]::Round($renderRect.left + ($renderRect.right - $renderRect.left) * $xRatio)
    y = [Math]::Round($renderRect.top + ($renderRect.bottom - $renderRect.top) * $yRatio)
  }
}

$deadline = [DateTime]::UtcNow.AddSeconds(15)
$chromeWindow = $null
do {
  $chromeWindow = Get-Process -Name chrome -ErrorAction SilentlyContinue |
    Where-Object {
      $_.MainWindowHandle -ne [IntPtr]::Zero -and
      $_.MainWindowTitle.Contains($WindowTitleToken)
    } |
    Select-Object -First 1

  if ($null -eq $chromeWindow) { Start-Sleep -Milliseconds 100 }
} while ($null -eq $chromeWindow -and [DateTime]::UtcNow -lt $deadline)

if ($null -eq $chromeWindow) {
  throw "Could not find the isolated Chrome window '$WindowTitleToken'."
}

$windowHandle = $chromeWindow.MainWindowHandle
$windowActivator = New-Object -ComObject WScript.Shell
$geometry = Get-TargetGeometry
$renderCenter = Get-ScreenPoint $geometry.renderRect 0.5 0.5
$touchInitialized = [UrocissaNativeInput]::InitializeTouch()
if (-not $touchInitialized) {
  $lastError = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
  throw "InitializeTouchInjection failed with Win32 error $lastError."
}

Write-JsonLine ([ordered]@{
  type = 'ready'
  chromePid = $chromeWindow.Id
  windowTitle = $chromeWindow.MainWindowTitle
  cursorX = $renderCenter.x
  cursorY = $renderCenter.y
  renderRect = $geometry.renderRect
  windowRect = $geometry.windowRect
  touchInitialized = $touchInitialized
  pointerInfoSize = [UrocissaNativeInput]::PointerInfoSize()
  pointerTouchInfoSize = [UrocissaNativeInput]::PointerTouchInfoSize()
})

$invariantCulture = [Globalization.CultureInfo]::InvariantCulture
try {
while ($null -ne ($line = [Console]::In.ReadLine())) {
  $parts = $line.Trim().Split(' ', [System.StringSplitOptions]::RemoveEmptyEntries)
  if ($parts.Count -eq 0) { continue }
  if ($parts[0] -eq 'quit') { break }

  $geometry = Get-TargetGeometry
  $activated = Activate-Target
  $foreground = [UrocissaNativeInput]::GetForegroundWindow() -eq $windowHandle
  if (-not $foreground) {
    Write-JsonLine ([ordered]@{
      type = 'input-refused'
      command = $parts[0]
      activated = $activated
      foreground = $false
      timestampUtc = [DateTime]::UtcNow.ToString('O')
    })
    continue
  }

  if ($parts[0] -eq 'move' -and $parts.Count -eq 3) {
    $xRatio = [double]::Parse($parts[1], $invariantCulture)
    $yRatio = [double]::Parse($parts[2], $invariantCulture)
    $point = Get-ScreenPoint $geometry.renderRect $xRatio $yRatio
    $moved = [UrocissaNativeInput]::SetCursorPos($point.x, $point.y)
    Write-JsonLine ([ordered]@{
      type = 'move'
      moved = $moved
      foreground = $foreground
      x = $point.x
      y = $point.y
      xRatio = $xRatio
      yRatio = $yRatio
      renderRect = $geometry.renderRect
      timestampUtc = [DateTime]::UtcNow.ToString('O')
    })
    continue
  }

  if ($parts[0] -eq 'wheel' -and ($parts.Count -eq 2 -or $parts.Count -eq 4)) {
    $delta = [int]$parts[1]
    $xRatio = if ($parts.Count -eq 4) { [double]::Parse($parts[2], $invariantCulture) } else { 0.5 }
    $yRatio = if ($parts.Count -eq 4) { [double]::Parse($parts[3], $invariantCulture) } else { 0.5 }
    $point = Get-ScreenPoint $geometry.renderRect $xRatio $yRatio
    if (-not [UrocissaNativeInput]::SetCursorPos($point.x, $point.y)) {
      throw 'SetCursorPos failed.'
    }
    $sent = [UrocissaNativeInput]::SendWheel($delta)
    Write-JsonLine ([ordered]@{
      type = 'wheel'
      delta = $delta
      sent = $sent
      activated = $activated
      foreground = $foreground
      x = $point.x
      y = $point.y
      timestampUtc = [DateTime]::UtcNow.ToString('O')
    })
    continue
  }

  if ($parts[0] -eq 'capture' -and $parts.Count -eq 2) {
    $label = $parts[1]
    if ($label -notmatch '^[A-Za-z0-9_-]+$') {
      throw 'Capture label contains unsupported characters.'
    }
    $capturePath = Save-TouchScreenCapture $geometry.renderRect $label 0
    if ($label -eq 'baseline') { $touchBaselinePath = $capturePath }
    $visualDifference = Get-VisualDifference $capturePath
    $authorityCapturePath = Save-AuthorityScreenCapture $geometry.windowRect $geometry.renderRect $label 0
    if ($label -eq 'baseline') { $authorityBaselinePath = $authorityCapturePath }
    $authorityVisualDifference = Get-AuthorityDifference $authorityCapturePath
    Write-JsonLine ([ordered]@{
      type = 'capture'
      label = $label
      screenCapturePath = $capturePath
      visualDifference = $visualDifference
      authorityScreenCapturePath = $authorityCapturePath
      authorityVisualDifference = $authorityVisualDifference
      foreground = $foreground
      timestampUtc = [DateTime]::UtcNow.ToString('O')
    })
    continue
  }

  if ($parts[0] -eq 'touch' -and $parts.Count -eq 6) {
    $touchGestureIndex += 1
    $xRatio = [double]::Parse($parts[1], $invariantCulture)
    $startYRatio = [double]::Parse($parts[2], $invariantCulture)
    $endYRatio = [double]::Parse($parts[3], $invariantCulture)
    $durationMs = [int]$parts[4]
    $steps = [int]$parts[5]
    if ($durationMs -le 0 -or $steps -le 0) {
      throw 'Touch duration and steps must be positive.'
    }

    $startPoint = Get-ScreenPoint $geometry.renderRect $xRatio $startYRatio
    $endPoint = Get-ScreenPoint $geometry.renderRect $xRatio $endYRatio
    $injectedCount = 0
    $downInjected = [UrocissaNativeInput]::SendTouch($startPoint.x, $startPoint.y, 'down')
    $downLastError = if ($downInjected) { 0 } else { [Runtime.InteropServices.Marshal]::GetLastWin32Error() }
    if ($downInjected) { $injectedCount += 1 }
    $downCapturePath = Save-TouchScreenCapture $geometry.renderRect 'down' 0
    $downVisualDifference = Get-VisualDifference $downCapturePath
    Write-JsonLine ([ordered]@{
      type = 'touch-checkpoint'
      phase = 'down'
      index = 0
      steps = $steps
      injected = $downInjected
      lastWin32Error = $downLastError
      x = $startPoint.x
      y = $startPoint.y
      screenCapturePath = $downCapturePath
      visualDifference = $downVisualDifference
      timestampUtc = [DateTime]::UtcNow.ToString('O')
    })
    $stepDelayMs = [Math]::Max(1, [Math]::Floor($durationMs / $steps))
    for ($step = 1; $step -le $steps; $step += 1) {
      if ([UrocissaNativeInput]::GetForegroundWindow() -ne $windowHandle) {
        throw "Chrome lost foreground during touch checkpoint $step."
      }
      Start-Sleep -Milliseconds $stepDelayMs
      $progress = $step / $steps
      $x = [Math]::Round($startPoint.x + ($endPoint.x - $startPoint.x) * $progress)
      $y = [Math]::Round($startPoint.y + ($endPoint.y - $startPoint.y) * $progress)
      $moveInjected = [UrocissaNativeInput]::SendTouch($x, $y, 'update')
      $moveLastError = if ($moveInjected) { 0 } else { [Runtime.InteropServices.Marshal]::GetLastWin32Error() }
      if ($moveInjected) { $injectedCount += 1 }
      $moveCapturePath = Save-TouchScreenCapture $geometry.renderRect 'update' $step
      $moveVisualDifference = Get-VisualDifference $moveCapturePath
      $authorityCapturePath = $null
      $authorityVisualDifference = $null
      if ($step -eq $steps) {
        $authorityCapturePath = Save-AuthorityScreenCapture $geometry.windowRect $geometry.renderRect 'peak' $step
        $authorityVisualDifference = Get-AuthorityDifference $authorityCapturePath
      }
      Write-JsonLine ([ordered]@{
        type = 'touch-checkpoint'
        phase = 'update'
        index = $step
        steps = $steps
        injected = $moveInjected
        lastWin32Error = $moveLastError
        x = $x
        y = $y
        screenCapturePath = $moveCapturePath
        visualDifference = $moveVisualDifference
        authorityScreenCapturePath = $authorityCapturePath
        authorityVisualDifference = $authorityVisualDifference
        timestampUtc = [DateTime]::UtcNow.ToString('O')
      })
    }

    $upInjected = [UrocissaNativeInput]::SendTouch($endPoint.x, $endPoint.y, 'up')
    $upLastError = if ($upInjected) { 0 } else { [Runtime.InteropServices.Marshal]::GetLastWin32Error() }
    if ($upInjected) { $injectedCount += 1 }
    $upCapturePath = Save-TouchScreenCapture $geometry.renderRect 'up' ($steps + 1)
    $upVisualDifference = Get-VisualDifference $upCapturePath
    Write-JsonLine ([ordered]@{
      type = 'touch-checkpoint'
      phase = 'up'
      index = $steps + 1
      steps = $steps
      injected = $upInjected
      lastWin32Error = $upLastError
      x = $endPoint.x
      y = $endPoint.y
      screenCapturePath = $upCapturePath
      visualDifference = $upVisualDifference
      timestampUtc = [DateTime]::UtcNow.ToString('O')
    })
    Write-JsonLine ([ordered]@{
      type = 'touch-complete'
      injectedCount = $injectedCount
      expectedCount = $steps + 2
      foreground = [UrocissaNativeInput]::GetForegroundWindow() -eq $windowHandle
      start = $startPoint
      end = $endPoint
      durationMs = $durationMs
      steps = $steps
      timestampUtc = [DateTime]::UtcNow.ToString('O')
    })
    continue
  }

  throw "Unknown native input command: $line"
}
} finally {
  if ($CaptureDirectory -ne '' -and $null -ne $originalClipboard) {
    [Windows.Forms.Clipboard]::SetDataObject($originalClipboard, $true)
  }
}
