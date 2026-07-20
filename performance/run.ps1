param(
  [ValidateSet('baseline', 'compare', 'smoke', 'storage')]
  [string]$Command = 'smoke',
  [switch]$Headed
)

$ErrorActionPreference = 'Stop'
$performanceRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
if ($Command -ne 'storage' -and -not (Test-Path (Join-Path $performanceRoot 'node_modules'))) {
  npm ci --prefix $performanceRoot
  npm run install-browser --prefix $performanceRoot
}

$arguments = @($Command)
if ($Headed) { $arguments += '--headed' }
node (Join-Path $performanceRoot 'run.mjs') @arguments
