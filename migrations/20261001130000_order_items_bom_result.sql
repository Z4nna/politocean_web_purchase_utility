-- Result of the last BOM generation for each item, shown on the "View BOM" page.
-- The chosen supplier is the one whose part number (mouser_pn / digikey_pn) is set;
-- when neither is, `bom_note` says why the item could not be sourced.
ALTER TABLE order_items ADD COLUMN IF NOT EXISTS unit_price DOUBLE PRECISION;
ALTER TABLE order_items ADD COLUMN IF NOT EXISTS bom_note TEXT;
