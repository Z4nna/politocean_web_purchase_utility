-- Add migration script here
INSERT INTO areas (division, sub_area) VALUES
('MATE', 'Hydrodynamics');
INSERT INTO users (username, password_hash, belonging_area_division, belonging_area_sub_area, role, email) VALUES
-- Advisors
('andreaforte', '$2b$10$ApizoCvmWIpj.hWmQzAnT.5AerRJWSYIekAEj2YOeiaSG5QaF7oJO', 'R&D', 'Mechanics', 'advisor', 'andrea.forte@studenti.polito.it'),
('davidecolabella', '$2b$10$X3ZFWqpvRNluVXP3vCs0ruddsEO3hsBWYi2Ck1XBM1oV8Np03QJKq', 'R&D', 'Software', 'advisor', 'davide.colabella@studenti.polito.it'),
('luigigentile', '$2b$10$M4IMNxGN5A6teRUaaudz0e73XMbjclzt.IF.rtthhqKdIoLQrVc6m', 'R&D', 'Hydrodynamics', 'advisor', 'luigi.gentile@studenti.polito.it'),
('rosariogianni', '$2b$10$WJ9a3wFajFCcFGPZQZJ6Se0Of18.ZRIXIQyJ2vYtQe09iLykFVSO6', 'MATE', 'Electronics', 'advisor', 'rosario.gianni@studenti.polito.it'),
('alessandromorra', '$2b$10$Ethfq5u/0lOWGxD0or5.9Ozb5B85F51Fx6nHtQG5pjqdAgpaE5lz2', 'MATE', 'Mechanics', 'advisor', 'alessandro.morra@studenti.polito.it'),
('vincenzosegreto', '$2b$10$/XhaGNVOLuvKvnScR2tPA.jVmWL3F8KKQbKB2kWcJkv1imiJuAYqK', 'MATE', 'Hydrodynamics', 'advisor', 'rosario.gianni@studenti.polito.it'),
-- Board members
('lucreziacaviasso', '$2b$10$S3kdIUZE2l7FghlkmvzuD.CBaGwsix8VC0TPAuuwDi4kA671SGuQi', 'R&D', 'Electronics', 'board', 'lucrezia.caviasso@studenti.polito.it'),
('giorgiasforza', '$2b$10$5GcuDbINFkKqcpG/xyUFGOIYsyQPBN3grj4P9C5ip0BP5RCS2XiJi', 'R&D', 'Board', 'board', 'giorgia.sforza@studenti.polito.it'),
('samuelerizza', '$2b$10$XOrQcMIucDxVmjOHjx6oneGqfoxAYcy4CvqgdfUhXaGK/mWezV1Re', 'MATE', 'Board', 'board', 'samuele.rizza@studenti.polito.it'),
('andreaballarati', '$2b$10$ulpYjWKfZCetAccTVHaRLeDLpaUg3Zazcm8Bx2Iw/rjW6akH3Lj8C', 'MATE', 'Electronics', 'board', 'andrea.ballarati@studenti.polito.it');
