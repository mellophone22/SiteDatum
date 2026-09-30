-- SiteDatum C3 licensing foundation.
-- This schema is for a dedicated licensing Supabase project. It must never share
-- project records, documents, paths, or the existing optional-sync project.

create schema if not exists licensing;
revoke all on schema licensing from public, anon, authenticated;

create table licensing.customers (
  id uuid primary key default gen_random_uuid(),
  auth_user_id uuid not null unique references auth.users(id) on delete cascade,
  created_at_utc timestamptz not null default clock_timestamp(),
  updated_at_utc timestamptz not null default clock_timestamp()
);

create table licensing.subscriptions (
  id uuid primary key default gen_random_uuid(),
  customer_id uuid not null unique references licensing.customers(id) on delete cascade,
  plan text not null check (plan in ('pro_monthly', 'pro_annual')),
  status text not null check (status in ('active', 'past_due', 'canceled', 'expired')),
  paid_through_utc timestamptz not null,
  source text not null check (source ~ '^[a-z][a-z0-9_]{1,31}$'),
  external_subscription_ref text not null,
  created_at_utc timestamptz not null default clock_timestamp(),
  updated_at_utc timestamptz not null default clock_timestamp(),
  check (length(external_subscription_ref) between 1 and 255)
);

create table licensing.devices (
  id uuid primary key default gen_random_uuid(),
  customer_id uuid not null references licensing.customers(id) on delete cascade,
  fingerprint_hash text not null check (fingerprint_hash ~ '^[0-9a-f]{64}$'),
  activated_at_utc timestamptz not null default clock_timestamp(),
  last_seen_at_utc timestamptz not null default clock_timestamp(),
  deactivated_at_utc timestamptz,
  unique (customer_id, fingerprint_hash)
);

create index licensing_devices_active_customer_idx
  on licensing.devices (customer_id, activated_at_utc)
  where deactivated_at_utc is null;

create table licensing.rate_limit_buckets (
  key_hash text not null check (key_hash ~ '^[0-9a-f]{64}$'),
  action text not null check (action ~ '^[a-z][a-z0-9_]{1,47}$'),
  window_started_at_utc timestamptz not null,
  request_count integer not null check (request_count > 0),
  primary key (key_hash, action)
);

create table licensing.audit_events (
  id bigint generated always as identity primary key,
  customer_id uuid references licensing.customers(id) on delete set null,
  device_id uuid references licensing.devices(id) on delete set null,
  event_type text not null check (event_type in (
    'entitlement_issued',
    'entitlement_denied',
    'device_activated',
    'device_reused',
    'device_deactivated',
    'subscription_projected',
    'rate_limited'
  )),
  outcome text not null check (outcome in ('succeeded', 'denied', 'failed')),
  reason_code text check (
    reason_code is null or reason_code ~ '^[A-Z][A-Z0-9_]{1,63}$'
  ),
  request_id uuid not null,
  occurred_at_utc timestamptz not null default clock_timestamp()
);

create index licensing_audit_events_customer_time_idx
  on licensing.audit_events (customer_id, occurred_at_utc desc);

create index licensing_audit_events_device_time_idx
  on licensing.audit_events (device_id, occurred_at_utc desc)
  where device_id is not null;

alter table licensing.customers enable row level security;
alter table licensing.subscriptions enable row level security;
alter table licensing.devices enable row level security;
alter table licensing.rate_limit_buckets enable row level security;
alter table licensing.audit_events enable row level security;

revoke all on all tables in schema licensing from public, anon, authenticated;
revoke all on all sequences in schema licensing from public, anon, authenticated;

create or replace function public.licensing_upsert_customer(
  p_auth_user_id uuid
)
returns uuid
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_customer_id uuid;
begin
  if p_auth_user_id is null then
    raise exception using errcode = '22004', message = 'LICENSING_AUTH_USER_REQUIRED';
  end if;

  insert into licensing.customers (auth_user_id)
  values (p_auth_user_id)
  on conflict (auth_user_id) do update
    set updated_at_utc = clock_timestamp()
  returning id into v_customer_id;

  return v_customer_id;
end;
$$;

