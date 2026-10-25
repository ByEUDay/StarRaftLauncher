//! Data 层访问。SQL 全部来自仓库根目录 data/，编译期嵌入。

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use anyhow::Context;
use rusqlite::{named_params, Connection, OptionalExtension, Row};
use serde_json::{Map, Value};

use crate::models::{Account, Instance};

const MIGRATIONS: &[(i32, &str)] = &[
    (1, include_str!("../../data/migrations/001_init.sql")),
    (2, include_str!("../../data/migrations/002_default_settings.sql")),
    (3, include_str!("../../data/migrations/003_mod_loaders.sql")),
];

const QUERY_FILES: &[&str] = &[
    include_str!("../../data/queries/instances.sql"),
    include_str!("../../data/queries/accounts.sql"),
    include_str!("../../data/queries/sessions.sql"),
    include_str!("../../data/queries/settings.sql"),
];

pub struct Db {
    conn: Mutex<Connection>,
    queries: HashMap<String, String>,
}

/// 解析 `-- name: xxx` 分段的具名查询
fn parse_queries() -> HashMap<String, String> {
    let mut map = HashMap::new();
    for file in QUERY_FILES {
        let mut current: Option<String> = None;
        let mut buf = String::new();
        for line in file.lines() {
            if let Some(name) = line.trim().strip_prefix("-- name:") {
                if let Some(prev) = current.take() {
                    map.insert(prev, buf.trim().to_string());
                }
                current = Some(name.trim().to_string());
                buf.clear();
            } else if current.is_some() {
                buf.push_str(line);
                buf.push('\n');
            }
        }
        if let Some(prev) = current {
            map.insert(prev, buf.trim().to_string());
        }
    }
    map
}

impl Db {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let conn = Connection::open(path).with_context(|| format!("打开数据库失败: {}", path.display()))?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;")?;
        let db = Self { conn: Mutex::new(conn), queries: parse_queries() };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> anyhow::Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let current: i32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        for (version, sql) in MIGRATIONS.iter().filter(|(v, _)| *v > current) {
            let tx = conn.transaction()?;
            tx.execute_batch(sql).with_context(|| format!("迁移 {version} 失败"))?;
            tx.execute_batch(&format!("PRAGMA user_version = {version}"))?;
            tx.commit()?;
            tracing::info!("applied migration {version}");
        }
        Ok(())
    }

    fn q(&self, name: &str) -> &str {
        self.queries.get(name).unwrap_or_else(|| panic!("missing query: {name}"))
    }

    // ---------- instances ----------

    pub fn list_instances(&self) -> anyhow::Result<Vec<Instance>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(self.q("instances.list"))?;
        let rows = stmt.query_map([], row_instance)?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_instance(&self, id: &str) -> anyhow::Result<Option<Instance>> {
        let conn = self.conn.lock().unwrap();
        Ok(conn.query_row(self.q("instances.get"), named_params! { ":id": id }, row_instance).optional()?)
    }

    pub fn insert_instance(&self, i: &Instance) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            self.q("instances.insert"),
            named_params! {
                ":id": i.id, ":name": i.name, ":version_id": i.version_id, ":loader": i.loader,
                ":loader_version": i.loader_version, ":java_path": i.java_path, ":memory_mb": i.memory_mb, ":jvm_args": i.jvm_args,
                ":game_args": i.game_args, ":icon": i.icon, ":created_at": i.created_at,
            },
        )?;
        Ok(())
    }

    pub fn update_instance(&self, i: &Instance) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            self.q("instances.update"),
            named_params! {
                ":id": i.id, ":name": i.name, ":java_path": i.java_path, ":memory_mb": i.memory_mb,
                ":jvm_args": i.jvm_args, ":game_args": i.game_args, ":icon": i.icon,
            },
        )?;
        Ok(())
    }

    pub fn delete_instance(&self, id: &str) -> anyhow::Result<bool> {
        let conn = self.conn.lock().unwrap();
        Ok(conn.execute(self.q("instances.delete"), named_params! { ":id": id })? > 0)
    }

    pub fn mark_installed(&self, id: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(self.q("instances.mark_installed"), named_params! { ":id": id })?;
        Ok(())
    }

    pub fn set_profile(&self, id: &str, profile_id: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(self.q("instances.set_profile"), named_params! { ":id": id, ":profile_id": profile_id })?;
        Ok(())
    }

    pub fn touch_instance(&self, id: &str, ts: i64) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(self.q("instances.touch"), named_params! { ":id": id, ":ts": ts })?;
        Ok(())
    }

    // ---------- accounts ----------

    pub fn list_accounts(&self) -> anyhow::Result<Vec<Account>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(self.q("accounts.list"))?;
        let rows = stmt.query_map([], row_account)?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn get_account(&self, id: &str) -> anyhow::Result<Option<Account>> {
        let conn = self.conn.lock().unwrap();
        Ok(conn.query_row(self.q("accounts.get"), named_params! { ":id": id }, row_account).optional()?)
    }

    pub fn default_account(&self) -> anyhow::Result<Option<Account>> {
        let conn = self.conn.lock().unwrap();
        Ok(conn.query_row(self.q("accounts.get_default"), [], row_account).optional()?)
    }

    pub fn upsert_account(&self, a: &Account) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            self.q("accounts.upsert"),
            named_params! {
                ":id": a.id, ":kind": a.kind, ":username": a.username, ":uuid": a.uuid,
                ":access_token": a.access_token, ":refresh_token": a.refresh_token,
                ":expires_at": a.expires_at, ":skin_url": a.skin_url, ":created_at": a.created_at,
            },
        )?;
        Ok(())
    }

    pub fn set_default_account(&self, id: &str) -> anyhow::Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute(self.q("accounts.clear_default"), [])?;
        tx.execute(self.q("accounts.set_default"), named_params! { ":id": id })?;
        tx.commit()?;
        Ok(())
    }

    pub fn delete_account(&self, id: &str) -> anyhow::Result<bool> {
        let conn = self.conn.lock().unwrap();
        Ok(conn.execute(self.q("accounts.delete"), named_params! { ":id": id })? > 0)
    }

    // ---------- play sessions ----------

    pub fn start_session(&self, instance_id: &str, account_id: &str, started_at: i64) -> anyhow::Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            self.q("sessions.start"),
            named_params! { ":instance_id": instance_id, ":account_id": account_id, ":started_at": started_at },
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn end_session(&self, id: i64, ended_at: i64, exit_code: Option<i32>) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(self.q("sessions.end"), named_params! { ":id": id, ":ended_at": ended_at, ":exit_code": exit_code })?;
        Ok(())
    }

    /// instance_id -> (累计秒数, 启动次数)
    pub fn playtime(&self) -> anyhow::Result<HashMap<String, (i64, i64)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(self.q("sessions.playtime"))?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, (r.get::<_, Option<i64>>(1)?.unwrap_or(0), r.get(2)?))))?
            .collect::<Result<HashMap<_, _>, _>>()?;
        Ok(rows)
    }

    // ---------- settings ----------

    pub fn settings(&self) -> anyhow::Result<Map<String, Value>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(self.q("settings.all"))?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        let mut map = Map::new();
        for row in rows {
            let (k, v) = row?;
            map.insert(k, serde_json::from_str(&v).unwrap_or(Value::String(v)));
        }
        Ok(map)
    }

    pub fn setting(&self, key: &str) -> anyhow::Result<Option<Value>> {
        let conn = self.conn.lock().unwrap();
        let raw: Option<String> =
            conn.query_row(self.q("settings.get"), named_params! { ":key": key }, |r| r.get(0)).optional()?;
        Ok(raw.map(|v| serde_json::from_str(&v).unwrap_or(Value::String(v))))
    }

    pub fn set_setting(&self, key: &str, value: &Value) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(self.q("settings.set"), named_params! { ":key": key, ":value": value.to_string() })?;
        Ok(())
    }

    // ---------- plugins ----------

    /// id -> (enabled, config)
    pub fn plugin_states(&self) -> anyhow::Result<HashMap<String, (bool, Value)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare_cached(self.q("plugins.states"))?;
        let rows = stmt
            .query_map([], |r| {
                let config: String = r.get(2)?;
                Ok((r.get::<_, String>(0)?, (r.get::<_, i64>(1)? != 0, serde_json::from_str(&config).unwrap_or(Value::Null))))
            })?
            .collect::<Result<HashMap<_, _>, _>>()?;
        Ok(rows)
    }

    pub fn set_plugin_enabled(&self, id: &str, enabled: bool) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(self.q("plugins.set_enabled"), named_params! { ":id": id, ":enabled": enabled as i64 })?;
        Ok(())
    }
}

