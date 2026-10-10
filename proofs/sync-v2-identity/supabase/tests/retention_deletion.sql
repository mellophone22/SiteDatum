begin;

create extension if not exists pgtap with schema extensions;
select plan(35);

select has_column('sync_v2_private', 'workspaces', 'purge_after', 'disabled workspace has a purge deadline');
select has_table('sync_v2_private', 'deletion_receipts', 'content-free deletion receipts are private');
select is((select relforcerowsecurity from pg_class where oid = 'sync_v2_private.deletion_receipts'::regclass), true, 'deletion receipts force RLS');
select ok(not has_table_privilege('authenticated', 'sync_v2_private.deletion_receipts', 'SELECT'), 'authenticated clients cannot read receipts directly');
select ok(not has_table_privilege('service_role', 'sync_v2_private.deletion_receipts', 'SELECT'), 'service role cannot bypass the receipt wrapper');
select ok(not has_function_privilege('authenticated', 'public.sync_v2_lifecycle_bridge_delete_workspace(uuid,uuid,uuid,uuid,uuid)', 'EXECUTE'), 'authenticated clients cannot delete directly');
select ok(has_function_privilege('service_role', 'public.sync_v2_lifecycle_bridge_delete_workspace(uuid,uuid,uuid,uuid,uuid)', 'EXECUTE'), 'trusted bridge may invoke deletion wrapper');
select ok(not has_function_privilege('authenticated', 'sync_v2_private.run_retention_sweep(timestamptz)', 'EXECUTE'), 'retention sweep is not client callable');
select is((select count(*)::integer from cron.job where jobname='sitedatum-sync-v2-retention-v1' and active), 1, 'daily retention job is installed once');

insert into auth.users (
  id, instance_id, aud, role, email, encrypted_password,
  email_confirmed_at, created_at, updated_at
) values (
  'c1000000-0000-4000-8000-000000000001','00000000-0000-0000-0000-000000000000',
  'authenticated','authenticated','lifecycle@example.invalid','',clock_timestamp(),clock_timestamp(),clock_timestamp()
), (
  'c1000000-0000-4000-8000-000000000002','00000000-0000-0000-0000-000000000000',
  'authenticated','authenticated','other-lifecycle@example.invalid','',clock_timestamp(),clock_timestamp(),clock_timestamp()
);
insert into auth.sessions (id, user_id, created_at, updated_at) values
  ('c2000000-0000-4000-8000-000000000001','c1000000-0000-4000-8000-000000000001',clock_timestamp(),clock_timestamp()),
  ('c2000000-0000-4000-8000-000000000002','c1000000-0000-4000-8000-000000000002',clock_timestamp(),clock_timestamp());
insert into sync_v2_private.devices (id, owner_id, proof_algorithm, proof_public_key) values
  ('c3000000-0000-4000-8000-000000000001','c1000000-0000-4000-8000-000000000001','ed25519-v1',decode(repeat('31',32),'hex')),
  ('c3000000-0000-4000-8000-000000000002','c1000000-0000-4000-8000-000000000002','ed25519-v1',decode(repeat('32',32),'hex'));
insert into sync_v2_private.sessions (
  id, owner_id, device_id, issued_at, last_seen_at, idle_expires_at, absolute_expires_at
) values
  ('c2000000-0000-4000-8000-000000000001','c1000000-0000-4000-8000-000000000001','c3000000-0000-4000-8000-000000000001',clock_timestamp(),clock_timestamp(),clock_timestamp()+interval '30 minutes',clock_timestamp()+interval '8 hours'),
  ('c2000000-0000-4000-8000-000000000002','c1000000-0000-4000-8000-000000000002','c3000000-0000-4000-8000-000000000002',clock_timestamp(),clock_timestamp(),clock_timestamp()+interval '30 minutes',clock_timestamp()+interval '8 hours');

select is(public.sync_v2_record_bridge_create_workspace(
  'c1000000-0000-4000-8000-000000000001','c2000000-0000-4000-8000-000000000001',
  'c3000000-0000-4000-8000-000000000001','c4000000-0000-4000-8000-000000000001'
), 'created', 'owner creates workspace for lifecycle tests');
select is((public.sync_v2_lifecycle_bridge_disable_workspace(
  'c1000000-0000-4000-8000-000000000001','c2000000-0000-4000-8000-000000000001',
  'c3000000-0000-4000-8000-000000000001','c4000000-0000-4000-8000-000000000001'
)->>'outcome'), 'disabled', 'active device disables workspace');
select is((select state from sync_v2_private.workspaces where id='c4000000-0000-4000-8000-000000000001'), 'disabled', 'workspace is disabled');
select is((select purge_after-disabled_at from sync_v2_private.workspaces where id='c4000000-0000-4000-8000-000000000001'), interval '30 days', 'disable sets exact thirty-day retention');
select is((public.sync_v2_lifecycle_bridge_disable_workspace(
  'c1000000-0000-4000-8000-000000000001','c2000000-0000-4000-8000-000000000001',
  'c3000000-0000-4000-8000-000000000001','c4000000-0000-4000-8000-000000000001'
)->>'outcome'), 'already_disabled', 'disable retry is idempotent');
select is(public.sync_v2_lifecycle_bridge_restore_workspace(
  'c1000000-0000-4000-8000-000000000002','c2000000-0000-4000-8000-000000000002',
  'c3000000-0000-4000-8000-000000000002','c4000000-0000-4000-8000-000000000001'
), 'not_restored', 'other owner cannot restore workspace');
select is(public.sync_v2_lifecycle_bridge_restore_workspace(
  'c1000000-0000-4000-8000-000000000001','c2000000-0000-4000-8000-000000000001',
  'c3000000-0000-4000-8000-000000000001','c4000000-0000-4000-8000-000000000001'
), 'restored', 'owner restores during recovery window');
select is((select state from sync_v2_private.workspaces where id='c4000000-0000-4000-8000-000000000001'), 'active', 'restored workspace is active');
select is((select purge_after is null from sync_v2_private.workspaces where id='c4000000-0000-4000-8000-000000000001'), true, 'restore clears purge deadline');

