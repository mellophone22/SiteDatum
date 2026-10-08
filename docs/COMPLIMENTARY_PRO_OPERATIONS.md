# Complimentary Pro operations

Complimentary Pro is a private licensing grant for founders, core team members, advisors, and selected early supporters. It does not create a Stripe customer or subscription, and it never stores a second copy of the person's email address.

## Grant access

1. Ask the recipient to create a SiteDatum account and confirm the email address.
2. In Supabase Authentication > Users, copy the user's immutable UUID. Do not copy project or device information.
3. In the Supabase SQL editor, run the following as an authorized operator, replacing only the UUID and allowlisted reason:

```sql
select public.licensing_grant_complimentary_pro(
  'AUTH-USER-UUID',
  'CORE_TEAM',
  gen_random_uuid()
);
```

Allowed grant reasons are `FOUNDER`, `CORE_TEAM`, `ADVISOR`, and `EARLY_SUPPORTER`. The operation refuses accounts that still have paid access so a complimentary grant cannot hide an active charge.

The recipient then signs in to SiteDatum and selects **Refresh entitlement**. Complimentary access uses the same two-computer limit, signed token, weekly verification, and non-destructive downgrade rules as paid Pro.

## Revoke access

```sql
select public.licensing_revoke_complimentary_pro(
  'AUTH-USER-UUID',
  'ACCESS_ENDED',
  gen_random_uuid()
);
```

Allowed revocation reasons are `ACCESS_ENDED`, `ACCOUNT_COMPROMISED`, `GRANTED_IN_ERROR`, and `USER_REQUEST`. Revocation is audited. It does not delete, move, hide, or make any local record uneditable. Cached Pro access ends at its normal signed verification boundary.

## Confirmation email delivery

Production authentication email is delivered through Supabase Auth using Resend SMTP. The reviewed template is `supabase/templates/confirmation.html`.

- Keep the Resend API key only in Supabase's hosted SMTP configuration.
- Use a dedicated authenticated sending subdomain and a monitored transactional sender.
- Never place the SMTP password, service-role key, or DNS verification values in the repository.
- Keep the Supabase Site URL and redirect allowlist on `https://sitedatum.site` so confirmation never points customers to localhost.
- Send a real inbox test after any template, sender, DNS, or Auth URL change.
