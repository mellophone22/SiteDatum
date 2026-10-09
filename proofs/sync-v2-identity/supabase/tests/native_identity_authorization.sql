begin;

create extension if not exists pgtap with schema extensions;
select plan(21);

select has_function('sync_v2_private', 'uuid_or_null', array['text'], 'private UUID parser exists');
select has_function('sync_v2_private', 'custom_access_token_hook', array['jsonb'], 'custom access-token hook exists');
select ok(has_function_privilege('supabase_auth_admin', 'sync_v2_private.custom_access_token_hook(jsonb)', 'EXECUTE'), 'Auth may execute the token hook');
select ok(not has_function_privilege('authenticated', 'sync_v2_private.custom_access_token_hook(jsonb)', 'EXECUTE'), 'authenticated callers cannot execute the token hook');
select ok(not has_function_privilege('anon', 'sync_v2_private.custom_access_token_hook(jsonb)', 'EXECUTE'), 'anonymous callers cannot execute the token hook');
select ok(not has_function_privilege('authenticated', 'sync_v2_private.uuid_or_null(text)', 'EXECUTE'), 'authenticated callers cannot execute the UUID parser');

insert into sync_v2_private.devices (id, owner_id) values
  ('22000000-0000-4000-8000-000000000001', '11000000-0000-4000-8000-000000000001'),
  ('22000000-0000-4000-8000-000000000002', '11000000-0000-4000-8000-000000000002');
insert into sync_v2_private.sessions (
  id,
  owner_id,
  device_id,
  issued_at,
  last_seen_at,
  idle_expires_at,
  absolute_expires_at
) values
  (
    '33000000-0000-4000-8000-000000000001',
    '11000000-0000-4000-8000-000000000001',
    '22000000-0000-4000-8000-000000000001',
    clock_timestamp(),
    clock_timestamp(),
    clock_timestamp() + interval '30 minutes',
    clock_timestamp() + interval '8 hours'
  ),
  (
    '33000000-0000-4000-8000-000000000002',
    '11000000-0000-4000-8000-000000000002',
    '22000000-0000-4000-8000-000000000002',
    clock_timestamp(),
    clock_timestamp(),
    clock_timestamp() + interval '30 minutes',
    clock_timestamp() + interval '8 hours'
  );
insert into sync_v2_proof.envelopes (id, owner_id, ciphertext) values
  ('44000000-0000-4000-8000-000000000001', '11000000-0000-4000-8000-000000000001', decode('a1', 'hex')),
  ('44000000-0000-4000-8000-000000000002', '11000000-0000-4000-8000-000000000002', decode('b2', 'hex'));

select is(
  sync_v2_private.custom_access_token_hook(
    '{"user_id":"11000000-0000-4000-8000-000000000001","claims":{"sub":"11000000-0000-4000-8000-000000000001","session_id":"33000000-0000-4000-8000-000000000001"}}'::jsonb
  ) -> 'claims' ->> 'sync_device_id',
  '22000000-0000-4000-8000-000000000001',
  'hook adds the server-bound device claim'
);
select is(
  sync_v2_private.custom_access_token_hook(
    '{"user_id":"11000000-0000-4000-8000-000000000001","claims":{"sub":"11000000-0000-4000-8000-000000000001","session_id":"33000000-0000-4000-8000-000000000001","sync_device_id":"22000000-0000-4000-8000-000000000002"}}'::jsonb
  ) -> 'claims' ->> 'sync_device_id',
  '22000000-0000-4000-8000-000000000001',
  'hook overwrites an injected device claim with the server binding'
);
select is(
  sync_v2_private.custom_access_token_hook(
    '{"user_id":"11000000-0000-4000-8000-000000000001","claims":{"sub":"11000000-0000-4000-8000-000000000001","session_id":"33000000-0000-4000-8000-999999999999","sync_device_id":"22000000-0000-4000-8000-000000000001"}}'::jsonb
  ) -> 'claims' ->> 'sync_device_id',
  null::text,
  'unknown session receives no device claim'
);
select is(
  sync_v2_private.custom_access_token_hook(
    '{"user_id":"11000000-0000-4000-8000-000000000002","claims":{"sub":"11000000-0000-4000-8000-000000000002","session_id":"33000000-0000-4000-8000-000000000001","sync_device_id":"22000000-0000-4000-8000-000000000001"}}'::jsonb
  ) -> 'claims' ->> 'sync_device_id',
  null::text,
  'session belonging to another owner receives no device claim'
);
select is(
  sync_v2_private.custom_access_token_hook(
    '{"user_id":"11000000-0000-4000-8000-000000000001","claims":{"sub":"11000000-0000-4000-8000-000000000002","session_id":"33000000-0000-4000-8000-000000000001","sync_device_id":"22000000-0000-4000-8000-000000000001"}}'::jsonb
  ) -> 'claims' ->> 'sync_device_id',
  null::text,
  'hook refuses a token subject that differs from the trusted event owner'
);

