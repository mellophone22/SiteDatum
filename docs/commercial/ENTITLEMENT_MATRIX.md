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
| Project Controls and field/commissioning registers | Full manual creation and editing | Full |
| Saved views | Full manual use | Full |
| Project templates | No | Full |
| Bulk operations | No | Yes |
| CSV/Excel import | No | Yes |
| Complete machine-readable CSV export (all core entities, relationships, references, templates, and activity) | Always available | Always available |
| Native Excel export | No | Yes |
| Branded/formatted reports and generated professional deliverables | No | Yes |
| Local backup, restore, and recovery | Always available | Always available |
| Global search and keyboard workflow | Full | Full |
| Optional metadata synchronization | Not in initial public launch | Deferred; may become Pro after redesign/security validation |
| Billing portal and entitlement recovery | Not applicable | Yes |

## Downgrade rules

- Cancellation does not change access until the paid-through timestamp.
- Expiration does not delete, move, hide, redact, or make existing data read-only.
- An expired user with more than three active projects may continue editing every existing project.
- The user may archive projects. Creating a new project or restoring an archived project is allowed only when the resulting active count is three or fewer.
- Existing records created through a Pro capability remain editable.
- Backup, restore, missing-file recovery, and a reasonable data-export path are never paywalled.
- Resubscription restores Pro creation and advanced operations after verified entitlement refresh.

## Offline and device policy

- One individual Pro subscription supports two active Windows computers.
- Reinstalling on the same device should reuse its activation when identity can be established safely.
- Customers must be able to deactivate or replace an old computer without routine founder support.
- A last-known-active entitlement receives up to 21 days of offline verification grace.
- Grace covers inability to verify; it does not extend a paid-through date after authoritative expiration is known.
- Days 1–13 require no interruption, days 14–17 may show a quiet verification notice, and days 18–21 show a stronger recovery notice. Grace exhaustion applies the non-destructive Free policy.

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

## Initial commercial positioning

Free provides the complete SiteDatum manual workflow for three active projects. Pro removes the project limit and adds time-saving templates, bulk operations, advanced interchange, professional outputs, and—only after a separate redesign and security gate—optional metadata synchronization.
