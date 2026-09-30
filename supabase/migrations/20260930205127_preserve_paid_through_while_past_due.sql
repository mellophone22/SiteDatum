-- Stripe can emit customer.subscription.updated with status past_due before
-- invoice.payment_failed. Defend at the subscription boundary so no past-due
-- event can grant a period that has not been paid.

create or replace function licensing.preserve_paid_through_while_past_due()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
  if new.source = 'stripe'
     and new.status = 'past_due'
     and new.paid_through_utc > old.paid_through_utc then
    new.paid_through_utc := old.paid_through_utc;
  end if;
  return new;
end;
$$;

revoke all on function licensing.preserve_paid_through_while_past_due() from public, anon, authenticated;

drop trigger if exists subscriptions_preserve_paid_through_while_past_due
  on licensing.subscriptions;
create trigger subscriptions_preserve_paid_through_while_past_due
before update on licensing.subscriptions
for each row
execute function licensing.preserve_paid_through_while_past_due();
