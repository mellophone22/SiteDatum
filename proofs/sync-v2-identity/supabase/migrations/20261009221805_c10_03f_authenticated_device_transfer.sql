-- C10-03F: service-only encrypted workspace-key relay and trusted bridge
-- boundary for one approved second device.
--
-- The relay stores only a bounded HPKE envelope and random identifiers. It
-- cannot decrypt the workspace key. This disposable proof does not expose a
-- desktop command, enable Sync, or deploy a production resource.

create table sync_v2_private.approved_device_transfers (
  enrollment_id uuid primary key
    references sync_v2_private.approved_device_enrollments (id) on delete cascade,
  owner_id uuid not null,
  source_device_id uuid not null,
  target_device_id uuid not null,
  workspace_id uuid not null,
  workspace_key_version integer not null check (workspace_key_version between 1 and 2147483647),
  source_transfer_public_key bytea not null check (pg_catalog.octet_length(source_transfer_public_key) = 32),
  target_transfer_public_key bytea not null check (pg_catalog.octet_length(target_transfer_public_key) = 32),
  encapsulated_key bytea not null check (pg_catalog.octet_length(encapsulated_key) = 32),
  ciphertext bytea not null check (pg_catalog.octet_length(ciphertext) = 48),
  created_at timestamptz not null,
  expires_at timestamptz not null,
  fetched_at timestamptz,
  check (source_device_id <> target_device_id),
  check (created_at < expires_at)
);

alter table sync_v2_private.approved_device_transfers enable row level security;
alter table sync_v2_private.approved_device_transfers force row level security;

create index sync_v2_approved_device_transfers_owner_created_idx
  on sync_v2_private.approved_device_transfers (owner_id, created_at desc);

revoke all on table sync_v2_private.approved_device_transfers
  from public, anon, authenticated, service_role;

create table sync_v2_private.approved_device_bridge_windows (
  owner_id uuid not null,
  auth_session_id uuid not null,
  action text not null check (
    action in (
      'register_transfer_key',
      'begin_target',
      'prove_target',
      'prepare_source',
      'approve_source',
      'fetch_transfer'
    )
  ),
  window_started_at timestamptz not null,
  request_count smallint not null check (request_count between 1 and 30),
  last_request_at timestamptz not null,
  primary key (owner_id, auth_session_id, action, window_started_at)
);

alter table sync_v2_private.approved_device_bridge_windows enable row level security;
alter table sync_v2_private.approved_device_bridge_windows force row level security;

revoke all on table sync_v2_private.approved_device_bridge_windows
  from public, anon, authenticated, service_role;

create function sync_v2_private.consume_approved_device_bridge_limit(
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
    interval '5 minutes', now_at, '2000-01-01 00:00:00+00'::timestamptz
  );
  accepted_count smallint;
begin
  if p_owner_id is null
     or p_auth_session_id is null
     or p_action not in (
       'register_transfer_key', 'begin_target', 'prove_target', 'prepare_source',
       'approve_source', 'fetch_transfer'
     )
     or p_limit is null
     or p_limit < 1
     or p_limit > 30
     or not sync_v2_private.auth_session_is_live(p_owner_id, p_auth_session_id) then
    return false;
  end if;

  perform pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended(
      p_owner_id::text || ':' || p_auth_session_id::text || ':' || p_action, 0
    )
  );

  delete from sync_v2_private.approved_device_bridge_windows w
   where w.owner_id = p_owner_id
     and w.window_started_at < now_at - interval '24 hours';

  insert into sync_v2_private.approved_device_bridge_windows (
    owner_id, auth_session_id, action, window_started_at, request_count, last_request_at
  ) values (
    p_owner_id, p_auth_session_id, p_action, window_at, 1, now_at
  )
  on conflict (owner_id, auth_session_id, action, window_started_at)
  do update
     set request_count = sync_v2_private.approved_device_bridge_windows.request_count + 1,
         last_request_at = excluded.last_request_at
   where sync_v2_private.approved_device_bridge_windows.request_count < p_limit
  returning request_count into accepted_count;

  return accepted_count is not null;
end;
$$;

