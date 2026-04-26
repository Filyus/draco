param(
    [switch]$Debug,
    [switch]$NoOptimize,
    [string[]]$Features,
    [switch]$Serve,
    [int]$Port = 8080
)

# Build script for Draco Web WASM modules
# Requires wasm-pack to be installed: cargo install wasm-pack
# Usage examples:
#  .\build.ps1                 # Release build (default)
#  .\build.ps1 -Debug         # Debug/dev build (no wasm-opt, dev profile)
#  .\build.ps1 -NoOptimize    # Skip wasm-opt step
#  .\build.ps1 -Features console_error_panic_hook  # Pass cargo features to wasm-pack
#  .\build.ps1 -Serve         # Build and start web server on port 8080
#  .\build.ps1 -Serve -Port 9000  # Build and start web server on port 9000

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
        # Build with wasm-pack. Debug builds use --dev; release builds use --release and run wasm-opt by default.
        # Remove -wasm suffix and convert remaining dashes to underscores
        $outputName = ($module -replace '-wasm$', '') -replace '-', '_'
        Get-ChildItem $outputDir -Filter ($outputName + "*_bg.wasm") -ErrorAction SilentlyContinue |
            Remove-Item -Force

        $wasmPackArgs = @('build')
        if ($Debug) { $wasmPackArgs += '--dev' } else { $wasmPackArgs += '--release'; $wasmPackArgs += '--no-opt' }
        $wasmPackArgs += '--target'; $wasmPackArgs += 'web'
        $wasmPackArgs += '--out-dir'; $wasmPackArgs += $outputDir
        $wasmPackArgs += '--out-name'; $wasmPackArgs += $outputName

        # Auto-enable console_error_panic_hook for Debug builds unless the user specified features.
        if ($Debug) {
            if (-not $Features -or $Features.Count -eq 0) {
                $Features = @('console_error_panic_hook')
                Write-Host "  Debug build: enabling feature 'console_error_panic_hook'" -ForegroundColor Gray
            } elseif (-not ($Features -contains 'console_error_panic_hook')) {
                $Features += 'console_error_panic_hook'
                Write-Host "  Debug build: appending feature 'console_error_panic_hook'" -ForegroundColor Gray
            }
        }

        if ($Features -and $Features.Count -gt 0) {
            $featStr = $Features -join ","
            $wasmPackArgs += '--'; $wasmPackArgs += '--features'; $wasmPackArgs += $featStr
        }

        Write-Host "  Running: wasm-pack $($wasmPackArgs -join ' ')" -ForegroundColor Gray
        & wasm-pack @wasmPackArgs
        
        if ($LASTEXITCODE -eq 0) {
            Write-Host "  Success!" -ForegroundColor Green
            
            # Run wasm-opt manually with all necessary WASM features enabled (skip during Debug or when NoOptimize is set)
            $wasmFile = Join-Path $outputDir ($outputName + "_bg.wasm")
            if (-not $Debug -and -not $NoOptimize -and (Test-Path $wasmFile)) {
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
            }

            # Rename _bg.wasm to .wasm to remove the suffix if present
            if (Test-Path $wasmFile) {
                $cleanWasmFile = Join-Path $outputDir ($outputName + ".wasm")
                Move-Item -Path $wasmFile -Destination $cleanWasmFile -Force
                Write-Host "  Renamed to $(Split-Path $cleanWasmFile -Leaf)" -ForegroundColor Gray
            }

            Get-ChildItem $outputDir -Filter ($outputName + "*_bg.wasm") -ErrorAction SilentlyContinue |
                Remove-Item -Force
            
            # Update the .js file to reference the new filename
            $jsFile = Join-Path $outputDir ($outputName + ".js")
            if (Test-Path $jsFile) {
                $jsContent = Get-Content $jsFile -Raw
                $jsContent = $jsContent -replace '_bg\.wasm', '.wasm'
                Set-Content $jsFile $jsContent -NoNewline
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

if ($Serve) {
    $wwwDir = Join-Path $webDir "www"
    $serverManifest = Join-Path $webDir "dev-server\Cargo.toml"

    Write-Host "`nStarting web server..." -ForegroundColor Cyan
    Write-Host "Serving from: $wwwDir" -ForegroundColor Gray
    Write-Host "WASM gzip compression: enabled" -ForegroundColor Gray
    
    cargo run --manifest-path $serverManifest -- $wwwDir $Port
} else {
    Write-Host "`nTo serve the web app, run:" -ForegroundColor White
    Write-Host "  .\build.ps1 -Serve" -ForegroundColor Gray
    Write-Host "`nThen open http://localhost:8080 in your browser" -ForegroundColor White
}
