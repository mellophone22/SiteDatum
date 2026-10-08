begin;

create extension if not exists pgtap with schema extensions;
select plan(43);

select has_schema('licensing', 'licensing schema exists');
select has_table('licensing', 'customers', 'customers table exists');
select has_table('licensing', 'subscriptions', 'subscriptions table exists');
select has_table('licensing', 'devices', 'devices table exists');
select has_table('licensing', 'rate_limit_buckets', 'rate-limit table exists');
select has_table('licensing', 'audit_events', 'audit table exists');

select ok((select relrowsecurity from pg_class where oid = 'licensing.customers'::regclass), 'customers uses RLS');
select ok((select relrowsecurity from pg_class where oid = 'licensing.subscriptions'::regclass), 'subscriptions uses RLS');
select ok((select relrowsecurity from pg_class where oid = 'licensing.devices'::regclass), 'devices uses RLS');
select ok((select relrowsecurity from pg_class where oid = 'licensing.rate_limit_buckets'::regclass), 'rate limits use RLS');
select ok((select relrowsecurity from pg_class where oid = 'licensing.audit_events'::regclass), 'audit events use RLS');

select ok(not has_schema_privilege('anon', 'licensing', 'USAGE'), 'anon cannot use licensing schema');
select ok(not has_schema_privilege('authenticated', 'licensing', 'USAGE'), 'authenticated cannot use licensing schema');

select ok(not has_function_privilege('anon', 'public.licensing_activate_device(uuid,text,timestamptz)', 'EXECUTE'), 'anon cannot activate devices');
select ok(not has_function_privilege('authenticated', 'public.licensing_activate_device(uuid,text,timestamptz)', 'EXECUTE'), 'authenticated cannot activate devices directly');
select ok(not has_function_privilege('anon', 'public.licensing_project_subscription(uuid,text,text,timestamptz,text,text,uuid)', 'EXECUTE'), 'anon cannot project subscriptions');
select ok(not has_function_privilege('authenticated', 'public.licensing_project_subscription(uuid,text,text,timestamptz,text,text,uuid)', 'EXECUTE'), 'authenticated cannot project subscriptions directly');
select ok(not has_function_privilege('anon', 'public.licensing_consume_rate_limit(text,text,integer,integer,timestamptz)', 'EXECUTE'), 'anon cannot consume rate-limit buckets');
select ok(not has_function_privilege('authenticated', 'public.licensing_consume_rate_limit(text,text,integer,integer,timestamptz)', 'EXECUTE'), 'authenticated cannot consume rate-limit buckets');
select ok(not has_function_privilege('anon', 'public.licensing_list_devices(uuid,text)', 'EXECUTE'), 'anon cannot list devices');
select ok(not has_function_privilege('authenticated', 'public.licensing_list_devices(uuid,text)', 'EXECUTE'), 'authenticated cannot list devices directly');

insert into auth.users (
  id, instance_id, aud, role, email, encrypted_password,
  email_confirmed_at, created_at, updated_at
) values (
  '10000000-0000-4000-8000-000000000001',
  '00000000-0000-0000-0000-000000000000',
  'authenticated', 'authenticated', 'licensing-test@example.invalid', '',
  clock_timestamp(), clock_timestamp(), clock_timestamp()
);

select ok(
  public.licensing_project_subscription(
    '10000000-0000-4000-8000-000000000001',
    'pro_monthly', 'active', '2035-01-01T00:00:00Z',
    'manual_test', 'subscription-test-1',
    '90000000-0000-4000-8000-000000000001'
  ) is not null,
  'subscription projection creates a provider-neutral subscription'
);
select is(
  (select count(*)::integer from licensing.audit_events
    where event_type = 'subscription_projected'
      and request_id = '90000000-0000-4000-8000-000000000001'),
  1, 'subscription projection writes an allowlisted audit event'
);