create or replace function public.licensing_project_subscription(
  p_auth_user_id uuid,
  p_plan text,
  p_status text,
  p_paid_through_utc timestamptz,
  p_source text,
  p_external_subscription_ref text,
  p_request_id uuid
)
returns uuid
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_customer_id uuid;
  v_subscription_id uuid;
begin
  v_customer_id := public.licensing_upsert_customer(p_auth_user_id);

  insert into licensing.subscriptions (
    customer_id,
    plan,
    status,
    paid_through_utc,
    source,
    external_subscription_ref
  )
  values (
    v_customer_id,
    p_plan,
    p_status,
    p_paid_through_utc,
    p_source,
    p_external_subscription_ref
  )
  on conflict (customer_id) do update
    set plan = excluded.plan,
        status = excluded.status,
        paid_through_utc = excluded.paid_through_utc,
        source = excluded.source,
        external_subscription_ref = excluded.external_subscription_ref,
        updated_at_utc = clock_timestamp()
  returning id into v_subscription_id;

  insert into licensing.audit_events (
    customer_id, event_type, outcome, request_id
  )
  values (
    v_customer_id, 'subscription_projected', 'succeeded', p_request_id
  );

  return v_subscription_id;
end;
$$;

create or replace function public.licensing_consume_rate_limit(
  p_key_hash text,
  p_action text,
  p_limit integer,
  p_window_seconds integer,
  p_now_utc timestamptz default clock_timestamp()
)
returns boolean
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_count integer;
begin
  if p_limit < 1 or p_window_seconds < 1 then
    raise exception using errcode = '22023', message = 'LICENSING_RATE_LIMIT_INVALID';
  end if;

  insert into licensing.rate_limit_buckets (
    key_hash, action, window_started_at_utc, request_count
  )
  values (p_key_hash, p_action, p_now_utc, 1)
  on conflict (key_hash, action) do update
    set window_started_at_utc = case
          when licensing.rate_limit_buckets.window_started_at_utc
            <= p_now_utc - make_interval(secs => p_window_seconds)
          then p_now_utc
          else licensing.rate_limit_buckets.window_started_at_utc
        end,
        request_count = case
          when licensing.rate_limit_buckets.window_started_at_utc
            <= p_now_utc - make_interval(secs => p_window_seconds)
          then 1
          else licensing.rate_limit_buckets.request_count + 1
        end
  returning request_count into v_count;

  return v_count <= p_limit;
end;
$$;

create or replace function public.licensing_activate_device(
  p_auth_user_id uuid,
  p_fingerprint_hash text,
  p_now_utc timestamptz default clock_timestamp()
)
returns table (
  customer_id uuid,
  device_id uuid,
  activation_state text
)
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_customer_id uuid;
  v_device_id uuid;
  v_active_devices integer;
begin
  v_customer_id := public.licensing_upsert_customer(p_auth_user_id);

  perform 1
  from licensing.customers
  where id = v_customer_id
  for update;

  if not exists (
    select 1
    from licensing.subscriptions s
    where s.customer_id = v_customer_id
      and s.plan in ('pro_monthly', 'pro_annual')
      and s.status in ('active', 'past_due', 'canceled')
      and s.paid_through_utc > p_now_utc
  ) then
    raise exception using errcode = 'P0001', message = 'LICENSING_PRO_REQUIRED';
  end if;

  select id into v_device_id
  from licensing.devices
  where licensing.devices.customer_id = v_customer_id
    and fingerprint_hash = p_fingerprint_hash
    and deactivated_at_utc is null;

  if v_device_id is not null then
    update licensing.devices
    set last_seen_at_utc = p_now_utc
    where id = v_device_id;

    return query select v_customer_id, v_device_id, 'reused'::text;
    return;
  end if;

  select count(*) into v_active_devices
  from licensing.devices
  where licensing.devices.customer_id = v_customer_id
    and deactivated_at_utc is null;

  if v_active_devices >= 2 then
    raise exception using errcode = 'P0001', message = 'LICENSING_DEVICE_LIMIT_REACHED';
  end if;

  insert into licensing.devices (
    customer_id, fingerprint_hash, activated_at_utc, last_seen_at_utc
  )
  values (
    v_customer_id, p_fingerprint_hash, p_now_utc, p_now_utc
  )
  on conflict on constraint devices_customer_id_fingerprint_hash_key do update
    set activated_at_utc = excluded.activated_at_utc,
        last_seen_at_utc = excluded.last_seen_at_utc,
        deactivated_at_utc = null
  returning id into v_device_id;

  return query select v_customer_id, v_device_id, 'activated'::text;
