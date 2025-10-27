-- Add migration script here
UPDATE users 
SET role = 'advisor'
WHERE username = 'advisors'