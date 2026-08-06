[CmdletBinding()]
param(
  [ValidateSet('Quick', 'Full')]
  [string]$Profile = 'Quick',
  [string]$Url = 'http://localhost:5173',
  [string]$BackendUrl = 'http://127.0.0.1:5673'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if ($PSVersionTable.PSVersion.Major -lt 7) {
  throw 'The hybrid scroll gate requires PowerShell 7 or newer.'
}
if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
  $PSNativeCommandUseErrorActionPreference = $false
}

$performanceRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$repositoryRoot = Split-Path -Parent $performanceRoot
$frontendRoot = Join-Path $repositoryRoot 'gallery-frontend'
$timestamp = [DateTimeOffset]::UtcNow.ToString('yyyyMMdd-HHmmssfff')
$runRoot = Join-Path $performanceRoot ".performance\hybrid-scroll\$timestamp-$($Profile.ToLowerInvariant())"
$logsRoot = Join-Path $runRoot 'logs'
$scenariosRoot = Join-Path $runRoot 'scenarios'
$null = New-Item -ItemType Directory -Force -Path $logsRoot, $scenariosRoot

$contract = [ordered]@{
  coordinateInvariant = 'V = p - O'
  behaviorTolerancePx = 1
  handoffProjectionResidualBudgetPx = 1
  handoffFrameGapBudgetMs = 25
  inputToFirstVisualMotionBudgetMs = 25
  scrollInteractionWorkBudgetMsPerEvent = 0.7
  transitionInternalWriteCount = 1
  nativeBoundaryInternalWriteCount = 0
  truncatedTransitionPulsePolicy = 'advisory; the next same-direction control pulse must be complete'
}

function Get-SafeName {
  param([Parameter(Mandatory)][string]$Value)
  return ($Value -replace '[^a-zA-Z0-9.-]+', '-').Trim('-')
}

function Assert-NativeCommand {
  param([Parameter(Mandatory)][string]$Name)
  $command = Get-Command $Name -ErrorAction SilentlyContinue
  if ($null -eq $command) {
    throw "Required command '$Name' was not found. The gate never installs dependencies."
  }
  return $command.Source
}

function Find-ChromeExecutable {
  $candidates = [System.Collections.Generic.List[string]]::new()
  foreach ($registryPath in @(
      'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\chrome.exe',
      'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\chrome.exe'
    )) {
    $key = Get-Item -LiteralPath $registryPath -ErrorAction SilentlyContinue
    if ($null -ne $key) {
      $value = $key.GetValue('')
      if ($value -is [string]) { $candidates.Add($value) }
    }
  }
  foreach ($candidate in @(
      (Join-Path $env:ProgramFiles 'Google\Chrome\Application\chrome.exe'),
      (Join-Path ${env:ProgramFiles(x86)} 'Google\Chrome\Application\chrome.exe'),
      (Join-Path $env:LOCALAPPDATA 'Google\Chrome\Application\chrome.exe')
    )) {
    if (-not [string]::IsNullOrWhiteSpace($candidate)) { $candidates.Add($candidate) }
  }
  return $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
}

function Assert-TcpEndpoint {
  param([Parameter(Mandatory)][uri]$Endpoint)
  $port = if ($Endpoint.IsDefaultPort) {
    if ($Endpoint.Scheme -eq 'https') { 443 } else { 80 }
  } else {
    $Endpoint.Port
  }
  $client = [System.Net.Sockets.TcpClient]::new()
  try {
    $connect = $client.ConnectAsync($Endpoint.Host, $port)
    if (-not $connect.Wait([TimeSpan]::FromSeconds(5))) {
      throw "Timed out connecting to $($Endpoint.Host):$port."
    }
    if (-not $client.Connected) {
      throw "Could not connect to $($Endpoint.Host):$port."
    }
  } finally {
    $client.Dispose()
  }
}

