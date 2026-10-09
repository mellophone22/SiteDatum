-- C10-03C: service-only database boundary for the authenticated enrollment
-- Edge Function. This remains a disposable local proof and does not enable
-- desktop Sync or create a hosted resource.

create table sync_v2_private.enrollment_bridge_windows (
  owner_id uuid not null,
  auth_session_id uuid not null,
  action text not null check (action in ('begin', 'complete')),
  window_started_at timestamptz not null,
  request_count smallint not null check (request_count between 1 and 30),
  last_request_at timestamptz not null,
  primary key (owner_id, auth_session_id, action, window_started_at)
);

alter table sync_v2_private.enrollment_bridge_windows enable row level security;
alter table sync_v2_private.enrollment_bridge_windows force row level security;

revoke all on table sync_v2_private.enrollment_bridge_windows
  from public, anon, authenticated, service_role;

create function sync_v2_private.auth_session_is_live(
  p_owner_id uuid,
  p_auth_session_id uuid
)
returns boolean
language sql
stable
security definer
set search_path = ''
as $$
  select exists (
    select 1
      from auth.sessions s
     where s.id = p_auth_session_id
       and s.user_id = p_owner_id
  );
$$;

create function sync_v2_private.consume_enrollment_bridge_limit(
  p_owner_id uuid,
  p_auth_session_id uuid,
  p_action text,
  p_limit smallint
)
returns boolean
language plpgsql
volatile
security definer
set search_path = ''
as $$
declare
  now_at timestamptz := pg_catalog.clock_timestamp();
  window_at timestamptz := pg_catalog.date_bin(
    interval '5 minutes',
    now_at,
    '2000-01-01 00:00:00+00'::timestamptz
  );
  accepted_count smallint;
begin
  if p_owner_id is null
     or p_auth_session_id is null
     or p_action is null
     or p_action not in ('begin', 'complete')
     or p_limit is null
     or p_limit < 1
     or p_limit > 30 then
    return false;
  end if;

  if not sync_v2_private.auth_session_is_live(p_owner_id, p_auth_session_id) then
    return false;
  end if;

  perform pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended(
      p_owner_id::text || ':' || p_auth_session_id::text || ':' || p_action,
      0
    )
  );

  delete from sync_v2_private.enrollment_bridge_windows w
   where w.owner_id = p_owner_id
     and w.window_started_at < now_at - interval '24 hours';

  insert into sync_v2_private.enrollment_bridge_windows (
    owner_id, auth_session_id, action, window_started_at, request_count, last_request_at
  ) values (
    p_owner_id, p_auth_session_id, p_action, window_at, 1, now_at
  )
  on conflict (owner_id, auth_session_id, action, window_started_at)
  do update
     set request_count = sync_v2_private.enrollment_bridge_windows.request_count + 1,
         last_request_at = excluded.last_request_at
   where sync_v2_private.enrollment_bridge_windows.request_count < p_limit
  returning request_count into accepted_count;

  return accepted_count is not null;
end;
$$;

create function public.sync_v2_enrollment_bridge_authorize(
  p_owner_id uuid,
  p_auth_session_id uuid,
  p_action text
)
returns text
language plpgsql
volatile
security definer
set search_path = ''
as $$
declare
  request_limit smallint;
begin
  if not sync_v2_private.auth_session_is_live(p_owner_id, p_auth_session_id) then
    return 'session_invalid';
  end if;

  request_limit := case p_action when 'begin' then 10 when 'complete' then 30 else null end;
  if request_limit is null then
    return 'request_invalid';
  end if;

  if not sync_v2_private.consume_enrollment_bridge_limit(
    p_owner_id, p_auth_session_id, p_action, request_limit
  ) then
    return 'rate_limited';
  end if;
  return 'authorized';
end;
$$;

