begin;

create extension if not exists pgtap with schema extensions;
select plan(30);

select has_table('sync_v2_private', 'device_enrollments', 'private enrollment table exists');
select has_function('sync_v2_private', 'begin_initial_device_enrollment', array['uuid','uuid','uuid','bytea'], 'begin function exists');
select has_function('sync_v2_private', 'complete_initial_device_enrollment', array['uuid','uuid','uuid','boolean'], 'complete function exists');
select ok(has_function_privilege('service_role', 'sync_v2_private.begin_initial_device_enrollment(uuid,uuid,uuid,bytea)', 'EXECUTE'), 'trusted service may begin enrollment');
select ok(has_function_privilege('service_role', 'sync_v2_private.complete_initial_device_enrollment(uuid,uuid,uuid,boolean)', 'EXECUTE'), 'trusted service may complete enrollment');
select ok(not has_function_privilege('authenticated', 'sync_v2_private.begin_initial_device_enrollment(uuid,uuid,uuid,bytea)', 'EXECUTE'), 'authenticated client cannot begin directly');
select ok(not has_function_privilege('authenticated', 'sync_v2_private.complete_initial_device_enrollment(uuid,uuid,uuid,boolean)', 'EXECUTE'), 'authenticated client cannot complete directly');
select ok(not has_function_privilege('anon', 'sync_v2_private.begin_initial_device_enrollment(uuid,uuid,uuid,bytea)', 'EXECUTE'), 'anonymous client cannot begin directly');
select ok(not has_table_privilege('service_role', 'sync_v2_private.device_enrollments', 'SELECT'), 'trusted service has no direct enrollment-table read');

create temporary table enrollment_result as
select * from sync_v2_private.begin_initial_device_enrollment(
  '51000000-0000-4000-8000-000000000001',
  '52000000-0000-4000-8000-000000000001',
  '53000000-0000-4000-8000-000000000001',
  decode(repeat('11', 32), 'hex')
);

select is((select octet_length(challenge) from enrollment_result), 32, 'server creates a 32-byte challenge');
select ok((select expires_at > clock_timestamp() from enrollment_result), 'enrollment expires in the future');
select ok((select expires_at <= clock_timestamp() + interval '5 minutes 5 seconds' from enrollment_result), 'enrollment lifetime is bounded to five minutes');

create temporary table invalid_result as
select * from sync_v2_private.complete_initial_device_enrollment(
  (select enrollment_id from enrollment_result),
  '51000000-0000-4000-8000-000000000001',
  '52000000-0000-4000-8000-000000000001',
  false
);
select is((select accepted from invalid_result), false, 'invalid proof is refused');
select is((select outcome from invalid_result), 'invalid_proof', 'invalid proof records a content-free outcome');
select is(
  (select outcome from sync_v2_private.device_enrollments where id = (select enrollment_id from enrollment_result)),
  'invalid_proof',
  'invalid proof durably consumes the enrollment'
);
select is(
  (select outcome from sync_v2_private.complete_initial_device_enrollment(
    (select enrollment_id from enrollment_result),
    '51000000-0000-4000-8000-000000000001',
    '52000000-0000-4000-8000-000000000001',
    true
  )),
  'not_accepted',
  'consumed enrollment cannot be replayed with a valid proof'
);

create temporary table accepted_enrollment as
select * from sync_v2_private.begin_initial_device_enrollment(
  '51000000-0000-4000-8000-000000000001',
  '52000000-0000-4000-8000-000000000002',
  '53000000-0000-4000-8000-000000000002',
  decode(repeat('22', 32), 'hex')
);
create temporary table accepted_result as
select * from sync_v2_private.complete_initial_device_enrollment(
  (select enrollment_id from accepted_enrollment),
  '51000000-0000-4000-8000-000000000001',
  '52000000-0000-4000-8000-000000000002',
  true
);
select is((select accepted from accepted_result), true, 'valid proof accepts the first device');
select is((select outcome from accepted_result), 'accepted', 'successful completion records accepted');
select is((select count(*)::integer from sync_v2_private.devices where owner_id = '51000000-0000-4000-8000-000000000001'), 1, 'one device is created');
select is((select count(*)::integer from sync_v2_private.sessions where id = '52000000-0000-4000-8000-000000000002'), 1, 'authenticated session is bound to the device');
select is((select proof_algorithm from sync_v2_private.devices where id = '53000000-0000-4000-8000-000000000002'), 'ed25519-v1', 'device records the versioned proof algorithm');

select throws_ok(
  $$select * from sync_v2_private.begin_initial_device_enrollment(
    '51000000-0000-4000-8000-000000000001',
    '52000000-0000-4000-8000-000000000003',
    '53000000-0000-4000-8000-000000000003',
    decode(repeat('33', 32), 'hex')
  )$$,
  'P0001',
  'ENROLLMENT_EXISTING_DEVICE_REQUIRES_APPROVAL',
  'an owner with an active device cannot self-bootstrap another'
);

