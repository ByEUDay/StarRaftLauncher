//! Lua 插件宿主（mlua / Lua 5.4）
//!
//! 每个插件是一个目录，入口 `init.lua` 返回清单表：
//!
//! ```lua
//! return {
//!   id = "hello", name = "Hello", version = "1.0.0",
//!   setup = function(sr) sr.on("launch.before", function(ctx) ... end) end,
//! }
//! ```
//!
//! 每个插件运行在独立的 Lua 状态中，仅开放 table/string/math/utf8 标准库（无 io/os/load 文件访问）。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use mlua::{Function, Lua, LuaOptions, LuaSerdeExt, StdLib, Table};
use serde::Serialize;
use serde_json::{json, Value};

use crate::db::Db;
use crate::events::EventBus;

const PRELUDE: &str = r#"
return function(sr)
  local handlers = {}

  function sr.on(event, fn)
    assert(type(event) == "string", "event must be a string")
    assert(type(fn) == "function", "handler must be a function")
    handlers[event] = handlers[event] or {}
    table.insert(handlers[event], fn)
  end

  -- 由宿主调用：依次执行处理器；处理器可原地修改 ctx，或返回一个新表替换它
  function sr.__emit(event, ctx)
    for _, fn in ipairs(handlers[event] or {}) do
      local result = fn(ctx)
      if type(result) == "table" then ctx = result end
    end
    return ctx
  end

  function sr.has(event) return handlers[event] ~= nil end
end
"#;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub path: String,
    pub enabled: bool,
    pub error: Option<String>,
}

struct Plugin {
    info: PluginInfo,
    enabled: AtomicBool,
    lua: Option<Mutex<Lua>>,
}

pub struct PluginHost {
    plugins: Vec<Plugin>,
    events: EventBus,
}

impl PluginHost {
    pub fn load(dirs: &[PathBuf], db: &Db, events: EventBus) -> Self {
        let states = db.plugin_states().unwrap_or_default();
        let mut plugins: Vec<Plugin> = Vec::new();

        for dir in dirs {
            let Ok(entries) = std::fs::read_dir(dir) else { continue };
            let mut paths: Vec<PathBuf> =
                entries.flatten().map(|e| e.path()).filter(|p| p.join("init.lua").is_file()).collect();
            paths.sort();

            for path in paths {
                match load_one(&path, &states, &events) {
                    Ok((lua, info)) => {
                        if plugins.iter().any(|p| p.info.id == info.id) {
                            tracing::warn!("duplicate plugin id {} at {}, skipped", info.id, path.display());
                            continue;
                        }
                        tracing::info!("loaded plugin {} v{} (enabled: {})", info.id, info.version, info.enabled);
                        plugins.push(Plugin { enabled: AtomicBool::new(info.enabled), info, lua: Some(Mutex::new(lua)) });
                    }
                    Err(e) => {
                        let id = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                        tracing::warn!("failed to load plugin {}: {e:#}", path.display());
                        plugins.push(Plugin {
                            info: PluginInfo {
                                name: id.clone(),
                                id,
                                version: "-".into(),
                                description: String::new(),
                                author: String::new(),
                                path: path.to_string_lossy().into_owned(),
                                enabled: false,
                                error: Some(format!("{e:#}")),
                            },
                            enabled: AtomicBool::new(false),
                            lua: None,
                        });
                    }
                }
            }
        }
        Self { plugins, events }
    }

    pub fn list(&self) -> Vec<PluginInfo> {
        self.plugins
            .iter()
            .map(|p| PluginInfo { enabled: p.enabled.load(Ordering::Relaxed), ..p.info.clone() })
            .collect()
    }

    pub fn set_enabled(&self, id: &str, enabled: bool) -> bool {
        match self.plugins.iter().find(|p| p.info.id == id && p.lua.is_some()) {
            Some(p) => {
                p.enabled.store(enabled, Ordering::Relaxed);
                true
            }
            None => false,
        }
    }

    /// 广播事件给所有启用的插件，返回（可能被修改过的）上下文
    pub fn emit(&self, event: &str, mut ctx: Value) -> Value {
        for p in &self.plugins {
            if !p.enabled.load(Ordering::Relaxed) {
                continue;
            }
            let Some(lua) = &p.lua else { continue };
            let lua = lua.lock().unwrap();
            let result = (|| -> mlua::Result<Value> {
                let api: Table = lua.globals().get("starraft")?;
                let emit: Function = api.get("__emit")?;
                let out: mlua::Value = emit.call((event, lua.to_value(&ctx)?))?;
                lua.from_value(out)
            })();
            match result {
                Ok(v) => ctx = v,
                Err(e) => {
                    tracing::warn!("plugin {} failed on {event}: {e}", p.info.id);
                    self.events.emit("plugin.error", json!({ "plugin": p.info.id, "event": event, "message": e.to_string() }));
                }
            }
        }
        ctx
    }
}

fn load_one(path: &Path, states: &HashMap<String, (bool, Value)>, events: &EventBus) -> anyhow::Result<(Lua, PluginInfo)> {
    let lua = Lua::new_with(StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::UTF8, LuaOptions::default())?;
    let entry = path.join("init.lua");
    let source = std::fs::read_to_string(&entry)?;

    let info = {
        let module: Table = lua.load(source.as_str()).set_name(format!("@{}", entry.display())).eval()?;
        let id: String = module.get("id").map_err(|_| anyhow::anyhow!("init.lua 必须返回包含 id 字段的表"))?;
        let (enabled, mut config) = states.get(&id).cloned().unwrap_or((true, json!({})));
        if !config.is_object() {
            config = json!({});
        }

        let api = lua.create_table()?;
        let prelude: Function = lua.load(PRELUDE).set_name("=prelude").eval()?;
        prelude.call::<_, ()>(api.clone())?;

        let log_id = id.clone();
        let log_events = events.clone();
        api.set(
            "log",
            lua.create_function(move |_, msg: String| {
                tracing::info!(target: "plugin", "[{log_id}] {msg}");
                log_events.emit("plugin.log", json!({ "plugin": log_id, "message": msg }));
                Ok(())
            })?,
        )?;
        api.set("version", env!("CARGO_PKG_VERSION"))?;
        api.set("config", lua.to_value(&config)?)?;
        lua.globals().set("starraft", api.clone())?;

        if let Some(setup) = module.get::<_, Option<Function>>("setup")? {
            setup.call::<_, ()>(api)?;
        }

        PluginInfo {
            name: module.get::<_, Option<String>>("name")?.unwrap_or_else(|| id.clone()),
            version: module.get::<_, Option<String>>("version")?.unwrap_or_else(|| "0.0.0".into()),
            description: module.get::<_, Option<String>>("description")?.unwrap_or_default(),
            author: module.get::<_, Option<String>>("author")?.unwrap_or_default(),
            path: path.to_string_lossy().into_owned(),
            enabled,
            error: None,
            id,
        }
    };
    Ok((lua, info))
}
