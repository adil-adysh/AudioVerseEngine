# PowerShell script to configure MSVC/SDK environment and build Resonance Audio with FFI

# --- MSVC root ---
$msvcRoot = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207"
if (!(Test-Path $msvcRoot)) {
    Write-Error "MSVC root not found: $msvcRoot"
    exit 1
}

# --- Windows 10 SDK root ---
$sdkRoot  = "C:\Program Files (x86)\Windows Kits\10"
if (!(Test-Path $sdkRoot)) {
    Write-Error "Windows SDK root not found: $sdkRoot"
    exit 1
}
$sdkVer   = "10.0.26100.0"

# --- INCLUDE paths ---
$includePaths = @(
    "$msvcRoot\include",
    "$sdkRoot\Include\$sdkVer\ucrt",
    "$sdkRoot\Include\$sdkVer\um",
    "$sdkRoot\Include\$sdkVer\shared"
)
foreach ($incPath in $includePaths) {
    if (!(Test-Path $incPath)) {
        Write-Error "INCLUDE path not found: $incPath"
        exit 1
    }
}
$env:INCLUDE = $includePaths -join ";"

# --- LIB paths ---
$libPaths = @(
    "$msvcRoot\lib\x64",
    "$sdkRoot\Lib\$sdkVer\ucrt\x64",
    "$sdkRoot\Lib\$sdkVer\um\x64"
)
foreach ($libPath in $libPaths) {
    if (!(Test-Path $libPath)) {
        Write-Error "LIB path not found: $libPath"
        exit 1
    }
}
$env:LIB = $libPaths -join ";"

# --- PATH update (compiler + tools) ---
$msvcBin = "$msvcRoot\bin\Hostx64\x64"
if (!(Test-Path $msvcBin)) {
    Write-Error "MSVC bin path not found: $msvcBin"
    exit 1
}
$env:PATH = "$msvcBin;$env:PATH"

# --- Check required binaries ---
$requiredBinaries = @('cmake', 'cl', 'link', 'lib')
foreach ($bin in $requiredBinaries) {
    $found = $false
    $paths = $env:PATH -split ';'
    foreach ($p in $paths) {
        if ([string]::IsNullOrWhiteSpace($p)) { continue }
        $exePath = Join-Path $p ($bin + ".exe")
        if (Test-Path $exePath) {
            $found = $true
            break
        }
    }
    # If not found and binary is cmake, try to find it in Visual Studio Build Tools
    if (-not $found -and $bin -eq 'cmake') {
        $vsCmake = Get-ChildItem -Path 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools' -Recurse -Filter 'cmake.exe' -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty DirectoryName
        if ($vsCmake) {
            $env:PATH = "$vsCmake;$env:PATH"
            $exePath = Join-Path $vsCmake ($bin + ".exe")
            if (Test-Path $exePath) {
                $found = $true
            }
        }
    }
    if (-not $found) {
        Write-Error "Required binary not found in PATH: $bin.exe"
        exit 1
    }
}

# --- MSVC root ---
$msvcRoot = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207"
if (!(Test-Path $msvcRoot)) {
    Write-Error "MSVC root not found: $msvcRoot"
    exit 1
}

# --- Windows 10 SDK root ---
$sdkRoot  = "C:\Program Files (x86)\Windows Kits\10"
if (!(Test-Path $sdkRoot)) {
    Write-Error "Windows SDK root not found: $sdkRoot"
    exit 1
}
$sdkVer   = "10.0.26100.0"

# --- INCLUDE paths ---
$includePaths = @(
    "$msvcRoot\include",
    "$sdkRoot\Include\$sdkVer\ucrt",
    "$sdkRoot\Include\$sdkVer\um",
    "$sdkRoot\Include\$sdkVer\shared"
)
foreach ($incPath in $includePaths) {
    if (!(Test-Path $incPath)) {
        Write-Error "INCLUDE path not found: $incPath"
        exit 1
    }
}
$env:INCLUDE = $includePaths -join ";"

# --- LIB paths ---
$libPaths = @(
    "$msvcRoot\lib\x64",
    "$sdkRoot\Lib\$sdkVer\ucrt\x64",
    "$sdkRoot\Lib\$sdkVer\um\x64"
)
foreach ($libPath in $libPaths) {
    if (!(Test-Path $libPath)) {
        Write-Error "LIB path not found: $libPath"
        exit 1
    }
}
$env:LIB = $libPaths -join ";"

# --- PATH update (compiler + tools) ---
$msvcBin = "$msvcRoot\bin\Hostx64\x64"
if (!(Test-Path $msvcBin)) {
    Write-Error "MSVC bin path not found: $msvcBin"
    exit 1
}
$env:PATH = "$msvcBin;$env:PATH"

Write-Host "`nEnvironment configured ✅"
Write-Host "INCLUDE = $env:INCLUDE"
Write-Host "LIB     = $env:LIB"
Write-Host "PATH    = $env:PATH"


# --- Save original directory and use try/finally to restore it ---
$originalDir = Get-Location
try {
    # --- Navigate to resonance-audio directory ---
    $resonanceAudioDir = Join-Path $PSScriptRoot "resonance-audio"
    if (!(Test-Path $resonanceAudioDir)) {
        Write-Error "resonance-audio directory not found: $resonanceAudioDir"
        exit 1
    }
    Set-Location $resonanceAudioDir

    # --- Patch CMakeLists.txt for FFI_SOURCES if needed ---
    $cmakeFile = Join-Path $resonanceAudioDir "resonance_audio\CMakeLists.txt"
    $patchText = @'
# Allow extra FFI sources to be added via CMake variable
if(DEFINED FFI_SOURCES)
  list(APPEND RA_SOURCES ${FFI_SOURCES})
endif()
'@

    $cmakeContent = Get-Content $cmakeFile -Raw
    if ($cmakeContent -notmatch "if\(DEFINED FFI_SOURCES\)") {
        $pattern = "set\(RA_SOURCES[^\)]*\)"
        $cmakeContent = [System.Text.RegularExpressions.Regex]::Replace(
            $cmakeContent,
            $pattern,
            { param($m) "$($m.Value)`n$patchText" }
        )
        Set-Content $cmakeFile $cmakeContent
        Write-Host "CMakeLists.txt patched for FFI_SOURCES."
    } else {
        Write-Host "Patch already present, skipping."
    }

    # --- Build with CMake ---
    $ffiSources = "$PSScriptRoot/resonance-ffi/resonance_c_api.cc;$PSScriptRoot/resonance-ffi/resonance_c_api.h"
    $ffiSources = $ffiSources -replace '\\', '/'
    Write-Host "\nRunning CMake configure in $resonanceAudioDir..."
    cmake -S . -B build -DSTATIC_MSVC_RUNTIME=OFF -DBUILD_RESONANCE_AUDIO_API=ON -DFFI_SOURCES="$ffiSources"
    if ($LASTEXITCODE -ne 0) {
        Write-Error "CMake configure failed."
        exit $LASTEXITCODE
    }

    Write-Host "\nRunning CMake build in $resonanceAudioDir..."
    cmake --build build --config Release
    if ($LASTEXITCODE -ne 0) {
        Write-Error "CMake build failed."
        exit $LASTEXITCODE
    }

    Write-Host "`nCMake build complete."
} finally {
    Set-Location $originalDir
}
