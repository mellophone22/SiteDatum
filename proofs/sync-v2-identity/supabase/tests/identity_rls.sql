begin;

create extension if not exists pgtap with schema extensions;
select plan(20);

select has_schema('sync_v2_proof', 'disposable proof schema exists');
select has_table('sync_v2_proof', 'envelopes', 'encrypted-envelope proof table exists');
select ok((select relrowsecurity from pg_class where oid = 'sync_v2_proof.envelopes'::regclass), 'RLS is enabled');
select ok((select relforcerowsecurity from pg_class where oid = 'sync_v2_proof.envelopes'::regclass), 'RLS is forced');
select ok(not has_schema_privilege('anon', 'sync_v2_proof', 'USAGE'), 'anonymous callers cannot use the proof schema');
select ok(not has_schema_privilege('authenticated', 'sync_v2_private', 'USAGE'), 'authenticated callers cannot use the private schema');

insert into sync_v2_private.sessions (id, owner_id) values
  ('30000000-0000-4000-8000-000000000001', '10000000-0000-4000-8000-000000000001'),
  ('30000000-0000-4000-8000-000000000002', '10000000-0000-4000-8000-000000000002');
insert into sync_v2_private.devices (id, owner_id) values
  ('20000000-0000-4000-8000-000000000001', '10000000-0000-4000-8000-000000000001'),
  ('20000000-0000-4000-8000-000000000002', '10000000-0000-4000-8000-000000000002');
insert into sync_v2_proof.envelopes (id, owner_id, ciphertext) values
  ('40000000-0000-4000-8000-000000000001', '10000000-0000-4000-8000-000000000001', decode('a1', 'hex')),
  ('40000000-0000-4000-8000-000000000002', '10000000-0000-4000-8000-000000000002', decode('b2', 'hex'));

set local role authenticated;
set local request.jwt.claims = '{"sub":"10000000-0000-4000-8000-000000000001","role":"authenticated","session_id":"30000000-0000-4000-8000-000000000001","sync_device_id":"20000000-0000-4000-8000-000000000001"}';

select is((select count(*)::integer from sync_v2_proof.envelopes), 1, 'owner sees only one owned envelope');
select is((select owner_id from sync_v2_proof.envelopes limit 1), '10000000-0000-4000-8000-000000000001'::uuid, 'visible envelope belongs to authenticated subject');
select lives_ok($$insert into sync_v2_proof.envelopes values ('40000000-0000-4000-8000-000000000003', '10000000-0000-4000-8000-000000000001', decode('c3','hex'))$$, 'owner can insert an owned envelope');
select throws_ok($$insert into sync_v2_proof.envelopes values ('40000000-0000-4000-8000-000000000004', '10000000-0000-4000-8000-000000000002', decode('d4','hex'))$$, '42501', null, 'owner cannot insert another owner envelope');
select results_eq(
  $$with changed as (
      update sync_v2_proof.envelopes
      set ciphertext = decode('ee','hex')
      where id = '40000000-0000-4000-8000-000000000002'
      returning 1
    ) select count(*)::integer from changed$$,
  array[0],
  'owner cannot update another owner envelope'
);
select throws_ok($$update sync_v2_proof.envelopes set owner_id = '10000000-0000-4000-8000-000000000002' where id = '40000000-0000-4000-8000-000000000001'$$, '42501', null, 'owner cannot transfer row ownership');
select results_eq(
  $$with deleted as (
      delete from sync_v2_proof.envelopes
      where id = '40000000-0000-4000-8000-000000000002'
      returning 1
    ) select count(*)::integer from deleted$$,
  array[0],
  'owner cannot delete another owner envelope'
);

reset role;
update sync_v2_private.sessions set revoked_at = clock_timestamp() where id = '30000000-0000-4000-8000-000000000001';
set local role authenticated;
select is((select count(*)::integer from sync_v2_proof.envelopes), 0, 'revoked session loses access on the next statement');
select throws_ok($$insert into sync_v2_proof.envelopes values ('40000000-0000-4000-8000-000000000005', '10000000-0000-4000-8000-000000000001', decode('f5','hex'))$$, '42501', null, 'revoked session cannot insert');

reset role;
update sync_v2_private.sessions set revoked_at = null where id = '30000000-0000-4000-8000-000000000001';
update sync_v2_private.devices set revoked_at = clock_timestamp() where id = '20000000-0000-4000-8000-000000000001';
set local role authenticated;
select is((select count(*)::integer from sync_v2_proof.envelopes), 0, 'revoked device loses access on the next statement');

reset role;
delete from sync_v2_private.sessions where owner_id = '10000000-0000-4000-8000-000000000001';
delete from sync_v2_private.devices where owner_id = '10000000-0000-4000-8000-000000000001';
set local role authenticated;
select is((select count(*)::integer from sync_v2_proof.envelopes), 0, 'account deletion removes authorization state');

reset role;
set local role anon;
select throws_ok($$select * from sync_v2_proof.envelopes$$, '42501', null, 'anonymous callers cannot read envelopes');
select throws_ok($$insert into sync_v2_proof.envelopes values ('40000000-0000-4000-8000-000000000006', '10000000-0000-4000-8000-000000000001', decode('a6','hex'))$$, '42501', null, 'anonymous callers cannot insert envelopes');

reset role;
select is((select count(*)::integer from sync_v2_proof.envelopes where owner_id = '10000000-0000-4000-8000-000000000002'), 1, 'negative tests never changed the other owner row');

select * from finish();
rollback;
