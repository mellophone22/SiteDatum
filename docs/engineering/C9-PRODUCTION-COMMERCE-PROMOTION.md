# C9 production commerce promotion

**Status:** In progress — production environment, provider configuration, authenticated no-payment Checkout inspection, replacement key custody, and focused acceptance of the staged 1.4.3 production installer are complete. Controlled soft-launch publication and validation of the first genuine customer transaction remain gated.

## Decision

SiteDatum will offer paid Pro subscriptions through a controlled unsigned Windows Early Access channel. Code signing remains a future improvement, not a prerequisite for charging, provided the unsigned status, Windows compatibility risk, canonical installer hash, support route, and refund path remain disclosed before purchase and download.

SiteDatum will not create a founder self-purchase or ask anyone to submit a live payment merely as a test. Stripe sandbox coverage remains the acceptance mechanism for simulated payment, cancellation, refund, failure, and recovery scenarios. Production payment details are reserved for genuine customer purchases. The first genuine customer transaction will be handled as a closely monitored soft-launch event, and broad promotion will remain paused until its receipt, webhook, entitlement, and portal results are verified.

Production commerce is a separate environment from the existing Stripe and Supabase sandboxes. The 1.4.1 sandbox-configured installer is not eligible to become the paid production binary. A new candidate must be built from a clean commit with the `commercial-production` Cargo feature and separate production licensing coordinates.

## Approved architecture

- Stripe-hosted Checkout in subscription mode with Managed Payments enabled.
- Flat-rate Pro Monthly ($15) and Pro Annual ($150) products.
- Accountless local Free tier; an account is required only to purchase, recover, verify, or manage Pro.
- Stripe Customer Portal and Link order management for customer self-service.
- Stripe webhooks are authoritative for paid entitlement state.
- Supabase Auth and Edge Functions form the narrow licensing boundary. Project content never enters Stripe or Supabase.
- Expiration, cancellation, refund, webhook delay, or licensing outage never deletes, moves, hides, or makes existing local records uneditable.

## Environment boundary

The production Supabase project, database, signing key, Stripe webhook endpoint, and Stripe prices must be distinct from sandbox. Server functions require `SITEDATUM_COMMERCE_ENVIRONMENT` and reject a Stripe secret whose mode does not match. They also verify the authenticated Stripe account ID before making billing requests. The desktop production build refuses to compile without an explicit production project URL, publishable key, entitlement verification key, and non-test key ID.

No Stripe secret, Supabase secret key, webhook secret, or entitlement private key is permitted in the desktop, repository, website, release manifest, CI logs, or support messages.

## Production resource checklist

1. Connect and fully activate the live Stripe account; confirm payouts, public business details, support email, statement descriptor, and Managed Payments eligibility.
2. Create live monthly and annual products/prices with the unsigned Early Access name and canonical disclosure URL.
3. Configure Checkout policies, Customer Portal cancellation at period end, payment-method updates, invoice history, and Stripe revenue recovery.
4. Create a dedicated SiteDatum production Supabase project. Apply the reviewed migrations and RLS policies; configure Auth email confirmation and only the canonical SiteDatum redirect URLs.
5. Generate a new production entitlement signing key outside the repository. Store the private key only as a production Edge Function secret and compile the public key/key ID into the production desktop candidate.
6. Configure production Edge Function secrets, including the live restricted Stripe key, expected Stripe account ID, live price IDs, canonical URLs, and an independently generated webhook secret.
7. Deploy functions and register the exact production webhook events. Verify signatures, replay handling, idempotency, delayed delivery, cancellation, refunds, and failed-payment projection.
8. Run a production health check and a live Checkout Session creation without completing payment. Inspect the hosted page for the correct product, amount, renewal terms, unsigned disclosure, support identity, and Managed Payments seller presentation.
9. Build a new unsigned installer with `npm run tauri:build:production`, run the complete C8 acceptance suite, publish a new immutable hash/manifest, upload it without overwriting an earlier installer, and update the canonical Early Access page.
10. Open purchase only as a controlled soft launch after the preceding gates pass. Closely monitor the first genuine customer purchase and verify its receipt, webhook, entitlement, and portal behavior before broad promotion. Exercise cancellation or refund in production only when legitimately requested; sandbox evidence remains authoritative for forced lifecycle scenarios.

