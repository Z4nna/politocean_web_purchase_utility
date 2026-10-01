-- The same part may be bought for two different proposals/projects in one order.
-- With the old key (order_id, manufacturer, manufacturer_pn) the second row was
-- merged into the first and its proposal/project silently dropped.
ALTER TABLE order_items DROP CONSTRAINT order_items_pkey;
ALTER TABLE order_items ADD PRIMARY KEY (order_id, manufacturer, manufacturer_pn, proposal, project);

-- Quantities must be positive. NOT VALID: enforced for new/updated rows only, so
-- the migration does not fail on historical rows that already hold a bad value.
ALTER TABLE order_items ADD CONSTRAINT order_items_quantity_positive CHECK (quantity > 0) NOT VALID;
