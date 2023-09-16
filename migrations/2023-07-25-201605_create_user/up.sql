-- Your SQL goes here

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE "user" (
                        user_id                     UUID                            PRIMARY KEY DEFAULT uuid_generate_v4(),
                        email                       VARCHAR                         UNIQUE NOT NULL,
                        display                     VARCHAR                         NOT NULL,
                        password                    VARCHAR                         NOT NULL
);