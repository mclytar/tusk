// @generated automatically by Diesel CLI.

diesel::table! {
    user (user_id) {
        user_id -> Uuid,
        username -> Varchar,
        password -> Varchar,
    }
}
