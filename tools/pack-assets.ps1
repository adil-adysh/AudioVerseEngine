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
}
finally {
    Pop-Location
}
