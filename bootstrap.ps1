#!/usr/bin/env pwsh
# bootstrap.ps1 - Build script for packager-rs
#
# Commands:
#   build   - Build Rust CLI (release by default)
#   python  - Build Python module via maturin
#   test    - Run tests
#   bench   - Run benchmarks
#   docs    - Build documentation (mdbook + rustdoc)
#   clean   - Clean build artifacts
#   help    - Show this help

param(
    [Parameter(Position=0)]
    [ValidateSet("build", "python", "test", "bench", "docs", "clean", "help", "")]
    [string]$Mode = "",
    
[Alias("d")]
    [switch]$DebugBuild,      # Debug build (default: release)
    
    [Alias("i")]
    [switch]$Install,    # For python: install in current venv
    
    [Alias("h", "?")]
    [switch]$Help
)

$ErrorActionPreference = "Stop"
$script:RootDir = $PSScriptRoot

# ============================================================
# HELPERS
# ============================================================

function Format-Time {
    param([double]$ms)
    if ($ms -lt 1000) { return "{0:N0}ms" -f $ms }
    elseif ($ms -lt 60000) { return "{0:N1}s" -f ($ms / 1000) }
    else {
        $min = [math]::Floor($ms / 60000)
        $sec = ($ms % 60000) / 1000
        return "{0}m{1:N0}s" -f $min, $sec
    }
}

function Write-Header {
    param([string]$Text)
    $line = "=" * 50
    Write-Host ""
    Write-Host $line -ForegroundColor Cyan
    Write-Host " $Text" -ForegroundColor Cyan
    Write-Host $line -ForegroundColor Cyan
}

# ============================================================
# HELP
# ============================================================

function Show-Help {
    Write-Host @"

 PACKAGER-RS BUILD SCRIPT

 COMMANDS
   build     Build Rust CLI binary
   python    Build Python module (maturin develop)
   test      Run cargo tests
   bench     Run criterion benchmarks
   docs      Build documentation
   clean     Clean build artifacts
   help      Show this help

 OPTIONS
   -DebugBuild    Build in debug mode (default: release)
   -Install  For python: install in current venv (default: build only)

 EXAMPLES
   .\bootstrap.ps1 build              # Build release CLI
   .\bootstrap.ps1 build -d            # Build debug CLI
   .\bootstrap.ps1 python              # Build wheel only (release)
   .\bootstrap.ps1 python -i            # Build and install in venv
   .\bootstrap.ps1 python -d           # Build wheel (debug)
   .\bootstrap.ps1 test               # Run all tests

 PYTHON USAGE (after python build)
   from packager import Package, Env, Evar, App, Storage, Solver

   pkg = Package("maya", "2026.1.0")
   pkg.add_req("redshift@>=3.5")
   
   storage = Storage.scan()
   pkg.solve(storage.packages)
   print(pkg.deps)

"@ -ForegroundColor White
}

# ============================================================
# BUILD
# ============================================================

function Invoke-Build {
    $buildType = if ($DebugBuild) { "debug" } else { "release" }
    Write-Header "BUILD ($buildType)"
    
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    
    Push-Location $script:RootDir
    try {
        if ($DebugBuild) {
            Write-Host "  cargo build..." -ForegroundColor Yellow
            cargo build
        } else {
            Write-Host "  cargo build --release..." -ForegroundColor Yellow
            cargo build --release
        }
        
        $sw.Stop()
        
        if ($LASTEXITCODE -eq 0) {
            $exePath = if ($DebugBuild) { 
                Join-Path $script:RootDir "target\debug\pkg.exe" 
            } else { 
                Join-Path $script:RootDir "target\release\pkg.exe" 
            }
            
            Write-Host ""
            Write-Host "  Done! ($(Format-Time $sw.ElapsedMilliseconds))" -ForegroundColor Green
            Write-Host "  Binary: $exePath" -ForegroundColor Cyan
            Write-Host ""
        } else {
            Write-Host ""
            Write-Host "  Build failed!" -ForegroundColor Red
            exit 1
        }
    } finally {
        Pop-Location
    }
}

# ============================================================
# PYTHON
# ============================================================