end;
$$;

create or replace function public.licensing_deactivate_device(
  p_auth_user_id uuid,
  p_device_id uuid,
  p_now_utc timestamptz default clock_timestamp()
)
returns boolean
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_changed integer;
begin
  update licensing.devices
  set deactivated_at_utc = p_now_utc,
      last_seen_at_utc = p_now_utc
  where id = p_device_id
    and customer_id = (
      select id
      from licensing.customers
      where auth_user_id = p_auth_user_id
    )
    and deactivated_at_utc is null;

  get diagnostics v_changed = row_count;
  return v_changed = 1;
end;
$$;

create or replace function public.licensing_current_entitlement(
  p_auth_user_id uuid,
  p_device_id uuid
)
returns table (
  subject_id uuid,
  plan text,
  subscription_status text,
  paid_through_utc timestamptz,
  device_id uuid
)
language sql
stable
security definer
set search_path = ''
as $$
  select
    c.auth_user_id,
    s.plan,
    s.status,
    s.paid_through_utc,
    d.id
  from licensing.customers c
  join licensing.subscriptions s on s.customer_id = c.id
  join licensing.devices d on d.customer_id = c.id
  where c.auth_user_id = p_auth_user_id
    and d.id = p_device_id
    and d.deactivated_at_utc is null
  limit 1;
$$;

create or replace function public.licensing_record_audit_event(
  p_auth_user_id uuid,
  p_device_id uuid,
  p_event_type text,
  p_outcome text,
  p_reason_code text,
  p_request_id uuid
)
returns void
language sql
volatile
security definer
set search_path = ''
as $$
  insert into licensing.audit_events (
    customer_id,
    device_id,
    event_type,
    outcome,
    reason_code,
    request_id
  )
  select
    c.id,
    p_device_id,
    p_event_type,
    p_outcome,
    p_reason_code,
    p_request_id
  from licensing.customers c
  where c.auth_user_id = p_auth_user_id;
$$;

create or replace function public.licensing_health()
returns boolean
language sql
stable
security definer
set search_path = ''
as $$
  select to_regclass('licensing.customers') is not null
    and to_regclass('licensing.subscriptions') is not null
    and to_regclass('licensing.devices') is not null;
$$;

revoke all on function public.licensing_upsert_customer(uuid) from public, anon, authenticated;
revoke all on function public.licensing_project_subscription(uuid, text, text, timestamptz, text, text, uuid) from public, anon, authenticated;
revoke all on function public.licensing_consume_rate_limit(text, text, integer, integer, timestamptz) from public, anon, authenticated;
revoke all on function public.licensing_activate_device(uuid, text, timestamptz) from public, anon, authenticated;
revoke all on function public.licensing_deactivate_device(uuid, uuid, timestamptz) from public, anon, authenticated;
revoke all on function public.licensing_current_entitlement(uuid, uuid) from public, anon, authenticated;
revoke all on function public.licensing_record_audit_event(uuid, uuid, text, text, text, uuid) from public, anon, authenticated;
revoke all on function public.licensing_health() from public, anon, authenticated;

grant execute on function public.licensing_upsert_customer(uuid) to service_role;
grant execute on function public.licensing_project_subscription(uuid, text, text, timestamptz, text, text, uuid) to service_role;
grant execute on function public.licensing_consume_rate_limit(text, text, integer, integer, timestamptz) to service_role;
grant execute on function public.licensing_activate_device(uuid, text, timestamptz) to service_role;
grant execute on function public.licensing_deactivate_device(uuid, uuid, timestamptz) to service_role;
grant execute on function public.licensing_current_entitlement(uuid, uuid) to service_role;
grant execute on function public.licensing_record_audit_event(uuid, uuid, text, text, text, uuid) to service_role;
grant execute on function public.licensing_health() to service_role;