## Stop conditions

Do not enable even limited soft-launch purchase or describe the release as paid-production-ready if any of the following is true:

- the connected Stripe account is sandbox-only or not fully activated;
- Managed Payments eligibility or live availability is unresolved;
- the production Supabase project or production entitlement signing key does not exist;
- a live secret, webhook secret, or private signing key appears in source, logs, desktop resources, or the website;
- account ID, price ID, environment, or webhook endpoint validation fails;
- the complete sandbox lifecycle has not demonstrated entitlement grant, cancellation, refund, payment failure/recovery, and safe fallback;
- first-customer monitoring, private support intake, or the purchase-disable rollback path is unavailable;
- the downloadable installer was built for sandbox, differs from the published hash, or lacks the unsigned Early Access disclosures.

Do not begin broad promotion until the first genuine customer transaction has demonstrated a correct receipt, applied webhook, Pro entitlement, and customer-portal route. A founder self-purchase or other contrived live transaction does not satisfy this gate.

## Rollback

Disable the public purchase buttons first, then disable Checkout Session creation while leaving entitlement refresh, portal access, cancellation, refund processing, local editing, backup, and essential export available. Preserve audit records and correlation IDs without collecting project content. Correct forward, repeat the production gates, and publish a new candidate rather than changing an already published installer in place.

## Production checkpoint — October 3, 2026

- Dedicated production Supabase project `dfooiwiukhltiijnzhnb` is active, migrated, and has the licensing Edge Functions deployed.
- Production entitlement signing key ID: `prod-2026-10-03-1`.
- Production Ed25519 public verification key (raw base64): `CZNFz1xGAuNZJLa2bKT5FeB0Y9vtlZhvsr3sI7kuubU=`.
- The private PKCS#8 signing key and reconciliation credential were generated outside the repository and stored as production Edge Function secrets. A Windows-user-encrypted recovery package was created outside the repository; copy it to an independent protected backup before public release.
- The production Supabase secret inventory contains the signing, reconciliation, live Stripe restricted-key, expected-account, live-price, API-version, and canonical return-URL settings. Secret values and private material are not recorded here.
- The commerce boundary requires a least-privilege `rk_live_` Stripe key in production and continues to require an `sk_test_` key in the sandbox. An unrestricted `sk_live_` key is rejected.
- `STRIPE_WEBHOOK_SECRET` was set in production on October 4, 2026 after the production webhook endpoint was registered. Its value was not retrieved or recorded during verification.

## C9-01 production environment boundary — October 5, 2026

- Production Edge Functions require a least-privilege `rk_live_` Stripe key. They reject unrestricted `sk_live_` keys, and the sandbox rejects both restricted and unrestricted live keys.
- The production build remains fail-closed unless its dedicated Supabase origin, publishable key, 32-byte entitlement verification key, and non-test signing-key ID are supplied explicitly.
- The complete inherited release gate passed locally: 17 checks passed, 3 controlled Early Access items remained explicitly deferred, and 0 checks failed.
- No production webhook was registered, no public purchase control was enabled, and no live charge was attempted in this slice.

## C9-02 production provider configuration — October 5, 2026

- The live Stripe webhook is enabled at the production Supabase `stripe-webhook` function. Its event allowlist covers Checkout completion, subscription create/update/delete, invoice paid, invoice payment failed, and invoice payment action required.
- The production Supabase secret inventory confirms that a Stripe webhook signing secret is present. Verification inspected names only; no secret value was retrieved or recorded.
- The production licensing health endpoint returned HTTP 200 with schema version 1. An unsigned webhook request was rejected with HTTP 400 and `SIGNATURE_INVALID`, confirming the public endpoint fails closed.
- The live Early Access product is active with an eligible digital-software tax code, canonical disclosure URL, and $15 monthly and $150 annual recurring prices. Both prices are active and use exclusive tax behavior.
- The default live Customer Portal permits payment-method updates, invoice history, and cancellation at period end without proration. Customer subscription switching remains disabled.
- Stripe Managed Payments is enabled by the server-side Checkout integration. Stripe acts as merchant of record and controls the applicable tax parameters in supported countries, so no conflicting manual `automatic_tax` override was added. Tax obligations outside Managed Payments coverage remain an owner responsibility and must be reviewed before selling into those jurisdictions.
- Supabase security-advisor notices for RLS-enabled tables without public policies were reviewed and accepted because the licensing tables are private, service-managed, and intentionally fail closed. Unused-index notices were retained pending representative production traffic.
- No Checkout Session, customer, subscription, payment, or charge was created during this configuration audit. Public purchase enablement remains gated on the authenticated no-payment Checkout inspection and the remaining lifecycle tests below.

