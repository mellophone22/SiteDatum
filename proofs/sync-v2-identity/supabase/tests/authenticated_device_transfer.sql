begin;

create extension if not exists pgtap with schema extensions;
select plan(41);

select has_table('sync_v2_private', 'approved_device_transfers', 'encrypted transfer relay is private');
select has_table('sync_v2_private', 'approved_device_bridge_windows', 'approved-device rate windows are private');
select has_function('public', 'sync_v2_approved_bridge_authorize', array['uuid','uuid','text'], 'bridge authorization exists');
select has_function('public', 'sync_v2_approved_bridge_device_context', array['uuid','uuid','uuid'], 'device context exists');
select has_function('public', 'sync_v2_approved_bridge_register_transfer_key', array['uuid','uuid','uuid','text','boolean'], 'transfer-key registration bridge exists');
select has_function('public', 'sync_v2_approved_bridge_begin_target', array['uuid','uuid','uuid','uuid','text','text'], 'target begin bridge exists');
select has_function('public', 'sync_v2_approved_bridge_target_context', array['uuid','uuid','uuid'], 'target proof context exists');
select has_function('public', 'sync_v2_approved_bridge_verify_target', array['uuid','uuid','uuid','boolean'], 'target verification bridge exists');
select has_function('public', 'sync_v2_approved_bridge_source_context', array['uuid','uuid','uuid','uuid'], 'source approval context exists');
select has_function('public', 'sync_v2_approved_bridge_accept_transfer', array['uuid','uuid','uuid','uuid','uuid','integer','text','text','boolean'], 'atomic acceptance bridge exists');
select has_function('public', 'sync_v2_approved_bridge_fetch_transfer', array['uuid','uuid','uuid','uuid'], 'target fetch bridge exists');
select is((select relforcerowsecurity from pg_class where oid = 'sync_v2_private.approved_device_transfers'::regclass), true, 'transfer relay forces RLS');
select ok(not has_table_privilege('authenticated', 'sync_v2_private.approved_device_transfers', 'SELECT'), 'authenticated clients cannot read relay rows');
select ok(not has_table_privilege('service_role', 'sync_v2_private.approved_device_transfers', 'SELECT'), 'service role cannot read relay rows directly');
select ok(not has_function_privilege('authenticated', 'public.sync_v2_approved_bridge_accept_transfer(uuid,uuid,uuid,uuid,uuid,integer,text,text,boolean)', 'EXECUTE'), 'authenticated clients cannot accept directly');
select ok(not has_function_privilege('anon', 'public.sync_v2_approved_bridge_fetch_transfer(uuid,uuid,uuid,uuid)', 'EXECUTE'), 'anonymous clients cannot fetch directly');
select ok(has_function_privilege('service_role', 'public.sync_v2_approved_bridge_authorize(uuid,uuid,text)', 'EXECUTE'), 'service role may authorize bridge requests');
select ok(has_function_privilege('service_role', 'public.sync_v2_approved_bridge_accept_transfer(uuid,uuid,uuid,uuid,uuid,integer,text,text,boolean)', 'EXECUTE'), 'service role may accept a verified transfer');

insert into auth.users (
  id, instance_id, aud, role, email, encrypted_password,
  email_confirmed_at, created_at, updated_at
) values (
  'a1000000-0000-4000-8000-000000000001',
  '00000000-0000-0000-0000-000000000000',
  'authenticated', 'authenticated', 'approved-transfer@example.invalid', '',
  clock_timestamp(), clock_timestamp(), clock_timestamp()
);
insert into auth.sessions (id, user_id, created_at, updated_at) values
  ('a2000000-0000-4000-8000-000000000001','a1000000-0000-4000-8000-000000000001',clock_timestamp(),clock_timestamp()),
  ('a2000000-0000-4000-8000-000000000002','a1000000-0000-4000-8000-000000000001',clock_timestamp(),clock_timestamp());
