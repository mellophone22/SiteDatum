-- Only active state can ever carry a later paid-through boundary. Stripe's
-- terminal and collection-failure states must retain the last paid period.

create or replace function licensing.preserve_paid_through_while_past_due()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
  if new.source = 'stripe'
     and new.status in ('past_due', 'canceled', 'expired')
     and new.paid_through_utc > old.paid_through_utc then
    new.paid_through_utc := old.paid_through_utc;
  end if;
  return new;
end;
$$;

revoke all on function licensing.preserve_paid_through_while_past_due() from public, anon, authenticated;
