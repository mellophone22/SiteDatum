-- C10-03E: durable, service-only authorization boundary for a second device.
--
-- This migration remains a disposable local proof. It does not expose a
-- client-callable RPC, transfer a workspace key, or enable desktop Sync.

alter table sync_v2_private.devices
  add column transfer_algorithm text,
  add column transfer_public_key bytea,
  add constraint sync_v2_devices_transfer_key_pair
    check (
      (transfer_algorithm is null and transfer_public_key is null)
      or (
        transfer_algorithm = 'x25519-hpke-auth-v1'
        and pg_catalog.octet_length(transfer_public_key) = 32
      )
    );

create unique index sync_v2_devices_transfer_public_key_unique
  on sync_v2_private.devices (transfer_public_key)
  where transfer_public_key is not null;

create table sync_v2_private.approved_device_enrollments (
  id uuid primary key,
  owner_id uuid not null,
  source_device_id uuid not null,
  target_auth_session_id uuid not null,
  target_device_id uuid not null,
  target_proof_algorithm text not null default 'ed25519-v1'
    check (target_proof_algorithm = 'ed25519-v1'),
  target_proof_public_key bytea not null
    check (pg_catalog.octet_length(target_proof_public_key) = 32),
  target_transfer_algorithm text not null default 'x25519-hpke-auth-v1'
    check (target_transfer_algorithm = 'x25519-hpke-auth-v1'),
  target_transfer_public_key bytea not null
    check (pg_catalog.octet_length(target_transfer_public_key) = 32),
  challenge bytea not null
    check (pg_catalog.octet_length(challenge) = 32),
  created_at timestamptz not null,
  expires_at timestamptz not null,
  target_attempt_count smallint not null default 0
    check (target_attempt_count between 0 and 1),
  target_attempted_at timestamptz,
  target_verified_at timestamptz,
  source_attempt_count smallint not null default 0
    check (source_attempt_count between 0 and 1),
  source_attempted_at timestamptz,
  consumed_at timestamptz,
  outcome text check (
    outcome in (
      'accepted',
      'device_collision',
      'device_limit',
      'expired',
      'invalid_source_approval',
      'invalid_target_proof',
      'session_collision',
      'target_session_invalid'
    )
  ),
  check (source_device_id <> target_device_id),
  check (created_at < expires_at),
  check (
    (target_attempt_count = 0 and target_attempted_at is null and target_verified_at is null)
    or
    (target_attempt_count = 1 and target_attempted_at is not null)
  ),
  check (
    (source_attempt_count = 0 and source_attempted_at is null)
    or
    (source_attempt_count = 1 and source_attempted_at is not null)
  ),
  check (
    (consumed_at is null and outcome is null)
    or
    (consumed_at is not null and outcome is not null)
  ),
  check (
    outcome is distinct from 'accepted'
    or (
      target_verified_at is not null
      and source_attempt_count = 1
    )
  )
);

alter table sync_v2_private.approved_device_enrollments enable row level security;
alter table sync_v2_private.approved_device_enrollments force row level security;

create index sync_v2_approved_device_enrollments_owner_created_idx
  on sync_v2_private.approved_device_enrollments (owner_id, created_at desc);
create unique index sync_v2_approved_device_enrollments_target_session_pending
  on sync_v2_private.approved_device_enrollments (target_auth_session_id)
  where consumed_at is null;
create unique index sync_v2_approved_device_enrollments_target_device_pending
  on sync_v2_private.approved_device_enrollments (target_device_id)
  where consumed_at is null;

revoke all on table sync_v2_private.approved_device_enrollments
  from public, anon, authenticated, service_role;

create function sync_v2_private.device_session_is_active(
  p_owner_id uuid,
  p_auth_session_id uuid,
  p_device_id uuid
)
returns boolean
language sql
stable
security definer
set search_path = ''
as $$
  select sync_v2_private.auth_session_is_live(p_owner_id, p_auth_session_id)
    and exists (
      select 1
        from sync_v2_private.sessions s
        join sync_v2_private.devices d
          on d.id = s.device_id
         and d.owner_id = s.owner_id
       where s.id = p_auth_session_id
         and s.owner_id = p_owner_id
         and s.device_id = p_device_id
         and s.revoked_at is null
         and d.revoked_at is null
         and s.idle_expires_at > pg_catalog.now()
         and s.absolute_expires_at > pg_catalog.now()
    );
$$;

create function sync_v2_private.register_device_transfer_key(
  p_owner_id uuid,
  p_auth_session_id uuid,
  p_device_id uuid,
  p_transfer_public_key bytea,
  p_proof_valid boolean
)
returns text
language plpgsql
volatile
security definer
set search_path = ''
as $$
declare
  current_key bytea;
