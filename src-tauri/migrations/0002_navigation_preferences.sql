-- Navigation and preference data is stored with the portable SQLite database,
-- so a copied application directory retains the user's desktop layout.
CREATE TABLE IF NOT EXISTS app_settings (
  key TEXT PRIMARY KEY NOT NULL,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS navigation_menus (
  id TEXT PRIMARY KEY NOT NULL,
  label TEXT NOT NULL,
  icon_svg TEXT NOT NULL,
  sort_order INTEGER NOT NULL,
  is_system INTEGER NOT NULL DEFAULT 0 CHECK(is_system IN (0, 1))
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_navigation_menus_sort_order
  ON navigation_menus(sort_order);

-- Existing reports are assigned to their historic fixed navigation entries.
ALTER TABLE records ADD COLUMN menu_id TEXT NOT NULL DEFAULT 'daily';
CREATE INDEX IF NOT EXISTS idx_records_menu_id ON records(menu_id);
