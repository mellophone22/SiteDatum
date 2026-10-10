-- C10-03H: disabled-workspace retention and authenticated hosted deletion.
--
-- This migration intentionally affects only the isolated Sync v2 proof. It
-- never reaches the desktop SQLite database or Windows project files.

alter table sync_v2_private.workspaces
  add column purge_after timestamptz;

alter table sync_v2_private.workspaces
  drop constraint workspaces_check,
  add constraint sync_v2_workspaces_state_times check (
    (state = 'active' and disabled_at is null and purge_after is null)
    or (
      state = 'disabled'
      and disabled_at is not null
      and purge_after = disabled_at + interval '30 days'
    )
  );

create index sync_v2_workspaces_disabled_purge_idx
  on sync_v2_private.workspaces (purge_after)
  where state = 'disabled';

create table sync_v2_private.deletion_receipts (
  owner_id uuid not null,
  request_id uuid not null,
  receipt_id uuid not null unique,
  deletion_scope text not null check (deletion_scope in ('workspace', 'account_sync_data')),
  workspace_id uuid,
  policy_version text not null check (policy_version = 'sync-v2-retention-v1'),
  deletion_reason text not null check (deletion_reason in ('delete_now', 'retention_expired', 'account_deletion')),
  completed_at timestamptz not null,
  expires_at timestamptz not null,
  primary key (owner_id, request_id),
  check (
    (deletion_scope = 'workspace' and workspace_id is not null)
    or (deletion_scope = 'account_sync_data' and workspace_id is null)
  ),
  check (expires_at = completed_at + interval '90 days')
);

create index sync_v2_deletion_receipts_expiry_idx
  on sync_v2_private.deletion_receipts (expires_at);

create table sync_v2_private.lifecycle_bridge_windows (
  owner_id uuid not null,
  auth_session_id uuid not null,
  action text not null check (
    action in ('disable_workspace', 'restore_workspace', 'delete_workspace', 'delete_account_sync_data')
  ),
  window_started_at timestamptz not null,
  request_count smallint not null check (request_count between 1 and 10),
  last_request_at timestamptz not null,
  primary key (owner_id, auth_session_id, action, window_started_at)
);

alter table sync_v2_private.deletion_receipts enable row level security;
alter table sync_v2_private.deletion_receipts force row level security;
alter table sync_v2_private.lifecycle_bridge_windows enable row level security;
alter table sync_v2_private.lifecycle_bridge_windows force row level security;

revoke all on table sync_v2_private.deletion_receipts,
  sync_v2_private.lifecycle_bridge_windows
  from public, anon, authenticated, service_role;

create function sync_v2_private.consume_lifecycle_bridge_limit(
  p_owner_id uuid,
  p_auth_session_id uuid,
  p_action text
)
returns boolean
language plpgsql
volatile
security definer
set search_path = ''
as $$
declare
  now_at timestamptz := pg_catalog.clock_timestamp();
  window_at timestamptz := pg_catalog.date_bin(
    interval '5 minutes', now_at, '2000-01-01 00:00:00+00'::timestamptz
  );
  accepted_count smallint;
begin
  if p_owner_id is null
     or p_auth_session_id is null
     or p_action not in (
       'disable_workspace', 'restore_workspace', 'delete_workspace', 'delete_account_sync_data'
     )
     or not sync_v2_private.auth_session_is_live(p_owner_id, p_auth_session_id) then
    return false;
  end if;

  perform pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended(
      p_owner_id::text || ':' || p_auth_session_id::text || ':' || p_action, 0
    )
  );

  delete from sync_v2_private.lifecycle_bridge_windows w
   where w.owner_id = p_owner_id
     and w.window_started_at < now_at - interval '24 hours';

  insert into sync_v2_private.lifecycle_bridge_windows (
    owner_id, auth_session_id, action, window_started_at, request_count, last_request_at
  ) values (
    p_owner_id, p_auth_session_id, p_action, window_at, 1, now_at
  )
  on conflict (owner_id, auth_session_id, action, window_started_at)
  do update
     set request_count = sync_v2_private.lifecycle_bridge_windows.request_count + 1,
         last_request_at = excluded.last_request_at
   where sync_v2_private.lifecycle_bridge_windows.request_count < 10
  returning request_count into accepted_count;

  return accepted_count is not null;
