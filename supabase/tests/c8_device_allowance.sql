-- C8-10 hosted sandbox acceptance.
-- This test uses fictional identities inside one transaction and rolls every row back.

begin;

create temporary table c8_device_allowance_results (
  sequence integer primary key,
  step text not null,
  outcome text not null
) on commit drop;

insert into auth.users (
  id, instance_id, aud, role, email, encrypted_password,
  email_confirmed_at, created_at, updated_at
) values (
  'c8100000-0000-4000-8000-000000000001',
  '00000000-0000-0000-0000-000000000000',
  'authenticated', 'authenticated', 'c8-device-test@example.invalid', '',
  clock_timestamp(), clock_timestamp(), clock_timestamp()
);

select public.licensing_project_subscription(
  'c8100000-0000-4000-8000-000000000001',
  'pro_monthly',
  'active',
  clock_timestamp() + interval '1 day',
  'c8_test',
  'c8-device-allowance',
  'c8100000-0000-4000-8000-000000000002'
);

do $$
declare
  v_first_device uuid;
  v_second_device uuid;
  v_replacement_device uuid;
  v_reused_device uuid;
  v_state text;
  v_active_count integer;
  v_third_rejected boolean := false;
begin
  select device_id, activation_state
  into v_first_device, v_state
  from public.licensing_activate_device(
    'c8100000-0000-4000-8000-000000000001',
    repeat('a', 64),
    clock_timestamp()
  );
  if v_state <> 'activated' then
    raise exception 'C8_10_FIRST_DEVICE_NOT_ACTIVATED';
  end if;
  insert into c8_device_allowance_results values (1, 'first device', 'activated');

  select device_id, activation_state
  into v_reused_device, v_state
  from public.licensing_activate_device(
    'c8100000-0000-4000-8000-000000000001',
    repeat('a', 64),
    clock_timestamp()
  );
  if v_state <> 'reused' or v_reused_device <> v_first_device then
    raise exception 'C8_10_EXISTING_DEVICE_NOT_REUSED';
  end if;
  insert into c8_device_allowance_results values (2, 'same device', 'reused without another slot');

  select device_id, activation_state
  into v_second_device, v_state
  from public.licensing_activate_device(
    'c8100000-0000-4000-8000-000000000001',
    repeat('b', 64),
    clock_timestamp()
  );
  if v_state <> 'activated' or v_second_device = v_first_device then
    raise exception 'C8_10_SECOND_DEVICE_NOT_ACTIVATED';
  end if;
  insert into c8_device_allowance_results values (3, 'second device', 'activated');

  begin
    perform * from public.licensing_activate_device(
      'c8100000-0000-4000-8000-000000000001',
      repeat('c', 64),
      clock_timestamp()
    );
  exception
    when sqlstate 'P0001' then
      if sqlerrm = 'LICENSING_DEVICE_LIMIT_REACHED' then
        v_third_rejected := true;
      else
        raise;
      end if;
  end;
  if not v_third_rejected then
    raise exception 'C8_10_THIRD_DEVICE_WAS_NOT_REJECTED';
  end if;
  insert into c8_device_allowance_results values (4, 'third device', 'rejected at two-device limit');

  select count(*)::integer
  into v_active_count
  from licensing.devices
  where customer_id = (
    select id from licensing.customers
    where auth_user_id = 'c8100000-0000-4000-8000-000000000001'
  ) and deactivated_at_utc is null;
  if v_active_count <> 2 then
    raise exception 'C8_10_ACTIVE_DEVICE_COUNT_INVALID_BEFORE_REPLACEMENT';
  end if;

  if not public.licensing_deactivate_device(
    'c8100000-0000-4000-8000-000000000001',
    v_first_device,
    clock_timestamp()
  ) then
    raise exception 'C8_10_DEVICE_DEACTIVATION_FAILED';
  end if;
  insert into c8_device_allowance_results values (5, 'retired device', 'deactivated');

  select device_id, activation_state
  into v_replacement_device, v_state
  from public.licensing_activate_device(
    'c8100000-0000-4000-8000-000000000001',
    repeat('c', 64),
    clock_timestamp()
  );
  if v_state <> 'activated' or v_replacement_device in (v_first_device, v_second_device) then
    raise exception 'C8_10_REPLACEMENT_DEVICE_NOT_ACTIVATED';
  end if;

  select count(*)::integer
  into v_active_count
  from licensing.devices
  where customer_id = (
    select id from licensing.customers
    where auth_user_id = 'c8100000-0000-4000-8000-000000000001'
  ) and deactivated_at_utc is null;
  if v_active_count <> 2 then
    raise exception 'C8_10_ACTIVE_DEVICE_COUNT_INVALID_AFTER_REPLACEMENT';
  end if;
  if exists (
    select 1 from public.licensing_current_entitlement(
      'c8100000-0000-4000-8000-000000000001',
      v_first_device
    )
  ) then
    raise exception 'C8_10_DEACTIVATED_DEVICE_RETAINED_ENTITLEMENT';
  end if;
  if (select plan from public.licensing_current_entitlement(
    'c8100000-0000-4000-8000-000000000001',
    v_replacement_device
  )) <> 'pro_monthly' then
    raise exception 'C8_10_REPLACEMENT_ENTITLEMENT_MISSING';
  end if;
  insert into c8_device_allowance_results values (6, 'replacement device', 'activated with Pro entitlement');
end;
$$;

select sequence, step, outcome
from c8_device_allowance_results
order by sequence;

rollback;
