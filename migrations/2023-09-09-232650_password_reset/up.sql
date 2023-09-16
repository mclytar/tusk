-- Your SQL goes here

CREATE TABLE "password_reset" (
                                  request_id                UUID                            PRIMARY KEY DEFAULT uuid_generate_v4(),
                                  user_id                   UUID                            NOT NULL,
                                  expiration                TIMESTAMP                       NOT NULL DEFAULT current_timestamp + interval '24' hour,
                                  FOREIGN KEY (user_id) REFERENCES "user"(user_id)
                                      ON UPDATE CASCADE
                                      ON DELETE CASCADE
);