update sync_v2_private.sessions
set issued_at = clock_timestamp() - interval '2 hours',
    last_seen_at = clock_timestamp() - interval '1 hour',
    idle_expires_at = clock_timestamp() - interval '30 minutes',
    absolute_expires_at = clock_timestamp() + interval '1 hour'
where id = '33000000-0000-4000-8000-000000000001';
select is(
  sync_v2_private.custom_access_token_hook(
    '{"user_id":"11000000-0000-4000-8000-000000000001","claims":{"sub":"11000000-0000-4000-8000-000000000001","session_id":"33000000-0000-4000-8000-000000000001"}}'::jsonb
  ) -> 'claims' ->> 'sync_device_id',
  null::text,
  'expired session receives no device claim'
);

reset role;
update sync_v2_private.sessions
set issued_at = clock_timestamp(),
    last_seen_at = clock_timestamp(),
    idle_expires_at = clock_timestamp() + interval '30 minutes',
    absolute_expires_at = clock_timestamp() + interval '8 hours'
where id = '33000000-0000-4000-8000-000000000001';
set local role authenticated;
set local request.jwt.claims = '{"sub":"11000000-0000-4000-8000-000000000001","role":"authenticated","session_id":"33000000-0000-4000-8000-000000000001","sync_device_id":"22000000-0000-4000-8000-000000000001"}';
select is((select count(*)::integer from sync_v2_proof.envelopes), 1, 'valid bound identity sees its owned row');

set local request.jwt.claims = '{"sub":"not-a-uuid","role":"authenticated","session_id":"33000000-0000-4000-8000-000000000001","sync_device_id":"22000000-0000-4000-8000-000000000001"}';
select is((select count(*)::integer from sync_v2_proof.envelopes), 0, 'non-UUID subject fails closed without aborting');
set local request.jwt.claims = '{"sub":"","role":"authenticated","session_id":"33000000-0000-4000-8000-000000000001","sync_device_id":"22000000-0000-4000-8000-000000000001"}';
select is((select count(*)::integer from sync_v2_proof.envelopes), 0, 'empty subject fails closed without aborting');
set local request.jwt.claims = '{"sub":123,"role":"authenticated","session_id":"33000000-0000-4000-8000-000000000001","sync_device_id":"22000000-0000-4000-8000-000000000001"}';
select is((select count(*)::integer from sync_v2_proof.envelopes), 0, 'numeric subject fails closed without aborting');
set local request.jwt.claims = '{"sub":{"nested":"value"},"role":"authenticated","session_id":"33000000-0000-4000-8000-000000000001","sync_device_id":"22000000-0000-4000-8000-000000000001"}';
select is((select count(*)::integer from sync_v2_proof.envelopes), 0, 'object subject fails closed without aborting');
set local request.jwt.claims = '{"role":"authenticated","session_id":"33000000-0000-4000-8000-000000000001","sync_device_id":"22000000-0000-4000-8000-000000000001"}';
select is((select count(*)::integer from sync_v2_proof.envelopes), 0, 'missing subject fails closed without aborting');
set local request.jwt.claims = 'not-json';
select is((select count(*)::integer from sync_v2_proof.envelopes), 0, 'malformed claims JSON fails closed without aborting');

set local request.jwt.claims = '{"sub":"11000000-0000-4000-8000-000000000001","role":"authenticated","session_id":"33000000-0000-4000-8000-000000000002","sync_device_id":"22000000-0000-4000-8000-000000000002"}';
select is((select count(*)::integer from sync_v2_proof.envelopes), 0, 'another owner session and device cannot be mixed with this subject');
set local request.jwt.claims = '{"sub":"11000000-0000-4000-8000-000000000001","role":"authenticated","session_id":"33000000-0000-4000-8000-000000000001","sync_device_id":"22000000-0000-4000-8000-000000000002"}';
select is((select count(*)::integer from sync_v2_proof.envelopes), 0, 'another owner device cannot be borrowed');

select * from finish();
rollback;
