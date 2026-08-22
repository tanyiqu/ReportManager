-- The action bar in Menu Management has its own user-configurable order.
-- Keep this preference in the portable SQLite database alongside the rest of
-- the interface settings; it must not affect navigation-menu ordering.
INSERT OR IGNORE INTO app_settings (key, value)
VALUES ('menu_action_order', '["visibility","period","rename","icon","delete"]');

-- Repair any visibility value written by an older client before the system
-- entries were protected from being hidden.
UPDATE navigation_menus
SET is_hidden = 0
WHERE id IN ('home', 'settings');
