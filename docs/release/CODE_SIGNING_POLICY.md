# Code signing policy

ApriReader is applying for the free open-source code-signing service provided
by SignPath.io, with a certificate issued to the SignPath Foundation. Until
that application is approved and a release passes the verification below,
ApriReader artifacts remain explicitly labelled as unsigned.

## Project and team

- Source repository: <https://github.com/Aprilarius/ApriReader>
- Committer and reviewer: [Aprilarius](https://github.com/Aprilarius)
- Release approver: [Aprilarius](https://github.com/Aprilarius)

Contributions from people without direct commit access are accepted only after
review. Access to GitHub and the signing service must be protected with
multi-factor authentication. A signing request must be manually approved and
must originate from the repository's trusted GitHub Actions build.

## What may be signed

Only public ApriReader release artifacts built from this repository may be
submitted for release signing. Third-party binaries are not re-signed as
ApriReader components. Test builds, locally modified binaries, and artifacts
whose source revision cannot be verified must not use the release certificate.

The signing workflow must:

1. start from a tagged, source-controlled revision;
2. run the complete formatting, lint, test, build, security, SBOM, and license
   gate on a GitHub-hosted Windows runner;
3. submit the resulting artifact through SignPath's verified GitHub build
   integration;
4. require manual approval for the release-signing request;
5. verify the Authenticode signature, signer, and trusted timestamp before
   publication; and
6. publish a SHA-256 checksum for the exact signed download.

Changing any byte after signing invalidates the artifact. A failed or missing
signature must stop publication through the signed-release workflow.

## Privacy and network access

ApriReader has no advertising or telemetry and does not send library data in
the background. Network access happens only after a user explicitly requests
an online feature such as metadata search, selected-text translation, or an
optional cloud speech provider. The complete policy is published in
[PRIVACY.md](../../PRIVACY.md).

## Incident response

Suspected compromise of the repository, build workflow, signing account, or a
published artifact must stop new signing requests. The maintainer will review
the affected revision and audit trail, notify SignPath when appropriate, and
replace or withdraw affected downloads. A release is restored only after its
source, build origin, signature, timestamp, and checksum have been verified.
