# ADR-009 — Windows release signing and artifact custody

## Status

Accepted on 2026-10-02.

## Context

SiteDatum is distributed as a Windows desktop application outside the Microsoft Store. Public releases require two independent trust controls:

- Windows Authenticode proves the publisher of the executable and installer; and
- Tauri updater signatures prove that an update artifact was authorized by SiteDatum.

Neither private signing authority may be committed to the public repository, embedded in the desktop client, written to job logs, or shared with billing and licensing services. Release artifacts must be reproducible from a tagged commit and remain immutable after publication.

## Decision

1. SiteDatum uses Azure Artifact Signing with a **Public Trust** certificate profile for Windows Authenticode. Microsoft retains the certificate key; SiteDatum receives signing authorization, not an exportable private certificate.
2. A dedicated Microsoft Entra workload identity receives only the `Artifact Signing Certificate Profile Signer` role scoped to the selected certificate profile. It receives no subscription-wide owner or contributor role.
3. GitLab release jobs run only for protected version tags on an ephemeral GitLab-hosted Windows runner. Azure identity values are protected CI variables. Secrets are masked, never echoed, and unavailable to unprotected branches and merge requests.
4. Tauri invokes `scripts/sign-windows-artifact.ps1` only through the release configuration overlay. Ordinary local development and unsigned verification builds continue to use `tauri.conf.json` and do not require Azure access.
5. Tauri updater signing remains a separate Ed25519 keypair. The public key is embedded in the release build. The encrypted private key is held outside the repository with an independent encrypted recovery copy and is exposed only to protected release jobs.
6. GitLab Generic Package Registry stores immutable, versioned installer and updater artifacts. A GitLab Release links to those versioned objects. The stable updater manifest is promoted only after Authenticode verification, updater-signature verification, hash verification, install/startup smoke, and migration checks pass.
7. Release jobs fail closed when signing configuration is absent or invalid. An unsigned artifact must never be published under a production release tag.

## Consequences

- Azure enrollment, identity validation, an active Public Trust certificate profile, and a narrowly scoped CI identity are prerequisites for a production release.
- Azure authentication material authorizes signing but does not expose the certificate private key.
- New publishers may still receive Microsoft SmartScreen reputation prompts while reputation develops; signing is necessary but does not promise immediate reputation.
- Losing the Tauri updater private key prevents publishing updates that existing installations will accept. The encrypted recovery copy is therefore a launch gate.
- The repository may contain the Azure endpoint, account name, certificate profile name, updater public key, and download URLs. It must not contain Azure client secrets, federated tokens, updater private keys/passwords, or other signing credentials.

