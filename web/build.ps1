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
        # Build with wasm-pack (--no-opt to skip wasm-opt, we'll optimize manually)
        # Remove -wasm suffix and convert remaining dashes to underscores
        $outputName = ($module -replace '-wasm$', '') -replace '-', '_'
        wasm-pack build --release --target web --out-dir "$outputDir" --out-name $outputName --no-opt
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host "  Success!" -ForegroundColor Green
            
            # Run wasm-opt manually with all necessary WASM features enabled
            $outputName = ($module -replace '-wasm$', '') -replace '-', '_'
            $wasmFile = Join-Path $outputDir ($outputName + "_bg.wasm")
            if (Test-Path $wasmFile) {
                Write-Host "  Optimizing with wasm-opt..." -ForegroundColor Gray
                $wasmOptPath = "$env:USERPROFILE\.cargo\bin\wasm-opt.exe"
                if (-not (Test-Path $wasmOptPath)) {
                    # Try to find wasm-opt in wasm-pack cache
                    $wasmOptPath = (Get-ChildItem "$env:LOCALAPPDATA\.wasm-pack\wasm-opt-*\bin\wasm-opt.exe" -ErrorAction SilentlyContinue | Select-Object -First 1).FullName
                }
                if ($wasmOptPath -and (Test-Path $wasmOptPath)) {
                    & $wasmOptPath $wasmFile -Oz --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals -o $wasmFile
                    if ($LASTEXITCODE -eq 0) {
                        Write-Host "  Optimization complete!" -ForegroundColor Green
                    }
                }
                
                # Rename _bg.wasm to .wasm to remove the suffix
                $cleanWasmFile = Join-Path $outputDir ($outputName + ".wasm")
                Move-Item -Path $wasmFile -Destination $cleanWasmFile -Force
                Write-Host "  Renamed to $(Split-Path $cleanWasmFile -Leaf)" -ForegroundColor Gray
                
                # Update the .js file to reference the new filename
                $jsFile = Join-Path $outputDir ($outputName + ".js")
                if (Test-Path $jsFile) {
                    $jsContent = Get-Content $jsFile -Raw
                    $jsContent = $jsContent -replace '_bg\.wasm', '.wasm'
                    Set-Content $jsFile $jsContent -NoNewline
                }
            }
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
