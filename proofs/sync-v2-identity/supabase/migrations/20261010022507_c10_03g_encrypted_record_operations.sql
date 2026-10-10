-- C10-03G: service-only encrypted record storage and atomic checkpoint bridge.
--
-- This disposable proof stores only opaque identifiers, constrained routing
-- metadata, bounded ciphertext, and server timestamps. It does not expose a
-- desktop command, enable Sync, or deploy a production resource.

create table sync_v2_private.workspaces (
  id uuid primary key,
  owner_id uuid not null,
  state text not null default 'active' check (state in ('active', 'disabled')),
  current_checkpoint_counter bigint not null default 0
    check (current_checkpoint_counter >= 0),
  last_change_seq bigint not null default 0 check (last_change_seq >= 0),
  created_at timestamptz not null,
  updated_at timestamptz not null,
  disabled_at timestamptz,
  unique (id, owner_id),
  check (
    (state = 'active' and disabled_at is null)
    or (state = 'disabled' and disabled_at is not null)
  )
);

create index sync_v2_workspaces_owner_created_idx
  on sync_v2_private.workspaces (owner_id, created_at desc);

create table sync_v2_private.record_envelopes (
  owner_id uuid not null,
  workspace_id uuid not null,
  record_id uuid not null,
  record_kind smallint not null check (record_kind between 1 and 13),
  server_version bigint not null check (server_version >= 1),
  client_mutation_id uuid not null,
  protocol_version smallint not null check (protocol_version = 1),
  workspace_key_version integer not null
    check (workspace_key_version between 1 and 2147483647),
  ciphertext bytea not null
    check (pg_catalog.octet_length(ciphertext) between 16 and 262144),
  is_tombstone boolean not null,
  created_at timestamptz not null,
  updated_at timestamptz not null,
  deleted_at timestamptz,
  primary key (owner_id, workspace_id, record_id),
  foreign key (workspace_id, owner_id)
    references sync_v2_private.workspaces (id, owner_id) on delete cascade,
  check (
    (is_tombstone and deleted_at is not null)
    or (not is_tombstone and deleted_at is null)
  ),
  check (created_at <= updated_at)
);

create table sync_v2_private.record_changes (
  change_seq bigint generated always as identity primary key,
  owner_id uuid not null,
  workspace_id uuid not null,
  batch_id uuid not null,
  record_id uuid not null,
  record_kind smallint not null check (record_kind between 1 and 13),
  server_version bigint not null check (server_version >= 1),
  client_mutation_id uuid not null,
  protocol_version smallint not null check (protocol_version = 1),
  workspace_key_version integer not null
    check (workspace_key_version between 1 and 2147483647),
  ciphertext bytea not null
    check (pg_catalog.octet_length(ciphertext) between 16 and 262144),
  is_tombstone boolean not null,
  changed_at timestamptz not null,
  deleted_at timestamptz,
  unique (owner_id, client_mutation_id),
  foreign key (workspace_id, owner_id)
    references sync_v2_private.workspaces (id, owner_id) on delete cascade,
  check (
    (is_tombstone and deleted_at is not null)
    or (not is_tombstone and deleted_at is null)
  )
);

create index sync_v2_record_changes_workspace_cursor_idx
  on sync_v2_private.record_changes (owner_id, workspace_id, change_seq);

create table sync_v2_private.workspace_checkpoints (
  owner_id uuid not null,
  workspace_id uuid not null,
  checkpoint_counter bigint not null check (checkpoint_counter >= 1),
  through_change_seq bigint not null check (through_change_seq >= 1),
  protocol_version smallint not null check (protocol_version = 1),
  workspace_key_version integer not null
    check (workspace_key_version between 1 and 2147483647),
  ciphertext bytea not null
    check (pg_catalog.octet_length(ciphertext) between 16 and 131072),
  created_at timestamptz not null,
  primary key (owner_id, workspace_id, checkpoint_counter),
  unique (owner_id, workspace_id, through_change_seq),
  foreign key (workspace_id, owner_id)
    references sync_v2_private.workspaces (id, owner_id) on delete cascade
);

