# StarRaftLauncher ✶

跨平台 Minecraft: Java Edition 桌面启动器，由 9 种语言协作实现。

| 模块 | 语言 | 目录 | 职责 |
| --- | --- | --- | --- |
| FrontEnd | TypeScript + React | [`frontend/`](frontend) | 界面：启动、实例、账户、下载、插件、设置 |
| GUI 宿主 | C# (.NET 8 + Photino) | [`desktop/`](desktop) | 原生窗口与 WebView、托管后端进程、原生对话框 |
| Core | C++17 | [`core/`](core) | Java 探测、SHA-1、内存信息、平台识别（C ABI） |
| BackEnd | Rust | [`backend/`](backend) | 业务引擎：HTTP/SSE API、安装与启动编排 |
| Download | Go | [`download/`](download) | 并发下载、镜像回退、校验 |
| Launcher 适配 | Java 17 | [`launcher/`](launcher) | 解析版本 JSON、依赖、natives、生成启动命令 |
| Data | SQL + SQLite | [`data/`](data) | 结构迁移与具名查询 |
| Account | Kotlin | [`account/`](account) | 微软设备码登录、离线账户 |
| Plugins | Lua 5.4 | [`plugins/`](plugins) | 沙箱插件与事件钩子 |

架构、进程模型与各模块间协议见 **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)**。

## 功能

- 版本清单浏览（正式版 / 快照），一键安装并启动
- 多实例：独立游戏目录、内存、Java、JVM/游戏参数，记录游戏时长
- 模组加载器：Fabric、Quilt、Forge、NeoForge 一键安装（含 Forge/NeoForge 安装器处理器，支持 1.12.2 至 26.x）
- 微软正版登录（设备码流程，自动刷新令牌）与离线账户
- BMCLAPI 镜像（失败自动回退官方源），并发下载 + SHA-1 校验 + 断点跳过
- 自动探测本机 Java 并按版本要求匹配（Java 8 / 17 / 21）
- 实时游戏日志、下载进度、插件日志（SSE 推送）
- Lua 插件：内置 G1GC 参数优化、内存保护、游戏会话报告

## 构建

依赖：Node.js 18+、Go 1.22+、Rust 1.75+、CMake 3.16+ 与 C++17 编译器、JDK 17+ 与 Gradle 8+、.NET 8 SDK。

```bash
./scripts/build.sh
```

Windows：

```powershell
./scripts/build.ps1
```

产物在 `dist/`：

```
dist/
├── StarRaftLauncher(.exe)     C# 宿主（及其依赖 DLL）
├── bin/srbackend              Rust 业务引擎（已静态链接 C++ Core 与 SQL）
├── bin/srdl                   Go 下载引擎
├── lib/launcher-adapter.jar   Java 启动适配层
├── lib/account.jar            Kotlin 账户模块
├── web/                       React 前端
└── plugins/                   内置 Lua 插件
```

## 开发

```bash
# 终端 1：构建侧车并以固定端口（17890）和令牌（dev-token）运行后端
./scripts/dev.sh

# 终端 2：前端热更新，浏览器访问 http://localhost:5173
cd frontend && npm install && npm run dev
```

用桌面窗口加载 Vite 开发服务器：

```bash
STARRAFT_DEV_URL=http://localhost:5173 \
STARRAFT_BACKEND="$PWD/backend/target/debug/srbackend" \
dotnet run --project desktop
```

单模块测试：

```bash
(cd download && go test ./...)
(cd backend && cargo test)
gradle :launcher:test :account:test
cmake -S core -B core/build -DSR_CORE_BUILD_TESTS=ON && cmake --build core/build && ctest --test-dir core/build
```