## C9-03 authenticated live Checkout inspection — October 5, 2026

- A production-feature desktop test run authenticated against the dedicated production Supabase project and opened a live Stripe-hosted Checkout Session from the SiteDatum Pro Monthly control.
- The hosted page had no sandbox badge and presented `SiteDatum Pro — Unsigned Windows Early Access` at $15 USD per month, billed monthly, with the canonical Early Access disclosure link visible before payment.
- Stripe reported the session as live, open, and unpaid. The session used subscription mode, the active `sitedatum_pro_monthly` lookup key, quantity one, exclusive tax behavior, Managed Payments, dynamic eligible payment methods, and an integration identifier with a randomized suffix.
- Stripe was the automatic-tax liability provider and required customer location input before calculating tax. The canonical success and cancellation URLs both returned to the SiteDatum Early Access page with the appropriate status query.
- No customer, subscription, invoice, PaymentIntent, payment, or charge was created. No payment information was entered and the Subscribe control was not activated.
- C9-03 passed. Limited purchase enablement remains gated on the production installer acceptance pass and the controlled soft-launch safeguards. Broad promotion remains gated on validation of the first genuine customer transaction.

## Production installer candidate checkpoint — October 6, 2026

- Source commit `8a5e25f1c8bac3de861ae16a88a0ee8389036cc3` synchronizes the desktop, Tauri, Rust, lockfile, and disposable-profile runner at version 1.4.2.
- A clean isolated worktree built `SiteDatum_1.4.2_x64-setup.exe` with the `commercial-production` Cargo feature and the dedicated production Supabase origin, production publishable key, production entitlement public verification key, and key ID `prod-2026-10-03-1`.
- The compiled executable contains the production Supabase origin and does not contain the sandbox origin. No private signing key, Stripe credential, webhook secret, reconciliation credential, or service-role key was supplied to the desktop build.
- The unsigned installer is 4,681,448 bytes with SHA-256 `86CB0CFDC1106F022745875C6DC8DA6BB52C891D0B4C445C9FC6F1AE289EEFA3`; Authenticode reports `NotSigned` as expected for the approved Early Access route.
- Automated acceptance passed: 88 frontend tests, 50 Rust tests compiled with `commercial-production`, lint, production frontend build, eight-page commercial-site validation, release-manifest tests, the high-severity npm audit with zero vulnerabilities, and the full C8 audit with 17 passes, 3 documented deferrals, and 0 failures.
- The installer and publication-candidate manifest are staged locally under the ignored release-candidate workspace. They have not been uploaded, deployed, tagged as a release, or made available to customers. The public 1.4.1 download remains unchanged.
- The Codex application sandbox could not execute the Windows installer because Windows rejected process startup with an isolation-specific `Illegal System DLL Relocation` error before installation. Clean install/startup, upgrade, uninstall/reinstall, existing-record preservation, and the focused keyboard/scaling review therefore remain pending in the established disposable Windows profile.
- No live Checkout submission, payment method, customer, subscription, invoice, PaymentIntent, charge, cancellation, or refund was created during this checkpoint. Public purchase remains disabled.

## Production entitlement-key recovery — October 6, 2026