insert into sync_v2_private.devices (
  id, owner_id, proof_algorithm, proof_public_key
) values (
  'a3000000-0000-4000-8000-000000000001',
  'a1000000-0000-4000-8000-000000000001',
  'ed25519-v1', decode(repeat('11', 32), 'hex')
);
insert into sync_v2_private.sessions (
  id, owner_id, device_id, issued_at, last_seen_at, idle_expires_at, absolute_expires_at
) values (
  'a2000000-0000-4000-8000-000000000001',
  'a1000000-0000-4000-8000-000000000001',
  'a3000000-0000-4000-8000-000000000001',
  clock_timestamp(), clock_timestamp(), clock_timestamp() + interval '30 minutes',
  clock_timestamp() + interval '8 hours'
);

select is(public.sync_v2_approved_bridge_authorize(
  'a1000000-0000-4000-8000-000000000001','a2000000-0000-4000-8000-000000000001','register_transfer_key'
), 'authorized', 'live Auth session authorizes a known action');
select is(public.sync_v2_approved_bridge_authorize(
  'a1000000-0000-4000-8000-000000000001','a2000000-0000-4000-8000-000000000001','unknown'
), 'request_invalid', 'unknown bridge action is refused');
select is(public.sync_v2_approved_bridge_authorize(
  'a1000000-0000-4000-8000-000000000001','a2000000-0000-4000-8000-000000000099','fetch_transfer'
), 'session_invalid', 'unknown Auth session is refused');
select is((select octet_length(decode(proof_public_key_base64, 'base64'))
  from public.sync_v2_approved_bridge_device_context(
    'a1000000-0000-4000-8000-000000000001','a2000000-0000-4000-8000-000000000001','a3000000-0000-4000-8000-000000000001'
  )), 32, 'device context exposes only the registered public proof key');
select is(public.sync_v2_approved_bridge_register_transfer_key(
  'a1000000-0000-4000-8000-000000000001','a2000000-0000-4000-8000-000000000001','a3000000-0000-4000-8000-000000000001',
  encode(decode(repeat('22', 32), 'hex'), 'base64'), true
), 'registered', 'verified source registers its transfer key');
select is(public.sync_v2_approved_bridge_register_transfer_key(
  'a1000000-0000-4000-8000-000000000001','a2000000-0000-4000-8000-000000000001','a3000000-0000-4000-8000-000000000001',
  encode(decode(repeat('22', 32), 'hex'), 'base64'), true
), 'already_registered', 'exact transfer-key retry is idempotent');

create temporary table transfer_enrollment as
select * from public.sync_v2_approved_bridge_begin_target(
  'a1000000-0000-4000-8000-000000000001',
  'a2000000-0000-4000-8000-000000000002',
  'a3000000-0000-4000-8000-000000000001',
  'a3000000-0000-4000-8000-000000000002',
  encode(decode(repeat('33', 32), 'hex'), 'base64'),
  encode(decode(repeat('44', 32), 'hex'), 'base64')
);

select is((select octet_length(decode(challenge_base64, 'base64')) from transfer_enrollment), 32, 'target begin returns a 32-byte challenge');
select is((select target_device_id from public.sync_v2_approved_bridge_target_context(
  (select enrollment_id from transfer_enrollment),
  'a1000000-0000-4000-8000-000000000001','a2000000-0000-4000-8000-000000000002'
)), 'a3000000-0000-4000-8000-000000000002'::uuid, 'target context is bound to the exact device');
select is((select count(*)::integer from public.sync_v2_approved_bridge_target_context(
  (select enrollment_id from transfer_enrollment),
  'a1000000-0000-4000-8000-000000000001','a2000000-0000-4000-8000-000000000001'
)), 0, 'wrong Auth session cannot read target context');
select is((select ready_for_approval from public.sync_v2_approved_bridge_verify_target(
  (select enrollment_id from transfer_enrollment),
  'a1000000-0000-4000-8000-000000000001','a2000000-0000-4000-8000-000000000002',true
)), true, 'verified target becomes ready for source approval');
select is((select target_device_id from public.sync_v2_approved_bridge_source_context(
  (select enrollment_id from transfer_enrollment),
  'a1000000-0000-4000-8000-000000000001','a2000000-0000-4000-8000-000000000001','a3000000-0000-4000-8000-000000000001'
)), 'a3000000-0000-4000-8000-000000000002'::uuid, 'live source receives the exact verified target context');

