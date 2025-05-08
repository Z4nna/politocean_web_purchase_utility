-- Add migration script here
CREATE TABLE IF NOT EXISTS areas (
    sub_area TEXT NOT NULL,
    division TEXT NOT NULL,
    PRIMARY KEY (sub_area, division)
);

CREATE TABLE IF NOT EXISTS proposals (
    name TEXT NOT NULL PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS projects(
    name TEXT NOT NULL PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    email TEXT,
    password_hash TEXT NOT NULL,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    role TEXT NOT NULL DEFAULT 'advisor',
    belonging_area_division TEXT NOT NULL,
    belonging_area_sub_area TEXT NOT NULL,
    FOREIGN KEY (belonging_area_division, belonging_area_sub_area) REFERENCES areas(division, sub_area) ON UPDATE CASCADE
);

CREATE TABLE IF NOT EXISTS orders (
    id SERIAL PRIMARY KEY,
    author_id INT NOT NULL,
    FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE CASCADE ON UPDATE CASCADE,
    date DATE NOT NULL DEFAULT CURRENT_DATE,
    ready BOOLEAN NOT NULL DEFAULT FALSE,
    confirmed BOOLEAN NOT NULL DEFAULT FALSE,
    description TEXT NOT NULL DEFAULT '',
    area_division TEXT NOT NULL,
    area_sub_area TEXT NOT NULL,
    FOREIGN KEY (area_division, area_sub_area) REFERENCES areas(division, sub_area) ON UPDATE CASCADE
);

CREATE TABLE IF NOT EXISTS order_items (
    order_id INT NOT NULL,
    manufacturer TEXT NOT NULL,
    manufacturer_pn TEXT NOT NULL,
    quantity INT NOT NULL,
    PRIMARY KEY (order_id, manufacturer, manufacturer_pn),
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE ON UPDATE CASCADE,
    proposal TEXT NOT NULL,
    FOREIGN KEY (proposal) REFERENCES proposals(name) ON UPDATE CASCADE,
    project TEXT NOT NULL,
    FOREIGN KEY (project) REFERENCES projects(name) ON UPDATE CASCADE,
    mouser_pn TEXT,
    digikey_pn TEXT
);

CREATE TABLE IF NOT EXISTS order_bom (
    order_id INT NOT NULL PRIMARY KEY,
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE ON UPDATE CASCADE,
    bom_file_mouser BYTEA,
    bom_file_digikey BYTEA,
    filename TEXT
);