# SiteDatum 1.4.3 publication plan

**State:** Published to the authorized controlled soft-launch channel on October 6, 2026

## Immutable candidate identity

- Source commit: `2344f184cd8c4ff6d3fa7e24cbd0a29b5668119c`
- Installer: `SiteDatum_1.4.3_x64-setup.exe`
- Size: 4,684,165 bytes
- SHA-256: `CB9853455DC77D397E86E08CD0BE2EF80370148202BF813F78271BBE6FB3E54B`
- Authenticode: `NotSigned`
- Production entitlement key ID: `prod-2026-10-06-2`
- Intended canonical URL: `https://sitedatum.site/downloads/SiteDatum_1.4.3_x64-setup.exe`

The local staged installer must remain byte-for-byte identical to this identity. If any source, configuration, packaging input, or executable byte changes, discard this publication plan and generate a new candidate, hash, and manifest.

## Key-rotation evidence

- The unrecoverable `prod-2026-10-03-1` key was superseded before public release.
- The replacement private signing key and reconciliation credential exist only in production Edge Function secrets and a passphrase-encrypted recovery package.
- Two byte-identical encrypted recovery copies were created in separate local and cloud-backed locations and were decrypted successfully before production rotation.
- The production secret hashes were verified after rotation without retrieving or recording the secret values.
- The production licensing health endpoint returned schema version 1 after rotation.
- The compiled application contains the production Supabase origin and replacement key ID, excludes the sandbox origin and prior key ID, and contains no private-key, Stripe-secret, webhook-secret, or reconciliation-secret identifiers.

## Automated acceptance

- 88 frontend tests passed.
- 51 Rust tests passed with `commercial-production` enabled.
- Lint, production frontend build, eight-page commercial-site validation, release-manifest tests, and the high-severity npm audit passed with zero vulnerabilities.
- The complete C8 audit reported 17 passed, 3 documented deferrals, and 0 failed.

## Focused acceptance completed

- The exact replacement 1.4.3 candidate was installed in the established disposable Windows profile.
- About SiteDatum reported 1.4.3 and the fictional workspace and existing records remained available.
- The existing production test account remained connected under the Free policy.
- Refreshing before purchase reported that no Pro subscription is linked and that SiteDatum Free remains available; it did not present the prior generic service failure or open Checkout.
- The broader 1.4.2 upgrade acceptance had already confirmed backup, essential CSV export, ordinary offline/local work, uninstall/reinstall preservation, and record preservation. The 1.4.3 correction changed only licensing-transition handling.

## Public-origin verification

- The exact accepted installer was published at `https://sitedatum.site/downloads/SiteDatum_1.4.3_x64-setup.exe` without replacing the preserved 1.4.1 artifact.
- A fresh public-origin download was 4,684,165 bytes and matched SHA-256 `CB9853455DC77D397E86E08CD0BE2EF80370148202BF813F78271BBE6FB3E54B`.
- The preserved public 1.4.1 download continued to match SHA-256 `CAB68C74EDBE1F8C79780C709701ECA52FE1C711240D3CDE2D186E6062CA28A7`.
- The home, Early Access, release notes, system requirements, support, privacy, terms, and refunds pages each returned HTTP 200 and displayed the 1.4.3 release identity without presenting 1.4.1 as current.

## Controlled soft-launch boundary

The founder separately authorized the controlled soft-launch publication. The public website and immutable installer are now available for limited genuine-customer use. This publication does not authorize a founder self-purchase, synthetic live transaction, or broad promotion.

No founder self-purchase or contrived live transaction is required. Simulated lifecycle acceptance remains in Stripe sandbox. After the exact installer, website disclosures, public-origin hash, private support intake, monitoring, and rollback controls are verified, purchase may open to a limited soft-launch audience. Keep broad promotion paused until the first genuine customer transaction demonstrates the correct receipt, successful webhook projection, Pro entitlement, and customer-portal route. Production cancellation or refund is performed only for a legitimate customer request or policy obligation.
