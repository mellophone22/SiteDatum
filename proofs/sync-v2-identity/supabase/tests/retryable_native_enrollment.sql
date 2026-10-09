begin;

create extension if not exists pgtap with schema extensions;
select plan(8);

insert into auth.users (
  id, instance_id, aud, role, email, encrypted_password,
  email_confirmed_at, created_at, updated_at
) values (
  '81000000-0000-4000-8000-000000000001',
  '00000000-0000-0000-0000-000000000000',
  'authenticated', 'authenticated', 'retryable-native@example.invalid', '',
  clock_timestamp(), clock_timestamp(), clock_timestamp()
);
insert into auth.sessions (id, user_id, created_at, updated_at)
values (
  '82000000-0000-4000-8000-000000000001',
  '81000000-0000-4000-8000-000000000001',
  clock_timestamp(), clock_timestamp()
);

create temporary table retryable_enrollment as
select * from public.sync_v2_enrollment_bridge_begin(
  '81000000-0000-4000-8000-000000000001',
  '82000000-0000-4000-8000-000000000001',
  '83000000-0000-4000-8000-000000000001',
  encode(decode(repeat('88', 32), 'hex'), 'base64')
);

select is(
  (select accepted from public.sync_v2_enrollment_bridge_complete(
    (select enrollment_id from retryable_enrollment),
    '81000000-0000-4000-8000-000000000001',
    '82000000-0000-4000-8000-000000000001',
    true
  )),
  true,
  'first verified completion is accepted'
);
select is(
  (select outcome from public.sync_v2_enrollment_bridge_complete(
    (select enrollment_id from retryable_enrollment),
    '81000000-0000-4000-8000-000000000001',
    '82000000-0000-4000-8000-000000000001',
    true
  )),
  'already_accepted',
  'verified completion retry reports the exact accepted enrollment'
);
select is(
  (select device_id from public.sync_v2_enrollment_bridge_complete(
    (select enrollment_id from retryable_enrollment),
    '81000000-0000-4000-8000-000000000001',
    '82000000-0000-4000-8000-000000000001',
    true
  )),
  '83000000-0000-4000-8000-000000000001'::uuid,
  'completion retry returns the originally accepted device'
);
select is(
  (select requested_device_id from public.sync_v2_enrollment_bridge_context(
    (select enrollment_id from retryable_enrollment),
    '81000000-0000-4000-8000-000000000001',
    '82000000-0000-4000-8000-000000000001'
  )),
  '83000000-0000-4000-8000-000000000001'::uuid,
  'trusted verifier can rebuild the proof after accepted-response loss'
);
select is(
  (select accepted from public.sync_v2_enrollment_bridge_complete(
    (select enrollment_id from retryable_enrollment),
    '81000000-0000-4000-8000-000000000001',
    '82000000-0000-4000-8000-000000000001',
    false
  )),
  false,
  'an invalid proof cannot use the idempotent success path'
);
select throws_ok(
  format(
    'select * from public.sync_v2_enrollment_bridge_context(%L,%L,%L)',
    (select enrollment_id from retryable_enrollment),
    '81000000-0000-4000-8000-000000000099',
    '82000000-0000-4000-8000-000000000001'
  ),
  'P0001',
  'BRIDGE_AUTH_SESSION_INVALID',
  'a different owner cannot retrieve accepted enrollment context'
);
select ok(
  not has_function_privilege(
    'authenticated',
    'public.sync_v2_enrollment_bridge_complete(uuid,uuid,uuid,boolean)',
    'EXECUTE'
  ),
  'authenticated clients still cannot call completion directly'
);

delete from auth.sessions where id = '82000000-0000-4000-8000-000000000001';
select throws_ok(
  format(
    'select * from public.sync_v2_enrollment_bridge_complete(%L,%L,%L,true)',
    (select enrollment_id from retryable_enrollment),
    '81000000-0000-4000-8000-000000000001',
    '82000000-0000-4000-8000-000000000001'
  ),
  'P0001',
  'BRIDGE_AUTH_SESSION_INVALID',
  'revoked Auth session cannot use idempotent completion'
);

select * from finish();
rollback;
