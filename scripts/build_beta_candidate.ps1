[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
$packagePath = Join-Path $repoRoot "package.json"
$cargoManifestPath = Join-Path $repoRoot "src-tauri\Cargo.toml"
$package = Get-Content -LiteralPath $packagePath -Raw | ConvertFrom-Json
$version = [string]$package.version

if ($version -notmatch '^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$') {
    throw "package.json contains an invalid semantic version: $version"
}

$cargoVersionLine = Select-String -LiteralPath $cargoManifestPath -Pattern '^version = "([^"]+)"$' | Select-Object -First 1
if (-not $cargoVersionLine -or $cargoVersionLine.Matches[0].Groups[1].Value -ne $version) {
    throw "package.json and src-tauri/Cargo.toml versions do not match."
}

$pnpm = Get-Command pnpm -ErrorAction Stop
$python = Get-Command python -ErrorAction Stop
$git = Get-Command git -ErrorAction SilentlyContinue

Push-Location $repoRoot
try {
    & $pnpm.Source check
    if ($LASTEXITCODE -ne 0) {
        throw "pnpm check failed."
    }

    & $pnpm.Source tauri build --bundles nsis
    if ($LASTEXITCODE -ne 0) {
        throw "Tauri NSIS build failed."
    }

    $bundleDirectory = Join-Path $repoRoot "src-tauri\target\release\bundle\nsis"
    $installer = Get-ChildItem -LiteralPath $bundleDirectory -Filter "*.exe" |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if (-not $installer) {
        throw "The NSIS build completed without an installer artifact."
    }

    $candidatesRoot = Join-Path $repoRoot "release\candidates"
    $candidateDirectory = Join-Path $candidatesRoot "ApriReader-$version-windows-x64"
    if (Test-Path -LiteralPath $candidateDirectory) {
        $resolvedRoot = [System.IO.Path]::GetFullPath($candidatesRoot).TrimEnd('\')
        $resolvedCandidate = [System.IO.Path]::GetFullPath($candidateDirectory)
        if (-not $resolvedCandidate.StartsWith("$resolvedRoot\", [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to replace a candidate outside release\candidates."
        }
        Remove-Item -LiteralPath $resolvedCandidate -Recurse -Force
    }
    New-Item -ItemType Directory -Path $candidateDirectory -Force | Out-Null

    $candidateInstaller = Join-Path $candidateDirectory "ApriReader-$version-windows-x64-setup.exe"
    Copy-Item -LiteralPath $installer.FullName -Destination $candidateInstaller -Force

    $evidenceFiles = @(
        "LICENSE",
        "NOTICE",
        "THIRD_PARTY_NOTICES.md",
        "release\aprireader-sbom.cdx.json",
        "docs\release\CLOSED_BETA_CHECKLIST.md",
        "docs\release\SECURITY_REVIEW.md",
        "docs\release\STEAM_RC_CHECKLIST.md",
        "docs\steam\TEST_CHECKLIST.md"
    )
    foreach ($relativePath in $evidenceFiles) {
        $source = Join-Path $repoRoot $relativePath
        if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
            throw "Required release evidence is missing: $relativePath"
        }
        Copy-Item -LiteralPath $source -Destination $candidateDirectory -Force
    }

    $commit = "UNCOMMITTED_WORKTREE"
    if ($git) {
        $previousErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        $resolvedCommit = & $git.Source rev-parse --verify HEAD 2>$null
        $gitExitCode = $LASTEXITCODE
        $ErrorActionPreference = $previousErrorActionPreference
        if ($gitExitCode -eq 0 -and $resolvedCommit) {
            $commit = $resolvedCommit.Trim()
        }
    }

    $windowsDescription = [System.Runtime.InteropServices.RuntimeInformation]::OSDescription
    $windowsVersion = [System.Environment]::OSVersion.Version.ToString()
    $hash = Get-FileHash -LiteralPath $candidateInstaller -Algorithm SHA256
    $timestamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    $record = [ordered]@{
        product = "ApriReader"
        version = $version
        channel = "closed-beta"
        architecture = "x64"
        builtAtUtc = $timestamp
        sourceCommit = $commit
        windowsCaption = $windowsDescription
        windowsVersion = $windowsVersion
        installer = Split-Path -Leaf $candidateInstaller
        installerSize = (Get-Item -LiteralPath $candidateInstaller).Length
        installerSha256 = $hash.Hash
        signed = $false
        publicProfile = $true
        protectedSteamFilesIncluded = $false
        automatedChecks = "passed"
        externalGates = @(
            "Code-signing certificate and timestamp",
            "Windows 10 closed-beta matrix",
            "Windows 11 closed-beta matrix",
            "Protected Steamworks build and synchronization matrix",
            "Product-owner go/no-go record"
        )
    }
    $record |
        ConvertTo-Json -Depth 4 |
        Set-Content -LiteralPath (Join-Path $candidateDirectory "candidate-record.json") -Encoding utf8

    "$($hash.Hash)  $($record.installer)" |
        Set-Content -LiteralPath (Join-Path $candidateDirectory "SHA256SUMS.txt") -Encoding ascii

    Compress-Archive `
        -Path (Join-Path $candidateDirectory "*") `
        -DestinationPath (Join-Path $repoRoot "release\candidates\ApriReader-$version-windows-x64-evidence.zip") `
        -CompressionLevel Optimal `
        -Force

    Write-Host "Closed-beta candidate prepared:"
    Write-Host "  Installer: $candidateInstaller"
    Write-Host "  SHA-256:   $($hash.Hash)"
    Write-Host "  Evidence:  $candidateDirectory"
}
finally {
    Pop-Location
}