begin
  if p_owner_id is null
     or p_auth_session_id is null
     or p_device_id is null
     or p_transfer_public_key is null
     or pg_catalog.octet_length(p_transfer_public_key) <> 32 then
    raise exception using errcode = '22023', message = 'TRANSFER_KEY_INVALID_INPUT';
  end if;

  if not sync_v2_private.device_session_is_active(
    p_owner_id, p_auth_session_id, p_device_id
  ) then
    return 'device_invalid';
  end if;

  if p_proof_valid is distinct from true then
    return 'invalid_proof';
  end if;

  perform pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended(p_device_id::text, 0)
  );

  select d.transfer_public_key
    into current_key
    from sync_v2_private.devices d
   where d.id = p_device_id
     and d.owner_id = p_owner_id
     and d.revoked_at is null
   for update;

  if current_key is not null then
    if current_key = p_transfer_public_key then
      return 'already_registered';
    end if;
    return 'key_conflict';
  end if;

  if exists (
    select 1
      from sync_v2_private.devices d
     where d.transfer_public_key = p_transfer_public_key
  ) then
    return 'key_conflict';
  end if;

  update sync_v2_private.devices
     set transfer_algorithm = 'x25519-hpke-auth-v1',
         transfer_public_key = p_transfer_public_key
   where id = p_device_id
     and owner_id = p_owner_id
     and revoked_at is null;

  return 'registered';
end;
$$;

create function sync_v2_private.begin_approved_device_enrollment(
  p_owner_id uuid,
  p_target_auth_session_id uuid,
  p_source_device_id uuid,
  p_target_device_id uuid,
  p_target_proof_public_key bytea,
  p_target_transfer_public_key bytea
)
returns table (
  enrollment_id uuid,
  challenge bytea,
  expires_at timestamptz
)
language plpgsql
volatile
security definer
set search_path = ''
as $$
declare
  now_at timestamptz := pg_catalog.clock_timestamp();
  new_enrollment_id uuid := pg_catalog.gen_random_uuid();
  new_challenge bytea := extensions.gen_random_bytes(32);
  new_expires_at timestamptz := now_at + interval '10 minutes';
  active_device_count integer;
  recent_count integer;
begin
  if p_owner_id is null
     or p_target_auth_session_id is null
     or p_source_device_id is null
     or p_target_device_id is null
     or p_source_device_id = p_target_device_id
     or p_target_proof_public_key is null
     or pg_catalog.octet_length(p_target_proof_public_key) <> 32
     or p_target_transfer_public_key is null
     or pg_catalog.octet_length(p_target_transfer_public_key) <> 32 then
    raise exception using errcode = '22023', message = 'APPROVED_DEVICE_INVALID_INPUT';
  end if;

  if not sync_v2_private.auth_session_is_live(
    p_owner_id, p_target_auth_session_id
  ) then
    raise exception using errcode = 'P0001', message = 'APPROVED_DEVICE_TARGET_SESSION_INVALID';
  end if;

  perform pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended(p_owner_id::text, 0)
  );

  if not exists (
    select 1
      from sync_v2_private.devices d
     where d.id = p_source_device_id
       and d.owner_id = p_owner_id
       and d.revoked_at is null
       and d.proof_algorithm = 'ed25519-v1'
       and d.proof_public_key is not null
       and d.transfer_algorithm = 'x25519-hpke-auth-v1'
       and d.transfer_public_key is not null
  ) then
    raise exception using errcode = 'P0001', message = 'APPROVED_DEVICE_SOURCE_INVALID';
  end if;

  select count(*)::integer
    into active_device_count
    from sync_v2_private.devices d
   where d.owner_id = p_owner_id
     and d.revoked_at is null;

  if active_device_count >= 2 then
    raise exception using errcode = 'P0001', message = 'APPROVED_DEVICE_LIMIT_REACHED';
  end if;

  if exists (
    select 1
      from sync_v2_private.devices d
     where d.id = p_target_device_id
        or d.proof_public_key = p_target_proof_public_key
        or d.transfer_public_key = p_target_transfer_public_key
  ) then
    raise exception using errcode = 'P0001', message = 'APPROVED_DEVICE_COLLISION';
  end if;

  select count(*)::integer
    into recent_count
    from sync_v2_private.approved_device_enrollments e
   where e.owner_id = p_owner_id
     and e.target_auth_session_id = p_target_auth_session_id
     and e.created_at > now_at - interval '1 hour';

  if recent_count >= 5 then
    raise exception using errcode = 'P0001', message = 'APPROVED_DEVICE_RATE_LIMITED';
  end if;

  insert into sync_v2_private.approved_device_enrollments (
    id,
    owner_id,
    source_device_id,
    target_auth_session_id,
    target_device_id,
    target_proof_public_key,
    target_transfer_public_key,
    challenge,
    created_at,
    expires_at
  ) values (
    new_enrollment_id,
    p_owner_id,
    p_source_device_id,
    p_target_auth_session_id,
    p_target_device_id,
    p_target_proof_public_key,
    p_target_transfer_public_key,
    new_challenge,
    now_at,
    new_expires_at
  );

  return query select new_enrollment_id, new_challenge, new_expires_at;
