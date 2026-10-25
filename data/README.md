# Data 数据层（SQL + SQLite）

| 目录 | 说明 |
| --- | --- |
| `migrations/` | 按编号顺序执行的结构迁移。BackEnd 用 `PRAGMA user_version` 记录已执行到的版本。 |
| `queries/` | 具名查询。每段以 `-- name: <模块>.<动作>` 开头，BackEnd 编译期 `include_str!` 嵌入并按名称调用。 |

约定：

- 所有时间戳为 Unix 秒（`INTEGER`）。
- `settings.value` 和 `plugins.config` 存 JSON 文本。
- 参数统一使用 `:name` 具名占位符。
- 新增迁移：添加 `00N_xxx.sql`，并在 `backend/src/db.rs` 的 `MIGRATIONS` 中登记。

> 注意：当前账户令牌以明文存于 `accounts` 表。正式发布前应改为写入系统钥匙串（macOS Keychain / Windows Credential Manager / libsecret），表中只保留引用。
