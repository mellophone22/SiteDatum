# ADR-006 — Commercial licensing boundary

**Status:** Accepted for commercial architecture — 2026-09-29

## Context

SiteDatum 1.4.0 is a local-first, single-user Windows desktop application. Rust owns SQLite persistence and filesystem operations; normal project documents remain Windows files. Optional Supabase metadata synchronization is already implemented, but it uses a manually provisioned user and is not a public customer identity or licensing system.

The commercial objective is a useful accountless Free edition plus recurring Pro subscriptions without turning the project workspace into a SaaS product or making customer work dependent on a network service.

## Decisions

1. **Plans and pricing.** Free supports at most three active projects. Pro Monthly is targeted at USD $15/month and Pro Annual at USD $150/year. Prices and provider product identifiers must be configuration, not scattered application constants.
2. **Accountless Free.** Creating and using a Free workspace does not require registration or connectivity. Identity is introduced only for purchase, entitlement recovery, and customer billing management.
3. **Local-first remains authoritative.** SQLite remains the operational metadata source of truth and Windows files remain the document source of truth. No licensing, billing, analytics, or diagnostic system receives project content.
4. **Central entitlement policy.** Application features ask a single typed policy layer for capabilities and limits. UI components do not inspect provider state or duplicate `isPro` conditions.
5. **Non-destructive downgrade.** Cancellation retains Pro through the paid-through date. After expiration, existing projects and records remain visible and editable, including records created with Pro capabilities. The user may archive projects and may create or restore an active project only when doing so keeps the active count at or below three. Backup and essential export remain available.
6. **Provider-neutral licensing service.** A small trusted service receives verified merchant-of-record webhooks, projects provider state into SiteDatum subscription state, and issues signed/otherwise tamper-resistant entitlements. Provider-specific payloads stay behind an adapter.
7. **No desktop secrets.** Merchant API credentials, webhook signing secrets, entitlement signing private keys, and service-role credentials remain server-side. Secure refresh material and cached entitlements use Windows Credential Manager or an equivalently protected platform facility.
8. **Offline grace.** Pro does not require a network request for every action. The desktop caches a bounded entitlement and reports its verification horizon. The exact grace interval is a pre-implementation founder decision; the architecture supports 14–30 days. Exhausting grace returns to the non-destructive Free policy.
9. **Existing sync is separate.** Optional Supabase metadata sync does not grant Pro and Pro does not require sync. Its current manually provisioned authentication must not be silently reused as commercial customer identity.
10. **Merchant of record.** Lemon Squeezy is the primary evaluation candidate and Paddle is the fallback. Selection remains pending account eligibility, payout support, lifecycle/webhook validation, provider-exit considerations, and test-mode proof.
11. **Updates are a launch gate.** Public paid launch requires a dependable download host, Windows code signing, signed Tauri updater artifacts, migration testing, and rollback instructions. GitHub is not assumed while the account is suspended.
12. **Telemetry is separately gated.** No product analytics is authorized by this ADR. A later proposal may add a small event allowlist with no project content and an explicit privacy disclosure.

## Consequences

- Commercial work can proceed in small packages without moving project data to the cloud.
- The licensing service is security-sensitive and must be tested independently of UI affordances.
- Users can always recover and continue editing their work after payment or connectivity problems.
- Existing Supabase synchronization remains operationally and conceptually independent.
- Legal policies, merchant approval, signing credentials, hosting, and support operations remain founder-owned launch prerequisites.

## Rejected for initial launch

- Requiring an account for Free.
- Hardware-only licensing.
- Trusting a plaintext local plan flag.
- Direct card processing.
- Multiple paid tiers, seats, organizations, or enterprise administration.
- Deleting, hiding, or making existing records read-only after expiration.
- Sending project content to telemetry or support systems.
