#!/usr/bin/env pwsh
# test.ps1 - Package manager test script
# Usage: ./test.ps1 [command]
# Commands: gen, basic, conflict

param(
    [Parameter(Position=0)]
    [string]$Command
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Pkg = Join-Path $Root "target\release\pkg.exe"
$TestDir = $PSScriptRoot
$Repo = Join-Path $TestDir "repo"
$RepoBad = Join-Path $TestDir "repo_bad"

# Check if pkg.exe exists
if (-not (Test-Path $Pkg)) {
    $Pkg = Join-Path $Root "target\debug\pkg.exe"
    if (-not (Test-Path $Pkg)) {
        Write-Host "ERROR: pkg.exe not found. Run 'cargo build --release' first." -ForegroundColor Red
        exit 1
    }
}

function Show-Help {
    Write-Host "test.ps1 - Package manager test script" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Usage: ./test.ps1 <command>" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Commands:"
    Write-Host "  gen      Generate test repositories (repo/ and repo_bad/)"
    Write-Host "  basic    Run basic operations test (list, info, env)"
    Write-Host "  conflict Run dependency conflict resolution test"
    Write-Host ""
    Write-Host "Examples:"
    Write-Host "  ./test.ps1 gen"
    Write-Host "  ./test.ps1 basic"
    Write-Host "  ./test.ps1 conflict"
}

function Run-Gen {
    Write-Host "=== GENERATING TEST REPOSITORIES ===" -ForegroundColor Green
    Write-Host ""
    
    Write-Host "1. Generating main repo ($Repo)..."
    & $Pkg gen-repo -n 200 -V 5 --dep-rate 0.4 --seed 42 -o $Repo
    
    Write-Host ""
    Write-Host "Done." -ForegroundColor Green
}

function Run-Basic {
    $env:REZ_PACKAGES_PATH = $Repo
    
    Write-Host "=== BASIC OPERATIONS TEST ===" -ForegroundColor Green
    Write-Host "Using: $env:REZ_PACKAGES_PATH"
    Write-Host ""
    
    Write-Host "1. Scan packages:" -ForegroundColor Yellow
    & $Pkg scan
    Write-Host ""
    
    Write-Host "2. List packages (sample):" -ForegroundColor Yellow
    & $Pkg list | Select-Object -First 15
    Write-Host "  ... (truncated)"
    Write-Host ""
    
    Write-Host "==================================================" -ForegroundColor Cyan
    Write-Host "3. TOOLSET ENV: maya-fx" -ForegroundColor Yellow
    & $Pkg env maya bifrost phoenix_fd fumefx mash boss redshift arnold mtoa usd alembic ocio python numpy pyside
    Write-Host ""
    
    Write-Host "=== TEST COMPLETE ===" -ForegroundColor Green
}

function Run-Conflict {
    $env:REZ_PACKAGES_PATH = $RepoBad
    
    Write-Host "=== DEPENDENCY RESOLUTION TEST ===" -ForegroundColor Green
    Write-Host "Using: $env:REZ_PACKAGES_PATH"
    Write-Host ""
    
    Write-Host "1. Available packages:" -ForegroundColor Yellow
    & $Pkg list
    Write-Host ""
    
    Write-Host "==================================================" -ForegroundColor Cyan
    Write-Host "2. vfx_legacy-1.0.0 (UNSOLVABLE CONFLICT):" -ForegroundColor Yellow
    & $Pkg solve vfx_legacy-1.0.0
    Write-Host ""
    
    Write-Host "==================================================" -ForegroundColor Cyan
    Write-Host "3. vfx_project-1.0.0 (SOLVER FINDS SOLUTION):" -ForegroundColor Yellow
    & $Pkg solve vfx_project-1.0.0
    Write-Host ""
    
    Write-Host "==================================================" -ForegroundColor Cyan
    Write-Host "4. vfx_project-2.0.0 (NO CONFLICT):" -ForegroundColor Yellow
    & $Pkg solve vfx_project-2.0.0
    Write-Host ""
    
    Write-Host "=== TEST COMPLETE ===" -ForegroundColor Green
}

# Main
switch ($Command) {
    "gen"      { Run-Gen }
    "basic"    { Run-Basic }
    "conflict" { Run-Conflict }
    default    { Show-Help }
}