end;
$$;

create function public.sync_v2_lifecycle_bridge_authorize(
  p_owner_id uuid,
  p_auth_session_id uuid,
  p_action text
)
returns text
language plpgsql
volatile
security definer
set search_path = ''
as $$
begin
  if not sync_v2_private.auth_session_is_live(p_owner_id, p_auth_session_id) then
    return 'session_invalid';
  end if;
  if p_action not in (
    'disable_workspace', 'restore_workspace', 'delete_workspace', 'delete_account_sync_data'
  ) then
    return 'request_invalid';
  end if;
  if not sync_v2_private.consume_lifecycle_bridge_limit(
    p_owner_id, p_auth_session_id, p_action
  ) then
    return 'rate_limited';
  end if;
  return 'authorized';
end;
$$;

create function public.sync_v2_lifecycle_bridge_disable_workspace(
  p_owner_id uuid,
  p_auth_session_id uuid,
  p_device_id uuid,
  p_workspace_id uuid
)
returns jsonb
language plpgsql
volatile
security definer
set search_path = ''
as $$
declare
  now_at timestamptz := pg_catalog.clock_timestamp();
  workspace sync_v2_private.workspaces%rowtype;
begin
  if p_workspace_id is null
     or not sync_v2_private.device_session_is_active(
       p_owner_id, p_auth_session_id, p_device_id
     ) then
    return pg_catalog.jsonb_build_object('outcome', 'not_disabled');
  end if;

  perform pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended(p_owner_id::text || ':' || p_workspace_id::text, 0)
  );
  select w.* into workspace
    from sync_v2_private.workspaces w
   where w.id = p_workspace_id and w.owner_id = p_owner_id
   for update;
  if not found then
    return pg_catalog.jsonb_build_object('outcome', 'not_disabled');
  end if;
  if workspace.state = 'disabled' then
    return pg_catalog.jsonb_build_object(
      'outcome', 'already_disabled', 'purgeAfter', workspace.purge_after
    );
  end if;

  update sync_v2_private.workspaces
     set state = 'disabled',
         disabled_at = now_at,
         purge_after = now_at + interval '30 days',
         updated_at = now_at
   where id = p_workspace_id and owner_id = p_owner_id;
  return pg_catalog.jsonb_build_object(
    'outcome', 'disabled', 'purgeAfter', now_at + interval '30 days'
  );
end;
$$;

create function public.sync_v2_lifecycle_bridge_restore_workspace(
  p_owner_id uuid,
  p_auth_session_id uuid,
  p_device_id uuid,
  p_workspace_id uuid
)
returns text
language plpgsql
volatile
security definer
set search_path = ''
as $$
declare
  now_at timestamptz := pg_catalog.clock_timestamp();
  workspace sync_v2_private.workspaces%rowtype;
begin
  if p_workspace_id is null
     or not sync_v2_private.device_session_is_active(
       p_owner_id, p_auth_session_id, p_device_id
     ) then
    return 'not_restored';
  end if;
  perform pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended(p_owner_id::text || ':' || p_workspace_id::text, 0)
  );
  select w.* into workspace
    from sync_v2_private.workspaces w
   where w.id = p_workspace_id and w.owner_id = p_owner_id
   for update;
  if not found then return 'not_restored'; end if;
  if workspace.state = 'active' then return 'already_active'; end if;
  if workspace.purge_after <= now_at then return 'recovery_expired'; end if;

  update sync_v2_private.workspaces
     set state = 'active', disabled_at = null, purge_after = null, updated_at = now_at
   where id = p_workspace_id and owner_id = p_owner_id;
  return 'restored';
end;
$$;

create function public.sync_v2_lifecycle_bridge_delete_workspace(
  p_owner_id uuid,
  p_auth_session_id uuid,
  p_device_id uuid,
  p_workspace_id uuid,
  p_request_id uuid
)
returns jsonb
language plpgsql
volatile
security definer
set search_path = ''
as $$
declare
  now_at timestamptz := pg_catalog.clock_timestamp();
  receipt sync_v2_private.deletion_receipts%rowtype;
