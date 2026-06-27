#Requires -Version 5.0
<#
.SYNOPSIS
    Build script for LX Music Mobile hybrid Rust/React Native architecture.
    
.DESCRIPTION
    This script builds the Rust core library, compiles the C++ JNI bridge,
    and prepares everything for integration with the React Native app.
    
    Architecture: JS -> React Native -> C++ TurboModule -> JNI -> Rust .so
    
.PARAMETER Action
    The build action to perform: Build, Clean, All, or Info.
    
.PARAMETER Target
    Build target: Debug or Release. Default is Release.
    
.PARAMETER RustOnly
    Build only the Rust library, skip Android native parts.
    
.PARAMETER SkipRust
    Skip building the Rust library.
    
.EXAMPLE
    .\build-hybrid.ps1 -Action All -Target Release
    
.EXAMPLE
    .\build-hybrid.ps1 -Action Build -RustOnly
#>

param(
    [ValidateSet("Build", "Clean", "All", "Info")]
    [string]$Action = "Build",
    
    [ValidateSet("Debug", "Release")]
    [string]$Target = "Release",
    
    [switch]$RustOnly,
    
    [switch]$SkipRust,
    
    [string]$NDKPath = $env:ANDROID_NDK_HOME
)

$ErrorActionPreference = "Stop"
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Split-Path -Parent $scriptDir

# Colors for output
function Write-Step { param([string]$Message) Write-Host "[BUILD] $Message" -ForegroundColor Cyan }
function Write-Success { param([string]$Message) Write-Host "[OK] $Message" -ForegroundColor Green }
function Write-Warning { param([string]$Message) Write-Host "[WARN] $Message" -ForegroundColor Yellow }
function Write-Failure { param([string]$Message) Write-Host "[FAIL] $Message" -ForegroundColor Red }
function Write-Info { param([string]$Message) Write-Host "[INFO] $Message" -ForegroundColor Gray }

# ============================================================================
# Configuration
# ============================================================================

$RustSrcPath = Join-Path $projectRoot "src\rust"
$AndroidSrcPath = Join-Path $projectRoot "src\android"
$CPPPath = Join-Path $projectRoot "src\cpp"
$OutputPath = Join-Path $projectRoot "build"

# ABI configurations for Android
$AndroidABIs = @("armeabi-v7a", "arm64-v8a", "x86", "x86_64")

# ============================================================================
# Helper Functions
# ============================================================================

function Get-RustTarget {
    param([string]$Target)
    
    switch ($Target) {
        "Debug" { return "debug" }
        "Release" { return "release" }
    }
}

function Test-Command {
    param([string]$Command)
    
    $null -ne (Get-Command $Command -ErrorAction SilentlyContinue)
}

# ============================================================================
# Info Action
# ============================================================================

function Show-Info {
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Magenta
    Write-Host "  LX Music Hybrid Build System Info" -ForegroundColor Magenta
    Write-Host "========================================" -ForegroundColor Magenta
    Write-Host ""
    
    Write-Host "Project Structure:" -ForegroundColor Yellow
    Write-Info "Project Root: $projectRoot"
    Write-Info "Rust Source: $RustSrcPath"
    Write-Info "Android Source: $AndroidSrcPath"
    Write-Info "C++ Sources: $CPPPath"
    Write-Info "Output: $OutputPath"
    Write-Host ""
    
    Write-Host "Rust Library:" -ForegroundColor Yellow
    if (Test-Path (Join-Path $RustSrcPath "Cargo.toml")) {
        Write-Success "Cargo.toml found"
        
        # Parse rust toolchain
        $rustToolchain = Join-Path $RustSrcPath "rust-toolchain"
        if (Test-Path $rustToolchain) {
            $toolchain = Get-Content $rustToolchain -Raw
            Write-Info "Rust toolchain: $toolchain"
        } else {
            Write-Info "Rust toolchain: (default)"
        }
    } else {
        Write-Failure "Cargo.toml not found"
    }
    Write-Host ""
    
    Write-Host "Environment:" -ForegroundColor Yellow
    
    # Check Rust
    if (Test-Command "rustc") {
        $rustVersion = (rustc --version | Out-String).Trim()
        Write-Info "Rust: $rustVersion"
    } else {
        Write-Warning "Rust: not found"
    }
    
    # Check Cargo
    if (Test-Command "cargo") {
        $cargoVersion = (cargo --version | Out-String).Trim()
        Write-Info "Cargo: $cargoVersion"
    } else {
        Write-Warning "Cargo: not found"
    }
    
    # Check NDK
    if ($NDKPath -and (Test-Path $NDKPath)) {
        Write-Info "Android NDK: $NDKPath"
        
        # Find NDK version
        $ndkVersion = Get-ChildItem $NDKPath -Directory | Select-Object -First 1
        if ($ndkVersion) {
            Write-Info "NDK Version: $($ndkVersion.Name)"
        }
    } else {
        Write-Warning "Android NDK: not found (set ANDROID_NDK_HOME)"
    }
    
    # Check cargo-ndk
    if (Test-Command "cargo-ndk") {
        $ndkVersion = (cargo-ndk --version | Out-String).Trim()
        Write-Info "cargo-ndk: $ndkVersion"
    } else {
        Write-Warning "cargo-ndk: not installed (run: cargo install cargo-ndk)"
    }
    
    # Check Android SDK
    if ($env:ANDROID_HOME) {
        Write-Info "Android SDK: $env:ANDROID_HOME"
    } else {
        Write-Warning "Android SDK: not found (set ANDROID_HOME)"
    }
    
    Write-Host ""
    Write-Host "Target ABIs:" -ForegroundColor Yellow
    foreach ($abi in $AndroidABIs) {
        Write-Info "  - $abi"
    }
    
    Write-Host ""
}

