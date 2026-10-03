# ADR-011 — Controlled unsigned Windows Early Access

## Status

Accepted by the founder on 2026-10-03. This decision supplements the deferred Microsoft Store route in `ADR-010-MICROSOFT-STORE-MSIX-DISTRIBUTION.md`; it does not describe the unsigned build as trusted, signed, certified, or generally available.

## Context

SiteDatum is operated by an individual developer who does not currently qualify for the intended Microsoft Store Company-account path and does not want to incur a recurring signing-service expense. Waiting for that route would prevent limited customer evaluation even though the local-first application, installer, recovery behavior, billing sandbox, and support boundary have substantial acceptance evidence.

An unsigned NSIS installer has material limitations. Windows Defender SmartScreen may show an unknown-publisher or reputation warning. Smart App Control, an organization policy, or another security product may block execution without offering an override. Explaining those limits does not make the installer trusted and must never be used to pressure a customer to weaken device security.

## Decision

SiteDatum may offer an **unsigned Windows Early Access** build under all of these controls:

1. The build is labeled `Early Access` everywhere it is offered. It is never described as signed, Microsoft-certified, trusted, generally available, or suitable for every managed computer.
2. The unsigned status and the possibility of a warning or complete policy block are disclosed before purchase, again before download, and in the installation guide.
3. Customers are told not to disable Microsoft Defender, Smart App Control, antivirus, or organizational security policy. If Windows or an administrator blocks installation, support does not instruct the customer to bypass that control.
4. Each release is built from an identified clean commit after the complete local C8 gate and the matching GitLab pipeline pass. The published page identifies the version, commit, SHA-256 digest, Authenticode status, and publication date.
5. The installer is delivered only from the canonical HTTPS SiteDatum domain or an immutable GitLab release owned by the SiteDatum publisher. Redirects, URL shorteners, mirrors, chat attachments, and emailed executables are not supported distribution channels.
6. The published installer is byte-for-byte identical to the verified candidate. Any rebuild, repackaging, or file change creates a new candidate and requires new evidence and a new digest.
7. Automatic application updating is not enabled for the unsigned channel. Customers obtain a newly verified installer manually; upgrade and rollback instructions must identify the exact version and digest.
8. A customer whose device or organizational policy blocks the unsigned installer may request private installation support or a refund through the published merchant workflow. Project content, payment-card data, credentials, and security-policy details are not requested.
9. Public purchase and download remain closed until the release page contains the real immutable artifact information, pre-purchase disclosure is present in the actual checkout journey, and the download/install path is re-tested from the public origin.
10. A trusted signed route remains the goal for general availability. Microsoft Store distribution or another separately approved Authenticode route can replace this Early Access channel without changing SiteDatum's local-first data boundary.

## Consequences

- Early Access customers may encounter significant installation friction or be unable to install SiteDatum.
- Disclosure reduces surprise but does not create publisher identity, executable integrity, or Windows reputation. The SHA-256 digest detects a mismatch only when the customer obtains the expected digest from the authentic SiteDatum page.
- Support and refunds must account for policy-blocked installation.
- The founder must treat any artifact mismatch, unexpected signature state, malware detection, compromised hosting, or misleading release wording as a release stop condition.
- C8-15 changes from a signing-only gate to a controlled-distribution gate for Early Access. Trusted signing remains a later general-availability gate.
