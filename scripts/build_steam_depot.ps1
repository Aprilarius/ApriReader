param(
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
$releaseRoot = Join-Path $projectRoot "release\steam"
$package = Get-Content -Raw -LiteralPath (Join-Path $projectRoot "package.json") | ConvertFrom-Json
$version = [string]$package.version

if ($version -ne "1.3.0") {
    throw "Steam Depot packaging requires final version 1.3.0; found $version."
}

Push-Location $projectRoot
try {
    if (-not $SkipBuild) {
        & pnpm tauri:steam
        if ($LASTEXITCODE -ne 0) {
            throw "Steam profile build failed with exit code $LASTEXITCODE."
        }
    }

    $executable = Join-Path $projectRoot "src-tauri\target\release\aprireader.exe"
    if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
        throw "Steam executable was not produced: $executable"
    }

    $depotName = "ApriReader-$version-windows-x64-depot"
    $depotPath = Join-Path $releaseRoot $depotName
    $archivePath = "$depotPath.zip"
    $expectedParent = [System.IO.Path]::GetFullPath($releaseRoot).TrimEnd('\')
    $actualParent = [System.IO.Path]::GetFullPath((Split-Path -Parent $depotPath)).TrimEnd('\')
    if ($actualParent -ne $expectedParent -or -not $depotName.StartsWith("ApriReader-")) {
        throw "Refusing to replace an unexpected Depot path: $depotPath"
    }

    New-Item -ItemType Directory -Force -Path $releaseRoot | Out-Null
    if (Test-Path -LiteralPath $depotPath) {
        Remove-Item -LiteralPath $depotPath -Recurse -Force
    }
    if (Test-Path -LiteralPath $archivePath) {
        Remove-Item -LiteralPath $archivePath -Force
    }
    New-Item -ItemType Directory -Path $depotPath | Out-Null

    Copy-Item -LiteralPath $executable -Destination (Join-Path $depotPath "aprireader.exe")
    Copy-Item -LiteralPath (Join-Path $projectRoot "LICENSE") -Destination $depotPath
    Copy-Item -LiteralPath (Join-Path $projectRoot "PRIVACY.md") -Destination $depotPath
    Copy-Item -LiteralPath (Join-Path $projectRoot "THIRD_PARTY_NOTICES.md") -Destination $depotPath
    Copy-Item -LiteralPath (Join-Path $projectRoot "release\THIRD_PARTY_LICENSES.md") -Destination $depotPath

    $manifest = [ordered]@{
        product = "ApriReader"
        version = $version
        platform = "windows-x64"
        entrypoint = "aprireader.exe"
        steamAchievements = "local-only-until-protected-app-id-build"
        steamAppIdIncluded = $false
        steamAppIdFileIncluded = $false
    }
    $manifest | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $depotPath "depot-manifest.json") -Encoding utf8

    $hashLines = Get-ChildItem -LiteralPath $depotPath -File |
        Sort-Object Name |
        ForEach-Object {
            $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName).Hash
            "$hash  $($_.Name)"
        }
    $hashLines | Set-Content -LiteralPath (Join-Path $depotPath "SHA256SUMS.txt") -Encoding ascii

    Compress-Archive -Path (Join-Path $depotPath "*") -DestinationPath $archivePath -CompressionLevel Optimal
    $archiveHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $archivePath).Hash
    Write-Host "Steam Depot: $depotPath"
    Write-Host "Archive: $archivePath"
    Write-Host "Archive SHA-256: $archiveHash"
}
finally {
    Pop-Location
}
