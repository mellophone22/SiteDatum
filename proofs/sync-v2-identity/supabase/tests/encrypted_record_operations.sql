begin;

create extension if not exists pgtap with schema extensions;
select no_plan();

select has_table('sync_v2_private', 'workspaces', 'workspace routing table is private');
select has_table('sync_v2_private', 'record_envelopes', 'current encrypted envelopes are private');
select has_table('sync_v2_private', 'record_changes', 'encrypted change log is private');
select has_table('sync_v2_private', 'workspace_checkpoints', 'encrypted checkpoints are private');
select has_table('sync_v2_private', 'applied_record_batches', 'idempotency ledger is private');
select is((select relforcerowsecurity from pg_class where oid = 'sync_v2_private.record_envelopes'::regclass), true, 'record envelopes force RLS');
select is((select relforcerowsecurity from pg_class where oid = 'sync_v2_private.record_changes'::regclass), true, 'record changes force RLS');
select is((select relforcerowsecurity from pg_class where oid = 'sync_v2_private.workspace_checkpoints'::regclass), true, 'checkpoints force RLS');
select ok(not has_table_privilege('authenticated', 'sync_v2_private.record_envelopes', 'SELECT'), 'authenticated clients cannot read envelopes directly');
select ok(not has_table_privilege('service_role', 'sync_v2_private.record_envelopes', 'SELECT'), 'service role cannot read envelopes directly');
select ok(not has_function_privilege('authenticated', 'public.sync_v2_record_bridge_push_batch(uuid,uuid,uuid,uuid,uuid,jsonb,bigint,smallint,integer,text)', 'EXECUTE'), 'authenticated clients cannot push directly');
select ok(has_function_privilege('service_role', 'public.sync_v2_record_bridge_push_batch(uuid,uuid,uuid,uuid,uuid,jsonb,bigint,smallint,integer,text)', 'EXECUTE'), 'service bridge may push through the wrapper');

insert into auth.users (
  id, instance_id, aud, role, email, encrypted_password,
  email_confirmed_at, created_at, updated_at
) values (
  'b1000000-0000-4000-8000-000000000001',
  '00000000-0000-0000-0000-000000000000',
  'authenticated', 'authenticated', 'records@example.invalid', '',
  clock_timestamp(), clock_timestamp(), clock_timestamp()
), (
  'b1000000-0000-4000-8000-000000000002',
  '00000000-0000-0000-0000-000000000000',
  'authenticated', 'authenticated', 'other-records@example.invalid', '',
  clock_timestamp(), clock_timestamp(), clock_timestamp()
);
insert into auth.sessions (id, user_id, created_at, updated_at) values
  ('b2000000-0000-4000-8000-000000000001','b1000000-0000-4000-8000-000000000001',clock_timestamp(),clock_timestamp()),
  ('b2000000-0000-4000-8000-000000000002','b1000000-0000-4000-8000-000000000002',clock_timestamp(),clock_timestamp());
insert into sync_v2_private.devices (id, owner_id, proof_algorithm, proof_public_key) values
  ('b3000000-0000-4000-8000-000000000001','b1000000-0000-4000-8000-000000000001','ed25519-v1',decode(repeat('11',32),'hex')),
  ('b3000000-0000-4000-8000-000000000002','b1000000-0000-4000-8000-000000000002','ed25519-v1',decode(repeat('22',32),'hex'));
insert into sync_v2_private.sessions (
  id, owner_id, device_id, issued_at, last_seen_at, idle_expires_at, absolute_expires_at
) values
  ('b2000000-0000-4000-8000-000000000001','b1000000-0000-4000-8000-000000000001','b3000000-0000-4000-8000-000000000001',clock_timestamp(),clock_timestamp(),clock_timestamp()+interval '30 minutes',clock_timestamp()+interval '8 hours'),
  ('b2000000-0000-4000-8000-000000000002','b1000000-0000-4000-8000-000000000002','b3000000-0000-4000-8000-000000000002',clock_timestamp(),clock_timestamp(),clock_timestamp()+interval '30 minutes',clock_timestamp()+interval '8 hours');

