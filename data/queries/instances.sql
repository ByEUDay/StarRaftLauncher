-- name: instances.list
SELECT id, name, version_id, loader, loader_version, profile_id, java_path, memory_mb, jvm_args, game_args, icon,
       installed, last_played, created_at
FROM instances
ORDER BY COALESCE(last_played, created_at) DESC;

-- name: instances.get
SELECT id, name, version_id, loader, loader_version, profile_id, java_path, memory_mb, jvm_args, game_args, icon,
       installed, last_played, created_at
FROM instances
WHERE id = :id;

-- name: instances.insert
INSERT INTO instances (id, name, version_id, loader, loader_version, java_path, memory_mb, jvm_args, game_args, icon, created_at)
VALUES (:id, :name, :version_id, :loader, :loader_version, :java_path, :memory_mb, :jvm_args, :game_args, :icon, :created_at);

-- name: instances.update
UPDATE instances
SET name = :name, java_path = :java_path, memory_mb = :memory_mb,
    jvm_args = :jvm_args, game_args = :game_args, icon = :icon
WHERE id = :id;

-- name: instances.delete
DELETE FROM instances WHERE id = :id;

-- name: instances.mark_installed
UPDATE instances SET installed = 1 WHERE id = :id;

-- name: instances.set_profile
UPDATE instances SET profile_id = :profile_id WHERE id = :id;

-- name: instances.touch
UPDATE instances SET last_played = :ts WHERE id = :id;
