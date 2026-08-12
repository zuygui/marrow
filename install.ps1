$ErrorActionPreference = "Stop"

$MarrowDir = "$HOME\.marrow"
$BinDir = "$MarrowDir\bin"
$StdDir = "$MarrowDir\std"

Write-Host "Installing Marrow v0.1.0 Toolchain for Windows..." -ForegroundColor Cyan

# 1. Création des répertoires
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
New-Item -ItemType Directory -Force -Path $StdDir | Out-Null

# 2. Téléchargement et extraction du ZIP de la release
$Repo = "zuygui/marrow"
$ZipUrl = "https://github.com/$Repo/releases/latest/download/marrow-x86_64-pc-windows-msvc.zip"
$ZipFile = "$env:TEMP\marrow.zip"
$ExtractPath = "$env:TEMP\marrow_extracted"

Write-Host "Downloading release from $ZipUrl..." -ForegroundColor Yellow
Invoke-WebRequest -Uri $ZipUrl -OutFile $ZipFile

if (Test-Path $ExtractPath) { Remove-Item -Recurse -Force $ExtractPath }
Expand-Archive -Path $ZipFile -DestinationPath $ExtractPath

# 3. Installation du binaire et de la stdlib
Copy-Item "$ExtractPath\marrow.exe" -Destination "$BinDir\marrow.exe" -Force

if (Test-Path "$ExtractPath\std") {
    Copy-Item "$ExtractPath\std\*" -Destination $StdDir -Recurse -Force
}

Remove-Item -Force $ZipFile
Remove-Item -Recurse -Force $ExtractPath

# 4. Ajout au PATH de l'utilisateur s'il n'y est pas déjà
$UserPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)

if ($UserPath -notlike "*$BinDir*") {
    $NewPath = "$UserPath;$BinDir"
    [Environment]::SetEnvironmentVariable("Path", $NewPath, [EnvironmentVariableTarget]::User)
    Write-Host "Added $BinDir to User PATH." -ForegroundColor Green
}

Write-Host "`nMarrow v0.1.0 installed successfully!" -ForegroundColor Green
Write-Host "Restart your terminal and run:" -ForegroundColor Cyan
Write-Host "  marrow --version" -ForegroundColor White