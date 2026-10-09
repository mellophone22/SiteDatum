-- C10-03A: native identity authorization foundation.
--
-- This migration is exercised only in the disposable local proof. It does not
-- enable customer Sync or configure a hosted project.

create function sync_v2_private.uuid_or_null(p_value text)
returns uuid
language plpgsql
immutable
set search_path = ''
as $$
begin
  if p_value is null or p_value = '' then
    return null;
  end if;
  return p_value::uuid;
exception
  when invalid_text_representation then
    return null;
end;
$$;

revoke all on function sync_v2_private.uuid_or_null(text) from public, anon, authenticated;

create or replace function sync_v2_private.jwt_uuid_claim(p_claim text)
returns uuid
language plpgsql
stable
security definer
set search_path = ''
as $$
declare
  claims jsonb;
begin
  -- Keep auth.jwt() inside the protected block. A malformed claims GUC must
  -- deny authorization rather than abort the caller's statement.
  claims := auth.jwt();
  return sync_v2_private.uuid_or_null(claims ->> p_claim);
exception
  when invalid_text_representation then
    return null;
end;
$$;

revoke all on function sync_v2_private.jwt_uuid_claim(text) from public, anon, authenticated;

alter table sync_v2_private.devices
  add constraint sync_v2_devices_owner_identity unique (id, owner_id);

alter table sync_v2_private.sessions
  add column device_id uuid not null,
  add column issued_at timestamptz not null,
  add column last_seen_at timestamptz not null,
  add column idle_expires_at timestamptz not null,
  add column absolute_expires_at timestamptz not null,
  add constraint sync_v2_sessions_device_owner_fk
    foreign key (device_id, owner_id)
    references sync_v2_private.devices (id, owner_id)
    on delete cascade,
  add constraint sync_v2_sessions_time_order
    check (
      issued_at <= last_seen_at
      and last_seen_at < idle_expires_at
      and idle_expires_at <= absolute_expires_at
    );

create or replace function sync_v2_private.request_is_active_owner(p_owner_id uuid)
returns boolean
language sql
stable
security definer
set search_path = ''
as $$
  select exists (
    select 1
    from sync_v2_private.sessions s
    join sync_v2_private.devices d
      on d.id = s.device_id
     and d.owner_id = s.owner_id
    where p_owner_id = (select sync_v2_private.jwt_uuid_claim('sub'))
      and s.id = (select sync_v2_private.jwt_uuid_claim('session_id'))
      and s.device_id = (select sync_v2_private.jwt_uuid_claim('sync_device_id'))
      and s.owner_id = p_owner_id
      and s.revoked_at is null
      and d.revoked_at is null
      and s.idle_expires_at > pg_catalog.now()
      and s.absolute_expires_at > pg_catalog.now()
  );
$$;

revoke all on function sync_v2_private.request_is_active_owner(uuid) from public, anon;
grant execute on function sync_v2_private.request_is_active_owner(uuid) to authenticated;

create function sync_v2_private.custom_access_token_hook(event jsonb)
returns jsonb
language plpgsql
stable
security definer
set search_path = ''
as $$
declare
  claims jsonb;
  event_owner uuid;
  claim_owner uuid;
  event_session uuid;
  bound_device uuid;
begin
  if pg_catalog.jsonb_typeof(event -> 'claims') = 'object' then
    claims := event -> 'claims';
  else
    claims := '{}'::jsonb;
  end if;

  -- Never trust or preserve a device identifier supplied in the event. The
  -- only accepted value is looked up from the server-owned session binding.
  claims := claims - 'sync_device_id';
  event_owner := sync_v2_private.uuid_or_null(event ->> 'user_id');
  claim_owner := sync_v2_private.uuid_or_null(claims ->> 'sub');
  event_session := sync_v2_private.uuid_or_null(claims ->> 'session_id');

  if event_owner is not null
     and claim_owner = event_owner
     and event_session is not null then
    select s.device_id
      into bound_device
      from sync_v2_private.sessions s
      join sync_v2_private.devices d
        on d.id = s.device_id
       and d.owner_id = s.owner_id
     where s.id = event_session
       and s.owner_id = event_owner
       and s.revoked_at is null
       and d.revoked_at is null
       and s.idle_expires_at > pg_catalog.now()
       and s.absolute_expires_at > pg_catalog.now();
  end if;

  if bound_device is not null then
    claims := pg_catalog.jsonb_set(
      claims,
      '{sync_device_id}',
      pg_catalog.to_jsonb(bound_device::text),
      true
    );
  end if;

  return pg_catalog.jsonb_build_object('claims', claims);
end;
$$;

revoke all on function sync_v2_private.custom_access_token_hook(jsonb)
  from public, anon, authenticated;
grant usage on schema sync_v2_private to supabase_auth_admin;
revoke all on all tables in schema sync_v2_private from supabase_auth_admin;
grant execute on function sync_v2_private.custom_access_token_hook(jsonb)
  to supabase_auth_admin;
