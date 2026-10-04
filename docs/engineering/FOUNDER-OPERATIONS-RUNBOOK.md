# SiteDatum founder operations and incident-response runbook

**Status:** Controlled unsigned Early Access operating baseline — updated 2026-10-03

## Operating boundary

This runbook is for one founder operating SiteDatum's narrow commercial perimeter. It does not authorize access to customer project records or documents. Project names, tasks, RFIs, submittals, notes, contacts, file paths, files, screenshots of private records, and workspace databases must never be copied into Stripe, Supabase, GitLab, Slack, support records, or operational telemetry.

Production-commerce promotion is founder-approved, but it remains gated by `C9-PRODUCTION-COMMERCE-PROMOTION.md`. Until those gates pass, every billing action continues to use the connected Stripe sandbox and the dedicated SiteDatum Supabase sandbox. Confirm the environment indicator before every mutation. Never copy or substitute sandbox and production keys, prices, webhooks, signing keys, or project references across environments.

## Minimum access and custody

- Use an individual operator account protected by a unique password and multi-factor authentication where available.
- Keep Stripe, Supabase, GitLab, signing, and domain credentials out of the repository and Slack.
- Store server-only secrets only in the provider's encrypted secret facility. The desktop and public repository receive public verification material only.
- Do not paste complete tokens, webhook payloads, database rows, customer email addresses, or payment details into an incident record.
- Rotate a credential immediately when exposure is plausible; do not wait for proof of misuse.

## Routine checks

### Before each release-candidate test

1. Run `npm run audit:c8:full` from the candidate commit.
2. Confirm zero failed gates and review every deferred gate.
3. Confirm GitLab frontend, Rust, Supabase, and secret-detection jobs passed for the same commit.
4. Record the commit, version, installer SHA-256, Authenticode status, and test environment in private operator notes.
5. Confirm an unsigned Early Access download remains unavailable unless the canonical page shows the exact candidate version, commit, SHA-256, `NotSigned` status, publication date, and required warning.
6. Generate the content-free staged manifest with `npm run release:manifest -- --source-commit <candidate-commit>` from a clean repository. Keep the output private until disposable-profile acceptance is complete; manifest generation never uploads or publishes the installer.

### Before each sandbox billing test

1. Confirm Stripe visibly reports sandbox/test mode.
2. Confirm the SiteDatum sandbox product and the expected monthly/annual sandbox prices.
3. Confirm the Stripe webhook destination is the sandbox `stripe-webhook` function and recent deliveries return HTTP 200.
4. Confirm `licensing-health` returns `{"status":"ok","schemaVersion":1}`.
5. Use fictional customer details and Stripe test payment methods only.

### Before each production billing action

1. Confirm Stripe visibly reports live mode and the expected account ID; stop if the account, mode, or product differs.
2. Confirm the production SiteDatum Supabase project and production webhook destination. Never use the sandbox project for a live charge.
3. Confirm the exact live product, monthly or annual price, renewal interval, Early Access disclosure, support email, and refund route.
4. Confirm the related deployment and desktop candidate passed the C9 environment checks without printing secrets.
5. Record a content-free private change record and rollback step before creating or changing a live resource.
6. Do not initiate a founder-controlled live purchase until all earlier C9 gates pass. Never use a real customer's identity or payment method for launch testing.

### Weekly while testing is active

1. Review failed Stripe webhook deliveries and the corresponding request IDs.
2. Compare known Stripe sandbox subscriptions with the licensing projection; use reconciliation only when drift is suspected or after a delivery incident.
3. Review Supabase function failures, rate-limit anomalies, and licensing audit outcomes without exporting private rows.
4. Review dependency and secret-detection results from the latest GitLab pipeline.
5. Confirm no legal draft is presented as effective and no unsigned installer is presented as signed, Microsoft-certified, trusted, or generally available.

### Before opening or changing an Early Access download

1. Confirm `npm run audit:c8:full` and the matching GitLab pipeline passed for the exact candidate commit.
2. Calculate SHA-256 from the immutable installer and confirm Authenticode reports `NotSigned`.
3. Confirm the canonical HTTPS release page and the actual checkout both disclose the unsigned status before payment or download.
4. Download the file from its public origin and confirm the downloaded digest exactly matches the published digest.
5. Exercise both outcomes on disposable Windows profiles: the warning path when an override is available and the blocked path when security policy refuses execution.
6. Never tell a customer to disable Defender, Smart App Control, antivirus, or an organization policy. Route a blocked installation to private support and the published refund process.
7. Remove the download immediately for an artifact mismatch, unexpected signature state, malware detection, misleading disclosure, compromised hosting, or failed install/upgrade/rollback test.

