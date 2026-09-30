# SiteDatum Free and Pro policy

**Status:** Phase 1 commercial policy baseline — 2026-09-29

## Plans

| Plan | Price | Active projects |
| --- | ---: | ---: |
| Free | $0 | 3 |
| Pro Monthly | $15/month | Unlimited |
| Pro Annual | $150/year | Unlimited |

An active project is a project whose metadata is not archived. Archiving does not move or delete its folder. Restoring an archived project counts as activating it.

## Capability baseline

| Capability | Free | Pro |
| --- | --- | --- |
| Local workspace without an account | Yes | Yes after entitlement is cached |
| Create/restore active projects | Up to 3 | Unlimited |
| View and edit existing projects/records | Yes | Yes |
| Tasks, Attention, notes, contacts, files | Full core use | Full |
| RFI and submittal core lifecycle | Full core use | Full |
| Project Controls and field/commissioning registers | Existing records editable; creation policy to be finalized before gating | Full |
| Saved views and project templates | Basic saved views; final boundary pending usability review | Full |
| Bulk operations | No | Yes |
| CSV/Excel import | No | Yes |
| Filtered CSV/Excel export and advanced reports | Essential export always available; advanced formats/reports pending exact classification | Full |
| Local backup, restore, and recovery | Always available | Always available |
| Global search and keyboard workflow | Full | Full |
| Optional metadata synchronization | No planned Free entitlement | Planned Pro, subject to separate sync review |
| Billing portal and entitlement recovery | Not applicable | Yes |

## Downgrade rules

- Cancellation does not change access until the paid-through timestamp.
- Expiration does not delete, move, hide, redact, or make existing data read-only.
- An expired user with more than three active projects may continue editing every existing project.
- The user may archive projects. Creating a new project or restoring an archived project is allowed only when the resulting active count is three or fewer.
- Existing records created through a Pro capability remain editable.
- Backup, restore, missing-file recovery, and a reasonable data-export path are never paywalled.
- Resubscription restores Pro creation and advanced operations after verified entitlement refresh.

## Policy API target

Future application code should depend on a provider-neutral interface conceptually equivalent to:

```ts
type Plan = "free" | "pro_monthly" | "pro_annual";

getCurrentPlan(): Plan;
getActiveProjectLimit(): number | null;
canActivateProject(currentActiveCount: number): boolean;
canUseFeature(feature: CommercialFeature): boolean;
getEntitlementFreshness(): "verified" | "grace" | "expired";
```

The backend and desktop must enforce consequential limits at their trusted mutation boundaries. Disabled buttons alone are not enforcement.

## Open product decisions before feature gating

1. Exact Project Controls creation available in Free.
2. Which report/export formats are essential portability versus Pro productivity features.
3. Whether optional metadata sync is included in Pro at launch or deferred.
4. Offline entitlement grace interval within the approved 14–30 day range.
5. Supported device/activation count for one individual subscription.