begin
  if p_workspace_id is null or p_request_id is null
     or not sync_v2_private.auth_session_is_live(p_owner_id, p_auth_session_id) then
    return pg_catalog.jsonb_build_object('outcome', 'not_deleted');
  end if;
  perform pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended(p_owner_id::text || ':' || p_request_id::text, 0)
  );
  select r.* into receipt
    from sync_v2_private.deletion_receipts r
   where r.owner_id = p_owner_id and r.request_id = p_request_id;
  if found then
    if receipt.deletion_scope <> 'workspace' or receipt.workspace_id <> p_workspace_id then
      return pg_catalog.jsonb_build_object('outcome', 'request_reused');
    end if;
    return pg_catalog.jsonb_build_object(
      'outcome', 'already_deleted', 'receiptId', receipt.receipt_id,
      'completedAt', receipt.completed_at, 'policyVersion', receipt.policy_version
    );
  end if;
  if not sync_v2_private.device_session_is_active(
    p_owner_id, p_auth_session_id, p_device_id
  ) or not exists (
    select 1 from sync_v2_private.workspaces w
     where w.id = p_workspace_id and w.owner_id = p_owner_id
  ) then
    return pg_catalog.jsonb_build_object('outcome', 'not_deleted');
  end if;

  delete from sync_v2_private.approved_device_transfers t
   where t.owner_id = p_owner_id and t.workspace_id = p_workspace_id;
  delete from sync_v2_private.workspaces w
   where w.id = p_workspace_id and w.owner_id = p_owner_id;
  insert into sync_v2_private.deletion_receipts (
    owner_id, request_id, receipt_id, deletion_scope, workspace_id,
    policy_version, deletion_reason, completed_at, expires_at
  ) values (
    p_owner_id, p_request_id, pg_catalog.gen_random_uuid(), 'workspace', p_workspace_id,
    'sync-v2-retention-v1', 'delete_now', now_at, now_at + interval '90 days'
  ) returning * into receipt;
  return pg_catalog.jsonb_build_object(
    'outcome', 'deleted', 'receiptId', receipt.receipt_id,
    'completedAt', receipt.completed_at, 'policyVersion', receipt.policy_version
  );
end;
$$;

create function public.sync_v2_lifecycle_bridge_delete_account_sync_data(
  p_owner_id uuid,
  p_auth_session_id uuid,
  p_device_id uuid,
  p_request_id uuid
)
returns jsonb
language plpgsql
volatile
security definer
set search_path = ''
as $$
declare
  now_at timestamptz := pg_catalog.clock_timestamp();
  receipt sync_v2_private.deletion_receipts%rowtype;
begin
  if p_request_id is null
     or not sync_v2_private.auth_session_is_live(p_owner_id, p_auth_session_id) then
    return pg_catalog.jsonb_build_object('outcome', 'not_deleted');
  end if;
  perform pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended(p_owner_id::text || ':' || p_request_id::text, 0)
  );
  select r.* into receipt
    from sync_v2_private.deletion_receipts r
   where r.owner_id = p_owner_id and r.request_id = p_request_id;
  if found then
    if receipt.deletion_scope <> 'account_sync_data' then
      return pg_catalog.jsonb_build_object('outcome', 'request_reused');
    end if;
    return pg_catalog.jsonb_build_object(
      'outcome', 'already_deleted', 'receiptId', receipt.receipt_id,
      'completedAt', receipt.completed_at, 'policyVersion', receipt.policy_version
    );
  end if;
  if not sync_v2_private.device_session_is_active(
    p_owner_id, p_auth_session_id, p_device_id
  ) then
    return pg_catalog.jsonb_build_object('outcome', 'not_deleted');
  end if;

  delete from sync_v2_private.approved_device_transfers where owner_id = p_owner_id;
  delete from sync_v2_private.workspaces where owner_id = p_owner_id;
  delete from sync_v2_private.approved_device_enrollments where owner_id = p_owner_id;
  delete from sync_v2_private.device_enrollments where owner_id = p_owner_id;
  update sync_v2_private.sessions set revoked_at = now_at
   where owner_id = p_owner_id and revoked_at is null;
  update sync_v2_private.devices set revoked_at = now_at
   where owner_id = p_owner_id and revoked_at is null;
  insert into sync_v2_private.deletion_receipts (
    owner_id, request_id, receipt_id, deletion_scope, workspace_id,
    policy_version, deletion_reason, completed_at, expires_at
  ) values (
    p_owner_id, p_request_id, pg_catalog.gen_random_uuid(), 'account_sync_data', null,
    'sync-v2-retention-v1', 'account_deletion', now_at, now_at + interval '90 days'
  ) returning * into receipt;
  return pg_catalog.jsonb_build_object(
    'outcome', 'deleted', 'receiptId', receipt.receipt_id,
    'completedAt', receipt.completed_at, 'policyVersion', receipt.policy_version
  );
