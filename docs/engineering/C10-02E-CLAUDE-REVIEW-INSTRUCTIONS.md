# C10-02E independent AI adversarial review instructions

## Reviewer role

Act as an independent, adversarial application-security and cryptographic
protocol reviewer. You did not design or implement this change. Do not assume
that its architecture, tests, comments, or internal preflight conclusions are
correct. Attempt to falsify every security claim using the supplied source.

This is an independent AI second-opinion review, not a professional security
certification. The founder may use it as evidence for a documented risk
decision, but must not represent the product as professionally audited or
independently certified.

## Product boundary

SiteDatum is a single-user, local-first Windows application. Ordinary local
work remains accountless. Project records and documents remain local. The
reviewed C10 material is a test-only foundation for a future optional encrypted
Sync v2 feature. Production Sync must remain disabled throughout this review.

The licensing, billing, and future Sync systems must never receive plaintext
project names, tasks, RFIs, submittals, notes, contacts, file paths, documents,
or workspace database contents. No privileged provider credential, signing
private key, recovery secret, or service-role key may ship in the client.

## Required method

Perform a blind source review before reading
`C10-02E-INDEPENDENT-REVIEW-PACKET.md`. Then read that packet, compare its
internal conclusions with your own, and identify anything it missed.

Review every supplied file. Trace each security control from its entry point to
its final authorization or cryptographic decision. Do not treat a passing test
as proof that an untested property is safe.

At minimum, challenge:

1. algorithm and construction suitability;
2. key generation, storage, derivation, rotation, transfer, revocation, and
   destruction;
3. nonce uniqueness and crash/retry persistence;
4. authenticated-data completeness and canonical-encoding ambiguity;
5. replay, substitution, downgrade, expiry, comparison-code, and version
   handling;
6. OAuth/OIDC authorization-code plus S256 PKCE binding, callback validation,
   token validation, JWKS rotation, logout, revocation, account deletion, and
   device loss;
7. owner, session, and device authorization composition in RLS and any
   privileged function;
8. cross-owner reads, writes, ownership transfer, deletion, and stale-session
   behavior;
9. provider-visible metadata and possible plaintext leakage through logs,
   errors, identifiers, diagnostics, or support artifacts;
10. test/dev dependency or proof-code reachability from a production build;
11. permanent data-loss and unrecoverable-key scenarios; and
12. whether any claimed control exists only in documentation or a model rather
   than enforceable production architecture.

## Important limitations

- The supplied identity issuer and Supabase schema are disposable proofs, not
  production services.
- The record protocol, hosted infrastructure, consent UI, deletion workflow,
  and operational restore system are not yet production implementations.
- A safe proof does not automatically make a later implementation safe.
- Do not request or include real customer information, credentials, provider
  payloads, project records, documents, local file paths, or database contents.

## Required report format

Return a dated Markdown report with:

1. the exact source snapshot identifier or archive SHA-256 supplied by the
   founder;
2. a one-paragraph executive conclusion;
3. findings ordered by severity: critical, high, medium, low, informational;
4. for each finding: affected file and line, root cause, realistic attack or
   failure path, impact, confidence, recommended correction, and retest;
5. explicit sections for confidentiality, integrity, availability, identity,
   RLS, key recovery, deletion/retention, and production reachability;
6. untested assumptions and evidence gaps;
7. residual risks that remain even if every finding is fixed; and
8. exactly one final decision:
   - `APPROVED FOR C10-03 PROTOTYPING`,
   - `APPROVED WITH CONDITIONS FOR C10-03 PROTOTYPING`, or
   - `NOT APPROVED FOR C10-03`.

Do not approve customer release or general availability. The most this review
can approve is beginning the next isolated implementation/prototype package.

Any high or critical finding blocks C10-03 until corrected and re-reviewed.
If evidence is insufficient, say so rather than assuming the control exists.