select is(public.sync_v2_record_bridge_authorize('b1000000-0000-4000-8000-000000000001','b2000000-0000-4000-8000-000000000001','push_batch'), 'authorized', 'live session authorizes a push');
select is(public.sync_v2_record_bridge_authorize('b1000000-0000-4000-8000-000000000001','b2000000-0000-4000-8000-000000000099','push_batch'), 'session_invalid', 'unknown session is refused');
select is(public.sync_v2_record_bridge_create_workspace('b1000000-0000-4000-8000-000000000001','b2000000-0000-4000-8000-000000000001','b3000000-0000-4000-8000-000000000001','b4000000-0000-4000-8000-000000000001'), 'created', 'active device creates an opaque workspace');
select is(public.sync_v2_record_bridge_create_workspace('b1000000-0000-4000-8000-000000000001','b2000000-0000-4000-8000-000000000001','b3000000-0000-4000-8000-000000000001','b4000000-0000-4000-8000-000000000001'), 'already_created', 'workspace creation retry is idempotent');
select is(public.sync_v2_record_bridge_create_workspace('b1000000-0000-4000-8000-000000000002','b2000000-0000-4000-8000-000000000002','b3000000-0000-4000-8000-000000000002','b4000000-0000-4000-8000-000000000001'), 'not_created', 'another owner cannot claim the identifier');
select is(octet_length(decode(public.sync_v2_record_bridge_device_key('b1000000-0000-4000-8000-000000000001','b2000000-0000-4000-8000-000000000001','b3000000-0000-4000-8000-000000000001'),'base64')), 32, 'bridge returns only the active public proof key');

create temporary table first_batch as
select public.sync_v2_record_bridge_push_batch(
  'b1000000-0000-4000-8000-000000000001','b2000000-0000-4000-8000-000000000001','b3000000-0000-4000-8000-000000000001',
  'b4000000-0000-4000-8000-000000000001','b5000000-0000-4000-8000-000000000001',
  jsonb_build_array(
    jsonb_build_object('recordId','b6000000-0000-4000-8000-000000000001','recordKind',1,'expectedServerVersion',0,'mutationId','b7000000-0000-4000-8000-000000000001','protocolVersion',1,'workspaceKeyVersion',1,'ciphertext',encode(decode(repeat('aa',24),'hex'),'base64'),'isTombstone',false),
    jsonb_build_object('recordId','b6000000-0000-4000-8000-000000000002','recordKind',2,'expectedServerVersion',0,'mutationId','b7000000-0000-4000-8000-000000000002','protocolVersion',1,'workspaceKeyVersion',1,'ciphertext',encode(decode(repeat('bb',24),'hex'),'base64'),'isTombstone',false)
  ),1,1::smallint,1,encode(decode(repeat('cc',32),'hex'),'base64')
) result;

select is((select result->>'outcome' from first_batch), 'applied', 'two-record batch is applied');
select is((select count(*)::integer from sync_v2_private.record_envelopes), 2, 'two current envelopes commit together');
select is((select count(*)::integer from sync_v2_private.record_changes), 2, 'two immutable changes commit together');
select is((select count(*)::integer from sync_v2_private.workspace_checkpoints), 1, 'one checkpoint commits with the batch');
select is((select current_checkpoint_counter from sync_v2_private.workspaces where id='b4000000-0000-4000-8000-000000000001'), 1::bigint, 'workspace checkpoint advances after commit');
select is((select count(*)::integer from sync_v2_private.record_envelopes where ciphertext in (decode(repeat('aa',24),'hex'),decode(repeat('bb',24),'hex'))), 2, 'opaque ciphertext is preserved exactly');
select is((public.sync_v2_record_bridge_push_batch(
  'b1000000-0000-4000-8000-000000000001','b2000000-0000-4000-8000-000000000001','b3000000-0000-4000-8000-000000000001','b4000000-0000-4000-8000-000000000001','b5000000-0000-4000-8000-000000000001',
  jsonb_build_array(jsonb_build_object('recordId','b6000000-0000-4000-8000-000000000001','recordKind',1,'expectedServerVersion',0,'mutationId','b7000000-0000-4000-8000-000000000001','protocolVersion',1,'workspaceKeyVersion',1,'ciphertext',encode(decode(repeat('aa',24),'hex'),'base64'),'isTombstone',false),jsonb_build_object('recordId','b6000000-0000-4000-8000-000000000002','recordKind',2,'expectedServerVersion',0,'mutationId','b7000000-0000-4000-8000-000000000002','protocolVersion',1,'workspaceKeyVersion',1,'ciphertext',encode(decode(repeat('bb',24),'hex'),'base64'),'isTombstone',false)),1,1::smallint,1,encode(decode(repeat('cc',32),'hex'),'base64')) ->> 'outcome'), 'already_applied', 'exact batch retry is idempotent');