end;
$$;

create function sync_v2_private.verify_approved_device_target(
  p_enrollment_id uuid,
  p_owner_id uuid,
  p_target_auth_session_id uuid,
  p_proof_valid boolean
)
returns table (ready_for_approval boolean, outcome text)
language plpgsql
volatile
security definer
set search_path = ''
as $$
declare
  now_at timestamptz := pg_catalog.clock_timestamp();
  enrollment sync_v2_private.approved_device_enrollments%rowtype;
begin
  if not sync_v2_private.auth_session_is_live(
    p_owner_id, p_target_auth_session_id
  ) then
    return query select false, 'session_invalid'::text;
    return;
  end if;

  select e.*
    into enrollment
    from sync_v2_private.approved_device_enrollments e
   where e.id = p_enrollment_id
     and e.owner_id = p_owner_id
     and e.target_auth_session_id = p_target_auth_session_id
   for update;

  if not found or enrollment.consumed_at is not null then
    return query select false, 'not_ready'::text;
    return;
  end if;

  if enrollment.target_verified_at is not null then
    return query select (p_proof_valid is true),
      case when p_proof_valid is true then 'already_verified' else 'not_ready' end::text;
    return;
  end if;

  if now_at >= enrollment.expires_at then
    update sync_v2_private.approved_device_enrollments
       set target_attempt_count = 1,
           target_attempted_at = now_at,
           consumed_at = now_at,
           outcome = 'expired'
     where id = enrollment.id;
    return query select false, 'expired'::text;
    return;
  end if;

  if p_proof_valid is distinct from true then
    update sync_v2_private.approved_device_enrollments
       set target_attempt_count = 1,
           target_attempted_at = now_at,
           consumed_at = now_at,
           outcome = 'invalid_target_proof'
     where id = enrollment.id;
    return query select false, 'invalid_target_proof'::text;
    return;
  end if;

  update sync_v2_private.approved_device_enrollments
     set target_attempt_count = 1,
         target_attempted_at = now_at,
         target_verified_at = now_at
   where id = enrollment.id;

  return query select true, 'awaiting_source_approval'::text;
end;
$$;

create function sync_v2_private.complete_approved_device_enrollment(
  p_enrollment_id uuid,
  p_owner_id uuid,
  p_source_auth_session_id uuid,
  p_source_device_id uuid,
  p_approval_valid boolean
)
returns table (accepted boolean, outcome text, device_id uuid)
language plpgsql
volatile
security definer
set search_path = ''
as $$
declare
  now_at timestamptz := pg_catalog.clock_timestamp();
  enrollment sync_v2_private.approved_device_enrollments%rowtype;
  active_device_count integer;