create table sync_v2_private.applied_record_batches (
  owner_id uuid not null,
  workspace_id uuid not null,
  batch_id uuid not null,
  request_hash bytea not null check (pg_catalog.octet_length(request_hash) = 32),
  checkpoint_counter bigint not null check (checkpoint_counter >= 1),
  through_change_seq bigint not null check (through_change_seq >= 1),
  mutation_count smallint not null check (mutation_count between 1 and 50),
  applied_at timestamptz not null,
  primary key (owner_id, workspace_id, batch_id),
  foreign key (workspace_id, owner_id)
    references sync_v2_private.workspaces (id, owner_id) on delete cascade
);

create table sync_v2_private.record_bridge_windows (
  owner_id uuid not null,
  auth_session_id uuid not null,
  action text not null check (action in ('create_workspace', 'push_batch', 'pull_changes')),
  window_started_at timestamptz not null,
  request_count smallint not null check (request_count between 1 and 60),
  last_request_at timestamptz not null,
  primary key (owner_id, auth_session_id, action, window_started_at)
);

alter table sync_v2_private.workspaces enable row level security;
alter table sync_v2_private.workspaces force row level security;
alter table sync_v2_private.record_envelopes enable row level security;
alter table sync_v2_private.record_envelopes force row level security;
alter table sync_v2_private.record_changes enable row level security;
alter table sync_v2_private.record_changes force row level security;
alter table sync_v2_private.workspace_checkpoints enable row level security;
alter table sync_v2_private.workspace_checkpoints force row level security;
alter table sync_v2_private.applied_record_batches enable row level security;
alter table sync_v2_private.applied_record_batches force row level security;
alter table sync_v2_private.record_bridge_windows enable row level security;
alter table sync_v2_private.record_bridge_windows force row level security;

revoke all on table sync_v2_private.workspaces,
  sync_v2_private.record_envelopes,
  sync_v2_private.record_changes,
  sync_v2_private.workspace_checkpoints,
  sync_v2_private.applied_record_batches,
  sync_v2_private.record_bridge_windows
  from public, anon, authenticated, service_role;

revoke all on sequence sync_v2_private.record_changes_change_seq_seq
  from public, anon, authenticated, service_role;

create function sync_v2_private.consume_record_bridge_limit(
  p_owner_id uuid,
  p_auth_session_id uuid,
  p_action text,
  p_limit smallint
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
     or p_action not in ('create_workspace', 'push_batch', 'pull_changes')
     or p_limit is null
     or p_limit < 1
     or p_limit > 60
     or not sync_v2_private.auth_session_is_live(p_owner_id, p_auth_session_id) then
    return false;
  end if;

  perform pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended(
      p_owner_id::text || ':' || p_auth_session_id::text || ':' || p_action, 0
    )
  );

  delete from sync_v2_private.record_bridge_windows w
   where w.owner_id = p_owner_id
     and w.window_started_at < now_at - interval '24 hours';

  insert into sync_v2_private.record_bridge_windows (
    owner_id, auth_session_id, action, window_started_at, request_count, last_request_at
  ) values (
    p_owner_id, p_auth_session_id, p_action, window_at, 1, now_at
  )
  on conflict (owner_id, auth_session_id, action, window_started_at)
  do update
     set request_count = sync_v2_private.record_bridge_windows.request_count + 1,
         last_request_at = excluded.last_request_at
   where sync_v2_private.record_bridge_windows.request_count < p_limit
  returning request_count into accepted_count;

  return accepted_count is not null;
end;
$$;