create function public.sync_v2_enrollment_bridge_begin(
  p_owner_id uuid,
  p_auth_session_id uuid,
  p_requested_device_id uuid,
  p_proof_public_key_base64 text
)
returns table (enrollment_id uuid, challenge_base64 text, expires_at timestamptz)
language plpgsql
volatile
security definer
set search_path = ''
as $$
begin
  if not sync_v2_private.auth_session_is_live(p_owner_id, p_auth_session_id) then
    raise exception using errcode = 'P0001', message = 'BRIDGE_AUTH_SESSION_INVALID';
  end if;
  return query
  select result.enrollment_id,
         pg_catalog.encode(result.challenge, 'base64'),
         result.expires_at
    from sync_v2_private.begin_initial_device_enrollment(
      p_owner_id,
      p_auth_session_id,
      p_requested_device_id,
      pg_catalog.decode(p_proof_public_key_base64, 'base64')
    ) result;
end;
$$;

create function public.sync_v2_enrollment_bridge_context(
  p_enrollment_id uuid,
  p_owner_id uuid,
  p_auth_session_id uuid
)
returns table (
  requested_device_id uuid,
  public_key_base64 text,
  challenge_base64 text,
  expires_at timestamptz
)
language plpgsql
volatile
security definer
set search_path = ''
as $$
begin
  if not sync_v2_private.auth_session_is_live(p_owner_id, p_auth_session_id) then
    raise exception using errcode = 'P0001', message = 'BRIDGE_AUTH_SESSION_INVALID';
  end if;
  return query
  select e.requested_device_id,
         pg_catalog.encode(e.proof_public_key, 'base64'),
         pg_catalog.encode(e.challenge, 'base64'),
         e.expires_at
    from sync_v2_private.device_enrollments e
   where e.id = p_enrollment_id
     and e.owner_id = p_owner_id
     and e.auth_session_id = p_auth_session_id
     and e.consumed_at is null;
end;
$$;

create function public.sync_v2_enrollment_bridge_complete(
  p_enrollment_id uuid,
  p_owner_id uuid,
  p_auth_session_id uuid,
  p_proof_valid boolean
)
returns table (accepted boolean, outcome text, device_id uuid)
language plpgsql
volatile
security definer
set search_path = ''
as $$
begin
  if not sync_v2_private.auth_session_is_live(p_owner_id, p_auth_session_id) then
    raise exception using errcode = 'P0001', message = 'BRIDGE_AUTH_SESSION_INVALID';
  end if;

  return query
  select result.accepted, result.outcome, result.device_id
    from sync_v2_private.complete_initial_device_enrollment(
      p_enrollment_id, p_owner_id, p_auth_session_id, p_proof_valid
    ) result;
end;
$$;

revoke all on function sync_v2_private.auth_session_is_live(uuid, uuid)
  from public, anon, authenticated, service_role;
revoke all on function sync_v2_private.consume_enrollment_bridge_limit(uuid, uuid, text, smallint)
  from public, anon, authenticated, service_role;
revoke all on function public.sync_v2_enrollment_bridge_authorize(uuid, uuid, text)
  from public, anon, authenticated;
revoke all on function public.sync_v2_enrollment_bridge_begin(uuid, uuid, uuid, text)
  from public, anon, authenticated;
revoke all on function public.sync_v2_enrollment_bridge_context(uuid, uuid, uuid)
  from public, anon, authenticated;
revoke all on function public.sync_v2_enrollment_bridge_complete(uuid, uuid, uuid, boolean)
  from public, anon, authenticated;

grant execute on function public.sync_v2_enrollment_bridge_begin(uuid, uuid, uuid, text)
  to service_role;
grant execute on function public.sync_v2_enrollment_bridge_authorize(uuid, uuid, text)
  to service_role;
grant execute on function public.sync_v2_enrollment_bridge_context(uuid, uuid, uuid)
  to service_role;
grant execute on function public.sync_v2_enrollment_bridge_complete(uuid, uuid, uuid, boolean)
  to service_role;
