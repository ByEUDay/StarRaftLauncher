-- name: sessions.start
INSERT INTO play_sessions (instance_id, account_id, started_at)
VALUES (:instance_id, :account_id, :started_at);

-- name: sessions.end
UPDATE play_sessions SET ended_at = :ended_at, exit_code = :exit_code WHERE id = :id;

-- name: sessions.playtime
-- 每个实例的累计游戏时长（秒）
SELECT instance_id, SUM(COALESCE(ended_at, started_at) - started_at) AS seconds, COUNT(*) AS launches
FROM play_sessions
GROUP BY instance_id;
