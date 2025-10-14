# PowerShell script to create a zip of easyp.exe with maximum compression
# Usage: .\ziplto.ps1

$sourceFile = "target/lto/easyp.exe"
$cargoTomlPath = "easyp-crate/Cargo.toml"

# Read version from Cargo.toml
if (Test-Path $cargoTomlPath) {
    $cargoContent = Get-Content $cargoTomlPath -Raw
    if ($cargoContent -match 'version\s*=\s*"([^"]+)"') {
        $version = $matches[1]
        Write-Host "Found version: $version"
    } else {
        Write-Error "Could not find version in $cargoTomlPath"
        exit 1
    }
} else {
    Write-Error "Cargo.toml not found at: $cargoTomlPath"
    exit 1
}

$zipFile = "easyp-$version.zip"

# Check if source file exists
if (Test-Path $sourceFile) {
    Write-Host "Creating zip file with maximum compression..."
    
    # Remove existing zip file if it exists
    if (Test-Path $zipFile) {
        Remove-Item $zipFile -Force
        Write-Host "Removed existing zip file: $zipFile"
    }
    
    # Create zip with maximum compression using .NET Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    
    # Create a new zip file and add only the specific file
    $zip = [System.IO.Compression.ZipFile]::Open($zipFile, [System.IO.Compression.ZipArchiveMode]::Create)
    $entry = $zip.CreateEntry("easyp.exe")
    $entryStream = $entry.Open()
    $fileStream = [System.IO.File]::OpenRead($sourceFile)
    $fileStream.CopyTo($entryStream)
    $fileStream.Close()
    $entryStream.Close()
    $zip.Dispose()
    
    Write-Host "Successfully created $zipFile with maximum compression"
    Write-Host "Source file: $sourceFile"
    Write-Host "Zip file size: $((Get-Item $zipFile).Length) bytes"
} else {
    Write-Error "Source file not found: $sourceFile"
    exit 1
}
