# HaloCord's one check: format, lints, and the HaloCord tests in the core and
# the window. Runs at idle so the PC stays usable. Exit 0 only when all pass;
# the release command refuses to ship otherwise.
#
#   powershell -NoProfile -File scripts/halocord-check.ps1 [-Fix] [-Log <file>]
#
# -Fix formats the code instead of only checking it.
param(
    [switch]$Fix,
    [string]$Log = ''
)

$ErrorActionPreference = 'Continue'
try { (Get-Process -Id $PID).PriorityClass = 'Idle' } catch {}
if (-not $env:CARGO_BUILD_JOBS) { $env:CARGO_BUILD_JOBS = '4' }
Set-Location (Split-Path -Parent $PSScriptRoot)

$failed = @()
function Say([string]$line) {
    if ($Log) { Add-Content -Path $Log -Value $line -Encoding utf8 } else { Write-Output $line }
}
# Yapfast's own code has a few warnings of its own (mostly Windows-only code
# upstream never builds), so a lint warning fails the check only in HaloCord's
# files: anything under halocord/ or mochacord, and the few others it owns.
$ours = 'halocord|mochacord|fonts[\\/]pick\.rs|discord_audio[\\/]flat\.rs'
function Step([string]$name, [string[]]$cargoArgs) {
    Say "== $name $(Get-Date -Format HH:mm)"
    $lines = & cargo @cargoArgs 2>&1 | ForEach-Object { "$_" }
    $code = $LASTEXITCODE
    $lines | Select-String -Pattern 'test result|FAILED|panicked|^error|error\[|: warning:|: error|^Diff in' |
        ForEach-Object { Say $_.Line }
    $mine = @($lines | Where-Object { $_ -match ': warning:' -and $_ -match $ours })
    Say "exit $code, HaloCord warnings $($mine.Count)"
    if ($code -ne 0 -or $mine.Count -gt 0) { $script:failed += $name }
}

if ($Log) { Remove-Item $Log -ErrorAction SilentlyContinue }
if ($Fix) { Step 'format' @('fmt', '--all') } else { Step 'format' @('fmt', '--all', '--', '--check') }
Step 'lints' @('clippy', '-p', 'yapfast-core', '-p', 'yapfast-discord', '-p', 'yapfast-iced', '--all-targets', '--message-format=short')
Step 'core tests' @('test', '-p', 'yapfast-core', '--lib', 'halocord')
Step 'window tests' @('test', '-p', 'yapfast-iced', '--lib', 'halocord')

if ($failed.Count -gt 0) {
    Say "== failed: $($failed -join ', ')"
    exit 1
}
Say '== all passed'
exit 0