select is((public.sync_v2_record_bridge_push_batch(
  'b1000000-0000-4000-8000-000000000001','b2000000-0000-4000-8000-000000000001','b3000000-0000-4000-8000-000000000001','b4000000-0000-4000-8000-000000000001','b5000000-0000-4000-8000-000000000002',
  jsonb_build_array(jsonb_build_object('recordId','b6000000-0000-4000-8000-000000000001','recordKind',1,'expectedServerVersion',0,'mutationId','b7000000-0000-4000-8000-000000000003','protocolVersion',1,'workspaceKeyVersion',1,'ciphertext',encode(decode(repeat('dd',24),'hex'),'base64'),'isTombstone',false)),2,1::smallint,1,encode(decode(repeat('ee',32),'hex'),'base64')) ->> 'outcome'), 'record_conflict', 'stale record version is refused');
select is((select count(*)::integer from sync_v2_private.workspace_checkpoints), 1, 'refused batch does not add a checkpoint');
select is((select current_checkpoint_counter from sync_v2_private.workspaces where id='b4000000-0000-4000-8000-000000000001'), 1::bigint, 'refused batch does not advance workspace state');

select is((public.sync_v2_record_bridge_push_batch(
  'b1000000-0000-4000-8000-000000000001','b2000000-0000-4000-8000-000000000001','b3000000-0000-4000-8000-000000000001','b4000000-0000-4000-8000-000000000001','b5000000-0000-4000-8000-000000000003',
  jsonb_build_array(jsonb_build_object('recordId','b6000000-0000-4000-8000-000000000001','recordKind',1,'expectedServerVersion',1,'mutationId','b7000000-0000-4000-8000-000000000001','protocolVersion',1,'workspaceKeyVersion',1,'ciphertext',encode(decode(repeat('ff',24),'hex'),'base64'),'isTombstone',true)),2,1::smallint,1,encode(decode(repeat('11',32),'hex'),'base64')) ->> 'outcome'), 'mutation_conflict', 'reused mutation identifier is refused');

select is((public.sync_v2_record_bridge_push_batch(
  'b1000000-0000-4000-8000-000000000001','b2000000-0000-4000-8000-000000000001','b3000000-0000-4000-8000-000000000001','b4000000-0000-4000-8000-000000000001','b5000000-0000-4000-8000-000000000004',
  jsonb_build_array(jsonb_build_object('recordId','b6000000-0000-4000-8000-000000000001','recordKind',1,'expectedServerVersion',1,'mutationId','b7000000-0000-4000-8000-000000000004','protocolVersion',1,'workspaceKeyVersion',1,'ciphertext',encode(decode(repeat('12',24),'hex'),'base64'),'isTombstone',true)),2,1::smallint,1,encode(decode(repeat('13',32),'hex'),'base64')) ->> 'outcome'), 'applied', 'fresh mutation can tombstone a record');
select is((select server_version from sync_v2_private.record_envelopes where record_id='b6000000-0000-4000-8000-000000000001'), 2::bigint, 'tombstone advances the server version');
select is((select is_tombstone from sync_v2_private.record_envelopes where record_id='b6000000-0000-4000-8000-000000000001'), true, 'current envelope marks the tombstone');
select ok((select deleted_at is not null from sync_v2_private.record_envelopes where record_id='b6000000-0000-4000-8000-000000000001'), 'tombstone records its server deletion timestamp');
select is((select current_checkpoint_counter from sync_v2_private.workspaces where id='b4000000-0000-4000-8000-000000000001'), 2::bigint, 'successful tombstone advances the checkpoint');
select is((public.sync_v2_record_bridge_pull_changes('b1000000-0000-4000-8000-000000000001','b2000000-0000-4000-8000-000000000001','b3000000-0000-4000-8000-000000000001','b4000000-0000-4000-8000-000000000001',0,100::smallint)->'checkpoint'->>'ciphertext'), encode(decode(repeat('13',32),'hex'),'base64'), 'full pull returns the exact latest opaque checkpoint');