- The recovery package for entitlement key `prod-2026-10-03-1` could not be located after a read-only search of the expected Windows profiles, normal user folders, temporary storage, Google Drive, the Recycle Bin, and PowerShell destination history. The key was treated as unrecoverable before public release.
- Replacement key `prod-2026-10-06-2` and a replacement reconciliation credential were generated outside the repository. No plaintext secret was printed, committed, included in a build, or retained in the release evidence.
- A passphrase-encrypted AES-256-GCM recovery package was written to separate local and cloud-backed locations. Both copies are byte-identical and were decrypted successfully before production rotation. The encrypted package SHA-256 is `A40678D0997D23D0D8B7AD9D0757AC3F00BD60CE0F39212D87E409DE0B52F819`.
- The production Edge Function key ID, private entitlement signing key, and reconciliation credential were rotated and hash-verified against the encrypted recovery payload. The production licensing health endpoint remained healthy at schema version 1.
- Source commit `2f66af53edfd81cd9128e6742fa2f7765968293c` produced `SiteDatum_1.4.3_x64-setup.exe`, size 4,682,726 bytes, SHA-256 `4518B64BE839A64BF07CA91DA90AB470AEF21284ABB74DA8403FEAAD73958264`, with expected Authenticode status `NotSigned`.
- The 1.4.3 executable contains the production Supabase origin and replacement key ID, excludes the sandbox origin and superseded key ID, and contains no private-key, Stripe-secret, webhook-secret, or reconciliation-secret identifiers.
- Automated acceptance passed: 88 frontend tests, 50 production-feature Rust tests, lint, production build, eight-page site validation, release-manifest tests, zero high-severity npm vulnerabilities, and the full C8 audit with 17 passes, 3 documented deferrals, and 0 failures.
- The installer is staged locally and in the disposable-profile test bundle only. It has not been uploaded, published, tagged, or linked from the website. No Step 4 Checkout submission, customer, subscription, invoice, PaymentIntent, charge, cancellation, or refund was created.

## Production entitlement-transition correction — October 6, 2026

- Disposable-profile acceptance exposed two pre-publication transition defects: an entitlement signed by the superseded key blocked sign-in status, and a connected Free account received a generic service failure when no production Pro subscription existed.
- Source commit `2344f184cd8c4ff6d3fa7e24cbd0a29b5668119c` now fails closed to the Free policy after discarding only an unverifiable cached entitlement credential. Project records, documents, workspace configuration, account credentials, and device identity are not changed.
- The entitlement refresh path now treats `PRO_SUBSCRIPTION_REQUIRED` as an expected Free-account result and reports that no Pro subscription is linked. Other service failures remain errors.
- Automated acceptance passed: 88 frontend tests, 51 production-feature Rust tests, lint, production frontend build, and the production-commerce configuration audit.
- The replacement unsigned installer is 4,684,165 bytes with SHA-256 `CB9853455DC77D397E86E08CD0BE2EF80370148202BF813F78271BBE6FB3E54B`; the compiled executable contains the production origin and replacement key ID, excludes the sandbox origin and superseded key ID, and contains no server-secret identifiers.
- The prior 1.4.3 candidate hash `4518B64BE839A64BF07CA91DA90AB470AEF21284ABB74DA8403FEAAD73958264` is superseded and must not be published.
- Focused reinstall acceptance of this exact replacement candidate passed in the established disposable Windows profile. Version 1.4.3 opened successfully, the existing fictional workspace and records remained intact, the production account remained connected under the Free policy, and refresh returned the expected no-Pro informational result.
- Step 4 live payment lifecycle work remains explicitly out of scope. No Checkout, subscription, invoice, PaymentIntent, charge, cancellation, or refund was created during acceptance.

## Step 4 soft-launch policy — October 6, 2026

- The founder elected not to purchase a SiteDatum subscription from the founder-operated Stripe account solely for validation.
- SiteDatum will follow Stripe's published testing boundary: simulated payment and lifecycle scenarios remain in a sandbox, while live payment details are used only for genuine transactions.
- Step 4 is now a controlled soft launch. Publishing the exact accepted installer and enabling limited purchase require separate explicit authorization; neither action is performed by this policy update.
- The first genuine customer purchase is the production observation point for receipt, webhook, entitlement, and portal behavior. Broad promotion stays paused until those results pass.
- Cancellation and refund paths remain covered by the completed sandbox lifecycle. In production they are exercised only for a legitimate customer request or policy obligation, never to manufacture test evidence.
- If the first genuine transaction fails or produces licensing drift, disable purchase immediately, preserve content-free correlation evidence, keep local Free use available, and follow the rollback runbook before reopening.
