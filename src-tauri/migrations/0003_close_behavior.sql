-- Preserve the historic close-to-tray behavior for existing installations.
-- The setting remains in the portable SQLite database with the other preferences.
INSERT OR IGNORE INTO app_settings (key, value)
VALUES ('minimize_to_tray', 'true');
