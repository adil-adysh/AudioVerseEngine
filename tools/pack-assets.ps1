# Convenience script to pack assets from assets/sfx and assets/audio
# Convenience script to pack assets from assets/sfx and assets/audio
# Compute repo root relative to this script's location so the script works
# regardless of the current working directory.
if (-not $PSScriptRoot) {
    # Fallback for older PS versions: use MyInvocation
    $scriptDir = Split-Path -Path $MyInvocation.MyCommand.Definition -Parent
}
else {
    $scriptDir = $PSScriptRoot
}

# tools/pack-assets.ps1 is located in <repo>/tools, repo root is parent of that
$root = Resolve-Path -Path (Join-Path $scriptDir "..")
Push-Location $root
try {
    cargo run -p asset-packer -- --pack-assets
    # Validate the produced package using the pkg-validator tool.
    # Build package path reliably on Windows
    $pkgPath = Join-Path -Path $root -ChildPath "assets\dest\out.pkg"
    if (Test-Path $pkgPath) {
        Write-Host "Validating package: $pkgPath"
        $validatorExit = & cargo run -p pkg-validator -- $pkgPath
        if ($LASTEXITCODE -ne 0) {
            Write-Error "pkg-validator failed. The created package is invalid."
            throw "pkg-validator failed"
        }
        else {
            Write-Host "pkg-validator: package OK"
        }
    }
    else {
        Write-Warning "Expected package not found at $pkgPath; skipping validation."
    }
}
finally {
    Pop-Location
}