fn row_instance(r: &Row) -> rusqlite::Result<Instance> {
    Ok(Instance {
        id: r.get("id")?,
        name: r.get("name")?,
        version_id: r.get("version_id")?,
        loader: r.get("loader")?,
        loader_version: r.get("loader_version")?,
        profile_id: r.get("profile_id")?,
        java_path: r.get("java_path")?,
        memory_mb: r.get("memory_mb")?,
        jvm_args: r.get("jvm_args")?,
        game_args: r.get("game_args")?,
        icon: r.get("icon")?,
        installed: r.get::<_, i64>("installed")? != 0,
        last_played: r.get("last_played")?,
        created_at: r.get("created_at")?,
    })
}

fn row_account(r: &Row) -> rusqlite::Result<Account> {
    Ok(Account {
        id: r.get("id")?,
        kind: r.get("kind")?,
        username: r.get("username")?,
        uuid: r.get("uuid")?,
        access_token: r.get("access_token")?,
        refresh_token: r.get("refresh_token")?,
        expires_at: r.get("expires_at")?,
        skin_url: r.get("skin_url")?,
        is_default: r.get::<_, i64>("is_default")? != 0,
        created_at: r.get("created_at")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_queries_parse_and_prepare() {
        let dir = std::env::temp_dir().join(format!("sr-test-{}", crate::util::random_hex(4)));
        std::fs::create_dir_all(&dir).unwrap();
        let db = Db::open(&dir.join("t.db")).unwrap();
        let conn = db.conn.lock().unwrap();
        for (name, sql) in &db.queries {
            conn.prepare(sql).unwrap_or_else(|e| panic!("{name}: {e}"));
        }
        assert!(db.queries.len() >= 20);
    }
}
