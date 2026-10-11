-- Disposable Sync proof only. No production deployment or Sync activation.
-- Validate existing rows without clamping, rewriting, or deleting them.
-- Keep original retry lookup, locks, RLS and function privileges unchanged.

alter table sync_v2_private.workspaces
  add constraint workspaces_current_checkpoint_counter_safe_integer
  check (current_checkpoint_counter <= 9007199254740991);
alter table sync_v2_private.workspaces
  add constraint workspaces_last_change_seq_safe_integer
  check (last_change_seq <= 9007199254740991);
alter table sync_v2_private.record_envelopes
  add constraint record_envelopes_server_version_safe_integer
  check (server_version <= 9007199254740991);
alter table sync_v2_private.record_changes
  add constraint record_changes_change_seq_safe_integer
  check (change_seq <= 9007199254740991);
alter table sync_v2_private.record_changes
  add constraint record_changes_server_version_safe_integer
  check (server_version <= 9007199254740991);
alter table sync_v2_private.workspace_checkpoints
  add constraint workspace_checkpoints_checkpoint_counter_safe_integer
  check (checkpoint_counter <= 9007199254740991);
alter table sync_v2_private.workspace_checkpoints
  add constraint workspace_checkpoints_through_change_seq_safe_integer
  check (through_change_seq <= 9007199254740991);
alter table sync_v2_private.applied_record_batches
  add constraint applied_record_batches_checkpoint_counter_safe_integer
  check (checkpoint_counter <= 9007199254740991);
alter table sync_v2_private.applied_record_batches
  add constraint applied_record_batches_through_change_seq_safe_integer
  check (through_change_seq <= 9007199254740991);

-- Refuses an already out-of-range sequence; never rewind or cycle identifiers.
alter sequence sync_v2_private.record_changes_change_seq_seq
  maxvalue 9007199254740991 no cycle;

create or replace function public.sync_v2_record_bridge_push_batch(
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
     or p_checkpoint_counter > 9007199254740991
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
       or expected_version >= 9007199254740991
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

create or replace function public.sync_v2_record_bridge_pull_changes(
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
     or p_after_cursor > 9007199254740991
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