select is(
  (select activation_state from public.licensing_activate_device(
    '10000000-0000-4000-8000-000000000001', repeat('a', 64), '2030-01-01T00:00:00Z'
  )),
  'activated', 'first device is activated'
);
select is(
  (select activation_state from public.licensing_activate_device(
    '10000000-0000-4000-8000-000000000001', repeat('a', 64), '2030-01-02T00:00:00Z'
  )),
  'reused', 'same device is reused without consuming a slot'
);
select is(
  (select activation_state from public.licensing_activate_device(
    '10000000-0000-4000-8000-000000000001', repeat('b', 64), '2030-01-02T00:00:00Z'
  )),
  'activated', 'second device is activated'
);
select throws_ok(
  $$select * from public.licensing_activate_device(
    '10000000-0000-4000-8000-000000000001', repeat('c', 64), '2030-01-02T00:00:00Z'
  )$$,
  'P0001', 'LICENSING_DEVICE_LIMIT_REACHED',
  'third active device is rejected'
);
select is(
  (select count(*)::integer from public.licensing_list_devices(
    '10000000-0000-4000-8000-000000000001', repeat('a', 64)
  )),
  2, 'device listing returns only the subject active devices'
);
select is(
  (select count(*)::integer from public.licensing_list_devices(
    '10000000-0000-4000-8000-000000000001', repeat('a', 64)
  ) where is_current),
  1, 'device listing identifies the current pseudonymous device'
);
select is(
  (select count(*)::integer from public.licensing_list_devices(
    '20000000-0000-4000-8000-000000000002', repeat('a', 64)
  )),
  0, 'device listing cannot cross customer ownership'
);
select is(
  (select plan from public.licensing_current_entitlement(
    '10000000-0000-4000-8000-000000000001',
    (select id from licensing.devices where fingerprint_hash = repeat('a', 64))
  )),
  'pro_monthly', 'current entitlement is scoped to the authenticated subject and device'
);
select ok(
  public.licensing_deactivate_device(
    '10000000-0000-4000-8000-000000000001',
    (select id from licensing.devices where fingerprint_hash = repeat('a', 64)),
    '2030-01-03T00:00:00Z'
  ),
  'customer can deactivate an owned device'
);
select is(
  (select activation_state from public.licensing_activate_device(
    '10000000-0000-4000-8000-000000000001', repeat('c', 64), '2030-01-03T00:00:00Z'
  )),
  'activated', 'a replacement device can activate after deactivation'
);

select ok(
  public.licensing_project_subscription(
    '10000000-0000-4000-8000-000000000001',
    'pro_monthly', 'expired', '2035-01-01T00:00:00Z',
    'manual_test', 'subscription-test-1',
    '90000000-0000-4000-8000-000000000002'
  ) is not null,
  'subscription projection records authoritative expiration'
);
select is(
  (select activation_state from public.licensing_activate_device(
    '10000000-0000-4000-8000-000000000001', repeat('b', 64), '2030-01-04T00:00:00Z'
  )),
  'reused', 'expired subscription can reuse its already-active matching device'
);
select throws_ok(
  $$select * from public.licensing_activate_device(
    '10000000-0000-4000-8000-000000000001', repeat('e', 64), '2030-01-04T00:00:00Z'
  )$$,
  'P0001', 'LICENSING_PRO_REQUIRED',
  'expired subscription cannot activate a new device'
);
select is(
  (select subscription_status from public.licensing_current_entitlement(
    '10000000-0000-4000-8000-000000000001',
    (select id from licensing.devices where fingerprint_hash = repeat('b', 64))
  )),
  'expired', 'existing device receives the authoritative expired subscription state'
);

select ok(public.licensing_consume_rate_limit(
  repeat('d', 64), 'entitlement_issue', 2, 3600, '2030-01-01T00:00:00Z'
), 'first request is within the rate limit');
select ok(public.licensing_consume_rate_limit(
  repeat('d', 64), 'entitlement_issue', 2, 3600, '2030-01-01T00:01:00Z'
), 'second request is within the rate limit');
select ok(not public.licensing_consume_rate_limit(
  repeat('d', 64), 'entitlement_issue', 2, 3600, '2030-01-01T00:02:00Z'
), 'request above the rate limit is denied');
select ok(public.licensing_consume_rate_limit(
  repeat('d', 64), 'stripe_checkout', 1, 3600, '2030-01-01T00:02:00Z'
), 'separate actions use separate rate-limit buckets');
select ok(not public.licensing_consume_rate_limit(
  repeat('d', 64), 'stripe_checkout', 1, 3600, '2030-01-01T00:03:00Z'
), 'checkout bucket denies requests above its limit');
select ok(public.licensing_consume_rate_limit(
  repeat('d', 64), 'stripe_checkout', 1, 3600, '2030-01-01T01:02:01Z'
), 'checkout bucket resets after its fixed window');

select * from finish();
rollback;
