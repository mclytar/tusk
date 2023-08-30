// @generated automatically by Diesel CLI.

diesel::table! {
    role (role_id) {
        role_id -> Uuid,
        name -> Varchar,
        display -> Varchar,
    }
}

diesel::table! {
    user (user_id) {
        user_id -> Uuid,
        username -> Varchar,
        password -> Varchar,
    }
}

diesel::table! {
    user_role (user_role_id) {
        user_role_id -> Uuid,
        user_id -> Uuid,
        role_id -> Uuid,
    }
}

diesel::joinable!(user_role -> role (role_id));
diesel::joinable!(user_role -> user (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    role,
    user,
    user_role,
);
