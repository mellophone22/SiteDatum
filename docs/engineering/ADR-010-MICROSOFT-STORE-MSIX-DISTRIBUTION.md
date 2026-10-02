# ADR-010 — Microsoft Store MSIX distribution

## Status

Deferred on 2026-10-02 before Partner Center registration or package-identity reservation.

Microsoft's current onboarding guidance reserves Individual developer accounts for non-commercial distribution and directs independent developers operating in relation to a business, trade, or profession to Company accounts. SiteDatum is a commercial product, but no verified business entity is currently available for Company-account enrollment. Public Store packaging therefore remains on hold rather than using an unsuitable account type.

## Context

Public distribution of an unsigned NSIS or MSI installer produces an untrusted Windows installation experience. Azure Artifact Signing would provide managed Authenticode signing for direct downloads but introduces a recurring service charge. Microsoft Store distribution of an MSIX package provides Microsoft-managed signing, hosting, and updates without a recurring code-signing charge.

SiteDatum is a local-first Tauri application. Store distribution must not introduce a cloud requirement for ordinary project work, move project records or documents out of normal local storage, or couple billing identity to project content.

## Decision

The following is the preferred no-recurring-signing-cost design once SiteDatum has a verified publisher entity. It is not yet an active production distribution channel.

1. SiteDatum's public Windows distribution channel is a Microsoft Store MSIX package.
2. Microsoft signs the submitted MSIX after certification and distributes Store updates. SiteDatum does not purchase Azure Artifact Signing while Store distribution remains sufficient.
3. The existing NSIS build remains an internal acceptance and recovery artifact. It is not represented as a trusted public production installer while unsigned.
4. Packaging uses Microsoft's `winapp` CLI with the release executable produced from the verified Tauri commit.
5. `Package.appxmanifest` must use the exact case-sensitive package identity and publisher values assigned in Partner Center after the SiteDatum product name is reserved. No invented identity or placeholder manifest may enter a release.
6. Local MSIX testing may use a self-signed development certificate that is generated and trusted only on disposable operator-controlled machines. Development certificates and passwords remain ignored and must never be published or treated as production trust.
7. Store submission occurs only after the Windows App Certification Kit, clean-profile installation, launch, local persistence, file access, backup/export, migration, uninstall/reinstall, and 200% scaling checks pass.
8. The Store listing and package may describe licensing, but Microsoft Store systems receive no SiteDatum project names, tasks, RFIs, submittals, notes, contacts, file paths, documents, or database contents.

## Consequences

- Development, local acceptance, and GitLab quality verification may continue while public distribution is deferred.
- No Partner Center account, Store product identity, Azure signing resource, production certificate, or public release is created by this decision.
- A free Partner Center developer account, identity verification, product-name reservation, and Store-assigned package identity are prerequisites for generating the committed release manifest.
- MSIX installation and update behavior must be tested independently of the existing NSIS path, especially local database persistence and file-system access.
- The Store controls certification, production signing, hosting, and update rollout for the public MSIX channel.
- Direct public distribution outside the Store still requires a separately trusted signing method and is not authorized by this decision.
- The Tauri updater is not used for the Store package; Store update delivery avoids a second updater-signing key and competing update mechanisms.

