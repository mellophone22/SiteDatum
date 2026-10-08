create or replace function public.licensing_list_devices(
  p_auth_user_id uuid,
  p_current_fingerprint_hash text
)
returns table (
  device_id uuid,
  activated_at_utc timestamptz,
  last_seen_at_utc timestamptz,
  is_current boolean
)
language plpgsql
stable
security definer
set search_path = ''
as $$
begin
  if p_auth_user_id is null then
    raise exception 'LICENSING_AUTH_REQUIRED';
  end if;
  if p_current_fingerprint_hash is null or p_current_fingerprint_hash !~ '^[0-9a-f]{64}$' then
    raise exception 'LICENSING_DEVICE_FINGERPRINT_INVALID';
  end if;

  return query
  select d.id, d.activated_at_utc, d.last_seen_at_utc,
         d.fingerprint_hash = p_current_fingerprint_hash
  from licensing.devices d
  join licensing.customers c on c.id = d.customer_id
  where c.auth_user_id = p_auth_user_id
    and d.deactivated_at_utc is null
  order by (d.fingerprint_hash = p_current_fingerprint_hash) desc,
           d.last_seen_at_utc desc,
           d.id;
end;
$$;

revoke all on function public.licensing_list_devices(uuid, text) from public, anon, authenticated;
grant execute on function public.licensing_list_devices(uuid, text) to service_role;
