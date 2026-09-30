-- Stripe can deliver checkout, subscription, and invoice events concurrently.
-- Serialize projection per subscription so the first event can consume the
-- one-time checkout correlation before later events re-read the new mapping.

create or replace function public.licensing_apply_stripe_subscription_event(
  p_event_ref text,
  p_event_type text,
  p_payload_sha256 text,
  p_provider_created_at_utc timestamptz,
  p_subscription_ref text,
  p_customer_ref text,
  p_correlation_id uuid,
  p_plan text,
  p_status text,
  p_paid_through_utc timestamptz,
  p_provider_updated_at_utc timestamptz
)
returns text
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_auth_user_id uuid;
  v_customer_id uuid;
  v_current_updated timestamptz;
begin
  if p_plan not in ('pro_monthly', 'pro_annual')
     or p_status not in ('active', 'past_due', 'canceled', 'expired') then
    raise exception using errcode = '22023', message = 'LICENSING_PROJECTION_INVALID';
  end if;

  perform pg_catalog.pg_advisory_xact_lock(pg_catalog.hashtextextended(p_subscription_ref, 0));

  if exists (select 1 from licensing.provider_events where provider = 'stripe' and external_event_ref = p_event_ref) then
    return 'duplicate';
  end if;

  select s.provider_updated_at_utc, s.customer_id
    into v_current_updated, v_customer_id
  from licensing.subscriptions s
  where s.source = 'stripe' and s.external_subscription_ref = p_subscription_ref
  for update;

  if v_customer_id is null then
    select c.auth_user_id into v_auth_user_id
    from licensing.checkout_correlations c
    where c.id = p_correlation_id
      and c.expires_at_utc >= p_provider_created_at_utc
      and c.consumed_at_utc is null
      and c.plan = p_plan
    for update;
    if v_auth_user_id is null then
      insert into licensing.provider_events (
        provider, external_event_ref, event_type, object_ref, payload_sha256,
        provider_created_at_utc, processing_status, reason_code
      ) values ('stripe', p_event_ref, p_event_type, p_subscription_ref, p_payload_sha256,
        p_provider_created_at_utc, 'rejected', 'CORRELATION_INVALID');
      return 'rejected';
    end if;
    v_customer_id := public.licensing_upsert_customer(v_auth_user_id);
    update licensing.checkout_correlations set consumed_at_utc = clock_timestamp() where id = p_correlation_id;
  end if;

  if v_current_updated is not null and p_provider_updated_at_utc < v_current_updated then
    insert into licensing.provider_events (
      provider, external_event_ref, event_type, object_ref, payload_sha256,
      provider_created_at_utc, processing_status, reason_code
    ) values ('stripe', p_event_ref, p_event_type, p_subscription_ref, p_payload_sha256,
      p_provider_created_at_utc, 'ignored_stale', 'STALE_PROVIDER_STATE');
    return 'ignored_stale';
  end if;

  insert into licensing.subscriptions (
    customer_id, plan, status, paid_through_utc, source,
    external_subscription_ref, provider_customer_ref, provider_updated_at_utc
  ) values (
    v_customer_id, p_plan, p_status, p_paid_through_utc, 'stripe',
    p_subscription_ref, p_customer_ref, p_provider_updated_at_utc
  )
  on conflict (customer_id) do update set
    plan = excluded.plan,
    status = excluded.status,
    paid_through_utc = excluded.paid_through_utc,
    source = excluded.source,
    external_subscription_ref = excluded.external_subscription_ref,
    provider_customer_ref = excluded.provider_customer_ref,
    provider_updated_at_utc = excluded.provider_updated_at_utc,
    updated_at_utc = clock_timestamp();

  insert into licensing.provider_events (
    provider, external_event_ref, event_type, object_ref, payload_sha256,
    provider_created_at_utc, processing_status
  ) values ('stripe', p_event_ref, p_event_type, p_subscription_ref, p_payload_sha256,
    p_provider_created_at_utc, 'applied');
  return 'applied';
end;
$$;
