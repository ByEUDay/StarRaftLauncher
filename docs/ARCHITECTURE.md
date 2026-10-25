# StarRaftLauncher 架构

## 进程与模块

```
┌──────────────────────── StarRaftLauncher（C# · Photino 原生窗口） ────────────────────────┐
│  BackendHost：启动 srbackend、生成令牌、握手        NativeBridge：文件对话框 / 打开目录与链接  │
│  ┌──────────────────────── 系统 WebView ────────────────────────┐                          │
│  │  FrontEnd（TypeScript + React）                               │◄── window.external ──►   │
│  └───────────────┬──────────────────────────────────────────────┘                          │
└──────────────────┼─────────────────────────────────────────────────────────────────────────┘
                   │ HTTP JSON + SSE（127.0.0.1，X-StarRaft-Token）
┌──────────────────▼──────────── srbackend（Rust · axum / tokio） ────────────────────────────┐
│  api ── services::game（安装/启动编排） ── plugins（mlua · Lua 5.4 沙箱）                      │
│   │            │            │             │                                               │
│   │      core_ffi（静态链接）  db（rusqlite）   sidecar（JSON-RPC over stdio）                    │
│   │            │            │             │                                               │
└───┼────────────┼────────────┼─────────────┼───────────────────────────────────────────────┘
    │            ▼            ▼             ├────────────► launcher-adapter.jar（Java）
    │     libstarraft_core   starraft.db    │               版本 JSON / 规则 / 依赖 / natives / 命令行
    │     （C++17）          （SQLite，     └────────────► account.jar（Kotlin）
    │                        data/*.sql）                   微软设备码登录 / 离线账户
    ▼
  srdl（Go）── HTTP ──► BMCLAPI / Mojang
```

| 模块 | 语言 | 目录 | 形态 | 与谁通信 |
| --- | --- | --- | --- | --- |
| FrontEnd | TypeScript + React | `frontend/` | Vite 静态产物 | BackEnd（HTTP/SSE）、宿主（web message） |
| GUI 宿主 | C# (.NET 8, Photino) | `desktop/` | 主程序 | 启动 BackEnd 子进程 |
| Core | C++17 | `core/` | 静态库，C ABI | 被 BackEnd 链接（FFI） |
| BackEnd | Rust | `backend/` | `srbackend` 子进程 | 全部模块的编排中心 |
| Download | Go | `download/` | `srdl` 子进程 | BackEnd（本地 HTTP） |
| Launcher 适配 | Java 17 | `launcher/` | `launcher-adapter.jar` 侧车 | BackEnd（JSON-RPC/stdio） |
| Data | SQL + SQLite | `data/` | 迁移 + 具名查询 | 编译期嵌入 BackEnd |
| Account | Kotlin | `account/` | `account.jar` 侧车 | BackEnd（JSON-RPC/stdio） |
| Plugins | Lua 5.4 | `plugins/` | `init.lua` 脚本 | 运行在 BackEnd 内（mlua） |

## 进程生命周期

1. **宿主**生成 24 字节随机令牌，以环境变量 `STARRAFT_TOKEN / STARRAFT_HOME / STARRAFT_DATA_DIR / STARRAFT_WATCH_STDIN=1` 启动 `bin/srbackend`。
2. **BackEnd** 打开 SQLite 并执行迁移，启动 `srdl` 并等待 `LISTEN <port>`，加载 Lua 插件，监听随机端口后在 stdout 打印 `LISTEN <port>`。
3. **宿主**读取端口，加载 `http://127.0.0.1:<port>/?token=…`（BackEnd 同时托管 `web/` 静态文件）。前端把令牌存入 sessionStorage 并从地址栏移除。
4. JVM 侧车在首次调用时懒启动，崩溃后下次调用自动重启。
5. 关闭窗口 → 宿主关闭 BackEnd 的 stdin → BackEnd 优雅退出，丢弃 `srdl` 与侧车的 stdin（它们都在 stdin EOF 时自行退出），超时则强杀。

所有子进程只监听 `127.0.0.1`，并分别使用独立的随机令牌。

## 协议

