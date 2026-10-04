-- Permit an already-activated matching device to receive an authoritative
-- expired entitlement while continuing to reject all new device activations.
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

  select exists (
    select 1
    from licensing.subscriptions s
    where s.customer_id = v_customer_id
      and s.plan in ('pro_monthly', 'pro_annual')
      and s.status in ('active', 'past_due', 'canceled')
      and s.paid_through_utc > p_now_utc
  ) into v_has_current_pro;

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

revoke all on function public.licensing_activate_device(uuid, text, timestamptz)
  from public, anon, authenticated;
grant execute on function public.licensing_activate_device(uuid, text, timestamptz)
  to service_role;
