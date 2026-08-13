$ErrorActionPreference = "Stop"

$MarrowDir = Join-Path $HOME ".marrow"
$BinDir    = Join-Path $MarrowDir "bin"
$StdDir    = Join-Path $MarrowDir "std"

Write-Host "Installing Marrow & QBE Toolchain..." -ForegroundColor Cyan

$Arch = switch ($env:PROCESSOR_ARCHITECTURE) {
    "AMD64" { "x86_64" }
    "ARM64" { "aarch64" }
    Default { 
        Write-Host "Unsupported Architecture: $env:PROCESSOR_ARCHITECTURE" -ForegroundColor Red
        exit 1 
    }
}

$Target = "${Arch}-pc-windows-msvc"
Write-Host "Detected platform: $Target"

New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
New-Item -ItemType Directory -Force -Path $StdDir | Out-Null

$Repo = "zuygui/marrow"
$Url = "https://github.com/$Repo/releases/latest/download/marrow-$Target.zip"

Write-Host "Downloading release from $Url..."
$TempZip = Join-Path $env:TEMP "marrow-install.zip"
$TempExtract = Join-Path $env:TEMP "marrow-install-tmp"

try {
    Invoke-WebRequest -Uri $Url -OutFile $TempZip -UseBasicParsing
    
    if (Test-Path $TempExtract) { Remove-Item $TempExtract -Recurse -Force }
    Expand-Archive -Path $TempZip -DestinationPath $TempExtract -Force

    if (Test-Path "$TempExtract\marrow.exe") {
        Copy-Item "$TempExtract\marrow.exe" -Destination "$BinDir\marrow.exe" -Force
    }
    if (Test-Path "$TempExtract\qbe.exe") {
        Copy-Item "$TempExtract\qbe.exe" -Destination "$BinDir\qbe.exe" -Force
    }
    if (Test-Path "$TempExtract\std") {
        Copy-Item "$TempExtract\std\*" -Destination $StdDir -Recurse -Force
    }
}
finally {
    Remove-Item $TempZip -ErrorAction SilentlyContinue
    Remove-Item $TempExtract -Recurse -ErrorAction SilentlyContinue
}


$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")

if (($UserPath -split ";") -notcontains $BinDir) {
    $NewUserPath = "$UserPath;$BinDir"
    [Environment]::SetEnvironmentVariable("Path", $NewUserPath, "User")
    
    $env:Path += ";$BinDir"
    
    Write-Host "Added $BinDir to User PATH" -ForegroundColor Green
}

Write-Host ""
Write-Host "Marrow & QBE installed successfully!" -ForegroundColor Green
Write-Host "Then test with:"
Write-Host "  marrow --version" -ForegroundColor Cyan
Write-Host "  qbe -h" -ForegroundColor Cyan