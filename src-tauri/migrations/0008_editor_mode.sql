-- Store the Vditor display mode with the other portable editor preferences.
-- Existing installations retain the default WYSIWYG experience after upgrade.
INSERT OR IGNORE INTO app_settings (key, value) VALUES ('editor_mode', 'wysiwyg');