## Evidence record

Keep one private, content-free record per test or incident:

```text
Record ID:
Environment: Stripe sandbox / Stripe production / Supabase sandbox / Supabase production / local Windows candidate
Started UTC:
Ended UTC:
Operator:
Candidate commit and version:
Installer SHA-256 and signature status:
Severity:
Observed symptom:
First safe action:
Stripe event ID(s), when relevant:
SiteDatum request ID(s), when relevant:
HTTP delivery status, when relevant:
Entitlement state before/after:
Device count before/after:
Customer-content exposure: none / suspected (escalate immediately)
Resolution or rollback:
Follow-up owner and date:
```

Do not include passwords, tokens, payment details, webhook bodies, customer emails, project content, local paths, or database exports. Provider object IDs belong only in private operator notes, never committed documentation or public issues.

## Severity and first response

| Severity | Definition | First response |
|---|---|---|
| SEV-1 | Data loss/corruption, secret exposure, live-mode contact, cross-customer access, or existing records made inaccessible | Stop all related testing or rollout, preserve non-sensitive evidence, rotate exposed credentials, and follow the rollback/security procedure |
| SEV-2 | Checkout, entitlement, renewal, cancellation, refund, migration, backup, export, or recovery behavior is materially wrong for one or more users | Stop the affected flow, prevent new exposure, identify request/event IDs, reconcile only after cause is understood |
| SEV-3 | Degraded service with a safe recovery path, delayed webhook, transient outage, or isolated UI defect without data risk | Record, monitor, retry safely, and schedule correction before release |
| SEV-4 | Documentation, cosmetic, or operator-process defect with no customer or data impact | Correct in the normal development workflow |

When uncertain, use the higher severity until evidence narrows the impact.

## Incident workflow

1. **Stop:** stop the affected test, deployment, webhook resend, reconciliation, or installer promotion. Do not delete or overwrite anything.
2. **Bound:** determine sandbox versus production, affected version, first/last known occurrence, and whether project content or credentials could be involved.
3. **Preserve:** retain immutable candidate hashes, redacted logs, request IDs, Stripe event IDs, delivery status, and timestamps. Obtain explicit user permission before receiving any workspace database.
4. **Protect:** rotate exposed secrets, revoke compromised sessions, and disable the narrow affected endpoint or distribution route when necessary.
5. **Diagnose:** compare authoritative provider state, the licensing projection, signed entitlement state, and local effective behavior. Do not infer cancellation or entitlement loss from a refund object alone.
6. **Correct:** prefer a forward fix. Use reconciliation only after webhook processing is healthy and the expected authoritative state is known.
7. **Verify:** repeat the smallest safe reproduction, then the relevant C8 matrix row and full preflight.
8. **Close:** record root cause, correction, remaining risk, and follow-up. Do not close while customer access or data integrity is uncertain.

## Billing and entitlement playbooks

### Checkout or portal unavailable

- Confirm licensing health before changing anything.
- Capture the SiteDatum request ID and HTTP status, not the authentication token.
- Confirm the user is authenticated and the request was not rate-limited.
- Confirm only `checkout.stripe.com` or `billing.stripe.com` URLs are returned.
- Do not manually grant Pro because hosted checkout failed. Restore the service or provide a truthful retry window.

### Webhook failed or delayed

- Inspect the event delivery and HTTP response in the explicitly recorded Stripe environment.
- Correct endpoint/configuration failures before using **Resend**.
- Resending the same event must remain idempotent; confirm one provider-ledger record.
- If provider and licensing state still differ after successful delivery, run authenticated reconciliation once and compare attempted versus reconciled counts.
- Never edit subscription rows manually as a convenience repair.

### Payment failure and recovery

- Confirm the subscription becomes past due while retaining the recorded paid-through boundary.
- Confirm existing local records remain editable and backup/export remain available.
- After a successful payment in the recorded environment, refresh entitlement and confirm recovery to the correct Pro plan.
- Do not shorten access merely because an invoice attempt failed when the paid-through period has not ended.

### Cancellation and expiration