function Get-MaximumNumber {
  param([object[]]$Values)
  $numbers = @($Values | Where-Object { $_ -is [byte] -or $_ -is [int] -or $_ -is [long] -or $_ -is [double] -or $_ -is [decimal] })
  if ($numbers.Count -eq 0) { return $null }
  return ($numbers | Measure-Object -Maximum).Maximum
}

function Get-SumNumber {
  param([object[]]$Values)
  $numbers = @($Values | Where-Object { $_ -is [byte] -or $_ -is [int] -or $_ -is [long] -or $_ -is [double] -or $_ -is [decimal] })
  if ($numbers.Count -eq 0) { return $null }
  return ($numbers | Measure-Object -Sum).Sum
}

if (-not $IsWindows) {
  throw 'The trusted wheel/touch gate requires Windows.'
}
if (-not [Environment]::UserInteractive) {
  throw 'The gate requires an interactive Windows desktop.'
}
if ([string]::IsNullOrWhiteSpace($env:UROCISSA_PASSWORD)) {
  throw 'Set UROCISSA_PASSWORD in the current shell. It is never written to reports.'
}

$currentSessionId = [System.Diagnostics.Process]::GetCurrentProcess().SessionId
$sessionExplorer = Get-Process explorer -ErrorAction SilentlyContinue |
  Where-Object { $_.SessionId -eq $currentSessionId } |
  Select-Object -First 1
if ($null -eq $sessionExplorer) {
  throw "No Explorer process was found in interactive session $currentSessionId."
}

$nodeCommand = Assert-NativeCommand 'node.exe'
$npmCommand = Assert-NativeCommand 'npm.cmd'
$gitCommand = Assert-NativeCommand 'git.exe'
$chromeExecutable = Find-ChromeExecutable
if ([string]::IsNullOrWhiteSpace($chromeExecutable)) {
  throw 'Google Chrome was not found. The gate does not install a browser.'
}
if (-not (Test-Path -LiteralPath (Join-Path $frontendRoot 'node_modules'))) {
  throw 'gallery-frontend/node_modules is missing. Install dependencies intentionally before running the gate.'
}
if (-not (Test-Path -LiteralPath (Join-Path $performanceRoot 'node_modules\playwright'))) {
  throw 'performance/node_modules/playwright is missing. Install performance dependencies intentionally first.'
}

$frontendUri = [uri]$Url
$backendUri = [uri]$BackendUrl
Assert-TcpEndpoint $backendUri
$null = Invoke-WebRequest -Uri $frontendUri -Method Get -TimeoutSec 5 -UseBasicParsing
$authBody = ConvertTo-Json -InputObject $env:UROCISSA_PASSWORD -Compress
$authToken = Invoke-RestMethod -Uri ([uri]::new($frontendUri, '/post/authenticate')) `
  -Method Post `
  -ContentType 'application/json' `
  -Body $authBody `
  -TimeoutSec 10
if ($authToken -isnot [string] -or [string]::IsNullOrWhiteSpace($authToken)) {
  throw 'Authentication preflight did not return a JWT string.'
}
$authToken = $null
$authBody = $null

