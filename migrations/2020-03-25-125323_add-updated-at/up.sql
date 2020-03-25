alter table histories add column updated_at timestamp with time zone not null default current_timestamp;
update histories set updated_at = created_at;
