-- Add migration script here
ALTER TABLE password_reset_tokens
ADD CONSTRAINT unique_user_token UNIQUE (user_id);