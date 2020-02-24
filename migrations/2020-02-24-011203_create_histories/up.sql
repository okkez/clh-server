create table if not exists histories (
  id serial primary key
  , hostname varchar not null
  , working_directory text
  , command text not null
  , created_at timestamp with time zone not null default current_timestamp
);
