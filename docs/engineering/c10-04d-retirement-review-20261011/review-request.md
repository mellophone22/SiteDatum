# Independent follow-up review request: C10-04D D1 and design corrections

Repository: https://github.com/mellophone22/SiteDatum  
Branch: `codex/c10-02e-claude-review`  
Prior reviewed packet: `8be1f5e8b415b62f9b696adb66a96d7d5e758b7c`  
Original production source base: `097e8daa4459c03655dc5046372aefca0c138d97`

Review the exact follow-up commit supplied in the accompanying message, not a moving branch head. Read [remediation](review-remediation.md), [D1 evidence](d1-proof.md), [proposal](proposals/safe-key-retirement.md), [portfolio](hardening.md), [structured analysis](hardening.json), [context](context.md) and [validation](validation.md).

Act as a critical independent reviewer. Do not change the repository or external services, use customer data, expose credentials, purchase anything or activate Sync. Disposable fictional experiments must be separately reported; AI review is not certification.

## Scope and questions

The source addition is test-only: `src-tauri/src/sync_v2_retirement_census_proof.rs`, its `#[cfg(test)]` module registration and `proofs/sync-v2-retirement-d1/`. No existing production trait, hosted SQL or migration changed. Verify that containment and that no pruning API exists.

- Verify all twelve ciphertext columns, nullable committed bases and conflict candidates, resolved rows, exact owner/workspace/version authentication, fixture-manifest limits, bounds and schema-drift refusal.
- Challenge the experimental sealed read/write role contract and both compile-fail cases. Verify their positive control compiles and failure is E0277 for the intended trait, not unrelated syntax. The production `WorkspaceKeys` trait is unchanged; this is a contract proof, not its migration.
- Verify the historical adapter always returns `None` for write authority and the existing queue refuses it without mutation.
- Challenge query-only and byte-preservation claims; the fixture deliberately populates all stores together and is not a valid production rotation state. No hosted/backup census or historical manifest discovery is claimed.
- Verify OS-2 correction against the actual `public.sync_v2_record_bridge_push_batch`: advisory lock, idempotency lookup, then workspace row lock. OS-1 lock re-keying remains deferred D4 work.
- Verify MP-1 is restated as preservation completeness, with census diagnostic and retained-version reads available. Distributed offline edits/floor rebasing remain unresolved.
- Verify D3's reviewed full-retained-set preservation format and passed 1/8/100/1000-version total-loss drills are hard prerequisites of D6. These drills are proposed, not run.
- Verify owner-root/restore rollback, abort cleanup, vault budget alternatives, diagrams, cloud-deletion scope and indefinite-offline capacity disclosures are accurately classified as corrected specification or outstanding proof.

Return separate verdicts on D1 test proof quality/containment, design-correction fidelity, and remaining D2-D7 gates. No blanket implementation approval. Separate tests you actually run from proposed tests. The founder policies are settled; no new root, storage format, key removal or individual retirement event is authorized. D6 and D7 remain blocked and Option 1 remains operational.