create function public.sync_v2_approved_bridge_authorize(
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

  request_limit := case p_action
    when 'register_transfer_key' then 10
    when 'begin_target' then 10
    when 'prove_target' then 20
    when 'prepare_source' then 20
    when 'approve_source' then 20
    when 'fetch_transfer' then 30
    else null
  end;
  if request_limit is null then
    return 'request_invalid';
  end if;
  if not sync_v2_private.consume_approved_device_bridge_limit(
    p_owner_id, p_auth_session_id, p_action, request_limit
  ) then
    return 'rate_limited';
  end if;
  return 'authorized';
end;
$$;

create function public.sync_v2_approved_bridge_device_context(
  p_owner_id uuid,
  p_auth_session_id uuid,
  p_device_id uuid
)
returns table (
  proof_public_key_base64 text,
  transfer_public_key_base64 text
)
language sql
stable
security definer
set search_path = ''
as $$
  select pg_catalog.encode(d.proof_public_key, 'base64'),
         case when d.transfer_public_key is null then null
              else pg_catalog.encode(d.transfer_public_key, 'base64') end
    from sync_v2_private.devices d
   where d.id = p_device_id
     and d.owner_id = p_owner_id
     and d.revoked_at is null
     and sync_v2_private.device_session_is_active(
       p_owner_id, p_auth_session_id, p_device_id
     );
$$;

create function public.sync_v2_approved_bridge_register_transfer_key(
  p_owner_id uuid,
  p_auth_session_id uuid,
  p_device_id uuid,
  p_transfer_public_key_base64 text,
  p_proof_valid boolean
)
returns text
language sql
volatile
security definer
set search_path = ''
as $$
  select sync_v2_private.register_device_transfer_key(
    p_owner_id,
    p_auth_session_id,
    p_device_id,
    pg_catalog.decode(p_transfer_public_key_base64, 'base64'),
    p_proof_valid
  );
$$;

create function public.sync_v2_approved_bridge_begin_target(
  p_owner_id uuid,
  p_target_auth_session_id uuid,
  p_source_device_id uuid,
  p_target_device_id uuid,
  p_target_proof_public_key_base64 text,
  p_target_transfer_public_key_base64 text
)
returns table (enrollment_id uuid, challenge_base64 text, expires_at timestamptz)
language sql
volatile
security definer
set search_path = ''
as $$
  select result.enrollment_id,
         pg_catalog.encode(result.challenge, 'base64'),
         result.expires_at
    from sync_v2_private.begin_approved_device_enrollment(
      p_owner_id,
      p_target_auth_session_id,
      p_source_device_id,
      p_target_device_id,
      pg_catalog.decode(p_target_proof_public_key_base64, 'base64'),
      pg_catalog.decode(p_target_transfer_public_key_base64, 'base64')
    ) result;
$$;

create function public.sync_v2_approved_bridge_target_context(
  p_enrollment_id uuid,
  p_owner_id uuid,
  p_target_auth_session_id uuid
)
returns table (
  source_device_id uuid,
  target_device_id uuid,
  target_proof_public_key_base64 text,
  target_transfer_public_key_base64 text,
  challenge_base64 text,
  expires_at timestamptz
)
language sql
stable
security definer
set search_path = ''
as $$
  select e.source_device_id,
         e.target_device_id,
         pg_catalog.encode(e.target_proof_public_key, 'base64'),
         pg_catalog.encode(e.target_transfer_public_key, 'base64'),
         pg_catalog.encode(e.challenge, 'base64'),
         e.expires_at
    from sync_v2_private.approved_device_enrollments e
   where e.id = p_enrollment_id
     and e.owner_id = p_owner_id
     and e.target_auth_session_id = p_target_auth_session_id
     and e.consumed_at is null
     and sync_v2_private.auth_session_is_live(p_owner_id, p_target_auth_session_id);
$$;

create function public.sync_v2_approved_bridge_verify_target(
  p_enrollment_id uuid,
  p_owner_id uuid,
  p_target_auth_session_id uuid,
  p_proof_valid boolean
)
returns table (ready_for_approval boolean, outcome text)
language sql
volatile
security definer
set search_path = ''
as $$
  select result.ready_for_approval, result.outcome
    from sync_v2_private.verify_approved_device_target(
      p_enrollment_id, p_owner_id, p_target_auth_session_id, p_proof_valid
    ) result;
$$;

create function public.sync_v2_approved_bridge_source_context(
  p_enrollment_id uuid,
  p_owner_id uuid,
  p_source_auth_session_id uuid,
  p_source_device_id uuid
)
returns table (
  target_auth_session_id uuid,
  target_device_id uuid,
  target_proof_public_key_base64 text,
  source_transfer_public_key_base64 text,
  target_transfer_public_key_base64 text,
  challenge_base64 text,
  expires_at timestamptz,
  outcome text
)
language sql
stable
security definer
set search_path = ''
as $$
  select e.target_auth_session_id,
         e.target_device_id,
         pg_catalog.encode(e.target_proof_public_key, 'base64'),
         pg_catalog.encode(source_device.transfer_public_key, 'base64'),
         pg_catalog.encode(e.target_transfer_public_key, 'base64'),
         pg_catalog.encode(e.challenge, 'base64'),
         e.expires_at,
         e.outcome
    from sync_v2_private.approved_device_enrollments e
    join sync_v2_private.devices source_device
      on source_device.id = e.source_device_id
     and source_device.owner_id = e.owner_id
   where e.id = p_enrollment_id
     and e.owner_id = p_owner_id
     and e.source_device_id = p_source_device_id
     and e.target_verified_at is not null
     and (e.consumed_at is null or e.outcome = 'accepted')
     and sync_v2_private.device_session_is_active(
       p_owner_id, p_source_auth_session_id, p_source_device_id
     );
$$;

create function public.sync_v2_approved_bridge_accept_transfer(
  p_enrollment_id uuid,
  p_owner_id uuid,
  p_source_auth_session_id uuid,
  p_source_device_id uuid,
  p_workspace_id uuid,
  p_workspace_key_version integer,
  p_encapsulated_key_base64 text,
  p_ciphertext_base64 text,
  p_approval_valid boolean
)
returns table (accepted boolean, outcome text, device_id uuid)
language plpgsql
volatile
security definer
set search_path = ''
as $$
declare
  enrollment sync_v2_private.approved_device_enrollments%rowtype;
  source_transfer_key bytea;
  encapsulated bytea;
  sealed_key bytea;
  completion record;
  existing sync_v2_private.approved_device_transfers%rowtype;
  now_at timestamptz := pg_catalog.clock_timestamp();
begin
  if p_workspace_id is null
     or p_workspace_key_version is null
     or p_workspace_key_version < 1 then
    return query select false, 'invalid_transfer'::text, null::uuid;
    return;
  end if;

  begin
    encapsulated := pg_catalog.decode(p_encapsulated_key_base64, 'base64');
    sealed_key := pg_catalog.decode(p_ciphertext_base64, 'base64');
  exception when others then
    return query select false, 'invalid_transfer'::text, null::uuid;
    return;
  end;
  if pg_catalog.octet_length(encapsulated) <> 32
     or pg_catalog.octet_length(sealed_key) <> 48 then
    return query select false, 'invalid_transfer'::text, null::uuid;
    return;
  end if;

  select e.*
    into enrollment
    from sync_v2_private.approved_device_enrollments e
   where e.id = p_enrollment_id
     and e.owner_id = p_owner_id
     and e.source_device_id = p_source_device_id;
  if not found then
    return query select false, 'not_accepted'::text, null::uuid;
    return;
  end if;

  select d.transfer_public_key
    into source_transfer_key
    from sync_v2_private.devices d
   where d.id = p_source_device_id
     and d.owner_id = p_owner_id
     and d.revoked_at is null;
  if source_transfer_key is null then
    return query select false, 'source_invalid'::text, null::uuid;
    return;
  end if;

  select t.* into existing
    from sync_v2_private.approved_device_transfers t
   where t.enrollment_id = p_enrollment_id;
  if found then
    if p_approval_valid is true
       and existing.owner_id = p_owner_id
       and existing.source_device_id = p_source_device_id
       and existing.workspace_id = p_workspace_id
       and existing.workspace_key_version = p_workspace_key_version
       and existing.encapsulated_key = encapsulated
       and existing.ciphertext = sealed_key
       and existing.source_transfer_public_key = source_transfer_key
       and existing.target_transfer_public_key = enrollment.target_transfer_public_key
       and sync_v2_private.device_session_is_active(
         p_owner_id, p_source_auth_session_id, p_source_device_id
       ) then
      return query select true, 'already_accepted'::text, existing.target_device_id;
      return;
    end if;
    return query select false, 'not_accepted'::text, null::uuid;
    return;
  end if;

  select * into completion
    from sync_v2_private.complete_approved_device_enrollment(
      p_enrollment_id,
      p_owner_id,
      p_source_auth_session_id,
      p_source_device_id,
      p_approval_valid
    );
  if completion.accepted is distinct from true or completion.outcome <> 'accepted' then
    return query select false, completion.outcome::text, completion.device_id::uuid;
    return;
  end if;

  insert into sync_v2_private.approved_device_transfers (
    enrollment_id,
    owner_id,
    source_device_id,
    target_device_id,
    workspace_id,
    workspace_key_version,
    source_transfer_public_key,
    target_transfer_public_key,
    encapsulated_key,
    ciphertext,
    created_at,
    expires_at
  ) values (
    enrollment.id,
    enrollment.owner_id,
    enrollment.source_device_id,
    enrollment.target_device_id,
    p_workspace_id,
    p_workspace_key_version,
    source_transfer_key,
    enrollment.target_transfer_public_key,
    encapsulated,
    sealed_key,
    now_at,
    enrollment.expires_at
  );

  return query select true, 'accepted'::text, enrollment.target_device_id;
end;
$$;

create function public.sync_v2_approved_bridge_fetch_transfer(
  p_enrollment_id uuid,
  p_owner_id uuid,
  p_target_auth_session_id uuid,
  p_target_device_id uuid
)
returns table (
  source_device_id uuid,
  target_device_id uuid,
  workspace_id uuid,
  workspace_key_version integer,
  source_transfer_public_key_base64 text,
  target_transfer_public_key_base64 text,
  encapsulated_key_base64 text,
  ciphertext_base64 text,
  expires_at timestamptz
)
language plpgsql
volatile
security definer
set search_path = ''
as $$
declare
  now_at timestamptz := pg_catalog.clock_timestamp();
begin
  if not sync_v2_private.device_session_is_active(
    p_owner_id, p_target_auth_session_id, p_target_device_id
  ) then
    return;
  end if;

  update sync_v2_private.approved_device_transfers t
     set fetched_at = coalesce(t.fetched_at, now_at)
   where t.enrollment_id = p_enrollment_id
     and t.owner_id = p_owner_id
     and t.target_device_id = p_target_device_id
     and t.expires_at > now_at;

  return query
  select t.source_device_id,
         t.target_device_id,
         t.workspace_id,
         t.workspace_key_version,
         pg_catalog.encode(t.source_transfer_public_key, 'base64'),
         pg_catalog.encode(t.target_transfer_public_key, 'base64'),
         pg_catalog.encode(t.encapsulated_key, 'base64'),
         pg_catalog.encode(t.ciphertext, 'base64'),
         t.expires_at
    from sync_v2_private.approved_device_transfers t
   where t.enrollment_id = p_enrollment_id
     and t.owner_id = p_owner_id
     and t.target_device_id = p_target_device_id
     and t.expires_at > now_at;
end;
$$;

revoke all on function sync_v2_private.consume_approved_device_bridge_limit(uuid, uuid, text, smallint)
  from public, anon, authenticated, service_role;
revoke all on function public.sync_v2_approved_bridge_authorize(uuid, uuid, text)
  from public, anon, authenticated;
revoke all on function public.sync_v2_approved_bridge_device_context(uuid, uuid, uuid)
  from public, anon, authenticated;
revoke all on function public.sync_v2_approved_bridge_register_transfer_key(uuid, uuid, uuid, text, boolean)
  from public, anon, authenticated;
revoke all on function public.sync_v2_approved_bridge_begin_target(uuid, uuid, uuid, uuid, text, text)
  from public, anon, authenticated;
revoke all on function public.sync_v2_approved_bridge_target_context(uuid, uuid, uuid)
  from public, anon, authenticated;
revoke all on function public.sync_v2_approved_bridge_verify_target(uuid, uuid, uuid, boolean)
  from public, anon, authenticated;
revoke all on function public.sync_v2_approved_bridge_source_context(uuid, uuid, uuid, uuid)
  from public, anon, authenticated;
revoke all on function public.sync_v2_approved_bridge_accept_transfer(uuid, uuid, uuid, uuid, uuid, integer, text, text, boolean)
  from public, anon, authenticated;
revoke all on function public.sync_v2_approved_bridge_fetch_transfer(uuid, uuid, uuid, uuid)
  from public, anon, authenticated;

grant execute on function public.sync_v2_approved_bridge_authorize(uuid, uuid, text)
  to service_role;
grant execute on function public.sync_v2_approved_bridge_device_context(uuid, uuid, uuid)
  to service_role;
grant execute on function public.sync_v2_approved_bridge_register_transfer_key(uuid, uuid, uuid, text, boolean)
  to service_role;
grant execute on function public.sync_v2_approved_bridge_begin_target(uuid, uuid, uuid, uuid, text, text)
  to service_role;
grant execute on function public.sync_v2_approved_bridge_target_context(uuid, uuid, uuid)
  to service_role;
grant execute on function public.sync_v2_approved_bridge_verify_target(uuid, uuid, uuid, boolean)
  to service_role;
grant execute on function public.sync_v2_approved_bridge_source_context(uuid, uuid, uuid, uuid)
  to service_role;
grant execute on function public.sync_v2_approved_bridge_accept_transfer(uuid, uuid, uuid, uuid, uuid, integer, text, text, boolean)
  to service_role;
grant execute on function public.sync_v2_approved_bridge_fetch_transfer(uuid, uuid, uuid, uuid)
  to service_role;