create temporary table pull_result as
select public.sync_v2_record_bridge_pull_changes('b1000000-0000-4000-8000-000000000001','b2000000-0000-4000-8000-000000000001','b3000000-0000-4000-8000-000000000001','b4000000-0000-4000-8000-000000000001',0,1::smallint) result;
select is((select result->>'outcome' from pull_result), 'available', 'owner can pull encrypted changes');
select is((select jsonb_array_length(result->'changes') from pull_result), 1, 'pull obeys the requested page size');
select is((select (result->>'hasMore')::boolean from pull_result), true, 'pull reports a following page');
select is((select result->'checkpoint' from pull_result), 'null'::jsonb, 'checkpoint after the page cursor is not exposed early');
select is((public.sync_v2_record_bridge_pull_changes('b1000000-0000-4000-8000-000000000002','b2000000-0000-4000-8000-000000000002','b3000000-0000-4000-8000-000000000002','b4000000-0000-4000-8000-000000000001',0,100::smallint)->>'outcome'), 'not_available', 'another owner cannot pull the workspace');
select ok(pg_get_functiondef('public.sync_v2_record_bridge_push_batch(uuid,uuid,uuid,uuid,uuid,jsonb,bigint,smallint,integer,text)'::regprocedure) like '%SET search_path TO %', 'push wrapper has a fixed empty search path');

-- Fictional boundary drill; every attempted storage update rolls back locally.
create function pg_temp.check_counter_boundary(t text, c text, n bigint)
returns void language plpgsql as $$
begin
  execute format('update sync_v2_private.%I set %I=$1 where ctid=(select ctid from sync_v2_private.%I limit 1)',t,c,t) using n;
  raise exception using errcode='P0002', message='rollback fictional boundary update';