select is((public.sync_v2_lifecycle_bridge_delete_workspace(
  'c1000000-0000-4000-8000-000000000002','c2000000-0000-4000-8000-000000000002',
  'c3000000-0000-4000-8000-000000000002','c4000000-0000-4000-8000-000000000001',
  'c5000000-0000-4000-8000-000000000001'
)->>'outcome'), 'not_deleted', 'other owner cannot delete workspace');
create temporary table first_delete as
select public.sync_v2_lifecycle_bridge_delete_workspace(
  'c1000000-0000-4000-8000-000000000001','c2000000-0000-4000-8000-000000000001',
  'c3000000-0000-4000-8000-000000000001','c4000000-0000-4000-8000-000000000001',
  'c5000000-0000-4000-8000-000000000001'
) result;
select is((select result->>'outcome' from first_delete), 'deleted', 'delete-now removes workspace');
select is((select count(*)::integer from sync_v2_private.workspaces where id='c4000000-0000-4000-8000-000000000001'), 0, 'workspace active row is gone');
select is((select count(*)::integer from sync_v2_private.deletion_receipts where workspace_id='c4000000-0000-4000-8000-000000000001'), 1, 'delete-now creates one receipt');
select is((public.sync_v2_lifecycle_bridge_delete_workspace(
  'c1000000-0000-4000-8000-000000000001','c2000000-0000-4000-8000-000000000001',
  'c3000000-0000-4000-8000-000000000001','c4000000-0000-4000-8000-000000000001',
  'c5000000-0000-4000-8000-000000000001'
)->>'outcome'), 'already_deleted', 'exact delete retry returns retained receipt');
select is((public.sync_v2_lifecycle_bridge_delete_workspace(
  'c1000000-0000-4000-8000-000000000001','c2000000-0000-4000-8000-000000000001',
  'c3000000-0000-4000-8000-000000000001','c4000000-0000-4000-8000-000000000099',
  'c5000000-0000-4000-8000-000000000001'
)->>'outcome'), 'request_reused', 'altered delete retry is refused');
select is((select expires_at-completed_at from sync_v2_private.deletion_receipts where request_id='c5000000-0000-4000-8000-000000000001'), interval '90 days', 'receipt has exact ninety-day retention');

select is(public.sync_v2_record_bridge_create_workspace(
  'c1000000-0000-4000-8000-000000000001','c2000000-0000-4000-8000-000000000001',
  'c3000000-0000-4000-8000-000000000001','c4000000-0000-4000-8000-000000000002'
), 'created', 'owner creates second workspace');
select is((public.sync_v2_lifecycle_bridge_delete_account_sync_data(
  'c1000000-0000-4000-8000-000000000001','c2000000-0000-4000-8000-000000000001',
  'c3000000-0000-4000-8000-000000000001','c5000000-0000-4000-8000-000000000002'
)->>'outcome'), 'deleted', 'account deletion removes all Sync data');
select is((select count(*)::integer from sync_v2_private.workspaces where owner_id='c1000000-0000-4000-8000-000000000001'), 0, 'account deletion removes owner workspaces');
select is((select count(*)::integer from sync_v2_private.sessions where owner_id='c1000000-0000-4000-8000-000000000001' and revoked_at is null), 0, 'account deletion revokes Sync sessions');
select is((select count(*)::integer from sync_v2_private.devices where owner_id='c1000000-0000-4000-8000-000000000001' and revoked_at is null), 0, 'account deletion revokes devices');
select is((select count(*)::integer from sync_v2_private.deletion_receipts where owner_id='c1000000-0000-4000-8000-000000000001'), 2, 'account deletion preserves content-free receipts only');
select is((select count(*)::integer from auth.users where id='c1000000-0000-4000-8000-000000000001'), 1, 'Sync-data deletion does not silently delete customer identity');

insert into sync_v2_private.workspaces (
  id, owner_id, state, created_at, updated_at, disabled_at, purge_after
) values (
  'c4000000-0000-4000-8000-000000000003','c1000000-0000-4000-8000-000000000002',
  'disabled','2026-08-01 00:00:00+00','2026-08-01 00:00:00+00',
  '2026-08-01 00:00:00+00','2026-08-31 00:00:00+00'
);
create temporary table sweep_result as
select * from sync_v2_private.run_retention_sweep('2026-10-09 00:00:00+00');
select is((select workspaces_deleted from sweep_result), 1, 'retention sweep purges expired disabled workspace');
select is((select count(*)::integer from sync_v2_private.workspaces where id='c4000000-0000-4000-8000-000000000003'), 0, 'expired disabled workspace is gone');
select is((select deletion_reason from sync_v2_private.deletion_receipts where workspace_id='c4000000-0000-4000-8000-000000000003'), 'retention_expired', 'automatic purge records reason without content');

select * from finish();
rollback;
