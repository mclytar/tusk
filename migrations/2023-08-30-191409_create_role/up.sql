-- Your SQL goes here

CREATE TABLE "role" (
                        role_id                     UUID                            PRIMARY KEY DEFAULT uuid_generate_v4(),
                        name                        VARCHAR                         UNIQUE NOT NULL,
                        display                     VARCHAR                         NOT NULL
);

INSERT INTO "role" (name, display) VALUES ('admin', 'Admin'), ('directory', 'Directory'), ('user', 'User');

CREATE TABLE "user_role" (
                        user_role_id                UUID                            PRIMARY KEY DEFAULT uuid_generate_v4(),
                        user_id                     UUID                            NOT NULL,
                        role_id                     UUID                            NOT NULL,
                        FOREIGN KEY (user_id) REFERENCES "user"(user_id),
                        FOREIGN KEY (role_id) REFERENCES "role"(role_id)
);