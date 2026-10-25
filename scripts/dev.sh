#!/usr/bin/env bash
# 开发模式：构建侧车后以固定端口/令牌运行 BackEnd，再另开终端运行前端：
#   cd frontend && npm run dev            # 浏览器打开 http://localhost:5173
# 或用桌面宿主加载 Vite：
#   STARRAFT_DEV_URL=http://localhost:5173 STARRAFT_BACKEND=... dotnet run --project desktop
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/.dev"
mkdir -p "$OUT"

GRADLE="gradle"
[ -x "$ROOT/gradlew" ] && GRADLE="$ROOT/gradlew"

echo "==> 构建 Go 下载引擎"
( cd "$ROOT/download" && go build -o "$OUT/srdl" ./cmd/srdl )

echo "==> 构建 Java / Kotlin 侧车"
"$GRADLE" -p "$ROOT" -q :launcher:jar :account:jar

export STARRAFT_PORT=17890
export STARRAFT_TOKEN=dev-token
export STARRAFT_DATA_DIR="$OUT/data"
export STARRAFT_DOWNLOAD_BIN="$OUT/srdl"
export STARRAFT_LAUNCHER_JAR="$ROOT/launcher/build/libs/launcher-adapter.jar"
export STARRAFT_ACCOUNT_JAR="$ROOT/account/build/libs/account.jar"
export STARRAFT_PLUGIN_DIR="$ROOT/plugins/builtin"
export RUST_LOG="${RUST_LOG:-info}"

echo "==> 启动 BackEnd: http://127.0.0.1:$STARRAFT_PORT  (token: $STARRAFT_TOKEN)"
cd "$ROOT/backend" && exec cargo run
