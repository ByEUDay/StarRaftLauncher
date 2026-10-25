-- name: settings.all
SELECT key, value FROM settings;

-- name: settings.get
SELECT value FROM settings WHERE key = :key;

-- name: settings.set
INSERT INTO settings (key, value) VALUES (:key, :value)
ON CONFLICT (key) DO UPDATE SET value = excluded.value;

-- name: plugins.states
SELECT id, enabled, config FROM plugins;

-- name: plugins.set_enabled
INSERT INTO plugins (id, enabled) VALUES (:id, :enabled)
ON CONFLICT (id) DO UPDATE SET enabled = excluded.enabled;