$gitSha = (& $gitCommand -C $repositoryRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Unable to read the current Git commit.' }
$gitStatus = @(& $gitCommand -C $repositoryRoot status --porcelain)
if ($LASTEXITCODE -ne 0) { throw 'Unable to read the current Git worktree status.' }
$nodeVersion = (& $nodeCommand --version).Trim()
$npmVersion = (& $npmCommand --version).Trim()

$refreshRateHz = $null
try {
  $refreshRateHz = Get-CimInstance Win32_VideoController |
    ForEach-Object { $_.CurrentRefreshRate } |
    Where-Object { $_ -is [uint32] -and $_ -gt 0 -and $_ -lt 1000 } |
    Select-Object -First 1
} catch {
  $refreshRateHz = $null
}

Add-Type -AssemblyName System.Windows.Forms
if ($null -eq ('UrocissaHybridGateDpi' -as [type])) {
  Add-Type -TypeDefinition @'
using System.Runtime.InteropServices;
public static class UrocissaHybridGateDpi {
  [DllImport("user32.dll")]
  public static extern uint GetDpiForSystem();
}
'@
}
$primaryBounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$environment = [ordered]@{
  operatingSystem = [Environment]::OSVersion.VersionString
  userInteractive = [Environment]::UserInteractive
  sessionId = $currentSessionId
  gitSha = $gitSha
  gitDirty = $gitStatus.Count -gt 0
  nodeVersion = $nodeVersion
  npmVersion = $npmVersion
  chromeExecutable = $chromeExecutable
  frontendUrl = $frontendUri.AbsoluteUri.TrimEnd('/')
  backendUrl = $backendUri.AbsoluteUri.TrimEnd('/')
  screen = [ordered]@{
    width = $primaryBounds.Width
    height = $primaryBounds.Height
    systemDpi = [UrocissaHybridGateDpi]::GetDpiForSystem()
    refreshRateHz = $refreshRateHz
  }
}

$stageResults = [System.Collections.Generic.List[object]]::new()
$scenarioResults = [System.Collections.Generic.List[object]]::new()

function Invoke-RecordedCommand {
  param(
    [Parameter(Mandatory)][string]$Name,
    [Parameter(Mandatory)][string]$WorkingDirectory,
    [Parameter(Mandatory)][string]$FilePath,
    [Parameter(Mandatory)][string[]]$Arguments
  )
  $safeName = Get-SafeName $Name
  $logPath = Join-Path $logsRoot "$safeName.log"
  $startedAt = [DateTimeOffset]::UtcNow
  $exitCode = 1
  Write-Host "[$Name] starting"
  Push-Location $WorkingDirectory
  try {
    & $FilePath @Arguments 2>&1 |
      Tee-Object -FilePath $logPath |
      ForEach-Object { Write-Host $_ }
    $exitCode = $LASTEXITCODE
  } catch {
    $message = $_.Exception.Message
    [System.IO.File]::AppendAllText($logPath, "$message$([Environment]::NewLine)")
    Write-Host "[$Name] $message"
    $exitCode = 1
  } finally {
    Pop-Location
  }
  $finishedAt = [DateTimeOffset]::UtcNow
  $result = [ordered]@{
    name = $Name
    status = if ($exitCode -eq 0) { 'passed' } else { 'failed' }
    exitCode = $exitCode
    startedAt = $startedAt.ToString('O')
    durationMs = [Math]::Round(($finishedAt - $startedAt).TotalMilliseconds, 3)
    logPath = $logPath
    command = [ordered]@{
      executable = $FilePath
      arguments = $Arguments
      workingDirectory = $WorkingDirectory
    }
  }
  $stageResults.Add($result)
  Write-Host "[$Name] $($result.status)"
  return $result
}

function Invoke-ScrollScenario {
  param([Parameter(Mandatory)][System.Collections.IDictionary]$Spec)
  $scenarioRoot = Join-Path $scenariosRoot $Spec.Name
  $checkpointRoot = Join-Path $scenarioRoot 'checkpoints'
  $traceRoot = Join-Path $scenarioRoot 'traces'
  $reportPath = Join-Path $scenarioRoot 'report.json'
  $null = New-Item -ItemType Directory -Force -Path $scenarioRoot, $checkpointRoot, $traceRoot

  $arguments = @(
    (Join-Path $performanceRoot 'scroll-lag.mjs'),
    '--url', $frontendUri.AbsoluteUri.TrimEnd('/'),
    '--browser', 'chrome',
    '--scenario', $Spec.Scenario,
    '--samples', [string]$Spec.Samples,
    '--viewport-width', [string]$Spec.Width,
    '--viewport-height', [string]$Spec.Height,
    '--theme', $Spec.Theme,
    '--expect', 'strict-smooth',
    '--behavior-tolerance', [string]$contract.behaviorTolerancePx,
    '--scroll-work-per-pulse-budget', [string]$contract.scrollInteractionWorkBudgetMsPerEvent,
    '--handoff-frame-gap-budget', [string]$contract.handoffFrameGapBudgetMs,
    '--input-to-first-motion-budget', [string]$contract.inputToFirstVisualMotionBudgetMs,
    '--checkpoint-dir', $checkpointRoot,
    '--trace-dir', $traceRoot,
    '--output', $reportPath,
    '--quiet'
  )
  if ($Spec.Headed) { $arguments += '--headed' }

  $stage = Invoke-RecordedCommand `
    -Name "scenario-$($Spec.Name)" `
    -WorkingDirectory $repositoryRoot `
    -FilePath $nodeCommand `
    -Arguments $arguments

  $summary = [ordered]@{
    name = $Spec.Name
    scenario = $Spec.Scenario
    theme = $Spec.Theme
    samples = $Spec.Samples
    status = $stage.status
    exitCode = $stage.exitCode
    reportPath = if (Test-Path -LiteralPath $reportPath) { $reportPath } else { $null }
    browserVersion = $null
    behaviorEquivalentSamples = $null
    jankySamples = $null
    maximumFrameGapMs = $null
    maximumScrollInteractionWorkPerEventMs = $null
    maximumHandoffProjectionResidualPx = $null
    maximumHandoffFrameGapMs = $null
    maximumInputToFirstVisualMotionMs = $null
    truncatedHandoffPulseCount = $null
  }

  if (Test-Path -LiteralPath $reportPath) {
    $report = Get-Content -Raw -LiteralPath $reportPath | ConvertFrom-Json -AsHashtable
    $samples = @($report['samples'])
    $summary.browserVersion = $report['browserVersion']
    $summary.behaviorEquivalentSamples = $report['aggregate']['behaviorEquivalentSamples']
    $summary.jankySamples = $report['aggregate']['jankySamples']
    $summary.maximumFrameGapMs = Get-MaximumNumber @(
      $samples | ForEach-Object { $_['frameGapMaxMs'] }
    )
    $summary.maximumScrollInteractionWorkPerEventMs = Get-MaximumNumber @(
      $samples | ForEach-Object { $_['scrollInteractionWorkPerEventMs'] }
    )
    $summary.maximumHandoffProjectionResidualPx = Get-MaximumNumber @(
      $samples | ForEach-Object { $_['scenarioDetails']['maximumHandoffProjectionResidualPx'] }
    )
    $summary.maximumHandoffFrameGapMs = Get-MaximumNumber @(
      $samples | ForEach-Object { $_['scenarioDetails']['maximumHandoffFrameGapMs'] }
    )
    $summary.maximumInputToFirstVisualMotionMs = Get-MaximumNumber @(
      $samples | ForEach-Object { $_['scenarioDetails']['maximumInputToFirstVisualMotionMs'] }
    )
    $summary.truncatedHandoffPulseCount = Get-SumNumber @(
      $samples | ForEach-Object { $_['scenarioDetails']['truncatedHandoffPulseCount'] }
    )
  }
  $scenarioResults.Add($summary)
}

$targetedTests = @(
  'src/script/hook/useHandleScroll.test.ts',
  'src/script/hook/useUpdateVisibleRows.test.ts',
  'src/script/utils/rowOffset.test.ts'
)

$null = Invoke-RecordedCommand `
  -Name 'frontend-check' `
  -WorkingDirectory $frontendRoot `
  -FilePath $npmCommand `
  -Arguments @('run', 'check')
$null = Invoke-RecordedCommand `
  -Name 'hybrid-metrics-tests' `
  -WorkingDirectory $repositoryRoot `
  -FilePath $nodeCommand `
  -Arguments @('--test', (Join-Path $performanceRoot 'hybrid-scroll-metrics.test.mjs'))

if ($Profile -eq 'Quick') {
  $null = Invoke-RecordedCommand `
    -Name 'targeted-unit-tests' `
    -WorkingDirectory $frontendRoot `
    -FilePath $npmCommand `
    -Arguments (@('run', 'test:unit', '--') + $targetedTests)
} else {
  $null = Invoke-RecordedCommand `
    -Name 'full-unit-tests' `
    -WorkingDirectory $frontendRoot `
    -FilePath $npmCommand `
    -Arguments @('run', 'test:unit')
  $null = Invoke-RecordedCommand `
    -Name 'frontend-lint' `
    -WorkingDirectory $frontendRoot `
    -FilePath $npmCommand `
    -Arguments @('run', 'lint')
  $null = Invoke-RecordedCommand `
    -Name 'frontend-build' `
    -WorkingDirectory $frontendRoot `
    -FilePath $npmCommand `
    -Arguments @('run', 'build:only')
}

$quickScenarios = @(
  [ordered]@{ Name = 'hybrid-top-dark'; Scenario = 'hybrid-top-handoff'; Samples = 1; Headed = $true; Theme = 'dark'; Width = 1920; Height = 1000 },
  [ordered]@{ Name = 'hybrid-bottom-dark'; Scenario = 'hybrid-bottom-handoff'; Samples = 1; Headed = $true; Theme = 'dark'; Width = 1920; Height = 1000 },
  [ordered]@{ Name = 'short-all-native'; Scenario = 'short-all-native'; Samples = 1; Headed = $false; Theme = 'dark'; Width = 1920; Height = 1000 },
  [ordered]@{ Name = 'height-clamp'; Scenario = 'height-clamp-projection'; Samples = 1; Headed = $false; Theme = 'dark'; Width = 1920; Height = 1000 }
)

$fullScenarios = @(
  [ordered]@{ Name = 'hybrid-top-dark'; Scenario = 'hybrid-top-handoff'; Samples = 3; Headed = $true; Theme = 'dark'; Width = 1920; Height = 1000 },
  [ordered]@{ Name = 'hybrid-bottom-dark'; Scenario = 'hybrid-bottom-handoff'; Samples = 3; Headed = $true; Theme = 'dark'; Width = 1920; Height = 1000 },
  [ordered]@{ Name = 'hybrid-top-light'; Scenario = 'hybrid-top-handoff'; Samples = 3; Headed = $true; Theme = 'light'; Width = 1920; Height = 1000 },
  [ordered]@{ Name = 'hybrid-bottom-light'; Scenario = 'hybrid-bottom-handoff'; Samples = 3; Headed = $true; Theme = 'light'; Width = 1920; Height = 1000 },
  [ordered]@{ Name = 'native-elastic-top'; Scenario = 'native-elastic-top'; Samples = 3; Headed = $true; Theme = 'dark'; Width = 390; Height = 844 },
  [ordered]@{ Name = 'native-elastic-bottom'; Scenario = 'native-elastic-bottom'; Samples = 3; Headed = $true; Theme = 'dark'; Width = 390; Height = 844 },
  [ordered]@{ Name = 'bottom-live-offset'; Scenario = 'hybrid-bottom-live-offset'; Samples = 5; Headed = $true; Theme = 'dark'; Width = 390; Height = 844 },
  [ordered]@{ Name = 'short-all-native'; Scenario = 'short-all-native'; Samples = 1; Headed = $false; Theme = 'dark'; Width = 1920; Height = 1000 },
  [ordered]@{ Name = 'height-clamp'; Scenario = 'height-clamp-projection'; Samples = 1; Headed = $false; Theme = 'dark'; Width = 1920; Height = 1000 },
  [ordered]@{ Name = 'mobile-interaction'; Scenario = 'mobile-interaction'; Samples = 1; Headed = $false; Theme = 'dark'; Width = 390; Height = 844 },
  [ordered]@{ Name = 'narrow-desktop'; Scenario = 'desktop-interaction'; Samples = 1; Headed = $false; Theme = 'dark'; Width = 390; Height = 844 }
)

$selectedScenarios = if ($Profile -eq 'Quick') { $quickScenarios } else { $fullScenarios }
foreach ($scenario in $selectedScenarios) {
  Invoke-ScrollScenario $scenario
}

$failedStages = @($stageResults | Where-Object { $_.status -ne 'passed' })
$finishedAt = [DateTimeOffset]::UtcNow
$gateReport = [ordered]@{
  schemaVersion = 1
  generatedAt = $finishedAt.ToString('O')
  profile = $Profile
  status = if ($failedStages.Count -eq 0) { 'passed' } else { 'failed' }
  outputRoot = $runRoot
  contract = $contract
  environment = $environment
  stages = $stageResults
  scenarios = $scenarioResults
}
$gateReportPath = Join-Path $runRoot 'gate-report.json'
[System.IO.File]::WriteAllText(
  $gateReportPath,
  (($gateReport | ConvertTo-Json -Depth 100) + [Environment]::NewLine),
  [System.Text.UTF8Encoding]::new($false)
)

$summaryLines = [System.Collections.Generic.List[string]]::new()
$summaryLines.Add('# Hybrid virtual-scroll local gate')
$summaryLines.Add('')
$summaryLines.Add("- Status: **$($gateReport.status)**")
$summaryLines.Add("- Profile: $Profile")
$summaryLines.Add("- Generated: $($gateReport.generatedAt)")
$summaryLines.Add("- Git: ``$gitSha``$(if ($environment.gitDirty) { ' (dirty)' } else { '' })")
$summaryLines.Add("- Chrome: ``$chromeExecutable``")
$refreshRateLabel = if ($null -eq $refreshRateHz) { 'unknown refresh rate' } else { "$refreshRateHz Hz" }
$summaryLines.Add("- Display: $($primaryBounds.Width)x$($primaryBounds.Height), $($environment.screen.systemDpi) DPI, $refreshRateLabel")
$summaryLines.Add('')
$summaryLines.Add('## Stages')
$summaryLines.Add('')
$summaryLines.Add('| Stage | Status | Duration (ms) |')
$summaryLines.Add('| --- | --- | ---: |')
foreach ($stage in $stageResults) {
  $summaryLines.Add("| $($stage.name) | $($stage.status) | $($stage.durationMs) |")
}
$summaryLines.Add('')
$summaryLines.Add('## Browser scenarios')
$summaryLines.Add('')
$summaryLines.Add('| Scenario | Theme | Status | Samples | Janky | Max frame gap | Work/event | Projection residual | Handoff gap | Input→motion | Truncated handoffs |')
$summaryLines.Add('| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |')
foreach ($scenario in $scenarioResults) {
  $summaryLines.Add(
    "| $($scenario.name) | $($scenario.theme) | $($scenario.status) | $($scenario.samples) | $($scenario.jankySamples) | $($scenario.maximumFrameGapMs) | $($scenario.maximumScrollInteractionWorkPerEventMs) | $($scenario.maximumHandoffProjectionResidualPx) | $($scenario.maximumHandoffFrameGapMs) | $($scenario.maximumInputToFirstVisualMotionMs) | $($scenario.truncatedHandoffPulseCount) |"
  )
}
$summaryLines.Add('')
$summaryLines.Add('A truncated transition pulse is advisory only. Projection residual, responsiveness, reverse movement, control-pulse displacement, write counts, browser errors, and strict-smooth budgets remain blocking.')
$summaryPath = Join-Path $runRoot 'summary.md'
[System.IO.File]::WriteAllLines($summaryPath, $summaryLines, [System.Text.UTF8Encoding]::new($false))

Write-Host "Gate report: $gateReportPath"
Write-Host "Summary: $summaryPath"
if ($failedStages.Count -gt 0) {
  Write-Host "Hybrid scroll gate failed in $($failedStages.Count) stage(s)."
  exit 1
}
Write-Host 'Hybrid scroll gate passed.'
