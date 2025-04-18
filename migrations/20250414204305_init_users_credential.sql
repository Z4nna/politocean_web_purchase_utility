-- Add migration script here
INSERT INTO areas (division, sub_area) VALUES
('MATE', 'Software'),
('MATE', 'Electronics'),
('MATE', 'Mechanics'),
('MATE', 'Control Systems'),
('MATE', 'Board'),

('R&D', 'Software'),
('R&D', 'Electronics'),
('R&D', 'Mechanics'),
('R&D', 'Control Systems'),
('R&D', 'Hydrodynamics'),
('R&D', 'Materials'),
('R&D', 'Board');

INSERT INTO proposals (name) VALUES
('Telecamera x 2'),
('Elettronica generale'),
('Fibra ottica'),
('GPS'),
('Motore scorta (t200) x 2'),
('Luci x 4'),
('Sensoristica AUV');

INSERT INTO projects (name) VALUES
('Nereo'),
('Proteo'),
('Nuovo ROV'),
('EVA'),
('Doc_materiali'),
('Varie per lab'),
('Float'),
('E20');

INSERT INTO users (username, password_hash, belonging_area_division, belonging_area_sub_area) VALUES
('admin', 'adminpwd', 'R&D', 'Board'),
('michelecarenini', 'MicheleCarenini25!', 'R&D', 'Software');
('lucreziacaviasso', 'LucreziaCaviasso25!', 'R&D', 'Electronics');