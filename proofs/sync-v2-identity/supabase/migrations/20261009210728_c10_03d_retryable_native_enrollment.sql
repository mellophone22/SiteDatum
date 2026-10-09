-- C10-03D: make trusted completion retryable after an accepted response is
-- lost. The Edge verifier must still reproduce and verify the exact Ed25519
-- proof before this wrapper can report the already-accepted device.

create or replace function public.sync_v2_enrollment_bridge_context(
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
     and e.auth_session_id = p_auth_session_id;
end;
$$;

create or replace function public.sync_v2_enrollment_bridge_complete(
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
declare
  completion record;
  accepted_device_id uuid;
begin
  if not sync_v2_private.auth_session_is_live(p_owner_id, p_auth_session_id) then
    raise exception using errcode = 'P0001', message = 'BRIDGE_AUTH_SESSION_INVALID';
  end if;

  select result.accepted, result.outcome, result.device_id
    into completion
    from sync_v2_private.complete_initial_device_enrollment(
      p_enrollment_id, p_owner_id, p_auth_session_id, p_proof_valid
    ) result;

  if completion.accepted is true then
    return query select true, completion.outcome::text, completion.device_id::uuid;
    return;
  end if;

  if p_proof_valid is true then
    select d.id
      into accepted_device_id
      from sync_v2_private.device_enrollments e
      join sync_v2_private.devices d
        on d.id = e.requested_device_id
       and d.owner_id = e.owner_id
       and d.proof_algorithm = e.proof_algorithm
       and d.proof_public_key = e.proof_public_key
       and d.revoked_at is null
      join sync_v2_private.sessions s
        on s.id = e.auth_session_id
       and s.owner_id = e.owner_id
       and s.device_id = e.requested_device_id
       and s.revoked_at is null
     where e.id = p_enrollment_id
       and e.owner_id = p_owner_id
       and e.auth_session_id = p_auth_session_id
       and e.consumed_at is not null
       and e.outcome = 'accepted';
  end if;

  if accepted_device_id is not null then
    return query select true, 'already_accepted'::text, accepted_device_id;
    return;
  end if;

  return query select false, 'not_accepted'::text, null::uuid;
end;
$$;

revoke all on function public.sync_v2_enrollment_bridge_context(uuid, uuid, uuid)
  from public, anon, authenticated;
revoke all on function public.sync_v2_enrollment_bridge_complete(uuid, uuid, uuid, boolean)
  from public, anon, authenticated;
grant execute on function public.sync_v2_enrollment_bridge_context(uuid, uuid, uuid)
  to service_role;
grant execute on function public.sync_v2_enrollment_bridge_complete(uuid, uuid, uuid, boolean)
  to service_role;
