-- Add migration script here
INSERT INTO users (username, password_hash, belonging_area_division, belonging_area_sub_area) VALUES
('admin', '$2b$10$7tdTuU8jyxFKR7uuScVOre9vAQ17wbgv4pKo.zuHWUXiiLklTOfQu', 'R&D', 'Board'),
('michelecarenini', '$2b$10$YJMQr/LFbFtXFZ82nkfB3.OJ5hqOPJEFSwVJ.pQosjUmgb3PQeqiq', 'R&D', 'Software'),
('advisors', '$2b$10$7tdTuU8jyxFKR7uuScVOre9vAQ17wbgv4pKo.zuHWUXiiLklTOfQu', 'R&D', 'Board');