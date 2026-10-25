-- name: accounts.list
SELECT id, kind, username, uuid, access_token, refresh_token, expires_at, skin_url, is_default, created_at
FROM accounts
ORDER BY is_default DESC, created_at ASC;

-- name: accounts.get
SELECT id, kind, username, uuid, access_token, refresh_token, expires_at, skin_url, is_default, created_at
FROM accounts
WHERE id = :id;

-- name: accounts.get_default
SELECT id, kind, username, uuid, access_token, refresh_token, expires_at, skin_url, is_default, created_at
FROM accounts
ORDER BY is_default DESC, created_at ASC
LIMIT 1;

-- name: accounts.upsert
INSERT INTO accounts (id, kind, username, uuid, access_token, refresh_token, expires_at, skin_url, is_default, created_at)
VALUES (:id, :kind, :username, :uuid, :access_token, :refresh_token, :expires_at, :skin_url, 0, :created_at)
ON CONFLICT (id) DO UPDATE SET
    username      = excluded.username,
    access_token  = excluded.access_token,
    refresh_token = excluded.refresh_token,
    expires_at    = excluded.expires_at,
    skin_url      = excluded.skin_url;

-- name: accounts.clear_default
UPDATE accounts SET is_default = 0 WHERE is_default = 1;

-- name: accounts.set_default
UPDATE accounts SET is_default = 1 WHERE id = :id;

-- name: accounts.delete
DELETE FROM accounts WHERE id = :id;