select is((select accepted from public.sync_v2_approved_bridge_accept_transfer(
  (select enrollment_id from transfer_enrollment),
  'a1000000-0000-4000-8000-000000000001','a2000000-0000-4000-8000-000000000001','a3000000-0000-4000-8000-000000000001',
  'a5000000-0000-4000-8000-000000000001',7,
  encode(decode(repeat('55', 32), 'hex'), 'base64'),
  encode(decode(repeat('66', 48), 'hex'), 'base64'),true
)), true, 'source approval atomically enrolls the target and stores the sealed key');
select is((select count(*)::integer from sync_v2_private.approved_device_transfers), 1, 'one encrypted relay row is stored');
select is((select workspace_id from public.sync_v2_approved_bridge_fetch_transfer(
  (select enrollment_id from transfer_enrollment),
  'a1000000-0000-4000-8000-000000000001','a2000000-0000-4000-8000-000000000002','a3000000-0000-4000-8000-000000000002'
)), 'a5000000-0000-4000-8000-000000000001'::uuid, 'accepted target fetches the exact random workspace identifier');
select is((select octet_length(decode(source_transfer_public_key_base64, 'base64')) from public.sync_v2_approved_bridge_fetch_transfer(
  (select enrollment_id from transfer_enrollment),
  'a1000000-0000-4000-8000-000000000001','a2000000-0000-4000-8000-000000000002','a3000000-0000-4000-8000-000000000002'
)), 32, 'fetch returns the authenticated source transfer public key');
select is((select outcome from public.sync_v2_approved_bridge_accept_transfer(
  (select enrollment_id from transfer_enrollment),
  'a1000000-0000-4000-8000-000000000001','a2000000-0000-4000-8000-000000000001','a3000000-0000-4000-8000-000000000001',
  'a5000000-0000-4000-8000-000000000001',7,
  encode(decode(repeat('55', 32), 'hex'), 'base64'),
  encode(decode(repeat('66', 48), 'hex'), 'base64'),true
)), 'already_accepted', 'exact accepted-response retry is idempotent');
select is((select accepted from public.sync_v2_approved_bridge_accept_transfer(
  (select enrollment_id from transfer_enrollment),
  'a1000000-0000-4000-8000-000000000001','a2000000-0000-4000-8000-000000000001','a3000000-0000-4000-8000-000000000001',
  'a5000000-0000-4000-8000-000000000001',8,
  encode(decode(repeat('55', 32), 'hex'), 'base64'),
  encode(decode(repeat('66', 48), 'hex'), 'base64'),true
)), false, 'altered accepted-response retry is refused');
select is((select count(*)::integer from public.sync_v2_approved_bridge_fetch_transfer(
  (select enrollment_id from transfer_enrollment),
  'a1000000-0000-4000-8000-000000000001','a2000000-0000-4000-8000-000000000002','a3000000-0000-4000-8000-000000000001'
)), 0, 'wrong device cannot fetch the envelope');
select is((select octet_length(encapsulated_key) from sync_v2_private.approved_device_transfers), 32, 'relay stores a bounded HPKE encapsulated key');
select is((select octet_length(ciphertext) from sync_v2_private.approved_device_transfers), 48, 'relay stores only a bounded sealed 32-byte workspace key');
select ok((select fetched_at is not null from sync_v2_private.approved_device_transfers), 'successful fetch records content-free delivery time');
select is((select count(*)::integer from sync_v2_private.approved_device_transfers where ciphertext = decode(repeat('66', 48), 'hex')), 1, 'relay preserves the exact opaque ciphertext');
select ok(pg_get_functiondef('public.sync_v2_approved_bridge_accept_transfer(uuid,uuid,uuid,uuid,uuid,integer,text,text,boolean)'::regprocedure) like '%SET search_path TO %', 'acceptance wrapper has a fixed empty search path');

select * from finish();
rollback;
