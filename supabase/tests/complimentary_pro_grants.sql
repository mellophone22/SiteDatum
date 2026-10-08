begin;

create extension if not exists pgtap with schema extensions;
select plan(22);

select has_table('licensing', 'complimentary_grants', 'complimentary grants table exists');
select ok(
  (select relrowsecurity from pg_class where oid = 'licensing.complimentary_grants'::regclass),
  'complimentary grants use RLS'
);
select ok(
  not has_table_privilege('anon', 'licensing.complimentary_grants', 'SELECT'),
  'anon cannot read complimentary grants'
);
select ok(
  not has_table_privilege('authenticated', 'licensing.complimentary_grants', 'SELECT'),
  'authenticated users cannot read complimentary grants directly'
);
select ok(
  not has_function_privilege('authenticated', 'public.licensing_grant_complimentary_pro(uuid,text,uuid)', 'EXECUTE'),
  'authenticated users cannot grant complimentary access'
);
select ok(
  not has_function_privilege('authenticated', 'public.licensing_revoke_complimentary_pro(uuid,text,uuid)', 'EXECUTE'),
  'authenticated users cannot revoke complimentary access'
);

insert into auth.users (
  id, instance_id, aud, role, email, encrypted_password,
  email_confirmed_at, created_at, updated_at
) values (
  'c9000000-0000-4000-8000-000000000001',
  '00000000-0000-0000-0000-000000000000',
  'authenticated', 'authenticated', 'complimentary-test@example.invalid', '',
  clock_timestamp(), clock_timestamp(), clock_timestamp()
);

select ok(
  public.licensing_grant_complimentary_pro(
    'c9000000-0000-4000-8000-000000000001',
    'CORE_TEAM',
    'c9000000-0000-4000-8000-000000000002'
  ) is not null,
  'service operation grants complimentary Pro by immutable Auth subject'
);
select is(
  (select count(*)::integer from licensing.complimentary_grants g
    join licensing.customers c on c.id = g.customer_id
    where c.auth_user_id = 'c9000000-0000-4000-8000-000000000001'
      and g.revoked_at_utc is null),
  1,
  'one active grant is stored without duplicating identity data'
);
select is(
  (select count(*)::integer from licensing.audit_events
    where event_type = 'complimentary_grant_created'
      and request_id = 'c9000000-0000-4000-8000-000000000002'),
  1,
  'grant creation is audited'
);
select is(
  (select activation_state from public.licensing_activate_device(
    'c9000000-0000-4000-8000-000000000001', repeat('a', 64), '2030-01-01T00:00:00Z'
  )),
  'activated',
  'complimentary Pro activates a computer'
);
select is(
  (select access_kind from public.licensing_current_entitlement(
    'c9000000-0000-4000-8000-000000000001',
    (select id from licensing.devices where fingerprint_hash = repeat('a', 64))
  )),
  'complimentary',
  'complimentary access is explicit in the entitlement projection'
);
select is(
  (select plan from public.licensing_current_entitlement(
    'c9000000-0000-4000-8000-000000000001',
    (select id from licensing.devices where fingerprint_hash = repeat('a', 64))
  )),
  'pro_annual',
  'complimentary access receives the complete Pro feature set'
);
select is(
  (select subscription_status from public.licensing_current_entitlement(
    'c9000000-0000-4000-8000-000000000001',
    (select id from licensing.devices where fingerprint_hash = repeat('a', 64))
  )),
  'active',
  'complimentary access is issued as active'
);
select is(
  (select activation_state from public.licensing_activate_device(
    'c9000000-0000-4000-8000-000000000001', repeat('b', 64), '2030-01-01T00:00:00Z'
  )),
  'activated',
  'complimentary access retains the second-computer allowance'
);
select throws_ok(
  $$select * from public.licensing_activate_device(
    'c9000000-0000-4000-8000-000000000001', repeat('c', 64), '2030-01-01T00:00:00Z'
  )$$,
  'P0001', 'LICENSING_DEVICE_LIMIT_REACHED',
  'complimentary access cannot exceed two active computers'
);
select ok(
  public.licensing_revoke_complimentary_pro(
    'c9000000-0000-4000-8000-000000000001',
    'ACCESS_ENDED',
    'c9000000-0000-4000-8000-000000000003'
  ),
  'complimentary access can be revoked'
);
select is(
  (select count(*)::integer from licensing.audit_events
    where event_type = 'complimentary_grant_revoked'
      and request_id = 'c9000000-0000-4000-8000-000000000003'),
  1,
  'grant revocation is audited'
);
select throws_ok(
  $$select * from public.licensing_activate_device(
    'c9000000-0000-4000-8000-000000000001', repeat('a', 64), '2030-01-02T00:00:00Z'
  )$$,
  'P0001', 'LICENSING_PRO_REQUIRED',
  'revoked complimentary access no longer verifies Pro'
);
select ok(
  public.licensing_grant_complimentary_pro(
    'c9000000-0000-4000-8000-000000000001',
    'CORE_TEAM',
    'c9000000-0000-4000-8000-000000000004'
  ) is not null,
  'a revoked grant can be intentionally restored'
);
select is(
  (select count(*)::integer from licensing.complimentary_grants g
    join licensing.customers c on c.id = g.customer_id
    where c.auth_user_id = 'c9000000-0000-4000-8000-000000000001'),
  1,
  'restoring access reuses the single grant row'
);

select ok(
  public.licensing_project_subscription(
    'c9000000-0000-4000-8000-000000000001',
    'pro_monthly', 'active', '2035-01-01T00:00:00Z',
    'manual_test', 'complimentary-conflict-test',
    'c9000000-0000-4000-8000-000000000005'
  ) is not null,
  'paid projection remains independent from a grant'
);
select throws_ok(
  $$select public.licensing_grant_complimentary_pro(
    'c9000000-0000-4000-8000-000000000001',
    'FOUNDER',
    'c9000000-0000-4000-8000-000000000006'
  )$$,
  'P0001', 'LICENSING_PAID_ACCESS_EXISTS',
  'grant operation refuses a currently paid account to prevent hidden double billing'
);

select * from finish();
rollback;