exception when no_data_found then null;
end;
$$;
-- Permit explicit identity updates only inside this rolled-back fixture.
alter table sync_v2_private.record_changes alter column change_seq set generated by default;
select throws_ok($q$select pg_temp.check_counter_boundary('workspaces','current_checkpoint_counter',9007199254740992)$q$,'23514',NULL,'workspaces.current_checkpoint_counter rejects MAX+1');
select lives_ok($q$select pg_temp.check_counter_boundary('workspaces','current_checkpoint_counter',9007199254740991)$q$,'workspaces.current_checkpoint_counter accepts MAX');
select throws_ok($q$select pg_temp.check_counter_boundary('workspaces','last_change_seq',9007199254740992)$q$,'23514',NULL,'workspaces.last_change_seq rejects MAX+1');
select lives_ok($q$select pg_temp.check_counter_boundary('workspaces','last_change_seq',9007199254740991)$q$,'workspaces.last_change_seq accepts MAX');
select throws_ok($q$select pg_temp.check_counter_boundary('record_envelopes','server_version',9007199254740992)$q$,'23514',NULL,'record_envelopes.server_version rejects MAX+1');
select lives_ok($q$select pg_temp.check_counter_boundary('record_envelopes','server_version',9007199254740991)$q$,'record_envelopes.server_version accepts MAX');
select throws_ok($q$select pg_temp.check_counter_boundary('record_changes','change_seq',9007199254740992)$q$,'23514',NULL,'record_changes.change_seq rejects MAX+1');
select lives_ok($q$select pg_temp.check_counter_boundary('record_changes','change_seq',9007199254740991)$q$,'record_changes.change_seq accepts MAX');
select throws_ok($q$select pg_temp.check_counter_boundary('record_changes','server_version',9007199254740992)$q$,'23514',NULL,'record_changes.server_version rejects MAX+1');
select lives_ok($q$select pg_temp.check_counter_boundary('record_changes','server_version',9007199254740991)$q$,'record_changes.server_version accepts MAX');
select throws_ok($q$select pg_temp.check_counter_boundary('workspace_checkpoints','checkpoint_counter',9007199254740992)$q$,'23514',NULL,'workspace_checkpoints.checkpoint_counter rejects MAX+1');
select lives_ok($q$select pg_temp.check_counter_boundary('workspace_checkpoints','checkpoint_counter',9007199254740991)$q$,'workspace_checkpoints.checkpoint_counter accepts MAX');
select throws_ok($q$select pg_temp.check_counter_boundary('workspace_checkpoints','through_change_seq',9007199254740992)$q$,'23514',NULL,'workspace_checkpoints.through_change_seq rejects MAX+1');
select lives_ok($q$select pg_temp.check_counter_boundary('workspace_checkpoints','through_change_seq',9007199254740991)$q$,'workspace_checkpoints.through_change_seq accepts MAX');
select throws_ok($q$select pg_temp.check_counter_boundary('applied_record_batches','checkpoint_counter',9007199254740992)$q$,'23514',NULL,'applied_record_batches.checkpoint_counter rejects MAX+1');
select lives_ok($q$select pg_temp.check_counter_boundary('applied_record_batches','checkpoint_counter',9007199254740991)$q$,'applied_record_batches.checkpoint_counter accepts MAX');
select throws_ok($q$select pg_temp.check_counter_boundary('applied_record_batches','through_change_seq',9007199254740992)$q$,'23514',NULL,'applied_record_batches.through_change_seq rejects MAX+1');
select lives_ok($q$select pg_temp.check_counter_boundary('applied_record_batches','through_change_seq',9007199254740991)$q$,'applied_record_batches.through_change_seq accepts MAX');
alter table sync_v2_private.record_changes alter column change_seq set generated always;
select is((public.sync_v2_record_bridge_pull_changes('b1000000-0000-4000-8000-000000000001','b2000000-0000-4000-8000-000000000001','b3000000-0000-4000-8000-000000000001','b4000000-0000-4000-8000-000000000001',9007199254740992,100::smallint)->>'outcome'), 'not_available', 'oversized direct RPC cursor is not echoed');
select is((public.sync_v2_record_bridge_pull_changes('b1000000-0000-4000-8000-000000000001','b2000000-0000-4000-8000-000000000001','b3000000-0000-4000-8000-000000000001','b4000000-0000-4000-8000-000000000001',9007199254740991,100::smallint)->>'throughCursor'), '9007199254740991', 'MAX cursor remains exact in JSON');
select is((public.sync_v2_record_bridge_push_batch('b1000000-0000-4000-8000-000000000001','b2000000-0000-4000-8000-000000000001','b3000000-0000-4000-8000-000000000001','b4000000-0000-4000-8000-000000000001','b5000000-0000-4000-8000-000000000090', jsonb_build_array(jsonb_build_object('recordId','b6000000-0000-4000-8000-000000000090','recordKind',1,'expectedServerVersion',0,'mutationId','b7000000-0000-4000-8000-000000000090','protocolVersion',1,'workspaceKeyVersion',1,'ciphertext',encode(decode(repeat('aa',24),'hex'),'base64'),'isTombstone',false)),9007199254740992,1::smallint,1,encode(decode(repeat('cc',32),'hex'),'base64'))->>'outcome'), 'invalid_request', 'oversized checkpoint rejected');
select is((select seqmax from pg_sequence where seqrelid='sync_v2_private.record_changes_change_seq_seq'::regclass),9007199254740991::bigint,'sequence has safe maximum');
select is((select seqcycle from pg_sequence where seqrelid='sync_v2_private.record_changes_change_seq_seq'::regclass),false,'sequence cannot cycle identifiers');
alter sequence sync_v2_private.record_changes_change_seq_seq restart with 9007199254740991;
update sync_v2_private.workspaces set current_checkpoint_counter=2;
select is((public.sync_v2_record_bridge_push_batch('b1000000-0000-4000-8000-000000000001','b2000000-0000-4000-8000-000000000001','b3000000-0000-4000-8000-000000000001','b4000000-0000-4000-8000-000000000001','b5000000-0000-4000-8000-000000000100', jsonb_build_array(jsonb_build_object('recordId','b6000000-0000-4000-8000-000000000090','recordKind',1,'expectedServerVersion',9007199254740991,'mutationId','b7000000-0000-4000-8000-000000000100','protocolVersion',1,'workspaceKeyVersion',1,'ciphertext',encode(decode(repeat('aa',24),'hex'),'base64'),'isTombstone',false)),3,1::smallint,1,encode(decode(repeat('cc',32),'hex'),'base64'))->>'outcome'),'invalid_request','exhausted expected version refuses with an available checkpoint');
select is((public.sync_v2_record_bridge_push_batch('b1000000-0000-4000-8000-000000000001','b2000000-0000-4000-8000-000000000001','b3000000-0000-4000-8000-000000000001','b4000000-0000-4000-8000-000000000001','b5000000-0000-4000-8000-000000000090', jsonb_build_array(jsonb_build_object('recordId','b6000000-0000-4000-8000-000000000090','recordKind',1,'expectedServerVersion',0,'mutationId','b7000000-0000-4000-8000-000000000090','protocolVersion',1,'workspaceKeyVersion',1,'ciphertext',encode(decode(repeat('aa',24),'hex'),'base64'),'isTombstone',false),jsonb_build_object('recordId','b6000000-0000-4000-8000-000000000091','recordKind',1,'expectedServerVersion',0,'mutationId','b7000000-0000-4000-8000-000000000091','protocolVersion',1,'workspaceKeyVersion',1,'ciphertext',encode(decode(repeat('aa',24),'hex'),'base64'),'isTombstone',false)),3,1::smallint,1,encode(decode(repeat('cc',32),'hex'),'base64'))->>'outcome'),'invalid_request','mid-batch sequence exhaustion refuses atomically');
select is((select count(*)::integer from sync_v2_private.record_envelopes where record_id in ('b6000000-0000-4000-8000-000000000090','b6000000-0000-4000-8000-000000000091')),0,'no partial envelope survives exhaustion');
select is((select current_checkpoint_counter from sync_v2_private.workspaces where id='b4000000-0000-4000-8000-000000000001'),2::bigint,'failed exhausted batch leaves checkpoint unchanged');
insert into sync_v2_private.record_envelopes (owner_id,workspace_id,record_id,record_kind,server_version,client_mutation_id,protocol_version,workspace_key_version,ciphertext,is_tombstone,created_at,updated_at)
select owner_id,workspace_id,'b6000000-0000-4000-8000-000000000090',1,9007199254740990,'b7000000-0000-4000-8000-000000000098',1,1,ciphertext,false,created_at,updated_at from sync_v2_private.record_envelopes limit 1;
alter sequence sync_v2_private.record_changes_change_seq_seq restart with 9007199254740991;
update sync_v2_private.workspaces set current_checkpoint_counter=9007199254740990;
select is((public.sync_v2_record_bridge_push_batch('b1000000-0000-4000-8000-000000000001','b2000000-0000-4000-8000-000000000001','b3000000-0000-4000-8000-000000000001','b4000000-0000-4000-8000-000000000001','b5000000-0000-4000-8000-000000000099', jsonb_build_array(jsonb_build_object('recordId','b6000000-0000-4000-8000-000000000090','recordKind',1,'expectedServerVersion',9007199254740990,'mutationId','b7000000-0000-4000-8000-000000000099','protocolVersion',1,'workspaceKeyVersion',1,'ciphertext',encode(decode(repeat('aa',24),'hex'),'base64'),'isTombstone',false)),9007199254740991,1::smallint,1,encode(decode(repeat('cc',32),'hex'),'base64'))->>'outcome'),'applied','final sequence and checkpoint values commit');
select is((public.sync_v2_record_bridge_push_batch('b1000000-0000-4000-8000-000000000001','b2000000-0000-4000-8000-000000000001','b3000000-0000-4000-8000-000000000001','b4000000-0000-4000-8000-000000000001','b5000000-0000-4000-8000-000000000099', jsonb_build_array(jsonb_build_object('recordId','b6000000-0000-4000-8000-000000000090','recordKind',1,'expectedServerVersion',9007199254740990,'mutationId','b7000000-0000-4000-8000-000000000099','protocolVersion',1,'workspaceKeyVersion',1,'ciphertext',encode(decode(repeat('aa',24),'hex'),'base64'),'isTombstone',false)),9007199254740991,1::smallint,1,encode(decode(repeat('cc',32),'hex'),'base64'))->>'outcome'),'already_applied','committed retry survives exhausted counters');
select is((public.sync_v2_record_bridge_pull_changes('b1000000-0000-4000-8000-000000000001','b2000000-0000-4000-8000-000000000001','b3000000-0000-4000-8000-000000000001','b4000000-0000-4000-8000-000000000001',0,100::smallint)->>'throughCursor'),'9007199254740991','final stored cursor JSON remains exact');
select is((public.sync_v2_record_bridge_push_batch('b1000000-0000-4000-8000-000000000001','b2000000-0000-4000-8000-000000000001','b3000000-0000-4000-8000-000000000001','b4000000-0000-4000-8000-000000000001','b5000000-0000-4000-8000-000000000100', jsonb_build_array(jsonb_build_object('recordId','b6000000-0000-4000-8000-000000000090','recordKind',1,'expectedServerVersion',9007199254740991,'mutationId','b7000000-0000-4000-8000-000000000100','protocolVersion',1,'workspaceKeyVersion',1,'ciphertext',encode(decode(repeat('aa',24),'hex'),'base64'),'isTombstone',false)),9007199254740991,1::smallint,1,encode(decode(repeat('cc',32),'hex'),'base64'))->>'outcome'),'checkpoint_conflict','new work at checkpoint capacity refuses without wrap');

select is((select server_version from sync_v2_private.record_envelopes where record_id='b6000000-0000-4000-8000-000000000090'),9007199254740991::bigint,'last valid record increment reaches MAX');
select * from finish();
rollback;