create temporary table mismatch_enrollment as
select * from sync_v2_private.begin_initial_device_enrollment(
  '51000000-0000-4000-8000-000000000010',
  '52000000-0000-4000-8000-000000000010',
  '53000000-0000-4000-8000-000000000010',
  decode(repeat('44', 32), 'hex')
);
select is(
  (select outcome from sync_v2_private.complete_initial_device_enrollment(
    (select enrollment_id from mismatch_enrollment),
    '51000000-0000-4000-8000-000000000099',
    '52000000-0000-4000-8000-000000000010',
    true
  )),
  'context_mismatch',
  'wrong owner consumes and refuses the enrollment'
);

create temporary table expired_enrollment as
select * from sync_v2_private.begin_initial_device_enrollment(
  '51000000-0000-4000-8000-000000000020',
  '52000000-0000-4000-8000-000000000020',
  '53000000-0000-4000-8000-000000000020',
  decode(repeat('55', 32), 'hex')
);
update sync_v2_private.device_enrollments
set created_at = clock_timestamp() - interval '10 minutes',
    expires_at = clock_timestamp() - interval '5 minutes'
where id = (select enrollment_id from expired_enrollment);
select is(
  (select outcome from sync_v2_private.complete_initial_device_enrollment(
    (select enrollment_id from expired_enrollment),
    '51000000-0000-4000-8000-000000000020',
    '52000000-0000-4000-8000-000000000020',
    true
  )),
  'expired',
  'server time refuses and consumes an expired enrollment'
);

select throws_ok(
  $$select * from sync_v2_private.begin_initial_device_enrollment(
    '51000000-0000-4000-8000-000000000030',
    '52000000-0000-4000-8000-000000000030',
    '53000000-0000-4000-8000-000000000030',
    decode(repeat('66', 31), 'hex')
  )$$,
  '22023',
  'ENROLLMENT_INVALID_INPUT',
  'invalid public-key length is refused'
);

do $$
begin
  perform * from sync_v2_private.begin_initial_device_enrollment('51000000-0000-4000-8000-000000000040','52000000-0000-4000-8000-000000000040','53000000-0000-4000-8000-000000000041',decode(repeat('71',32),'hex'));
  perform * from sync_v2_private.begin_initial_device_enrollment('51000000-0000-4000-8000-000000000040','52000000-0000-4000-8000-000000000040','53000000-0000-4000-8000-000000000042',decode(repeat('72',32),'hex'));
  perform * from sync_v2_private.begin_initial_device_enrollment('51000000-0000-4000-8000-000000000040','52000000-0000-4000-8000-000000000040','53000000-0000-4000-8000-000000000043',decode(repeat('73',32),'hex'));
  perform * from sync_v2_private.begin_initial_device_enrollment('51000000-0000-4000-8000-000000000040','52000000-0000-4000-8000-000000000040','53000000-0000-4000-8000-000000000044',decode(repeat('74',32),'hex'));
  perform * from sync_v2_private.begin_initial_device_enrollment('51000000-0000-4000-8000-000000000040','52000000-0000-4000-8000-000000000040','53000000-0000-4000-8000-000000000045',decode(repeat('75',32),'hex'));
end;
$$;
select throws_ok(
  $$select * from sync_v2_private.begin_initial_device_enrollment(
    '51000000-0000-4000-8000-000000000040',
    '52000000-0000-4000-8000-000000000040',
    '53000000-0000-4000-8000-000000000046',
    decode(repeat('76', 32), 'hex')
  )$$,
  'P0001',
  'ENROLLMENT_RATE_LIMITED',
  'sixth enrollment in one hour is rate limited'
);

select isnt(
  (select challenge from sync_v2_private.device_enrollments where requested_device_id = '53000000-0000-4000-8000-000000000041'),
  (select challenge from sync_v2_private.device_enrollments where requested_device_id = '53000000-0000-4000-8000-000000000042'),
  'server challenges are independently random'
);

select is(
  sync_v2_private.custom_access_token_hook(
    '{"user_id":"51000000-0000-4000-8000-000000000001","claims":{"sub":"51000000-0000-4000-8000-000000000001","session_id":"52000000-0000-4000-8000-000000000002"}}'::jsonb
  ) -> 'claims' ->> 'sync_device_id',
  '53000000-0000-4000-8000-000000000002',
  'accepted enrollment feeds the server-derived device claim'
);

select is((select count(*)::integer from sync_v2_private.device_enrollments where outcome is null), 5, 'pending rate-limit rows contain no content or outcome');
select is((select count(*)::integer from sync_v2_private.device_enrollments where attempt_count > 1), 0, 'no enrollment can record more than one attempt');

select * from finish();
rollback;