- Cancel through the hosted portal or authoritative Stripe control in the recorded environment.
- Confirm cancel-at-period-end preserves Pro through the paid-through date.
- At expiration, confirm existing records remain visible and editable; only new/restored active projects and Pro acceleration features use Free limits.
- If the projection differs from Stripe after healthy webhook delivery, use reconciliation and record the request ID.

### Refund

- Process sandbox refunds only through Stripe Managed Payments while testing. A founder-controlled live validation refund is permitted only under the C9 launch record after the live lifecycle gates are ready.
- A partial or full refund object alone must not revoke entitlement. The authoritative subscription state controls entitlement projection.
- Never request card information through SiteDatum support, GitLab, or Slack.
- Before production launch, the effective refund policy and private billing-support route must be published.

### Device recovery

- A normal reinstall on the same device should reuse its stable device identity and must not consume a third activation.
- For replacement, deactivate the retired device in the matching environment before activating the new one.
- Confirm two active devices are accepted and the third is rejected with a recoverable explanation.
- Never bypass the allowance by editing database rows or shipping a plaintext Pro override.

## Licensing or Supabase outage

- Confirm the health endpoint response and provider status before declaring an outage.
- Do not disable entitlement signature verification or expand desktop network permissions.
- Verified cached Pro entitlement follows the 21-day unavailable-verification grace policy. An invalid signature, expired paid-through date, or explicit non-active state is not an outage grace case.
- Free local work, existing-record editing, backup, and essential export must remain available.
- Communicate what is unavailable, what remains safe, and when the next update will occur; do not promise a resolution time without evidence.

## Security or privacy incident

- Treat suspected secret exposure, cross-customer access, live-mode contact, or project-content collection as SEV-1.
- Stop the affected endpoint or test, rotate relevant secrets, and preserve redacted evidence.
- Do not investigate by copying more customer content.
- Determine what credential or data class was involved, the exposure window, and affected provider resources.
- Obtain qualified legal/privacy guidance before external notification. This runbook does not invent notification deadlines or jurisdictional conclusions.

## Release and data-recovery incident

Use `RELEASE_ROLLBACK_RUNBOOK.md`. Prefer a forward fix after a schema advance. Never run ad-hoc downgrade SQL, silently replace a workspace database, or alter ordinary project documents. Validate any pre-migration snapshot on an operator-controlled copy and retain the newer database until recovery is confirmed.

## Communications

- Public GitLab issues are appropriate only for reproducible software problems stripped of customer and billing information.
- The private customer-facing intake for billing disputes, account or entitlement recovery, and security/privacy reports is `supportsitedatum@protonmail.com`.
- Keep the original message in the private support inbox. Assign a content-free record ID before referring to it in operator notes or Slack.
- Slack may carry only the record ID, severity, environment, owner, status, and next-update time. Never forward or paste the customer's message, email address, secrets, provider payloads, private customer data, payment information, or workspace content.
- If no private internal Slack channel is available, keep triage in the private inbox and private operator notes; never fall back to a public channel.
- Reply from the support address. Ask only for the smallest redacted evidence needed, and never request a workspace database, project document, password, authentication code, or full payment-card information by email.
- Every incident update should state environment, impact, safe workaround (if any), and next update time.

### Support intake triage

1. Classify the message as general support, billing/account, security/privacy, or software defect.
2. Assign a content-free record ID and severity. Do not put the customer's name or email address in the record ID.
3. For a public-safe software defect, reproduce it with fictional data before creating a GitLab issue; do not forward the customer's original message.
4. Keep billing/account and security/privacy cases private. Escalate suspected secret exposure, cross-customer access, live-mode contact, or project-content collection as SEV-1.
5. Record only redacted request/event identifiers that are necessary to diagnose the case. Never store authentication material or payment details.
6. Close the case only after recording the resolution, remaining risk, and any safe customer follow-up.

## Launch-day hold points

Do not open public paid purchase until all are true:

- C8 preflight and GitLab pipelines pass on the immutable release commit;
- disposable-profile install, upgrade, reinstall, offline, backup/export, recovery, keyboard, and 200% scaling tests pass;
- the complete Stripe/Supabase sandbox lifecycle and two-device recovery matrix pass;
- every C9 production account, environment, webhook, entitlement, lifecycle, and rollback gate passes;
- legal policies and publisher/support contacts are effective;
- the unsigned installer warning, immutable hash, canonical download, refund route, and no-security-bypass guidance are visible before purchase and download; and
- the founder can access every credential, provider dashboard, private support channel, and rollback procedure required above.