end;
$$;

create function sync_v2_private.run_retention_sweep(p_now timestamptz default pg_catalog.clock_timestamp())
returns table (workspaces_deleted integer, receipts_expired integer)
language plpgsql
volatile
security definer
set search_path = ''
as $$
declare
  workspace_row record;
  deleted_count integer := 0;
  expired_count integer := 0;
  request_id uuid;
begin
  if p_now is null then raise exception 'RETENTION_TIME_REQUIRED'; end if;
  for workspace_row in
    select w.id, w.owner_id
      from sync_v2_private.workspaces w
     where w.state = 'disabled' and w.purge_after <= p_now
     order by w.purge_after, w.id
     for update skip locked
  loop
    request_id := pg_catalog.gen_random_uuid();
    delete from sync_v2_private.approved_device_transfers t
     where t.owner_id = workspace_row.owner_id and t.workspace_id = workspace_row.id;
    delete from sync_v2_private.workspaces w
     where w.id = workspace_row.id and w.owner_id = workspace_row.owner_id;
    insert into sync_v2_private.deletion_receipts (
      owner_id, request_id, receipt_id, deletion_scope, workspace_id,
      policy_version, deletion_reason, completed_at, expires_at
    ) values (
      workspace_row.owner_id, request_id, pg_catalog.gen_random_uuid(),
      'workspace', workspace_row.id, 'sync-v2-retention-v1',
      'retention_expired', p_now, p_now + interval '90 days'
    );
    deleted_count := deleted_count + 1;
  end loop;
  delete from sync_v2_private.deletion_receipts r where r.expires_at <= p_now;
  get diagnostics expired_count = row_count;
  return query select deleted_count, expired_count;
end;
$$;

revoke all on function sync_v2_private.consume_lifecycle_bridge_limit(uuid, uuid, text)
  from public, anon, authenticated;
revoke all on function public.sync_v2_lifecycle_bridge_authorize(uuid, uuid, text)
  from public, anon, authenticated;
revoke all on function public.sync_v2_lifecycle_bridge_disable_workspace(uuid, uuid, uuid, uuid)
  from public, anon, authenticated;
revoke all on function public.sync_v2_lifecycle_bridge_restore_workspace(uuid, uuid, uuid, uuid)
  from public, anon, authenticated;
revoke all on function public.sync_v2_lifecycle_bridge_delete_workspace(uuid, uuid, uuid, uuid, uuid)
  from public, anon, authenticated;
revoke all on function public.sync_v2_lifecycle_bridge_delete_account_sync_data(uuid, uuid, uuid, uuid)
  from public, anon, authenticated;
revoke all on function sync_v2_private.run_retention_sweep(timestamptz)
  from public, anon, authenticated;

grant execute on function public.sync_v2_lifecycle_bridge_authorize(uuid, uuid, text)
  to service_role;
grant execute on function public.sync_v2_lifecycle_bridge_disable_workspace(uuid, uuid, uuid, uuid)
  to service_role;
grant execute on function public.sync_v2_lifecycle_bridge_restore_workspace(uuid, uuid, uuid, uuid)
  to service_role;
grant execute on function public.sync_v2_lifecycle_bridge_delete_workspace(uuid, uuid, uuid, uuid, uuid)
  to service_role;
grant execute on function public.sync_v2_lifecycle_bridge_delete_account_sync_data(uuid, uuid, uuid, uuid)
  to service_role;
grant execute on function sync_v2_private.run_retention_sweep(timestamptz)
  to service_role;

create extension if not exists pg_cron with schema pg_catalog;

select cron.schedule(
  'sitedatum-sync-v2-retention-v1',
  '17 3 * * *',
  $job$select * from sync_v2_private.run_retention_sweep();$job$
);

