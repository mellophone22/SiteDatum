-- C10-03B: durable first-device enrollment and proof-of-possession boundary.
--
-- This remains a disposable local proof. Client roles cannot call these
-- functions or read enrollment state. A future trusted service bridge must
-- authenticate the Supabase user/session and verify the Ed25519 signature
-- before passing the boolean proof result to the completion function.

create extension if not exists pgcrypto with schema extensions;

alter table sync_v2_private.devices
  add column proof_algorithm text,
  add column proof_public_key bytea,
  add constraint sync_v2_devices_proof_pair
    check (
      (proof_algorithm is null and proof_public_key is null)
      or (
        proof_algorithm = 'ed25519-v1'
        and octet_length(proof_public_key) = 32
      )
    );

create unique index sync_v2_devices_proof_public_key_unique
  on sync_v2_private.devices (proof_public_key)
  where proof_public_key is not null;

create table sync_v2_private.device_enrollments (
  id uuid primary key,
  owner_id uuid not null,
  auth_session_id uuid not null,
  requested_device_id uuid not null,
  proof_algorithm text not null default 'ed25519-v1'
    check (proof_algorithm = 'ed25519-v1'),
  proof_public_key bytea not null
    check (octet_length(proof_public_key) = 32),
  challenge bytea not null
    check (octet_length(challenge) = 32),
  created_at timestamptz not null,
  expires_at timestamptz not null,
  attempted_at timestamptz,
  consumed_at timestamptz,
  attempt_count smallint not null default 0
    check (attempt_count between 0 and 1),
  outcome text check (
    outcome in (
      'accepted',
      'context_mismatch',
      'device_collision',
      'existing_device',
      'expired',
      'invalid_proof',
      'session_collision'
    )
  ),
  check (created_at < expires_at),
  check (
    (attempt_count = 0 and attempted_at is null and consumed_at is null and outcome is null)
    or
    (attempt_count = 1 and attempted_at is not null and consumed_at is not null and outcome is not null)
  )
);

alter table sync_v2_private.device_enrollments enable row level security;
alter table sync_v2_private.device_enrollments force row level security;

create index sync_v2_device_enrollments_owner_session_created_idx
  on sync_v2_private.device_enrollments
    (owner_id, auth_session_id, created_at desc);

revoke all on table sync_v2_private.device_enrollments
  from public, anon, authenticated, service_role;

create function sync_v2_private.begin_initial_device_enrollment(
  p_owner_id uuid,
  p_auth_session_id uuid,
  p_requested_device_id uuid,
  p_proof_public_key bytea
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
  new_expires_at timestamptz := now_at + interval '5 minutes';
  recent_count integer;
begin
  if p_owner_id is null
     or p_auth_session_id is null
     or p_requested_device_id is null
     or p_proof_public_key is null
     or pg_catalog.octet_length(p_proof_public_key) <> 32 then
    raise exception using errcode = '22023', message = 'ENROLLMENT_INVALID_INPUT';
  end if;

  perform pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended(p_owner_id::text, 0)
  );

  if exists (
    select 1
    from sync_v2_private.devices d
    where d.owner_id = p_owner_id
      and d.revoked_at is null
  ) then
    raise exception using errcode = 'P0001',
      message = 'ENROLLMENT_EXISTING_DEVICE_REQUIRES_APPROVAL';
  end if;

  select count(*)::integer
    into recent_count
    from sync_v2_private.device_enrollments e
   where e.owner_id = p_owner_id
     and e.auth_session_id = p_auth_session_id
     and e.created_at > now_at - interval '1 hour';

  if recent_count >= 5 then
    raise exception using errcode = 'P0001',
      message = 'ENROLLMENT_RATE_LIMITED';
  end if;

  insert into sync_v2_private.device_enrollments (
    id,
    owner_id,
    auth_session_id,
    requested_device_id,
    proof_public_key,
    challenge,
    created_at,
    expires_at
  ) values (
    new_enrollment_id,
    p_owner_id,
    p_auth_session_id,
    p_requested_device_id,
    p_proof_public_key,
    new_challenge,
    now_at,
    new_expires_at
  );

  return query select new_enrollment_id, new_challenge, new_expires_at;
end;
$$;

create function sync_v2_private.complete_initial_device_enrollment(
  p_enrollment_id uuid,
  p_owner_id uuid,
  p_auth_session_id uuid,
  p_proof_valid boolean
)
returns table (
  accepted boolean,
  outcome text,
  device_id uuid
)
language plpgsql
volatile
security definer
set search_path = ''
as $$
declare
  now_at timestamptz := pg_catalog.clock_timestamp();
  enrollment sync_v2_private.device_enrollments%rowtype;
  final_outcome text;
begin
  perform pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended(coalesce(p_owner_id::text, ''), 0)
  );

  select e.*
    into enrollment
    from sync_v2_private.device_enrollments e
   where e.id = p_enrollment_id
   for update;

  if not found or enrollment.consumed_at is not null then
    return query select false, 'not_accepted'::text, null::uuid;
    return;
  end if;

  final_outcome := case
    when p_owner_id is distinct from enrollment.owner_id
      or p_auth_session_id is distinct from enrollment.auth_session_id
      then 'context_mismatch'
    when now_at >= enrollment.expires_at then 'expired'
    when p_proof_valid is distinct from true then 'invalid_proof'
    when exists (
      select 1 from sync_v2_private.devices d
       where d.owner_id = enrollment.owner_id and d.revoked_at is null
    ) then 'existing_device'
    when exists (
      select 1 from sync_v2_private.devices d
       where d.id = enrollment.requested_device_id
          or d.proof_public_key = enrollment.proof_public_key
    ) then 'device_collision'
    when exists (
      select 1 from sync_v2_private.sessions s
       where s.id = enrollment.auth_session_id
    ) then 'session_collision'
    else 'accepted'
  end;

  update sync_v2_private.device_enrollments
     set attempted_at = now_at,
         consumed_at = now_at,
         attempt_count = 1,
         outcome = final_outcome
   where id = enrollment.id;

  if final_outcome <> 'accepted' then
    return query select false, final_outcome, null::uuid;
    return;
  end if;

  insert into sync_v2_private.devices (
    id,
    owner_id,
    proof_algorithm,
    proof_public_key
  ) values (
    enrollment.requested_device_id,
    enrollment.owner_id,
    enrollment.proof_algorithm,
    enrollment.proof_public_key
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
    enrollment.auth_session_id,
    enrollment.owner_id,
    enrollment.requested_device_id,
    now_at,
    now_at,
    now_at + interval '30 minutes',
    now_at + interval '8 hours'
  );

  return query select true, 'accepted'::text, enrollment.requested_device_id;
end;
$$;

revoke all on function sync_v2_private.begin_initial_device_enrollment(
  uuid, uuid, uuid, bytea
) from public, anon, authenticated;
revoke all on function sync_v2_private.complete_initial_device_enrollment(
  uuid, uuid, uuid, boolean
) from public, anon, authenticated;

grant usage on schema sync_v2_private to service_role;
grant execute on function sync_v2_private.begin_initial_device_enrollment(
  uuid, uuid, uuid, bytea
) to service_role;
grant execute on function sync_v2_private.complete_initial_device_enrollment(
  uuid, uuid, uuid, boolean
) to service_role;
