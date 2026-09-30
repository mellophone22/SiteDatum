begin;

create extension if not exists pgtap with schema extensions;
select plan(17);

select has_table('licensing', 'checkout_correlations', 'checkout correlations exist');
select has_table('licensing', 'provider_events', 'provider event ledger exists');
select ok((select relrowsecurity from pg_class where oid = 'licensing.provider_events'::regclass), 'provider events use RLS');
select ok(not has_schema_privilege('authenticated', 'licensing', 'USAGE'), 'billing tables are private');
select ok(not has_function_privilege('authenticated', 'public.licensing_apply_stripe_subscription_event(text,text,text,timestamptz,text,text,uuid,text,text,timestamptz,timestamptz)', 'EXECUTE'), 'clients cannot project Stripe state');

insert into auth.users (id, instance_id, aud, role, email, encrypted_password, email_confirmed_at, created_at, updated_at)
values ('20000000-0000-4000-8000-000000000001', '00000000-0000-0000-0000-000000000000', 'authenticated', 'authenticated', 'stripe-test@example.invalid', '', clock_timestamp(), clock_timestamp(), clock_timestamp());

select lives_ok($$
  select public.licensing_create_checkout_correlation(
    '20000000-0000-4000-8000-000000000001', 'pro_monthly', '2030-01-01T00:00:00Z'
  )
$$, 'server can create an opaque checkout correlation');

select is(
  public.licensing_apply_stripe_subscription_event(
    'evt_created', 'customer.subscription.created', repeat('a', 64), '2030-01-01T00:01:00Z',
    'sub_test', 'cus_test', (select id from licensing.checkout_correlations limit 1),
    'pro_monthly', 'active', '2030-02-01T00:00:00Z', '2030-01-01T00:01:00Z'
  ), 'applied', 'created subscription is projected'
);
select is((select status from licensing.subscriptions where external_subscription_ref = 'sub_test'), 'active', 'active status is stored');
select is((select count(*)::integer from licensing.checkout_correlations where consumed_at_utc is not null), 1, 'correlation is one-time');

select is(
  public.licensing_apply_stripe_subscription_event(
    'evt_created', 'customer.subscription.created', repeat('a', 64), '2030-01-01T00:01:00Z',
    'sub_test', 'cus_test', null, 'pro_monthly', 'active', '2030-02-01T00:00:00Z', '2030-01-01T00:01:00Z'
  ), 'duplicate', 'duplicate event is idempotent'
);
select is((select count(*)::integer from licensing.provider_events where external_event_ref = 'evt_created'), 1, 'duplicate event has one ledger row');

select is(
  public.licensing_apply_stripe_subscription_event(
    'evt_cancel', 'customer.subscription.updated', repeat('b', 64), '2030-01-20T00:00:00Z',
    'sub_test', 'cus_test', null, 'pro_monthly', 'canceled', '2030-02-01T00:00:00Z', '2030-01-20T00:00:00Z'
  ), 'applied', 'cancellation preserves paid-through access'
);
select is((select status from licensing.subscriptions where external_subscription_ref = 'sub_test'), 'canceled', 'canceled status is stored');

select is(
  public.licensing_apply_stripe_subscription_event(
    'evt_delayed', 'invoice.paid', repeat('c', 64), '2030-01-10T00:00:00Z',
    'sub_test', 'cus_test', null, 'pro_monthly', 'active', '2030-02-01T00:00:00Z', '2030-01-10T00:00:00Z'
  ), 'ignored_stale', 'delayed state cannot regress cancellation'
);
select is((select status from licensing.subscriptions where external_subscription_ref = 'sub_test'), 'canceled', 'stale event leaves current state intact');

select is(
  public.licensing_apply_stripe_subscription_event(
    'evt_expired', 'customer.subscription.deleted', repeat('d', 64), '2030-02-01T00:00:01Z',
    'sub_test', 'cus_test', null, 'pro_monthly', 'expired', '2030-02-01T00:00:00Z', '2030-02-01T00:00:01Z'
  ), 'applied', 'expiration is projected without deleting customer data'
);
select is((select status from licensing.subscriptions where external_subscription_ref = 'sub_test'), 'expired', 'expired status is stored');

select * from finish();
rollback;
