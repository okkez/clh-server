table! {
    histories (id) {
        id -> Int4,
        hostname -> Varchar,
        working_directory -> Nullable<Text>,
        command -> Text,
        created_at -> Timestamptz,
    }
}