begin
  if not sync_v2_private.device_session_is_active(
    p_owner_id, p_source_auth_session_id, p_source_device_id
  ) then
    return query select false, 'source_invalid'::text, null::uuid;
    return;
  end if;

  perform pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended(p_owner_id::text, 0)
  );

  select e.*
    into enrollment
    from sync_v2_private.approved_device_enrollments e
   where e.id = p_enrollment_id
     and e.owner_id = p_owner_id
     and e.source_device_id = p_source_device_id
   for update;

  if not found then
    return query select false, 'not_accepted'::text, null::uuid;
    return;
  end if;

  if enrollment.consumed_at is not null then
    if enrollment.outcome = 'accepted'
       and p_approval_valid is true
       and exists (
         select 1
           from sync_v2_private.devices d
           join sync_v2_private.sessions s
             on s.device_id = d.id
            and s.owner_id = d.owner_id
          where d.id = enrollment.target_device_id
            and d.owner_id = enrollment.owner_id
            and d.proof_public_key = enrollment.target_proof_public_key
            and d.transfer_public_key = enrollment.target_transfer_public_key
            and d.revoked_at is null
            and s.id = enrollment.target_auth_session_id
            and s.revoked_at is null
       ) then
      return query select true, 'already_accepted'::text, enrollment.target_device_id;
      return;
    end if;
    return query select false, 'not_accepted'::text, null::uuid;
    return;
  end if;

  if enrollment.target_verified_at is null then
    return query select false, 'target_not_verified'::text, null::uuid;
    return;
  end if;

  if now_at >= enrollment.expires_at then
    update sync_v2_private.approved_device_enrollments
       set source_attempt_count = 1,
           source_attempted_at = now_at,
           consumed_at = now_at,
           outcome = 'expired'
     where id = enrollment.id;
    return query select false, 'expired'::text, null::uuid;
    return;
  end if;

  if p_approval_valid is distinct from true then
    update sync_v2_private.approved_device_enrollments
       set source_attempt_count = 1,
           source_attempted_at = now_at,
           consumed_at = now_at,
           outcome = 'invalid_source_approval'
     where id = enrollment.id;
    return query select false, 'invalid_source_approval'::text, null::uuid;
    return;
  end if;

  if not sync_v2_private.auth_session_is_live(
    enrollment.owner_id, enrollment.target_auth_session_id
  ) then
    update sync_v2_private.approved_device_enrollments
       set source_attempt_count = 1,
           source_attempted_at = now_at,
           consumed_at = now_at,
           outcome = 'target_session_invalid'
     where id = enrollment.id;
    return query select false, 'target_session_invalid'::text, null::uuid;
    return;
  end if;

  select count(*)::integer
    into active_device_count
    from sync_v2_private.devices d
   where d.owner_id = enrollment.owner_id
     and d.revoked_at is null;

  if active_device_count >= 2 then
    update sync_v2_private.approved_device_enrollments
       set source_attempt_count = 1,
           source_attempted_at = now_at,
           consumed_at = now_at,
           outcome = 'device_limit'
     where id = enrollment.id;
    return query select false, 'device_limit'::text, null::uuid;
    return;
  end if;

  if exists (
    select 1
      from sync_v2_private.devices d
     where d.id = enrollment.target_device_id
        or d.proof_public_key = enrollment.target_proof_public_key
        or d.transfer_public_key = enrollment.target_transfer_public_key
  ) then
    update sync_v2_private.approved_device_enrollments
       set source_attempt_count = 1,
           source_attempted_at = now_at,
           consumed_at = now_at,
           outcome = 'device_collision'
     where id = enrollment.id;
    return query select false, 'device_collision'::text, null::uuid;
    return;
  end if;

  if exists (
    select 1
      from sync_v2_private.sessions s
     where s.id = enrollment.target_auth_session_id
  ) then
    update sync_v2_private.approved_device_enrollments
       set source_attempt_count = 1,
           source_attempted_at = now_at,
           consumed_at = now_at,
           outcome = 'session_collision'
     where id = enrollment.id;
    return query select false, 'session_collision'::text, null::uuid;
    return;
  end if;

  insert into sync_v2_private.devices (
    id,
    owner_id,
    proof_algorithm,
    proof_public_key,
    transfer_algorithm,
    transfer_public_key
  ) values (
    enrollment.target_device_id,
    enrollment.owner_id,
    enrollment.target_proof_algorithm,
    enrollment.target_proof_public_key,
    enrollment.target_transfer_algorithm,
    enrollment.target_transfer_public_key
  );

  insert into sync_v2_private.sessions (
    id,
    owner_id,
    device_id,
    issued_at,
    last_seen_at,
    idle_expires_at,
    absolute_expires_at
  ) values (
    enrollment.target_auth_session_id,
    enrollment.owner_id,
    enrollment.target_device_id,
    now_at,
    now_at,
    now_at + interval '30 minutes',
    now_at + interval '8 hours'
  );

  update sync_v2_private.approved_device_enrollments
     set source_attempt_count = 1,
         source_attempted_at = now_at,
         consumed_at = now_at,
         outcome = 'accepted'
   where id = enrollment.id;

  return query select true, 'accepted'::text, enrollment.target_device_id;
end;
$$;

revoke all on function sync_v2_private.device_session_is_active(uuid, uuid, uuid)
  from public, anon, authenticated, service_role;
revoke all on function sync_v2_private.register_device_transfer_key(uuid, uuid, uuid, bytea, boolean)
  from public, anon, authenticated;
revoke all on function sync_v2_private.begin_approved_device_enrollment(uuid, uuid, uuid, uuid, bytea, bytea)
  from public, anon, authenticated;
revoke all on function sync_v2_private.verify_approved_device_target(uuid, uuid, uuid, boolean)
  from public, anon, authenticated;
revoke all on function sync_v2_private.complete_approved_device_enrollment(uuid, uuid, uuid, uuid, boolean)
  from public, anon, authenticated;

grant execute on function sync_v2_private.register_device_transfer_key(uuid, uuid, uuid, bytea, boolean)
  to service_role;
grant execute on function sync_v2_private.begin_approved_device_enrollment(uuid, uuid, uuid, uuid, bytea, bytea)
  to service_role;
grant execute on function sync_v2_private.verify_approved_device_target(uuid, uuid, uuid, boolean)
  to service_role;
grant execute on function sync_v2_private.complete_approved_device_enrollment(uuid, uuid, uuid, uuid, boolean)
  to service_role;
