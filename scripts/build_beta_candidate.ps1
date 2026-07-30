[CmdletBinding()]
param(
    [switch]$RequireCleanTree,
    [ValidateSet("closed-beta", "release-candidate")]
    [string]$Channel = "closed-beta"
)

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
$git = Get-Command git -ErrorAction Stop

if ($Channel -eq "release-candidate" -and -not $RequireCleanTree) {
    throw "A release-candidate build requires -RequireCleanTree."
}

function Get-SourceSnapshot {
    param(
        [Parameter(Mandatory = $true)]
        [string]$RepositoryRoot,
        [Parameter(Mandatory = $true)]
        [System.Management.Automation.CommandInfo]$GitCommand
    )

    $sourceFiles = @(
        & $GitCommand.Source -c core.quotepath=false ls-files --cached --others --exclude-standard
    )
    if ($LASTEXITCODE -ne 0 -or $sourceFiles.Count -eq 0) {
        throw "Unable to enumerate the release source tree."
    }

    $resolvedRoot = [System.IO.Path]::GetFullPath($RepositoryRoot).TrimEnd('\')
    $manifestLines = foreach ($relativePath in ($sourceFiles | Sort-Object -Unique)) {
        if ([System.IO.Path]::IsPathRooted($relativePath)) {
            throw "Git returned a rooted source path: $relativePath"
        }

        $sourcePath = [System.IO.Path]::GetFullPath((Join-Path $resolvedRoot $relativePath))
        if (-not $sourcePath.StartsWith("$resolvedRoot\", [System.StringComparison]::OrdinalIgnoreCase)) {
            throw "Git returned a source path outside the repository: $relativePath"
        }
        if (-not (Test-Path -LiteralPath $sourcePath -PathType Leaf)) {
            throw "Release source file is missing: $relativePath"
        }

        $sourceHash = Get-FileHash -LiteralPath $sourcePath -Algorithm SHA256
        "$($sourceHash.Hash)  $($relativePath.Replace('\', '/'))"
    }

    $manifestText = [string]::Join("`n", $manifestLines) + "`n"
    $manifestBytes = [System.Text.Encoding]::UTF8.GetBytes($manifestText)
    $sha256 = [System.Security.Cryptography.SHA256]::Create()
    try {
        $manifestHash = [System.BitConverter]::ToString(
            $sha256.ComputeHash($manifestBytes)
        ).Replace("-", "")
    }
    finally {
        $sha256.Dispose()
    }

    [pscustomobject]@{
        Lines = $manifestLines
        Text = $manifestText
        Hash = $manifestHash
    }
}

Push-Location $repoRoot
try {
    $resolvedCommit = & $git.Source rev-parse --verify HEAD
    if ($LASTEXITCODE -ne 0 -or -not $resolvedCommit) {
        throw "A valid Git commit is required for a closed-beta candidate."
    }
    $commit = $resolvedCommit.Trim()

    $sourceStatus = @(& $git.Source status --porcelain=v1 --untracked-files=all)
    if ($LASTEXITCODE -ne 0) {
        throw "Unable to inspect the release source tree."
    }
    $sourceTreeState = if ($sourceStatus.Count -eq 0) { "clean" } else { "modified" }
    if ($RequireCleanTree -and $sourceTreeState -ne "clean") {
        throw "The release source tree is modified. Commit or remove changes before a clean-tree build."
    }

    $sourceSnapshot = Get-SourceSnapshot -RepositoryRoot $repoRoot -GitCommand $git

    & $pnpm.Source check
    if ($LASTEXITCODE -ne 0) {
        throw "pnpm check failed."
    }

    & $pnpm.Source tauri build --bundles nsis
    if ($LASTEXITCODE -ne 0) {
        throw "Tauri NSIS build failed."
    }

    $postBuildSnapshot = Get-SourceSnapshot -RepositoryRoot $repoRoot -GitCommand $git
    if ($postBuildSnapshot.Hash -ne $sourceSnapshot.Hash) {
        throw "The release source tree changed while the candidate was building."
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
        "docs\steam\TEST_CHECKLIST.md",
        "docs\testing\MANUAL_TESTS.md"
    )
    foreach ($relativePath in $evidenceFiles) {
        $source = Join-Path $repoRoot $relativePath
        if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
            throw "Required release evidence is missing: $relativePath"
        }
        Copy-Item -LiteralPath $source -Destination $candidateDirectory -Force
    }
    Get-ChildItem -LiteralPath (Join-Path $repoRoot "docs\release") -Filter "*.md" |
        ForEach-Object {
            Copy-Item -LiteralPath $_.FullName -Destination $candidateDirectory -Force
        }

    $sourceManifestName = "SOURCE_SHA256SUMS.txt"
    $sourceManifestPath = Join-Path $candidateDirectory $sourceManifestName
    [System.IO.File]::WriteAllText(
        $sourceManifestPath,
        $sourceSnapshot.Text,
        [System.Text.UTF8Encoding]::new($false)
    )

    $windowsDescription = [System.Runtime.InteropServices.RuntimeInformation]::OSDescription
    $windowsVersion = [System.Environment]::OSVersion.Version.ToString()
    $hash = Get-FileHash -LiteralPath $candidateInstaller -Algorithm SHA256
    $timestamp = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    $record = [ordered]@{
        product = "ApriReader"
        version = $version
        channel = $Channel
        architecture = "x64"
        builtAtUtc = $timestamp
        sourceCommit = $commit
        sourceTreeState = $sourceTreeState
        sourceChangedFileCount = $sourceStatus.Count
        sourceManifest = $sourceManifestName
        sourceManifestSha256 = $sourceSnapshot.Hash
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

    Write-Host "Candidate prepared for channel '$Channel':"
    Write-Host "  Installer: $candidateInstaller"
    Write-Host "  SHA-256:   $($hash.Hash)"
    Write-Host "  Evidence:  $candidateDirectory"
}
finally {
    Pop-Location
}
