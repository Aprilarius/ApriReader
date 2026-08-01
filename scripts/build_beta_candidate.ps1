[CmdletBinding()]
param(
    [switch]$RequireCleanTree,
    [switch]$RequireSignature,
    [string]$SigningCertificateThumbprint = $env:APRIREADER_SIGNING_CERTIFICATE_THUMBPRINT,
    [string]$TimestampUrl = $env:APRIREADER_SIGNING_TIMESTAMP_URL,
    [switch]$TimestampUsesRfc3161,
    [ValidateSet("closed-beta", "release-candidate", "github-release")]
    [string]$Channel = "closed-beta"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
$packagePath = Join-Path $repoRoot "package.json"
$cargoManifestPath = Join-Path $repoRoot "src-tauri\Cargo.toml"
$package = Get-Content -LiteralPath $packagePath -Raw | ConvertFrom-Json
$version = [string]$package.version
$signingConfigPath = $null

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

if ($Channel -in @("release-candidate", "github-release") -and -not $RequireCleanTree) {
    throw "Release-candidate and GitHub release builds require -RequireCleanTree."
}

if ($RequireSignature) {
    if (-not [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
        [System.Runtime.InteropServices.OSPlatform]::Windows
    )) {
        throw "Authenticode signing is supported only on Windows."
    }

    $normalizedThumbprint = ($SigningCertificateThumbprint -replace '\s', '').ToUpperInvariant()
    if ($normalizedThumbprint -notmatch '^[0-9A-F]{40}$') {
        throw "Set APRIREADER_SIGNING_CERTIFICATE_THUMBPRINT to a 40-character SHA-1 certificate thumbprint."
    }

    $timestampUri = $null
    if (
        -not [System.Uri]::TryCreate($TimestampUrl, [System.UriKind]::Absolute, [ref]$timestampUri) -or
        $timestampUri.Scheme -notin @("http", "https")
    ) {
        throw "Set APRIREADER_SIGNING_TIMESTAMP_URL to the HTTP(S) timestamp service supplied by the certificate provider."
    }

    $matchingCertificates = @(
        Get-ChildItem Cert:\CurrentUser\My, Cert:\LocalMachine\My -CodeSigningCert |
            Where-Object Thumbprint -EQ $normalizedThumbprint
    )
    if ($matchingCertificates.Count -ne 1) {
        throw "The requested code-signing certificate was not found exactly once in the Windows certificate stores."
    }

    $signingCertificate = $matchingCertificates[0]
    $now = Get-Date
    if (-not $signingCertificate.HasPrivateKey) {
        throw "The requested code-signing certificate has no accessible private key."
    }
    if ($now -lt $signingCertificate.NotBefore -or $now -gt $signingCertificate.NotAfter) {
        throw "The requested code-signing certificate is not currently valid."
    }

    $signingConfigDirectory = Join-Path $repoRoot "src-tauri\target\.tauri"
    New-Item -ItemType Directory -Path $signingConfigDirectory -Force | Out-Null
    $signingConfigPath = Join-Path $signingConfigDirectory "aprireader-signing.json"
    $signingConfig = [ordered]@{
        bundle = [ordered]@{
            windows = [ordered]@{
                certificateThumbprint = $normalizedThumbprint
                digestAlgorithm = "sha256"
                timestampUrl = $timestampUri.AbsoluteUri
                tsp = [bool]$TimestampUsesRfc3161
            }
        }
    }
    [System.IO.File]::WriteAllText(
        $signingConfigPath,
        ($signingConfig | ConvertTo-Json -Depth 4),
        [System.Text.UTF8Encoding]::new($false)
    )
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

    $tauriBuildArguments = @("tauri", "build", "--bundles", "nsis", "--ci")
    if ($RequireSignature) {
        $tauriBuildArguments += @("--config", $signingConfigPath)
    }
    & $pnpm.Source @tauriBuildArguments
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

    $applicationExecutable = Join-Path $repoRoot "src-tauri\target\release\aprireader.exe"
    if (-not (Test-Path -LiteralPath $applicationExecutable -PathType Leaf)) {
        throw "The Tauri build completed without the application executable."
    }

    $applicationSignature = Get-AuthenticodeSignature -LiteralPath $applicationExecutable
    $installerSignature = Get-AuthenticodeSignature -LiteralPath $installer.FullName
    $isSigned = (
        $applicationSignature.Status -eq [System.Management.Automation.SignatureStatus]::Valid -and
        $installerSignature.Status -eq [System.Management.Automation.SignatureStatus]::Valid
    )
    if ($RequireSignature -and -not $isSigned) {
        throw "Authenticode verification failed for the application executable or NSIS installer."
    }
    if (
        $RequireSignature -and
        (
            $applicationSignature.SignerCertificate.Thumbprint -ne $normalizedThumbprint -or
            $installerSignature.SignerCertificate.Thumbprint -ne $normalizedThumbprint
        )
    ) {
        throw "The application executable or NSIS installer was signed with an unexpected certificate."
    }
    $isTimestamped = [bool](
        $applicationSignature.TimeStamperCertificate -and
        $installerSignature.TimeStamperCertificate
    )
    if ($RequireSignature -and -not $isTimestamped) {
        throw "The application executable or NSIS installer signature does not contain a trusted timestamp."
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
        "release\THIRD_PARTY_LICENSES.md",
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
        releaseScope = if ($Channel -eq "github-release" -and $isSigned) {
            "Signed public GitHub build without Steamworks."
        } elseif ($Channel -eq "github-release") {
            "Public GitHub build without Steamworks; unsigned installer accepted by product owner."
        } else {
            "Pre-release validation artifact."
        }
        windowsCaption = $windowsDescription
        windowsVersion = $windowsVersion
        installer = Split-Path -Leaf $candidateInstaller
        installerSize = (Get-Item -LiteralPath $candidateInstaller).Length
        installerSha256 = $hash.Hash
        signed = $isSigned
        applicationSignatureStatus = [string]$applicationSignature.Status
        installerSignatureStatus = [string]$installerSignature.Status
        signerSubject = if ($isSigned) { $installerSignature.SignerCertificate.Subject } else { $null }
        signerThumbprint = if ($isSigned) { $installerSignature.SignerCertificate.Thumbprint } else { $null }
        timestamped = $isTimestamped
        timestampSignerSubject = if ($installerSignature.TimeStamperCertificate) {
            $installerSignature.TimeStamperCertificate.Subject
        } else {
            $null
        }
        publicProfile = $true
        protectedSteamFilesIncluded = $false
        automatedChecks = "passed"
        externalGates = if ($Channel -eq "github-release" -and $isSigned) {
            @(
                "Separate protected Steamworks release and synchronization matrix"
            )
        } elseif ($Channel -eq "github-release") {
            @(
                "Future code-signing certificate and timestamp",
                "Separate protected Steamworks release and synchronization matrix"
            )
        } else {
            @(
                "Code-signing certificate and timestamp",
                "Windows 10 closed-beta matrix",
                "Windows 11 closed-beta matrix",
                "Protected Steamworks build and synchronization matrix",
                "Product-owner go/no-go record"
            )
        }
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
    if ($signingConfigPath -and (Test-Path -LiteralPath $signingConfigPath -PathType Leaf)) {
        Remove-Item -LiteralPath $signingConfigPath -Force
    }
    Pop-Location
}
