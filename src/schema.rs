// @generated automatically by Diesel CLI.

diesel::table! {
    connectors (id) {
        id -> Uuid,
        title -> Varchar,
        #[sql_name = "type"]
        type_ -> Varchar,
    }
}
