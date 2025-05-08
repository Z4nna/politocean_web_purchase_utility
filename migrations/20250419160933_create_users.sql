-- Add migration script here
INSERT INTO users (username, password_hash, belonging_area_division, belonging_area_sub_area, role) VALUES
('admin', '$2b$10$X8Y7/FvY1ksblhwOCUVGCuPHIkibhJvbPvxHAb9AB4EUyCveqbFM.', 'R&D', 'Board', 'board'),
('michelecarenini', '$2b$10$YJMQr/LFbFtXFZ82nkfB3.OJ5hqOPJEFSwVJ.pQosjUmgb3PQeqiq', 'R&D', 'Software', 'advisor'),
('advisors', '$2b$10$7tdTuU8jyxFKR7uuScVOre9vAQ17wbgv4pKo.zuHWUXiiLklTOfQu', 'R&D', 'Board', 'board'),
('claudiosansoe', '$2b$10$ohB3HMTn6aiMI6EBqKRxWuHOP52Cdv.OZ018CqRSVSqrw5j8SDWWK', 'R&D', 'Board', 'prof');