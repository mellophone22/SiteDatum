# C10-04D D5 — executable ordered-recovery model
Date: 2026-10-11
Base: 6c0dab796a4cc396289f6f4699695d52be471340
Implemented source: 698a84c70a783f6d06776f477aa84058eb62f268
Branch: codex/c10-02e-claude-review

## Outcome
We implemented the bounded D5 model slice, not the complete D5 recovery protocol. The model is excluded from production by cfg(test). It contains fictional key-version inventories only, never key material, customer records or provider credentials. No key removal function, cryptographic format, owner root, roster parser, production schema or product control is added.

I inspected the preservation-first proposal's ordered-recovery and retention-abort requirements, the D2 conditional handoff and current source containment. The preceding hosted counter patch is the only intervening runtime-proof drift from D2's original source base. We retained the no-removal fallback and chose the explicitly permitted D5 model work; unaccepted ForkResolution remains excluded.

## Implemented behavior and explicit assumptions
Two independently committed SQLite files stand for the local journal and hosted floor. Every step stores an expected revision; stale or competing local updates cannot both advance that revision. SQLite is actually opened, committed, closed and reopened. At each ordered boundary the tests interrupt the driver and resume from the stored state. A simulated lost response leaves the durable floor committed while the journal still holds CommitIntent; retry recognizes the exact event instead of reissuing or rolling back it.

The preservation oracle stands for D3: nine historical/staged versions with an unchanged eight-version hot ring. It is NOT a protected vault or recovery format. The hosted commit oracle stands for D4 authentication/admission and is NOT a PostgreSQL fence or distributed consensus protocol. The only terminal success state is AwaitingD6, never pruning or completed retirement. There is no read resolver migration, current-write publication or key-removal capability.

RetentionDependencyExpired and CloudDataDeleted abort causes are stored explicitly. Aborts before commit release fictional fences and void preparations, retaining hot inventories and labelling existing fictional copies as orphaned. After commit, or when the hosted outcome is unavailable, recovery requires stopping modeled Sync writes while retaining local work and the committed floor. An abort never recreates hosted data.

## Executed tests
Twelve new Rust tests:
- restart after every ordered boundary and resume the same event;
- lost hosted response and exact retry;
- abort on both sides of commit with no floor rollback or copy removal;
- changed generation/pin, missing copy or offline member blocks progress;
- newly acquired historical dependency remains preserved;
- stale journal revision and wrong event refuse;
- source containment under cfg(test);
- absent/contradictory hosted evidence refuses final observation;
- a copy of only the hot ring is insufficient;
- two independent SQLite connections compete for one revision, exactly one advances;
- malformed journal refuses without changing hosted state;
- both retention/deletion abort causes persist across reopening.

Final Windows Rust suite: 226 passed, zero failed, five ignored credential-writing drills not executed. cargo fmt --check passed. Production cargo check --locked --lib passed. All 114 Vitest tests passed across 35 files; npm lint and TypeScript/Vite build passed. No screen changed, so visual review is not applicable. The existing build-cache hard-link fallback warning is non-fatal.

## Limits and next integration gates
This is bounded example/model testing, not formal exhaustive model checking. Connection reopening and injected transition interruption are not an OS power-loss test. No multi-process hosted contention, malicious provider/fork protocol, signature/canonical encoding, Windows protected-store durability, D3 total-loss recovery, unsynced rebase, client negotiation or global event-admission property is proved. JSON is internal test-fixture storage, not an assigned wire or recovery format. No performance or storage budget is inferred from these small fixtures.

Local SQLite CAS does not prove global uniqueness across devices. The model's fences are state markers, not locks on real project edits. D2 canonical encoding/verifier, D3 reviewed preservation/custody and D4 authenticated fence/admission remain prerequisites for translating the model into production D5 orchestration. D6 and D7 remain blocked.

We can remove the test module without a customer migration. All uniquely created fictional temporary databases are cleaned up by the harness; no customer file is touched. This commit does not merge main, release, deploy, alter billing or activate Sync.
