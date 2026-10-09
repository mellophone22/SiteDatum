begin;

create extension if not exists pgtap with schema extensions;
select plan(24);

select has_table('sync_v2_private', 'enrollment_bridge_windows', 'bridge rate windows are private');
select has_function('public', 'sync_v2_enrollment_bridge_authorize', array['uuid','uuid','text'], 'authorization wrapper exists');
select has_function('public', 'sync_v2_enrollment_bridge_begin', array['uuid','uuid','uuid','text'], 'begin wrapper exists');
select has_function('public', 'sync_v2_enrollment_bridge_context', array['uuid','uuid','uuid'], 'context wrapper exists');
select has_function('public', 'sync_v2_enrollment_bridge_complete', array['uuid','uuid','uuid','boolean'], 'completion wrapper exists');
select ok(has_function_privilege('service_role', 'public.sync_v2_enrollment_bridge_authorize(uuid,uuid,text)', 'EXECUTE'), 'service role may authorize bridge requests');
select ok(has_function_privilege('service_role', 'public.sync_v2_enrollment_bridge_begin(uuid,uuid,uuid,text)', 'EXECUTE'), 'service role may begin through the bridge');
select ok(not has_function_privilege('authenticated', 'public.sync_v2_enrollment_bridge_authorize(uuid,uuid,text)', 'EXECUTE'), 'authenticated clients cannot authorize directly');
select ok(not has_function_privilege('authenticated', 'public.sync_v2_enrollment_bridge_begin(uuid,uuid,uuid,text)', 'EXECUTE'), 'authenticated clients cannot begin directly');
select ok(not has_function_privilege('anon', 'public.sync_v2_enrollment_bridge_complete(uuid,uuid,uuid,boolean)', 'EXECUTE'), 'anonymous clients cannot complete directly');
select ok(not has_table_privilege('service_role', 'sync_v2_private.enrollment_bridge_windows', 'SELECT'), 'service role cannot read rate rows directly');

insert into auth.users (
  id, instance_id, aud, role, email, encrypted_password,
  email_confirmed_at, created_at, updated_at
) values (
  '71000000-0000-4000-8000-000000000001',
  '00000000-0000-0000-0000-000000000000',
  'authenticated', 'authenticated', 'sync-bridge@example.invalid', '',
  clock_timestamp(), clock_timestamp(), clock_timestamp()
);
insert into auth.sessions (id, user_id, created_at, updated_at)
values (
  '72000000-0000-4000-8000-000000000001',
  '71000000-0000-4000-8000-000000000001',
  clock_timestamp(), clock_timestamp()
);

select is(
  public.sync_v2_enrollment_bridge_authorize(
    '71000000-0000-4000-8000-000000000001',
    '72000000-0000-4000-8000-000000000001',
    'begin'
  ),
  'authorized',
  'live Auth session authorizes a begin request'
);
select is(
  public.sync_v2_enrollment_bridge_authorize(
    '71000000-0000-4000-8000-000000000001',
    '72000000-0000-4000-8000-000000000099',
    'begin'
  ),
  'session_invalid',
  'unknown Auth session is refused'
);
select is(
  public.sync_v2_enrollment_bridge_authorize(
    '71000000-0000-4000-8000-000000000001',
    '72000000-0000-4000-8000-000000000001',
    'unknown'
  ),
  'request_invalid',
  'unknown bridge action is refused'
);

create temporary table bridge_enrollment as
select * from public.sync_v2_enrollment_bridge_begin(
  '71000000-0000-4000-8000-000000000001',
  '72000000-0000-4000-8000-000000000001',
  '73000000-0000-4000-8000-000000000001',
  encode(decode(repeat('77', 32), 'hex'), 'base64')
);
select is((select octet_length(decode(challenge_base64, 'base64')) from bridge_enrollment), 32, 'bridge returns a 32-byte challenge');
select is(
  (select requested_device_id from public.sync_v2_enrollment_bridge_context(
    (select enrollment_id from bridge_enrollment),
    '71000000-0000-4000-8000-000000000001',
    '72000000-0000-4000-8000-000000000001'
  )),
  '73000000-0000-4000-8000-000000000001'::uuid,
  'context is scoped to the authenticated owner and session'
);
select is(
  (select octet_length(decode(public_key_base64, 'base64')) from public.sync_v2_enrollment_bridge_context(
    (select enrollment_id from bridge_enrollment),
    '71000000-0000-4000-8000-000000000001',
    '72000000-0000-4000-8000-000000000001'
  )),
  32,
  'context returns the stored public key to the trusted verifier'
);
select is(
  (select accepted from public.sync_v2_enrollment_bridge_complete(
    (select enrollment_id from bridge_enrollment),
    '71000000-0000-4000-8000-000000000001',
    '72000000-0000-4000-8000-000000000001',
    false
  )),
  false,
  'invalid proof is refused through the bridge'
);
select is(
  (select outcome from public.sync_v2_enrollment_bridge_complete(
    (select enrollment_id from bridge_enrollment),
    '71000000-0000-4000-8000-000000000001',
    '72000000-0000-4000-8000-000000000001',
    true
  )),
  'not_accepted',
  'consumed bridge enrollment cannot be replayed'
);

select is(sync_v2_private.consume_enrollment_bridge_limit(
  '71000000-0000-4000-8000-000000000001','72000000-0000-4000-8000-000000000001','complete',2::smallint
), true, 'first request inside a small test limit passes');
select is(sync_v2_private.consume_enrollment_bridge_limit(
  '71000000-0000-4000-8000-000000000001','72000000-0000-4000-8000-000000000001','complete',2::smallint
), true, 'second request inside a small test limit passes');
select is(sync_v2_private.consume_enrollment_bridge_limit(
  '71000000-0000-4000-8000-000000000001','72000000-0000-4000-8000-000000000001','complete',2::smallint
), false, 'request beyond the durable limit is refused');

delete from auth.sessions where id = '72000000-0000-4000-8000-000000000001';
select is(
  public.sync_v2_enrollment_bridge_authorize(
    '71000000-0000-4000-8000-000000000001',
    '72000000-0000-4000-8000-000000000001',
    'complete'
  ),
  'session_invalid',
  'revoked Auth session is refused immediately'
);
select ok(
  pg_get_functiondef('public.sync_v2_enrollment_bridge_begin(uuid,uuid,uuid,text)'::regprocedure)
    like '%SET search_path TO %',
  'public security-definer wrapper has a fixed empty search path'
);

select * from finish();
rollback;
