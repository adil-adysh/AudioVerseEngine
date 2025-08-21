# Run ignored real-audio integration tests locally.
# Usage: Open PowerShell in repo root and run: ./scripts/run-real-audio-tests.ps1

# Ensure script runs from repo root (where Cargo.toml lives)
Set-Location -Path (Split-Path -Parent $MyInvocation.MyCommand.Path) | Out-Null
Set-Location -Path ".." | Out-Null

param(
	[switch]$UseIgnored
)

# Optionally enable backtrace for debugging
if (-not $env:RUST_BACKTRACE) { $env:RUST_BACKTRACE = '1' }

if ($UseIgnored) {
	Write-Host "Running real-audio integration tests by executing ignored tests in 'integration-tests'..."
	cargo test -p integration-tests -- --ignored --nocapture
	exit $LASTEXITCODE
} else {
	Write-Host "Running real-audio integration tests using cargo feature 'real-audio-tests' (will run ignored tests)..."
	# Run tests with the feature enabled. The real-device tests are gated by
	# the `real-audio-tests` feature and annotated `#[ignore]` so we pass
	# `--ignored` to execute them explicitly.
	cargo test -p integration-tests --features real-audio-tests -- --ignored --nocapture
	exit $LASTEXITCODE
}
