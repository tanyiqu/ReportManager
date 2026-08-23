-- Keep the report-editor font scale in the portable SQLite preferences so it
-- is restored after the application is restarted.
INSERT OR IGNORE INTO app_settings (key, value) VALUES ('editor_font_scale', '1');
