# Windows release signing

ApriReader signs public Windows artifacts with Authenticode. The certificate,
private key, password, provider credentials, and timestamp credentials must
never be committed to this repository or copied into release evidence.

## Required credential

Use a currently valid code-signing certificate with an accessible private key
in either `Cert:\CurrentUser\My` or `Cert:\LocalMachine\My`. A self-signed
certificate is suitable only for pipeline testing and does not establish
public Windows trust or SmartScreen reputation.

Set these process-level environment variables before the build:

```powershell
$env:APRIREADER_SIGNING_CERTIFICATE_THUMBPRINT = "40_CHARACTER_SHA1_THUMBPRINT"
$env:APRIREADER_SIGNING_TIMESTAMP_URL = "https://TIMESTAMP_URL_FROM_THE_CERTIFICATE_PROVIDER"
```

If the provider requires RFC 3161 timestamping, pass
`-TimestampUsesRfc3161` directly to `scripts/build_beta_candidate.ps1`.

## Build

The signed GitHub candidate requires a clean Git tree:

```powershell
pnpm github:signed-build
```

The candidate builder creates an ignored temporary Tauri configuration that
contains only the certificate thumbprint, SHA-256 digest algorithm, timestamp
URL, and timestamp protocol mode. Tauri signs both the application executable
and the NSIS installer. The temporary configuration is removed after the build.

The build fails when the certificate is missing, duplicated, expired, lacks an
accessible private key, or when either resulting signature is invalid. It also
requires a trusted timestamp on the installer. `candidate-record.json` records
only public signature metadata: status, signer subject, thumbprint, timestamp
presence, and timestamp signer subject.

## Verification

Verify the exact candidate before publication:

```powershell
Get-AuthenticodeSignature .\release\candidates\ApriReader-VERSION-windows-x64\ApriReader-VERSION-windows-x64-setup.exe |
    Format-List Status, StatusMessage, SignerCertificate, TimeStamperCertificate
```

The required result is `Status: Valid` with both signer and timestamp
certificates present. Installation and Explorer-open smoke tests remain
mandatory because signing does not replace functional release validation.
