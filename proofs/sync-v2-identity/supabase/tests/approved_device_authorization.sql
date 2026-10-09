begin;

create extension if not exists pgtap with schema extensions;
select plan(46);

select has_table('sync_v2_private', 'approved_device_enrollments', 'private approved-device table exists');
select has_column('sync_v2_private', 'devices', 'transfer_public_key', 'device has transfer public key');
select has_function('sync_v2_private', 'register_device_transfer_key', array['uuid','uuid','uuid','bytea','boolean'], 'transfer-key registration exists');
select has_function('sync_v2_private', 'begin_approved_device_enrollment', array['uuid','uuid','uuid','uuid','bytea','bytea'], 'approved-device begin exists');
select has_function('sync_v2_private', 'verify_approved_device_target', array['uuid','uuid','uuid','boolean'], 'target proof function exists');
select has_function('sync_v2_private', 'complete_approved_device_enrollment', array['uuid','uuid','uuid','uuid','boolean'], 'source approval function exists');
select ok(not has_table_privilege('service_role', 'sync_v2_private.approved_device_enrollments', 'SELECT'), 'service role cannot read approval rows directly');
select ok(has_function_privilege('service_role', 'sync_v2_private.begin_approved_device_enrollment(uuid,uuid,uuid,uuid,bytea,bytea)', 'EXECUTE'), 'service role can begin through the narrow function');
select ok(not has_function_privilege('authenticated', 'sync_v2_private.begin_approved_device_enrollment(uuid,uuid,uuid,uuid,bytea,bytea)', 'EXECUTE'), 'authenticated client cannot begin directly');
select ok(not has_function_privilege('anon', 'sync_v2_private.complete_approved_device_enrollment(uuid,uuid,uuid,uuid,boolean)', 'EXECUTE'), 'anonymous client cannot approve directly');
select ok(not has_function_privilege('authenticated', 'sync_v2_private.register_device_transfer_key(uuid,uuid,uuid,bytea,boolean)', 'EXECUTE'), 'authenticated client cannot register a transfer key directly');
select ok(not has_function_privilege('authenticated', 'sync_v2_private.verify_approved_device_target(uuid,uuid,uuid,boolean)', 'EXECUTE'), 'authenticated client cannot verify target possession directly');
select ok(not has_function_privilege('service_role', 'sync_v2_private.device_session_is_active(uuid,uuid,uuid)', 'EXECUTE'), 'service role cannot call the private session helper directly');

insert into auth.users (
  id, instance_id, aud, role, email, encrypted_password,
  email_confirmed_at, created_at, updated_at
) values
  (
    'a1000000-0000-4000-8000-000000000001',
    '00000000-0000-0000-0000-000000000000',
    'authenticated', 'authenticated', 'approval-owner@example.invalid', '',
    clock_timestamp(), clock_timestamp(), clock_timestamp()
  ),
  (
    'a1000000-0000-4000-8000-000000000002',
    '00000000-0000-0000-0000-000000000000',
    'authenticated', 'authenticated', 'approval-other@example.invalid', '',
    clock_timestamp(), clock_timestamp(), clock_timestamp()
  );

insert into auth.sessions (id, user_id, created_at, updated_at) values
  ('a2000000-0000-4000-8000-000000000001', 'a1000000-0000-4000-8000-000000000001', clock_timestamp(), clock_timestamp()),
  ('a2000000-0000-4000-8000-000000000002', 'a1000000-0000-4000-8000-000000000001', clock_timestamp(), clock_timestamp()),
  ('a2000000-0000-4000-8000-000000000003', 'a1000000-0000-4000-8000-000000000001', clock_timestamp(), clock_timestamp()),
  ('a2000000-0000-4000-8000-000000000004', 'a1000000-0000-4000-8000-000000000001', clock_timestamp(), clock_timestamp()),
  ('a2000000-0000-4000-8000-000000000005', 'a1000000-0000-4000-8000-000000000002', clock_timestamp(), clock_timestamp());

insert into sync_v2_private.devices (
  id, owner_id, proof_algorithm, proof_public_key
) values (
  'a3000000-0000-4000-8000-000000000001',
  'a1000000-0000-4000-8000-000000000001',
  'ed25519-v1',
  decode(repeat('11', 32), 'hex')
);
insert into sync_v2_private.sessions (
  id, owner_id, device_id, issued_at, last_seen_at,
  idle_expires_at, absolute_expires_at
) values (
  'a2000000-0000-4000-8000-000000000001',
  'a1000000-0000-4000-8000-000000000001',
  'a3000000-0000-4000-8000-000000000001',
  clock_timestamp(), clock_timestamp(),
  clock_timestamp() + interval '30 minutes',
  clock_timestamp() + interval '8 hours'
);

