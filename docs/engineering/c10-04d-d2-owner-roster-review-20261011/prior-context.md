# Prior policy and gate context

We preserve the five founder policies: every active device acknowledges; historical recovery files remain usable for captured material; old backups stay untouched and recoverable; each event needs explicit consent; membership is complete owner-approved history, not a provider-only list.

The [corrected preservation-first proposal](https://github.com/mellophone22/SiteDatum/blob/91973f57d9d4f9033dcf3455299046784ddf9742/docs/engineering/c10-04d-retirement-review-20261011/proposals/safe-key-retirement.md) adopts preservation completeness, not dependency stability. D1 is a test-only local census/role contract proof. D2 is specification only; no schema is authorized. D3's reviewed full-retained-set preservation format and actual total-loss drills are hard prerequisites of D6. D4 fences and D5 ordered recovery remain unproved. D6 mutation and D7 activation remain blocked.

The supplied 2026-10-11 review identifies MP-2: existing device signing keys do not automatically form an independent owner root, and restoring all SQLite anchors from pre-revocation backups can resurrect stale membership unless independent pins survive. Even outside-database storage cannot establish freshness after total environmental loss/rewind. This packet specifies the proposed response and its residual limit, not an executed fix.
