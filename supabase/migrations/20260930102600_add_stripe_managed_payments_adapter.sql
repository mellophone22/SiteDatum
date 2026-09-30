-- C4 Stripe Managed Payments adapter state. Provider payloads remain outside
-- the database; only opaque provider references and normalized lifecycle data
-- are retained.

alter table licensing.subscriptions
  add column provider_customer_ref text,
  add column provider_updated_at_utc timestamptz;

alter table licensing.subscriptions
  add constraint subscriptions_provider_ref_unique
  unique (source, external_subscription_ref);

create table licensing.checkout_correlations (
  id uuid primary key default gen_random_uuid(),
  auth_user_id uuid not null references auth.users(id) on delete cascade,
  plan text not null check (plan in ('pro_monthly', 'pro_annual')),
  created_at_utc timestamptz not null default clock_timestamp(),
  expires_at_utc timestamptz not null,
  consumed_at_utc timestamptz,
  check (expires_at_utc > created_at_utc)
);

create table licensing.provider_events (
  id bigint generated always as identity primary key,
  provider text not null check (provider ~ '^[a-z][a-z0-9_]{1,31}$'),
  external_event_ref text not null check (length(external_event_ref) between 1 and 255),
  event_type text not null check (length(event_type) between 1 and 127),
  object_ref text not null check (length(object_ref) between 1 and 255),
  payload_sha256 text not null check (payload_sha256 ~ '^[0-9a-f]{64}$'),
  provider_created_at_utc timestamptz not null,
  processing_status text not null check (processing_status in ('applied', 'duplicate', 'ignored_stale', 'rejected')),
  reason_code text check (reason_code is null or reason_code ~ '^[A-Z][A-Z0-9_]{1,63}$'),
  received_at_utc timestamptz not null default clock_timestamp(),
  unique (provider, external_event_ref)
);

create index licensing_checkout_correlations_user_expiry_idx
  on licensing.checkout_correlations (auth_user_id, expires_at_utc desc);
create index licensing_provider_events_object_time_idx
  on licensing.provider_events (provider, object_ref, provider_created_at_utc desc);

alter table licensing.checkout_correlations enable row level security;
alter table licensing.provider_events enable row level security;
revoke all on licensing.checkout_correlations, licensing.provider_events from public, anon, authenticated;
revoke all on all sequences in schema licensing from public, anon, authenticated;

create or replace function public.licensing_create_checkout_correlation(
  p_auth_user_id uuid,
  p_plan text,
  p_now_utc timestamptz default clock_timestamp()
)
returns uuid
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_id uuid;
begin
  if p_plan not in ('pro_monthly', 'pro_annual') then
    raise exception using errcode = '22023', message = 'LICENSING_PLAN_INVALID';
  end if;
  insert into licensing.checkout_correlations (auth_user_id, plan, created_at_utc, expires_at_utc)
  values (p_auth_user_id, p_plan, p_now_utc, p_now_utc + interval '30 minutes')
  returning id into v_id;
  return v_id;
end;
$$;

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

create or replace function public.licensing_stripe_subscription_ref(p_auth_user_id uuid)
returns text
language sql
stable
security definer
set search_path = ''
as $$
  select s.external_subscription_ref
  from licensing.customers c
  join licensing.subscriptions s on s.customer_id = c.id
  where c.auth_user_id = p_auth_user_id and s.source = 'stripe'
  limit 1;
$$;

create or replace function public.licensing_stripe_subscription_refs()
returns table (subscription_ref text)
language sql
stable
security definer
set search_path = ''
as $$
  select external_subscription_ref
  from licensing.subscriptions
  where source = 'stripe';
$$;

revoke all on function public.licensing_create_checkout_correlation(uuid, text, timestamptz) from public, anon, authenticated;
revoke all on function public.licensing_apply_stripe_subscription_event(text, text, text, timestamptz, text, text, uuid, text, text, timestamptz, timestamptz) from public, anon, authenticated;
revoke all on function public.licensing_stripe_subscription_ref(uuid) from public, anon, authenticated;
revoke all on function public.licensing_stripe_subscription_refs() from public, anon, authenticated;
grant execute on function public.licensing_create_checkout_correlation(uuid, text, timestamptz) to service_role;
grant execute on function public.licensing_apply_stripe_subscription_event(text, text, text, timestamptz, text, text, uuid, text, text, timestamptz, timestamptz) to service_role;
grant execute on function public.licensing_stripe_subscription_ref(uuid) to service_role;
grant execute on function public.licensing_stripe_subscription_refs() to service_role;