select throws_ok(
  $$select sync_v2_private.register_device_transfer_key(
    'a1000000-0000-4000-8000-000000000001',
    'a2000000-0000-4000-8000-000000000001',
    'a3000000-0000-4000-8000-000000000001',
    decode(repeat('20', 31), 'hex'),
    true
  )$$,
  '22023',
  'TRANSFER_KEY_INVALID_INPUT',
  'malformed transfer keys are rejected before proof evaluation'
);

select is(
  sync_v2_private.register_device_transfer_key(
    'a1000000-0000-4000-8000-000000000001',
    'a2000000-0000-4000-8000-000000000001',
    'a3000000-0000-4000-8000-000000000001',
    decode(repeat('21', 32), 'hex'),
    false
  ),
  'invalid_proof',
  'invalid possession proof cannot register a transfer key'
);
select is(
  (select transfer_public_key is null from sync_v2_private.devices where id = 'a3000000-0000-4000-8000-000000000001'),
  true,
  'invalid registration leaves the device unchanged'
);
select is(
  sync_v2_private.register_device_transfer_key(
    'a1000000-0000-4000-8000-000000000001',
    'a2000000-0000-4000-8000-000000000001',
    'a3000000-0000-4000-8000-000000000001',
    decode(repeat('21', 32), 'hex'),
    true
  ),
  'registered',
  'verified source registers its X25519 transfer key'
);
select is(
  sync_v2_private.register_device_transfer_key(
    'a1000000-0000-4000-8000-000000000001',
    'a2000000-0000-4000-8000-000000000001',
    'a3000000-0000-4000-8000-000000000001',
    decode(repeat('21', 32), 'hex'),
    true
  ),
  'already_registered',
  'exact transfer-key retry is idempotent'
);
select is(
  sync_v2_private.register_device_transfer_key(
    'a1000000-0000-4000-8000-000000000001',
    'a2000000-0000-4000-8000-000000000001',
    'a3000000-0000-4000-8000-000000000001',
    decode(repeat('22', 32), 'hex'),
    true
  ),
  'key_conflict',
  'registered transfer key cannot be silently replaced'
);

select throws_ok(
  $$select * from sync_v2_private.begin_approved_device_enrollment(
    'a1000000-0000-4000-8000-000000000002',
    'a2000000-0000-4000-8000-000000000005',
    'a3000000-0000-4000-8000-000000000001',
    'a3000000-0000-4000-8000-000000000099',
    decode(repeat('30', 32), 'hex'),
    decode(repeat('40', 32), 'hex')
  )$$,
  'P0001',
  'APPROVED_DEVICE_SOURCE_INVALID',
  'another owner cannot nominate the source device'
);

create temporary table invalid_target_enrollment as
select * from sync_v2_private.begin_approved_device_enrollment(
  'a1000000-0000-4000-8000-000000000001',
  'a2000000-0000-4000-8000-000000000002',
  'a3000000-0000-4000-8000-000000000001',
  'a3000000-0000-4000-8000-000000000002',
  decode(repeat('31', 32), 'hex'),
  decode(repeat('41', 32), 'hex')
);
select is((select octet_length(challenge) from invalid_target_enrollment), 32, 'server creates a target challenge');
select ok((select expires_at > clock_timestamp() from invalid_target_enrollment), 'approval expires in the future');
select ok((select expires_at <= clock_timestamp() + interval '10 minutes 5 seconds' from invalid_target_enrollment), 'approval lifetime is bounded to ten minutes');
select is(
  (select outcome from sync_v2_private.verify_approved_device_target(
    (select enrollment_id from invalid_target_enrollment),
    'a1000000-0000-4000-8000-000000000002',
    'a2000000-0000-4000-8000-000000000005',
    true
  )),
  'not_ready',
  'another owner cannot verify the target proof'
);
select is(
  (select outcome from sync_v2_private.verify_approved_device_target(
    (select enrollment_id from invalid_target_enrollment),
    'a1000000-0000-4000-8000-000000000001',
    'a2000000-0000-4000-8000-000000000002',
    false
  )),
  'invalid_target_proof',
  'invalid target proof consumes the enrollment'
);
select is(
  (select outcome from sync_v2_private.verify_approved_device_target(
    (select enrollment_id from invalid_target_enrollment),
    'a1000000-0000-4000-8000-000000000001',
    'a2000000-0000-4000-8000-000000000002',
    true
  )),
  'not_ready',
  'consumed target enrollment cannot be replayed'
);

