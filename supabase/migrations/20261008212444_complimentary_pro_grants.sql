-- Complimentary SiteDatum Pro access is deliberately separate from paid
-- subscriptions. Grants are attached to the immutable Supabase Auth subject,
-- never to an email address, and remain revocable without deleting local data.

create table licensing.complimentary_grants (
  id uuid primary key default gen_random_uuid(),
  customer_id uuid not null unique references licensing.customers(id) on delete cascade,
  reason_code text not null check (reason_code in (
    'FOUNDER',
    'CORE_TEAM',
    'ADVISOR',
    'EARLY_SUPPORTER'
  )),
  granted_at_utc timestamptz not null default clock_timestamp(),
  revoked_at_utc timestamptz,
  revoked_reason_code text check (
    revoked_reason_code is null or revoked_reason_code in (
      'ACCESS_ENDED',
      'ACCOUNT_COMPROMISED',
      'GRANTED_IN_ERROR',
      'USER_REQUEST'
    )
  ),
  updated_at_utc timestamptz not null default clock_timestamp(),
  check (
    (revoked_at_utc is null and revoked_reason_code is null)
    or (revoked_at_utc is not null and revoked_reason_code is not null)
  )
);

alter table licensing.complimentary_grants enable row level security;
revoke all on licensing.complimentary_grants from public, anon, authenticated;

alter table licensing.audit_events
  drop constraint audit_events_event_type_check;
alter table licensing.audit_events
  add constraint audit_events_event_type_check check (event_type in (
    'entitlement_issued',
    'entitlement_denied',
    'device_activated',
    'device_reused',
    'device_deactivated',
    'subscription_projected',
    'complimentary_grant_created',
    'complimentary_grant_revoked',
    'rate_limited'
  ));

create or replace function public.licensing_grant_complimentary_pro(
  p_auth_user_id uuid,
  p_reason_code text,
  p_request_id uuid
)
returns uuid
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_customer_id uuid;
  v_grant_id uuid;
begin
  if p_auth_user_id is null or p_request_id is null then
    raise exception using errcode = '22004', message = 'LICENSING_GRANT_IDENTITY_REQUIRED';
  end if;

  if p_reason_code not in ('FOUNDER', 'CORE_TEAM', 'ADVISOR', 'EARLY_SUPPORTER') then
    raise exception using errcode = '22023', message = 'LICENSING_GRANT_REASON_INVALID';
  end if;

  v_customer_id := public.licensing_upsert_customer(p_auth_user_id);

  perform 1
  from licensing.customers
  where id = v_customer_id
  for update;

  if exists (
    select 1
    from licensing.subscriptions s
    where s.customer_id = v_customer_id
      and s.status in ('active', 'past_due', 'canceled')
      and s.paid_through_utc > clock_timestamp()
  ) then
    raise exception using errcode = 'P0001', message = 'LICENSING_PAID_ACCESS_EXISTS';
  end if;

  insert into licensing.complimentary_grants (
    customer_id,
    reason_code
  ) values (
    v_customer_id,
    p_reason_code
  )
  on conflict (customer_id) do update
    set reason_code = excluded.reason_code,
        granted_at_utc = clock_timestamp(),
        revoked_at_utc = null,
        revoked_reason_code = null,
        updated_at_utc = clock_timestamp()
  returning id into v_grant_id;

  insert into licensing.audit_events (
    customer_id, event_type, outcome, reason_code, request_id
  ) values (
    v_customer_id, 'complimentary_grant_created', 'succeeded', p_reason_code, p_request_id
  );

  return v_grant_id;
end;
$$;

create or replace function public.licensing_revoke_complimentary_pro(
  p_auth_user_id uuid,
  p_reason_code text,
  p_request_id uuid
)
returns boolean
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_customer_id uuid;
  v_changed integer;
