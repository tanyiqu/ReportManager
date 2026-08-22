-- Report workspaces load an initial page and then append smaller pages while
-- the user scrolls. Keep both values in the portable database.
INSERT OR IGNORE INTO app_settings (key, value) VALUES ('default_report_load_count', '15');
INSERT OR IGNORE INTO app_settings (key, value) VALUES ('refresh_report_load_count', '15');

