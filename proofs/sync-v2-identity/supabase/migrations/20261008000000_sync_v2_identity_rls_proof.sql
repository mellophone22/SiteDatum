create schema sync_v2_proof;
create schema sync_v2_private;

revoke all on schema sync_v2_proof from public, anon;
grant usage on schema sync_v2_proof to authenticated;
revoke all on schema sync_v2_private from public, anon, authenticated;

create table sync_v2_private.sessions (
  id uuid primary key,
  owner_id uuid not null,
  revoked_at timestamptz
);

create table sync_v2_private.devices (
  id uuid primary key,
  owner_id uuid not null,
  revoked_at timestamptz
);

create table sync_v2_proof.envelopes (
  id uuid primary key,
  owner_id uuid not null,
  ciphertext bytea not null check (octet_length(ciphertext) > 0),
  created_at timestamptz not null default clock_timestamp()
);

alter table sync_v2_private.sessions enable row level security;
alter table sync_v2_private.devices enable row level security;
alter table sync_v2_proof.envelopes enable row level security;
alter table sync_v2_proof.envelopes force row level security;

revoke all on all tables in schema sync_v2_private from public, anon, authenticated;
revoke all on all tables in schema sync_v2_proof from public, anon, authenticated;
grant select, insert, update, delete on sync_v2_proof.envelopes to authenticated;

create function sync_v2_private.request_is_active_owner(p_owner_id uuid)
returns boolean
language sql
stable
security definer
set search_path = ''
as $$
  select
    p_owner_id = (select auth.uid())
    and exists (
      select 1
      from sync_v2_private.sessions s
      where s.id = nullif((select auth.jwt() ->> 'session_id'), '')::uuid
        and s.owner_id = p_owner_id
        and s.revoked_at is null
    )
    and exists (
      select 1
      from sync_v2_private.devices d
      where d.id = nullif((select auth.jwt() ->> 'sync_device_id'), '')::uuid
        and d.owner_id = p_owner_id
        and d.revoked_at is null
    );
$$;

revoke all on function sync_v2_private.request_is_active_owner(uuid) from public, anon;
grant execute on function sync_v2_private.request_is_active_owner(uuid) to authenticated;

create policy envelopes_select_owned_active
on sync_v2_proof.envelopes for select to authenticated
using ((select sync_v2_private.request_is_active_owner(owner_id)));

create policy envelopes_insert_owned_active
on sync_v2_proof.envelopes for insert to authenticated
with check ((select sync_v2_private.request_is_active_owner(owner_id)));

create policy envelopes_update_owned_active
on sync_v2_proof.envelopes for update to authenticated
using ((select sync_v2_private.request_is_active_owner(owner_id)))
with check ((select sync_v2_private.request_is_active_owner(owner_id)));

create policy envelopes_delete_owned_active
on sync_v2_proof.envelopes for delete to authenticated
using ((select sync_v2_private.request_is_active_owner(owner_id)));