begin
  if p_auth_user_id is null or p_request_id is null then
    raise exception using errcode = '22004', message = 'LICENSING_GRANT_IDENTITY_REQUIRED';
  end if;

  if p_reason_code not in (
    'ACCESS_ENDED', 'ACCOUNT_COMPROMISED', 'GRANTED_IN_ERROR', 'USER_REQUEST'
  ) then
    raise exception using errcode = '22023', message = 'LICENSING_REVOCATION_REASON_INVALID';
  end if;

  select id into v_customer_id
  from licensing.customers
  where auth_user_id = p_auth_user_id;

  update licensing.complimentary_grants
  set revoked_at_utc = clock_timestamp(),
      revoked_reason_code = p_reason_code,
      updated_at_utc = clock_timestamp()
  where customer_id = v_customer_id
    and revoked_at_utc is null;

  get diagnostics v_changed = row_count;
  if v_changed = 1 then
    insert into licensing.audit_events (
      customer_id, event_type, outcome, reason_code, request_id
    ) values (
      v_customer_id, 'complimentary_grant_revoked', 'succeeded', p_reason_code, p_request_id
    );
  end if;

  return v_changed = 1;
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
  v_has_current_pro boolean;
  v_can_report_expiration boolean;
begin
  v_customer_id := public.licensing_upsert_customer(p_auth_user_id);

  perform 1
  from licensing.customers
  where id = v_customer_id
  for update;

  select
    exists (
      select 1
      from licensing.complimentary_grants g
      where g.customer_id = v_customer_id
        and g.revoked_at_utc is null
    ) or exists (
      select 1
      from licensing.subscriptions s
      where s.customer_id = v_customer_id
        and s.plan in ('pro_monthly', 'pro_annual')
        and s.status in ('active', 'past_due', 'canceled')
        and s.paid_through_utc > p_now_utc
    )
  into v_has_current_pro;

  select id into v_device_id
  from licensing.devices
  where licensing.devices.customer_id = v_customer_id
    and fingerprint_hash = p_fingerprint_hash
    and deactivated_at_utc is null;

  select exists (
    select 1
    from licensing.subscriptions s
    where s.customer_id = v_customer_id
      and s.plan in ('pro_monthly', 'pro_annual')
      and s.status = 'expired'
      and s.paid_through_utc > p_now_utc
  ) into v_can_report_expiration;

  if not v_has_current_pro then
    if v_device_id is null or not v_can_report_expiration then
      raise exception using errcode = 'P0001', message = 'LICENSING_PRO_REQUIRED';
    end if;

    update licensing.devices
    set last_seen_at_utc = p_now_utc
    where id = v_device_id;

    return query select v_customer_id, v_device_id, 'reused'::text;
    return;
  end if;

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
  ) values (
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

drop function public.licensing_current_entitlement(uuid, uuid);
create function public.licensing_current_entitlement(
  p_auth_user_id uuid,
  p_device_id uuid
)
returns table (
  subject_id uuid,
  plan text,
  subscription_status text,
  paid_through_utc timestamptz,
  device_id uuid,
  access_kind text
)
language sql
stable
security definer
set search_path = ''
as $$
  select
    c.auth_user_id,
    case when g.id is not null then 'pro_annual' else s.plan end,
    case when g.id is not null then 'active' else s.status end,
    case when g.id is not null then '9999-12-31T23:59:59Z'::timestamptz else s.paid_through_utc end,
    d.id,
    case when g.id is not null then 'complimentary' else 'paid' end
  from licensing.customers c
  join licensing.devices d on d.customer_id = c.id
  left join licensing.complimentary_grants g
    on g.customer_id = c.id and g.revoked_at_utc is null
  left join licensing.subscriptions s on s.customer_id = c.id
  where c.auth_user_id = p_auth_user_id
    and d.id = p_device_id
    and d.deactivated_at_utc is null
    and (g.id is not null or s.id is not null)
  limit 1;
$$;

revoke all on function public.licensing_grant_complimentary_pro(uuid, text, uuid)
  from public, anon, authenticated;
revoke all on function public.licensing_revoke_complimentary_pro(uuid, text, uuid)
  from public, anon, authenticated;
revoke all on function public.licensing_activate_device(uuid, text, timestamptz)
  from public, anon, authenticated;
revoke all on function public.licensing_current_entitlement(uuid, uuid)
  from public, anon, authenticated;

grant execute on function public.licensing_grant_complimentary_pro(uuid, text, uuid)
  to service_role;
grant execute on function public.licensing_revoke_complimentary_pro(uuid, text, uuid)
  to service_role;
grant execute on function public.licensing_activate_device(uuid, text, timestamptz)
  to service_role;
grant execute on function public.licensing_current_entitlement(uuid, uuid)
  to service_role;