create temporary table expired_enrollment as
select * from sync_v2_private.begin_approved_device_enrollment(
  'a1000000-0000-4000-8000-000000000001',
  'a2000000-0000-4000-8000-000000000002',
  'a3000000-0000-4000-8000-000000000001',
  'a3000000-0000-4000-8000-000000000005',
  decode(repeat('35', 32), 'hex'),
  decode(repeat('45', 32), 'hex')
);
update sync_v2_private.approved_device_enrollments
   set created_at = clock_timestamp() - interval '11 minutes',
       expires_at = clock_timestamp() - interval '1 minute'
 where id = (select enrollment_id from expired_enrollment);
select is(
  (select outcome from sync_v2_private.verify_approved_device_target(
    (select enrollment_id from expired_enrollment),
    'a1000000-0000-4000-8000-000000000001',
    'a2000000-0000-4000-8000-000000000002',
    true
  )),
  'expired',
  'expired target authorization is consumed instead of accepted'
);
select ok(
  (select consumed_at is not null from sync_v2_private.approved_device_enrollments where id = (select enrollment_id from expired_enrollment)),
  'expired authorization records a durable consumption time'
);

create temporary table revoked_source_enrollment as
select * from sync_v2_private.begin_approved_device_enrollment(
  'a1000000-0000-4000-8000-000000000001',
  'a2000000-0000-4000-8000-000000000002',
  'a3000000-0000-4000-8000-000000000001',
  'a3000000-0000-4000-8000-000000000006',
  decode(repeat('36', 32), 'hex'),
  decode(repeat('46', 32), 'hex')
);
select is(
  (select outcome from sync_v2_private.verify_approved_device_target(
    (select enrollment_id from revoked_source_enrollment),
    'a1000000-0000-4000-8000-000000000001',
    'a2000000-0000-4000-8000-000000000002',
    true
  )),
  'awaiting_source_approval',
  'target verification alone does not bypass source authorization'
);
update sync_v2_private.sessions
   set revoked_at = clock_timestamp()
 where id = 'a2000000-0000-4000-8000-000000000001';
select is(
  (select outcome from sync_v2_private.complete_approved_device_enrollment(
    (select enrollment_id from revoked_source_enrollment),
    'a1000000-0000-4000-8000-000000000001',
    'a2000000-0000-4000-8000-000000000001',
    'a3000000-0000-4000-8000-000000000001',
    true
  )),
  'source_invalid',
  'a revoked source session cannot approve the target'
);
update sync_v2_private.sessions
   set revoked_at = null
 where id = 'a2000000-0000-4000-8000-000000000001';
select is(
  (select outcome from sync_v2_private.complete_approved_device_enrollment(
    (select enrollment_id from revoked_source_enrollment),
    'a1000000-0000-4000-8000-000000000001',
    'a2000000-0000-4000-8000-000000000001',
    'a3000000-0000-4000-8000-000000000001',
    false
  )),
  'invalid_source_approval',
  'revocation refusal does not consume the enrollment before a valid source session acts'
);

create temporary table invalid_source_enrollment as
select * from sync_v2_private.begin_approved_device_enrollment(
  'a1000000-0000-4000-8000-000000000001',
  'a2000000-0000-4000-8000-000000000003',
  'a3000000-0000-4000-8000-000000000001',
  'a3000000-0000-4000-8000-000000000003',
  decode(repeat('32', 32), 'hex'),
  decode(repeat('42', 32), 'hex')
);
select is(
  (select outcome from sync_v2_private.verify_approved_device_target(
    (select enrollment_id from invalid_source_enrollment),
    'a1000000-0000-4000-8000-000000000001',
    'a2000000-0000-4000-8000-000000000003',
    true
  )),
  'awaiting_source_approval',
  'valid target proof advances to source approval'
);
select is(
  (select outcome from sync_v2_private.verify_approved_device_target(
    (select enrollment_id from invalid_source_enrollment),
    'a1000000-0000-4000-8000-000000000001',
    'a2000000-0000-4000-8000-000000000003',
    true
  )),
  'already_verified',
  'exact target-proof retry is idempotent before approval'
);
select is(
  (select outcome from sync_v2_private.complete_approved_device_enrollment(
    (select enrollment_id from invalid_source_enrollment),
    'a1000000-0000-4000-8000-000000000002',
    'a2000000-0000-4000-8000-000000000005',
    'a3000000-0000-4000-8000-000000000001',
    true
  )),
  'source_invalid',
  'another owner cannot approve or consume the enrollment'
);
select is(
  (select outcome from sync_v2_private.complete_approved_device_enrollment(
    (select enrollment_id from invalid_source_enrollment),
    'a1000000-0000-4000-8000-000000000001',
    'a2000000-0000-4000-8000-000000000001',
    'a3000000-0000-4000-8000-000000000001',
    false
  )),
  'invalid_source_approval',
  'invalid source approval consumes the enrollment'
);
select is(
  (select outcome from sync_v2_private.complete_approved_device_enrollment(
    (select enrollment_id from invalid_source_enrollment),
    'a1000000-0000-4000-8000-000000000001',
    'a2000000-0000-4000-8000-000000000001',
    'a3000000-0000-4000-8000-000000000001',
    true
  )),
  'not_accepted',
  'consumed source approval cannot be replayed as valid'
);

