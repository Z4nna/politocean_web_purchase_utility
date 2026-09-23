-- Make areas, projects and proposals manageable from the board area.
--
-- An `archived` flag lets the board hide an option from the "new order" and
-- "manage users" dropdowns without deleting the underlying row, so historical
-- orders that still reference it keep their referential integrity.
--
-- Renames are handled by the existing `ON UPDATE CASCADE` foreign keys, so no
-- foreign-key changes are needed here.

ALTER TABLE areas     ADD COLUMN IF NOT EXISTS archived BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE projects  ADD COLUMN IF NOT EXISTS archived BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE proposals ADD COLUMN IF NOT EXISTS archived BOOLEAN NOT NULL DEFAULT FALSE;
