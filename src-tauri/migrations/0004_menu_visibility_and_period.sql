-- Menu visibility and report cadence are preferences of each navigation item.
-- Defaults preserve the existing navigation and give legacy report menus a
-- useful cadence until the user changes it from Menu Management.
ALTER TABLE navigation_menus ADD COLUMN is_hidden INTEGER NOT NULL DEFAULT 0;
ALTER TABLE navigation_menus ADD COLUMN report_period TEXT NOT NULL DEFAULT 'daily'
  CHECK(report_period IN ('daily', 'weekly', 'monthly', 'quarterly', 'yearly', 'custom'));

UPDATE navigation_menus
SET report_period = CASE id
  WHEN 'weekly' THEN 'weekly'
  ELSE 'daily'
END
WHERE id IN ('daily', 'weekly', 'meeting');