function Invoke-Python {
    $buildType = if ($DebugBuild) { "debug" } else { "release" }
    Write-Header "PYTHON BUILD ($buildType)"
    
    # Check maturin
    $maturin = Get-Command maturin -ErrorAction SilentlyContinue
    if (-not $maturin) {
        Write-Host ""
        Write-Host "  ERROR: maturin not found" -ForegroundColor Red
        Write-Host "  Install: pip install maturin" -ForegroundColor Yellow
        Write-Host ""
        exit 1
    }
    
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    
    Push-Location $script:RootDir
    try {
        if ($Install) {
            # Build and install in current venv
            if ($DebugBuild) {
                Write-Host "  maturin develop..." -ForegroundColor Yellow
                maturin develop
            } else {
                Write-Host "  maturin develop --release..." -ForegroundColor Yellow
                maturin develop --release
            }
        } else {
            # Build wheel only
            if ($DebugBuild) {
                Write-Host "  maturin build..." -ForegroundColor Yellow
                maturin build
            } else {
                Write-Host "  maturin build --release..." -ForegroundColor Yellow
                maturin build --release
            }
        }
        
        $sw.Stop()
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host ""
            Write-Host "  Done! ($(Format-Time $sw.ElapsedMilliseconds))" -ForegroundColor Green
            
            if ($Install) {
                Write-Host ""
                Write-Host "  Usage:" -ForegroundColor Cyan
                Write-Host "    python -c `"from packager import Package; print(Package('test', '1.0.0'))`"" -ForegroundColor White
            } else {
                # Show wheel location
                $wheelDir = Join-Path $script:RootDir "target\wheels"
                $wheel = Get-ChildItem $wheelDir -Filter "*.whl" -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
                if ($wheel) {
                    Write-Host "  Wheel: $($wheel.FullName)" -ForegroundColor Cyan
                    Write-Host ""
                    Write-Host "  Install with:" -ForegroundColor White
                    Write-Host "    pip install $($wheel.FullName)" -ForegroundColor Yellow
                }
            }
            Write-Host ""
        } else {
            Write-Host ""
            Write-Host "  Build failed!" -ForegroundColor Red
            exit 1
        }
    } finally {
        Pop-Location
    }
}

# ============================================================
# TEST
# ============================================================

function Invoke-Test {
    Write-Header "TEST"
    
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    
    Push-Location $script:RootDir
    try {
        # Unit tests
        Write-Host "  cargo test --lib..." -ForegroundColor Yellow
        cargo test --lib
        if ($LASTEXITCODE -ne 0) {
            Write-Host ""
            Write-Host "  Unit tests failed!" -ForegroundColor Red
            exit 1
        }
        
        # Integration tests
        Write-Host ""
        Write-Host "  cargo test --test integration..." -ForegroundColor Yellow
        cargo test --test integration
        
        $sw.Stop()
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host ""
            Write-Host "  All tests passed! ($(Format-Time $sw.ElapsedMilliseconds))" -ForegroundColor Green
            Write-Host ""
        } else {
            Write-Host ""
            Write-Host "  Integration tests failed!" -ForegroundColor Red
            exit 1
        }
    } finally {
        Pop-Location
    }
}

# ============================================================
# BENCH
# ============================================================

function Invoke-Bench {
    Write-Header "BENCH"
    
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    
    Push-Location $script:RootDir
    try {
        Write-Host "  cargo bench..." -ForegroundColor Yellow
        cargo bench --bench scan_bench
        
        $sw.Stop()
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host ""
            Write-Host "  Benchmarks complete! ($(Format-Time $sw.ElapsedMilliseconds))" -ForegroundColor Green
            Write-Host "  Report: target\criterion\report\index.html" -ForegroundColor Cyan
            Write-Host ""
        } else {
            Write-Host ""
            Write-Host "  Benchmarks failed!" -ForegroundColor Red
            exit 1
        }
    } finally {
        Pop-Location
    }
}

# ============================================================
# DOCS
# ============================================================

function Invoke-Docs {
    Write-Header "DOCS"
    
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    
    Push-Location $script:RootDir
    try {
        # Build rustdoc
        Write-Host "  cargo doc..." -ForegroundColor Yellow
        cargo doc --no-deps
        
        # Build mdbook if installed
        $mdbook = Get-Command mdbook -ErrorAction SilentlyContinue
        if ($mdbook) {
            Write-Host "  mdbook build..." -ForegroundColor Yellow
            Push-Location docs
            mdbook build
            Pop-Location
        } else {
            Write-Host "  mdbook not found, skipping (cargo install mdbook)" -ForegroundColor Yellow
        }
        
        $sw.Stop()
        
        Write-Host ""
        Write-Host "  Done! ($(Format-Time $sw.ElapsedMilliseconds))" -ForegroundColor Green
        Write-Host "  Rustdoc: target\doc\packager\index.html" -ForegroundColor Cyan
        if ($mdbook) {
            Write-Host "  Mdbook:  docs\book\index.html" -ForegroundColor Cyan
        }
        Write-Host ""
    } finally {
        Pop-Location
    }
}

# ============================================================
# CLEAN
# ============================================================

function Invoke-Clean {
    Write-Header "CLEAN"
    
    Push-Location $script:RootDir
    try {
        Write-Host "  cargo clean..." -ForegroundColor Yellow
        cargo clean
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host ""
            Write-Host "  Done!" -ForegroundColor Green
            Write-Host ""
        }
    } finally {
        Pop-Location
    }
}

# ============================================================
# MAIN
# ============================================================

if ($Help -or $Mode -eq "" -or $Mode -eq "help") {
    Show-Help
    exit 0
}

switch ($Mode) {
    "build"  { Invoke-Build }
    "python" { Invoke-Python }
    "test"   { Invoke-Test }
    "bench"  { Invoke-Bench }
    "docs"   { Invoke-Docs }
    "clean"  { Invoke-Clean }
}
