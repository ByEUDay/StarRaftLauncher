-- StarRaftLauncher 初始数据库结构
-- 由 BackEnd (Rust) 在启动时通过 PRAGMA user_version 按顺序执行

CREATE TABLE IF NOT EXISTS settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL              -- JSON 编码的值
);

CREATE TABLE IF NOT EXISTS accounts (
    id            TEXT PRIMARY KEY,  -- "<kind>:<uuid>"
    kind          TEXT NOT NULL CHECK (kind IN ('microsoft', 'offline')),
    username      TEXT NOT NULL,
    uuid          TEXT NOT NULL,
    access_token  TEXT,
    refresh_token TEXT,
    expires_at    INTEGER,           -- Unix 秒
    skin_url      TEXT,
    is_default    INTEGER NOT NULL DEFAULT 0,
    created_at    INTEGER NOT NULL
);

-- 同一时间最多只有一个默认账户
CREATE UNIQUE INDEX IF NOT EXISTS idx_accounts_single_default
    ON accounts (is_default) WHERE is_default = 1;

CREATE TABLE IF NOT EXISTS instances (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    version_id  TEXT NOT NULL,
    loader      TEXT NOT NULL DEFAULT 'vanilla',
    java_path   TEXT,
    memory_mb   INTEGER,
    jvm_args    TEXT,
    game_args   TEXT,
    icon        TEXT,
    installed   INTEGER NOT NULL DEFAULT 0,
    last_played INTEGER,
    created_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS play_sessions (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    instance_id TEXT NOT NULL REFERENCES instances (id) ON DELETE CASCADE,
    account_id  TEXT,
    started_at  INTEGER NOT NULL,
    ended_at    INTEGER,
    exit_code   INTEGER
);

CREATE INDEX IF NOT EXISTS idx_play_sessions_instance ON play_sessions (instance_id);

CREATE TABLE IF NOT EXISTS plugins (
    id      TEXT PRIMARY KEY,
    enabled INTEGER NOT NULL DEFAULT 1,
    config  TEXT NOT NULL DEFAULT '{}'
);
