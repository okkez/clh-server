alter table histories add constraint histories_unique_constraint unique (hostname, working_directory, command);