### FrontEnd ↔ BackEnd（HTTP）

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/api/system` | Core 提供的系统信息、Java 列表、内存 |
| GET | `/api/versions?snapshots=` | 版本清单（10 分钟缓存，镜像优先） |
| GET | `/api/loaders/:loader?mc=` | 加载器版本列表（fabric / quilt / forge / neoforge），含 stable / recommended 标记 |
| GET/POST | `/api/instances` | 实例列表（含游戏时长、运行状态）/ 新建 |
| GET/PUT/DELETE | `/api/instances/:id` | 读取 / 修改 / 删除（含游戏目录） |
| POST | `/api/instances/:id/install` | 后台安装，202 |
| POST | `/api/instances/:id/launch` | 后台启动（未安装则先安装），202 |
| POST | `/api/instances/:id/stop` | 结束游戏进程 |
| GET | `/api/accounts` | 账户列表（不含令牌） |
| POST | `/api/accounts/offline` | `{username}` |
| POST | `/api/accounts/microsoft` | 开始设备码登录，202 |
| POST | `/api/accounts/:id/default` · DELETE `/api/accounts/:id` | |
| GET · DELETE | `/api/downloads` · `/api/downloads/:id` | 下载批次 / 取消 |
| GET · PUT | `/api/plugins` · `/api/plugins/:id/enabled` | |
| GET · PUT | `/api/settings` | 键值设置（JSON 值） |
| GET | `/api/events?token=` | SSE，消息体 `{"kind": "...", "data": {...}}` |

事件 `kind`：`download.progress`、`install.stage`、`install.done`、`launch.stage`、`game.started`、`game.log`、`game.exit`、`task.error`、`account.deviceCode`、`account.added`、`account.error`、`account.refreshing`、`plugin.log`、`plugin.error`、`instance.changed`、`settings.changed`。

### BackEnd ↔ JVM 侧车（JSON-RPC over stdio）

每行一个 JSON 对象；stdout 只承载协议，日志写 stderr。侧车发送不带 `id` 的通知会被转发为前端事件。

**launcher-adapter.jar**

| 方法 | 参数 | 返回 |
| --- | --- | --- |
| `resolve` | `root, versionId` | `{versionId, mainClass, javaMajor, downloads[]}`（客户端 jar、库、natives、资源索引） |
| `resolveAssets` | `root, versionId` | `downloads[]`（资源对象） |
| `prepare` | `root, versionId, gameDir` | 解压 natives，展开旧版虚拟资源 |
| `installer.extract` | `root, installer` | 解析 Forge/NeoForge 安装器：写出版本配置、解出内嵌 maven/，返回 `{profileId, processors, downloads[]}` |
| `installer.run` | `root, installer, tag` | 依次执行客户端处理器（输出 SHA-1 已匹配则跳过），过程中发出 `loader.progress` 通知 |
| `buildCommand` | `root, versionId, gameDir, javaPath, memoryMb, jvmArgs[], gameArgs[], width, height, account{username,uuid,accessToken,userType}, launcherName, launcherVersion` | `{command[], workingDir, nativesDir}` |

支持：rules（os/arch/version/features）、新旧两种参数格式、`inheritsFrom` 合并（Fabric/Quilt 配置）、旧式与新式 natives、`virtual` / `map_to_resources` 旧版资源。

**account.jar**

| 方法 | 参数 | 说明 |
| --- | --- | --- |
| `offline.create` | `username` | UUID v3(`OfflinePlayer:<name>`) |
| `microsoft.login` | `clientId` | 设备码流程，发出 `account.deviceCode` 通知 |
| `microsoft.refresh` | `clientId, refreshToken` | 重走 XBL → XSTS → MC 链路 |

### BackEnd ↔ Core（C ABI）

见 `core/include/starraft/core.h`：`sr_os_info`、`sr_detect_java`、`sr_java_version`、`sr_sha1_file`、`sr_system_memory`、`sr_recommended_memory_mb`。返回的 `char*` 由 `sr_free` 释放。

### BackEnd ↔ Download（HTTP）

见 `download/README.md`。

## 安装与启动流程

```
launch(instance)
 ├─ 取账户；微软令牌 5 分钟内过期 → account.jar microsoft.refresh → 写回 SQLite
 ├─ 未安装 → install(instance)
 │    ├─ versions/<id>/<id>.json          ← 版本清单 + srdl
 │    ├─ launcher.resolve                 → srdl（客户端、库、natives、资源索引）
 │    ├─ launcher.resolveAssets           → srdl（资源对象）
 │    ├─ 模组加载器（instance.loader ≠ vanilla）
 │    │    ├─ Fabric / Quilt：meta 下载 profile JSON → versions/<profileId>/
 │    │    └─ Forge / NeoForge：srdl 下载安装器 → installer.extract → srdl 下载处理器依赖 → installer.run
 │    │    → instances.profile_id = profileId；launcher.resolve(profileId) 补齐加载器库
 │    └─ launcher.prepare                 （natives / 旧版资源）
 ├─ launcher.resolve → javaMajor → 选择 Java（实例 > 全局设置 > Core 自动探测）
 ├─ 内存：实例 > 全局 > Core 推荐值
 ├─ plugins.emit("launch.before", ctx)   ← Lua 可修改 javaPath/memoryMb/jvmArgs/gameArgs 或取消
 ├─ launcher.buildCommand → 启动 Java 进程，stdout/stderr → game.log 事件
 ├─ SQLite: play_sessions 开始记录
 └─ 进程退出 → play_sessions 结束、game.exit 事件、plugins.emit("launch.after")
```

## 目录布局

数据目录（`STARRAFT_DATA_DIR`，默认 macOS `~/Library/Application Support/StarRaftLauncher`、Windows `%APPDATA%\StarRaftLauncher`、Linux `~/.local/share/StarRaftLauncher`）：

```
starraft.db           SQLite
minecraft/            共享的 versions/ libraries/ assets/
instances/<id>/       每个实例独立的游戏目录（saves、mods、options.txt…）
plugins/<id>/init.lua 用户插件
logs/backend.log      宿主转存的后端日志
```
