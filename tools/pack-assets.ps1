# Convenience script to pack assets from assets/sfx and assets/audio
$root = Resolve-Path -Path "..\.." -Relative | Resolve-Path
Push-Location $root
try {
    cargo run -p asset-packer -- --pack-assets
} finally {
    Pop-Location
}
