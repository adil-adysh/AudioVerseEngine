# Run ignored real-audio integration tests locally.
# Usage: Open PowerShell in repo root and run: ./scripts/run-real-audio-tests.ps1

# Ensure script runs from repo root (where Cargo.toml lives)
Set-Location -Path (Split-Path -Parent $MyInvocation.MyCommand.Path) | Out-Null
Set-Location -Path ".." | Out-Null

# Set environment variable to enable the tests
$env:RUN_REAL_AUDIO = '1'

# Optionally enable backtrace for debugging
if (-not $env:RUST_BACKTRACE) { $env:RUST_BACKTRACE = '1' }

Write-Host "Running real-audio integration tests (RUN_REAL_AUDIO=1) ..."

# Run only the integration-tests package and include ignored tests
cargo test -p integration-tests -- --ignored --nocapture

# Exit with the cargo command's exit code
exit $LASTEXITCODE
