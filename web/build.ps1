# Build script for Draco Web WASM modules
# Requires wasm-pack to be installed: cargo install wasm-pack

$ErrorActionPreference = "Stop"

Write-Host "Building Draco Web WASM Modules" -ForegroundColor Cyan
Write-Host "================================" -ForegroundColor Cyan

$modules = @(
    "obj-reader-wasm",
    "obj-writer-wasm",
    "ply-reader-wasm",
    "ply-writer-wasm",
    "gltf-reader-wasm",
    "gltf-writer-wasm",
    "fbx-reader-wasm",
    "fbx-writer-wasm"
)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$webDir = $scriptDir
$outputDir = Join-Path $webDir "www\pkg"

# Create output directory
if (-not (Test-Path $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
}

Write-Host "`nOutput directory: $outputDir" -ForegroundColor Gray

foreach ($module in $modules) {
    Write-Host "`nBuilding $module..." -ForegroundColor Yellow
    
    $modulePath = Join-Path $webDir $module
    
    if (-not (Test-Path $modulePath)) {
        Write-Host "  Module not found: $modulePath" -ForegroundColor Red
        continue
    }
    
    Push-Location $modulePath
    
    try {
        # Build with wasm-pack
        wasm-pack build --target web --out-dir "$outputDir" --out-name ($module -replace '-', '_')
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host "  Success!" -ForegroundColor Green
        } else {
            Write-Host "  Build failed with exit code $LASTEXITCODE" -ForegroundColor Red
        }
    }
    catch {
        Write-Host "  Error: $_" -ForegroundColor Red
    }
    finally {
        Pop-Location
    }
}

Write-Host "`n================================" -ForegroundColor Cyan
Write-Host "Build complete!" -ForegroundColor Green
Write-Host "`nTo serve the web app, run:" -ForegroundColor White
Write-Host "  cd www" -ForegroundColor Gray
Write-Host "  python -m http.server 8080" -ForegroundColor Gray
Write-Host "  # Or use any static file server" -ForegroundColor Gray
Write-Host "`nThen open http://localhost:8080 in your browser" -ForegroundColor White