# ============================================================================
# Clean Action
# ============================================================================

function Invoke-Clean {
    Write-Step "Cleaning build artifacts..."
    
    # Clean Rust
    if (Test-Path $RustSrcPath) {
        $cargoToml = Join-Path $RustSrcPath "Cargo.toml"
        if (Test-Path $cargoToml) {
            Push-Location $RustSrcPath
            Write-Info "Running cargo clean..."
            cargo clean 2>&1 | Out-Null
            Pop-Location
        }
    }
    
    # Clean build output
    if (Test-Path $OutputPath) {
        Write-Info "Removing build output directory..."
        Remove-Item $OutputPath -Recurse -Force -ErrorAction SilentlyContinue
    }
    
    # Clean Android build artifacts
    $androidBuildPaths = @(
        (Join-Path $AndroidSrcPath "app\build"),
        (Join-Path $AndroidSrcPath ".gradle"),
        (Join-Path $AndroidSrcPath "build")
    )
    
    foreach ($path in $androidBuildPaths) {
        if (Test-Path $path) {
            Write-Info "Removing $path..."
            Remove-Item $path -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
    
    Write-Success "Clean completed"
}

# ============================================================================
# Build Rust Library
# ============================================================================

function Invoke-BuildRust {
    param(
        [string]$Target,
        [string[]]$ABIs
    )
    
    Write-Step "Building Rust core library..."
    Write-Info "Target: $Target"
    
    if (-not (Test-Path $RustSrcPath)) {
        Write-Failure "Rust source directory not found: $RustSrcPath"
        return $false
    }
    
    # Check if Cargo.toml exists
    $cargoToml = Join-Path $RustSrcPath "Cargo.toml"
    if (-not (Test-Path $cargoToml)) {
        Write-Failure "Cargo.toml not found: $cargoToml"
        return $false
    }
    
    Push-Location $RustSrcPath
    
    try {
        # Build for each Android ABI
        foreach ($abi in $ABIs) {
            Write-Info "Building for $abi..."
            
            $rustTarget = Get-RustTarget $Target
            
            # Build command
            if ($NDKPath) {
                # Use cargo-ndk if available
                if (Test-Command "cargo-ndk") {
                    $buildCmd = "cargo ndk -l $abi build --target-dir target -- $Target.ToLower()"
                    $process = Start-Process -FilePath "cmd" -ArgumentList "/c", "cargo ndk -l $abi build $Target.ToLower()" -NoNewWindow -Wait -PassThru -RedirectStandardOutput ".\build-$abi.log" -RedirectStandardError ".\build-$abi-error.log"
                    
                    if ($process.ExitCode -ne 0) {
                        Write-Failure "Build failed for $abi"
                        if (Test-Path ".\build-$abi-error.log") {
                            Get-Content ".\build-$abi-error.log" | Select-Object -First 20
                        }
                        Pop-Location
                        return $false
                    }
                } else {
                    # Fall back to regular cargo build
                    $env:CC = "clang"
                    $env:CXX = "clang++"
                    $env:TARGET_CC = "$NDKPath\toolchains\llvm\prebuilt\windows-x86_64\bin\clang.exe"
                    $env:TARGET_CXX = "$NDKPath\toolchains\llvm\prebuilt\windows-x86_64\bin\clang++.exe"
                    $env:AR = "$NDKPath\toolchains\llvm\prebuilt\windows-x86_64\bin\llvm-ar.exe"
                    
                    $buildCmd = "cargo build --target $abi --release"
                    $process = Start-Process -FilePath "cmd" -ArgumentList "/c", $buildCmd -NoNewWindow -Wait -PassThru
                    
                    if ($process.ExitCode -ne 0) {
                        Write-Failure "Build failed for $abi"
                        Pop-Location
                        return $false
                    }
                }
            } else {
                # No NDK, build for host
                Write-Warning "ANDROID_NDK_HOME not set, building for host..."
                $buildCmd = "cargo build --$Target.ToLower()"
                $process = Start-Process -FilePath "cmd" -ArgumentList "/c", $buildCmd -NoNewWindow -Wait -PassThru
                
                if ($process.ExitCode -ne 0) {
                    Write-Failure "Build failed"
                    Pop-Location
                    return $false
                }
            }
            
            # Copy output
            $libName = "liblx_music_core.so"
            $srcLib = Join-Path $RustSrcPath "target\$abi\$rustTarget\$libName"
            $destDir = Join-Path $OutputPath "jniLibs\$abi"
            
            if (Test-Path $srcLib) {
                if (-not (Test-Path $destDir)) {
                    New-Item -ItemType Directory -Path $destDir -Force | Out-Null
                }
                Copy-Item $srcLib $destDir -Force
                Write-Success "Built and copied: $abi"
            } else {
                Write-Warning "Library not found at: $srcLib"
            }
        }
        
        Write-Success "Rust library build completed"
        return $true
    }
    finally {
        Pop-Location
    }
}

# ============================================================================
# Build Android Native
# ============================================================================

function Invoke-BuildAndroidNative {
    param([string]$Target)
    
    Write-Step "Building Android native modules..."
    
    # Find Android build.gradle
    $buildGradle = Join-Path $AndroidSrcPath "app\build.gradle"
    
    if (-not (Test-Path $buildGradle)) {
        Write-Warning "build.gradle not found, skipping Android build"
        return $true
    }
    
    # Run Gradle build
    Push-Location $AndroidSrcPath
    try {
        Write-Info "Running Gradle assemble$Target..."
        
        $gradleCmd = if (Test-Path ".\gradlew.bat") { ".\gradlew.bat" } else { "gradle" }
        $process = Start-Process -FilePath "cmd" -ArgumentList "/c", "$gradleCmd assemble$Target" -NoNewWindow -Wait -PassThru
        
        if ($process.ExitCode -ne 0) {
            Write-Failure "Android build failed"
            return $false
        }
        
        Write-Success "Android build completed"
        return $true
    }
    finally {
        Pop-Location
    }
}

# ============================================================================
# Main Build Flow
# ============================================================================

function Invoke-Build {
    param(
        [string]$Target,
        [switch]$RustOnly
    )
    
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Magenta
    Write-Host "  Building LX Music Hybrid Architecture" -ForegroundColor Magenta
    Write-Host "  Target: $Target" -ForegroundColor Magenta
    Write-Host "========================================" -ForegroundColor Magenta
    Write-Host ""
    
    # Create output directory
    if (-not (Test-Path $OutputPath)) {
        New-Item -ItemType Directory -Path $OutputPath -Force | Out-Null
    }
    
    # Build Rust
    if (-not $SkipRust) {
        $rustResult = Invoke-BuildRust -Target $Target -ABIs $AndroidABIs
        if (-not $rustResult) {
            Write-Failure "Rust build failed"
            return $false
        }
    } else {
        Write-Info "Skipping Rust build (as requested)"
    }
    
    # Build Android native
    if (-not $RustOnly) {
        $androidResult = Invoke-BuildAndroidNative -Target $Target
        if (-not $androidResult) {
            Write-Failure "Android build failed"
            return $false
        }
    } else {
        Write-Info "Skipping Android native build (RustOnly mode)"
    }
    
    Write-Host ""
    Write-Success "========================================"
    Write-Success "  Build completed successfully!"
    Write-Success "========================================"
    Write-Host ""
    Write-Info "Output location: $OutputPath"
    
    return $true
}

# ============================================================================
# Main Entry Point
# ============================================================================

switch ($Action) {
    "Info" {
        Show-Info
    }
    "Clean" {
        Invoke-Clean
    }
    "Build" {
        Invoke-Build -Target $Target -RustOnly:$RustOnly
    }
    "All" {
        Invoke-Clean
        Invoke-Build -Target $Target -RustOnly:$RustOnly
    }
}