create temporary table accepted_enrollment as
select * from sync_v2_private.begin_approved_device_enrollment(
  'a1000000-0000-4000-8000-000000000001',
  'a2000000-0000-4000-8000-000000000004',
  'a3000000-0000-4000-8000-000000000001',
  'a3000000-0000-4000-8000-000000000004',
  decode(repeat('33', 32), 'hex'),
  decode(repeat('43', 32), 'hex')
);
select is(
  (select ready_for_approval from sync_v2_private.verify_approved_device_target(
    (select enrollment_id from accepted_enrollment),
    'a1000000-0000-4000-8000-000000000001',
    'a2000000-0000-4000-8000-000000000004',
    true
  )),
  true,
  'accepted target proves possession first'
);
select is(
  (select accepted from sync_v2_private.complete_approved_device_enrollment(
    (select enrollment_id from accepted_enrollment),
    'a1000000-0000-4000-8000-000000000001',
    'a2000000-0000-4000-8000-000000000001',
    'a3000000-0000-4000-8000-000000000001',
    true
  )),
  true,
  'verified source accepts the second device'
);
select is(
  (select count(*)::integer from sync_v2_private.devices where owner_id = 'a1000000-0000-4000-8000-000000000001' and revoked_at is null),
  2,
  'owner has exactly two active devices'
);
select is(
  (select transfer_algorithm from sync_v2_private.devices where id = 'a3000000-0000-4000-8000-000000000004'),
  'x25519-hpke-auth-v1',
  'target transfer key is registered with a versioned algorithm'
);
select is(
  (select device_id from sync_v2_private.sessions where id = 'a2000000-0000-4000-8000-000000000004'),
  'a3000000-0000-4000-8000-000000000004'::uuid,
  'target Auth session is bound to the accepted device'
);
select is(
  (select outcome from sync_v2_private.complete_approved_device_enrollment(
    (select enrollment_id from accepted_enrollment),
    'a1000000-0000-4000-8000-000000000001',
    'a2000000-0000-4000-8000-000000000001',
    'a3000000-0000-4000-8000-000000000001',
    true
  )),
  'already_accepted',
  'lost accepted response can be retried exactly'
);
select throws_ok(
  $$select * from sync_v2_private.begin_approved_device_enrollment(
    'a1000000-0000-4000-8000-000000000001',
    'a2000000-0000-4000-8000-000000000002',
    'a3000000-0000-4000-8000-000000000001',
    'a3000000-0000-4000-8000-000000000099',
    decode(repeat('39', 32), 'hex'),
    decode(repeat('49', 32), 'hex')
  )$$,
  'P0001',
  'APPROVED_DEVICE_LIMIT_REACHED',
  'a third active device is refused'
);
select is(
  (select outcome from sync_v2_private.approved_device_enrollments where id = (select enrollment_id from accepted_enrollment)),
  'accepted',
  'approval stores only a content-free outcome'
);
select is(
  (select target_attempt_count from sync_v2_private.approved_device_enrollments where id = (select enrollment_id from accepted_enrollment)),
  1::smallint,
  'target proof has one durable attempt'
);
select is(
  (select source_attempt_count from sync_v2_private.approved_device_enrollments where id = (select enrollment_id from accepted_enrollment)),
  1::smallint,
  'source approval has one durable attempt'
);

select * from finish();
rollback;
