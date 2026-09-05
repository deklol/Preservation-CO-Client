# Source: @digitalm1nd on x.com / _dek on Discord — Preservation Conquer project — https://discord.gg/CvKPXEHYRY
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$outputDirectory = Join-Path $projectRoot 'dist'
New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null
$outputPath = Join-Path $outputDirectory 'Preservation-Conquer-Skeleton-Source.zip'
$files = @('Cargo.toml','Cargo.lock','README.md','CONTRIBUTING.md','LICENSE','NOTICE','character.ini','.gitignore','tools/package-source.ps1','docs/images/logo.png','docs/movement.md') | ForEach-Object {Get-Item -LiteralPath (Join-Path $projectRoot $_)}
$files += Get-ChildItem -LiteralPath (Join-Path $projectRoot 'crates') -Recurse -File | Where-Object {$_.Extension -in '.rs','.wgsl','.toml'}
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$stream = [IO.File]::Open($outputPath,[IO.FileMode]::Create)
$zip = [IO.Compression.ZipArchive]::new($stream,[IO.Compression.ZipArchiveMode]::Create)
try {
    foreach ($file in $files) {
        $relative = $file.FullName.Substring($projectRoot.Length + 1).Replace('\','/')
        [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip,$file.FullName,$relative,[IO.Compression.CompressionLevel]::Optimal) | Out-Null
    }
} finally {
    $zip.Dispose()
    $stream.Dispose()
}
Get-Item -LiteralPath $outputPath | Select-Object FullName,Length
