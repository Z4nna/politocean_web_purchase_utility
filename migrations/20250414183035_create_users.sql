CREATE ROLE users_group;
CREATE ROLE michelecarenini LOGIN PASSWORD 'MicheleCarenini25!';
GRANT users_group TO michelecarenini;

CREATE ROLE admin_group;
CREATE ROLE admin LOGIN PASSWORD 'adminpwd';
GRANT admin_group TO admin;

-- ADMIN
GRANT ALL PRIVILEGES ON DATABASE politocean_db TO admin_group;
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO admin_group;

-- USER