create function public.sync_v2_record_bridge_authorize(
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
declare
  request_limit smallint;
begin
  if not sync_v2_private.auth_session_is_live(p_owner_id, p_auth_session_id) then
    return 'session_invalid';
  end if;

  request_limit := case p_action
    when 'create_workspace' then 10
    when 'push_batch' then 30
    when 'pull_changes' then 60
    else null
  end;
  if request_limit is null then
    return 'request_invalid';
  end if;
  if not sync_v2_private.consume_record_bridge_limit(
    p_owner_id, p_auth_session_id, p_action, request_limit
  ) then
    return 'rate_limited';
  end if;
  return 'authorized';
end;
$$;

create function public.sync_v2_record_bridge_create_workspace(
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
  existing_owner uuid;
begin
  if p_workspace_id is null
     or not sync_v2_private.device_session_is_active(
       p_owner_id, p_auth_session_id, p_device_id
     ) then
    return 'not_created';
  end if;

  perform pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended(p_workspace_id::text, 0)
  );

  select w.owner_id into existing_owner
    from sync_v2_private.workspaces w
   where w.id = p_workspace_id;

  if found then
    if existing_owner = p_owner_id and exists (
      select 1 from sync_v2_private.workspaces w
       where w.id = p_workspace_id and w.owner_id = p_owner_id and w.state = 'active'
    ) then
      return 'already_created';
    end if;
    return 'not_created';
  end if;

  insert into sync_v2_private.workspaces (
    id, owner_id, created_at, updated_at
  ) values (
    p_workspace_id, p_owner_id, now_at, now_at
  );
  return 'created';
end;
$$;

create function public.sync_v2_record_bridge_device_key(
  p_owner_id uuid,
  p_auth_session_id uuid,
  p_device_id uuid
)
returns text
language sql
stable
security definer
set search_path = ''
as $$
  select pg_catalog.encode(d.proof_public_key, 'base64')
    from sync_v2_private.devices d
   where d.id = p_device_id
     and d.owner_id = p_owner_id
     and d.revoked_at is null
     and d.proof_algorithm = 'ed25519-v1'
     and sync_v2_private.device_session_is_active(
       p_owner_id, p_auth_session_id, p_device_id
     );
$$;

create function public.sync_v2_record_bridge_push_batch(
  p_owner_id uuid,
  p_auth_session_id uuid,
  p_device_id uuid,
  p_workspace_id uuid,
  p_batch_id uuid,
  p_mutations jsonb,
  p_checkpoint_counter bigint,
  p_checkpoint_protocol_version smallint,
  p_checkpoint_key_version integer,
  p_checkpoint_ciphertext_base64 text
)
returns jsonb
language plpgsql
volatile
security definer
set search_path = ''
as $$
declare
  now_at timestamptz := pg_catalog.clock_timestamp();
  mutation_count integer;
  request_hash bytea;
  existing_batch sync_v2_private.applied_record_batches%rowtype;
  workspace sync_v2_private.workspaces%rowtype;
  mutation jsonb;
  record_uuid uuid;
  mutation_uuid uuid;
  expected_version bigint;
  current_version bigint;
  next_version bigint;
  kind smallint;
  protocol smallint;
  key_version integer;
  body bytea;
  tombstone boolean;
  batch_through_seq bigint := 0;
  changed_seq bigint;
begin
  if p_owner_id is null
     or p_auth_session_id is null
     or p_device_id is null
     or p_workspace_id is null
     or p_batch_id is null
     or p_checkpoint_counter is null
     or p_checkpoint_counter < 1
     or p_checkpoint_protocol_version <> 1
     or p_checkpoint_key_version is null
     or p_checkpoint_key_version < 1
     or p_checkpoint_ciphertext_base64 is null
     or pg_catalog.octet_length(
       pg_catalog.decode(p_checkpoint_ciphertext_base64, 'base64')
     ) not between 16 and 131072
     or pg_catalog.jsonb_typeof(p_mutations) <> 'array'
     or not sync_v2_private.device_session_is_active(
       p_owner_id, p_auth_session_id, p_device_id
     ) then
    return pg_catalog.jsonb_build_object('outcome', 'invalid_request');
  end if;

  mutation_count := pg_catalog.jsonb_array_length(p_mutations);
  if mutation_count not between 1 and 50 then
    return pg_catalog.jsonb_build_object('outcome', 'invalid_request');
  end if;

  request_hash := extensions.digest(
    pg_catalog.convert_to(
      pg_catalog.jsonb_build_object(
        'workspaceId', p_workspace_id,
        'batchId', p_batch_id,
        'mutations', p_mutations,
        'checkpointCounter', p_checkpoint_counter,
        'checkpointProtocolVersion', p_checkpoint_protocol_version,
        'checkpointKeyVersion', p_checkpoint_key_version,
        'checkpointCiphertext', p_checkpoint_ciphertext_base64
      )::text,
      'UTF8'
    ),
    'sha256'
  );

  perform pg_catalog.pg_advisory_xact_lock(
    pg_catalog.hashtextextended(
      p_owner_id::text || ':' || p_workspace_id::text, 0
    )
  );

  select b.* into existing_batch
    from sync_v2_private.applied_record_batches b
   where b.owner_id = p_owner_id
     and b.workspace_id = p_workspace_id
     and b.batch_id = p_batch_id;

  if found then
    if existing_batch.request_hash = request_hash then
      return pg_catalog.jsonb_build_object(
        'outcome', 'already_applied',
        'checkpointCounter', existing_batch.checkpoint_counter,
        'throughCursor', existing_batch.through_change_seq,
        'mutationCount', existing_batch.mutation_count
      );
    end if;
    return pg_catalog.jsonb_build_object('outcome', 'batch_conflict');
  end if;

  select w.* into workspace
    from sync_v2_private.workspaces w
   where w.id = p_workspace_id
     and w.owner_id = p_owner_id
     and w.state = 'active'
   for update;

  if not found then
    return pg_catalog.jsonb_build_object('outcome', 'workspace_not_available');
  end if;

  if p_checkpoint_counter <> workspace.current_checkpoint_counter + 1 then
    return pg_catalog.jsonb_build_object(
      'outcome', 'checkpoint_conflict',
      'currentCheckpointCounter', workspace.current_checkpoint_counter,
      'throughCursor', workspace.last_change_seq
    );
  end if;

  if (select pg_catalog.count(distinct item ->> 'recordId')
        from pg_catalog.jsonb_array_elements(p_mutations) item) <> mutation_count
     or (select pg_catalog.count(distinct item ->> 'mutationId')
           from pg_catalog.jsonb_array_elements(p_mutations) item) <> mutation_count then
    return pg_catalog.jsonb_build_object('outcome', 'invalid_request');
  end if;

  for mutation in select value from pg_catalog.jsonb_array_elements(p_mutations)
  loop
    if pg_catalog.jsonb_typeof(mutation) <> 'object'
       or not mutation ?& array[
         'recordId', 'recordKind', 'expectedServerVersion', 'mutationId',
         'protocolVersion', 'workspaceKeyVersion', 'ciphertext', 'isTombstone'
       ]
       or exists (
         select 1 from pg_catalog.jsonb_object_keys(mutation) key
          where key <> all (array[
            'recordId', 'recordKind', 'expectedServerVersion', 'mutationId',
            'protocolVersion', 'workspaceKeyVersion', 'ciphertext', 'isTombstone'
          ])
       ) then
      return pg_catalog.jsonb_build_object('outcome', 'invalid_request');
    end if;

    record_uuid := (mutation ->> 'recordId')::uuid;
    mutation_uuid := (mutation ->> 'mutationId')::uuid;
    expected_version := (mutation ->> 'expectedServerVersion')::bigint;
    kind := (mutation ->> 'recordKind')::smallint;
    protocol := (mutation ->> 'protocolVersion')::smallint;
    key_version := (mutation ->> 'workspaceKeyVersion')::integer;
    body := pg_catalog.decode(mutation ->> 'ciphertext', 'base64');
    tombstone := (mutation ->> 'isTombstone')::boolean;

    if expected_version < 0
       or kind not between 1 and 13
       or protocol <> 1
       or key_version < 1
       or pg_catalog.octet_length(body) not between 16 and 262144 then
      return pg_catalog.jsonb_build_object('outcome', 'invalid_request');
    end if;

    if exists (
      select 1 from sync_v2_private.record_changes c
       where c.owner_id = p_owner_id
         and c.client_mutation_id = mutation_uuid
    ) then
      return pg_catalog.jsonb_build_object('outcome', 'mutation_conflict');
    end if;

    select e.server_version into current_version
      from sync_v2_private.record_envelopes e
     where e.owner_id = p_owner_id
       and e.workspace_id = p_workspace_id
       and e.record_id = record_uuid;

    if coalesce(current_version, 0) <> expected_version then
      return pg_catalog.jsonb_build_object(
        'outcome', 'record_conflict',
        'recordId', record_uuid,
        'currentServerVersion', coalesce(current_version, 0)
      );
    end if;
  end loop;

  for mutation in select value from pg_catalog.jsonb_array_elements(p_mutations)
  loop
    record_uuid := (mutation ->> 'recordId')::uuid;
    mutation_uuid := (mutation ->> 'mutationId')::uuid;
    expected_version := (mutation ->> 'expectedServerVersion')::bigint;
    next_version := expected_version + 1;
    kind := (mutation ->> 'recordKind')::smallint;
    protocol := (mutation ->> 'protocolVersion')::smallint;
    key_version := (mutation ->> 'workspaceKeyVersion')::integer;
    body := pg_catalog.decode(mutation ->> 'ciphertext', 'base64');
    tombstone := (mutation ->> 'isTombstone')::boolean;

    insert into sync_v2_private.record_envelopes (
      owner_id, workspace_id, record_id, record_kind, server_version,
      client_mutation_id, protocol_version, workspace_key_version, ciphertext,
      is_tombstone, created_at, updated_at, deleted_at
    ) values (
      p_owner_id, p_workspace_id, record_uuid, kind, next_version,
      mutation_uuid, protocol, key_version, body, tombstone, now_at, now_at,
      case when tombstone then now_at else null end
    )
    on conflict (owner_id, workspace_id, record_id)
    do update set
      record_kind = excluded.record_kind,
      server_version = excluded.server_version,
      client_mutation_id = excluded.client_mutation_id,
      protocol_version = excluded.protocol_version,
      workspace_key_version = excluded.workspace_key_version,
      ciphertext = excluded.ciphertext,
      is_tombstone = excluded.is_tombstone,
      updated_at = excluded.updated_at,
      deleted_at = excluded.deleted_at;

    insert into sync_v2_private.record_changes (
      owner_id, workspace_id, batch_id, record_id, record_kind, server_version,
      client_mutation_id, protocol_version, workspace_key_version, ciphertext,
      is_tombstone, changed_at, deleted_at
    ) values (
      p_owner_id, p_workspace_id, p_batch_id, record_uuid, kind, next_version,
      mutation_uuid, protocol, key_version, body, tombstone, now_at,
      case when tombstone then now_at else null end
    ) returning change_seq into changed_seq;
    batch_through_seq := greatest(batch_through_seq, changed_seq);
  end loop;

  insert into sync_v2_private.workspace_checkpoints (
    owner_id, workspace_id, checkpoint_counter, through_change_seq,
    protocol_version, workspace_key_version, ciphertext, created_at
  ) values (
    p_owner_id, p_workspace_id, p_checkpoint_counter, batch_through_seq,
    p_checkpoint_protocol_version, p_checkpoint_key_version,
    pg_catalog.decode(p_checkpoint_ciphertext_base64, 'base64'), now_at
  );

  update sync_v2_private.workspaces
     set current_checkpoint_counter = p_checkpoint_counter,
         last_change_seq = batch_through_seq,
         updated_at = now_at
   where id = p_workspace_id and owner_id = p_owner_id;

  insert into sync_v2_private.applied_record_batches (
    owner_id, workspace_id, batch_id, request_hash, checkpoint_counter,
    through_change_seq, mutation_count, applied_at
  ) values (
    p_owner_id, p_workspace_id, p_batch_id, request_hash,
    p_checkpoint_counter, batch_through_seq, mutation_count, now_at
  );

  return pg_catalog.jsonb_build_object(
    'outcome', 'applied',
    'checkpointCounter', p_checkpoint_counter,
    'throughCursor', batch_through_seq,
    'mutationCount', mutation_count
  );
exception
  when invalid_text_representation or numeric_value_out_of_range
    or invalid_parameter_value or data_exception then
    return pg_catalog.jsonb_build_object('outcome', 'invalid_request');
end;
$$;

create function public.sync_v2_record_bridge_pull_changes(
  p_owner_id uuid,
  p_auth_session_id uuid,
  p_device_id uuid,
  p_workspace_id uuid,
  p_after_cursor bigint,
  p_limit smallint
)
returns jsonb
language plpgsql
stable
security definer
set search_path = ''
as $$
declare
  workspace_available boolean;
  through_cursor bigint;
  has_more boolean;
  changes jsonb;
  checkpoint jsonb;
begin
  if p_after_cursor is null
     or p_after_cursor < 0
     or p_limit is null
     or p_limit not between 1 and 100
     or not sync_v2_private.device_session_is_active(
       p_owner_id, p_auth_session_id, p_device_id
     ) then
    return pg_catalog.jsonb_build_object('outcome', 'not_available');
  end if;

  select exists (
    select 1 from sync_v2_private.workspaces w
     where w.id = p_workspace_id
       and w.owner_id = p_owner_id
       and w.state = 'active'
  ) into workspace_available;
  if not workspace_available then
    return pg_catalog.jsonb_build_object('outcome', 'not_available');
  end if;

  with page as (
    select c.*
      from sync_v2_private.record_changes c
     where c.owner_id = p_owner_id
       and c.workspace_id = p_workspace_id
       and c.change_seq > p_after_cursor
     order by c.change_seq
     limit p_limit
  )
  select coalesce(pg_catalog.max(p.change_seq), p_after_cursor),
         coalesce(
           pg_catalog.jsonb_agg(
             pg_catalog.jsonb_build_object(
               'cursor', p.change_seq,
               'recordId', p.record_id,
               'recordKind', p.record_kind,
               'serverVersion', p.server_version,
               'mutationId', p.client_mutation_id,
               'protocolVersion', p.protocol_version,
               'workspaceKeyVersion', p.workspace_key_version,
               'ciphertext', pg_catalog.encode(p.ciphertext, 'base64'),
               'isTombstone', p.is_tombstone,
               'changedAt', p.changed_at,
               'deletedAt', p.deleted_at
             ) order by p.change_seq
           ),
           '[]'::jsonb
         )
    into through_cursor, changes
    from page p;

  select exists (
    select 1 from sync_v2_private.record_changes c
     where c.owner_id = p_owner_id
       and c.workspace_id = p_workspace_id
       and c.change_seq > through_cursor
  ) into has_more;

  select pg_catalog.jsonb_build_object(
           'counter', c.checkpoint_counter,
           'throughCursor', c.through_change_seq,
           'protocolVersion', c.protocol_version,
           'workspaceKeyVersion', c.workspace_key_version,
           'ciphertext', pg_catalog.encode(c.ciphertext, 'base64'),
           'createdAt', c.created_at
         )
    into checkpoint
    from sync_v2_private.workspace_checkpoints c
   where c.owner_id = p_owner_id
     and c.workspace_id = p_workspace_id
     and c.through_change_seq <= through_cursor
   order by c.checkpoint_counter desc
   limit 1;

  return pg_catalog.jsonb_build_object(
    'outcome', 'available',
    'workspaceId', p_workspace_id,
    'afterCursor', p_after_cursor,
    'throughCursor', through_cursor,
    'hasMore', has_more,
    'changes', changes,
    'checkpoint', checkpoint
  );
end;
$$;

revoke all on function sync_v2_private.consume_record_bridge_limit(uuid, uuid, text, smallint)
  from public, anon, authenticated, service_role;
revoke all on function public.sync_v2_record_bridge_authorize(uuid, uuid, text)
  from public, anon, authenticated;
revoke all on function public.sync_v2_record_bridge_create_workspace(uuid, uuid, uuid, uuid)
  from public, anon, authenticated;
revoke all on function public.sync_v2_record_bridge_device_key(uuid, uuid, uuid)
  from public, anon, authenticated;
revoke all on function public.sync_v2_record_bridge_push_batch(uuid, uuid, uuid, uuid, uuid, jsonb, bigint, smallint, integer, text)
  from public, anon, authenticated;
revoke all on function public.sync_v2_record_bridge_pull_changes(uuid, uuid, uuid, uuid, bigint, smallint)
  from public, anon, authenticated;

grant execute on function public.sync_v2_record_bridge_authorize(uuid, uuid, text)
  to service_role;
grant execute on function public.sync_v2_record_bridge_create_workspace(uuid, uuid, uuid, uuid)
  to service_role;
grant execute on function public.sync_v2_record_bridge_device_key(uuid, uuid, uuid)
  to service_role;
grant execute on function public.sync_v2_record_bridge_push_batch(uuid, uuid, uuid, uuid, uuid, jsonb, bigint, smallint, integer, text)
  to service_role;
grant execute on function public.sync_v2_record_bridge_pull_changes(uuid, uuid, uuid, uuid, bigint, smallint)
  to service_role;
