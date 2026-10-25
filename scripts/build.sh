#!/usr/bin/env bash
# 构建全部 9 个模块并组装到 dist/（macOS / Linux / Windows Git Bash）
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIST="$ROOT/dist"
EXE=""
case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) EXE=".exe" ;; esac

step() { printf '\n\033[1;36m==> %s\033[0m\n' "$*"; }
need() { command -v "$1" >/dev/null 2>&1 || { echo "缺少依赖: $1（$2）" >&2; exit 1; }; }

need node    "FrontEnd 需要 Node.js 18+"
need go      "Download 需要 Go 1.22+"
need cargo   "BackEnd 需要 Rust 1.75+"
need cmake   "Core 需要 CMake 3.16+ 与 C++17 编译器"
need dotnet  "GUI 宿主需要 .NET 8 SDK"
need java    "JVM 模块需要 JDK 17+"

GRADLE="gradle"
[ -x "$ROOT/gradlew" ] && GRADLE="$ROOT/gradlew"
need "$GRADLE" "JVM 模块需要 Gradle 8+（或运行 gradle wrapper 生成 gradlew）"

rm -rf "$DIST"
mkdir -p "$DIST/bin" "$DIST/lib" "$DIST/web" "$DIST/plugins"

step "[1/6] FrontEnd · TypeScript + React"
( cd "$ROOT/frontend" && { [ -f package-lock.json ] && npm ci || npm install; } && npm run build )
cp -R "$ROOT/frontend/dist/." "$DIST/web/"

step "[2/6] Download · Go"
( cd "$ROOT/download" && go test ./... && go build -trimpath -ldflags "-s -w" -o "$DIST/bin/srdl$EXE" ./cmd/srdl )

step "[3/6] Core · C++  +  BackEnd · Rust  +  Data · SQL（编译期嵌入）"
( cd "$ROOT/backend" && cargo test --release && cargo build --release )
cp "$ROOT/backend/target/release/srbackend$EXE" "$DIST/bin/"

step "[4/6] Launcher · Java  +  Account · Kotlin"
"$GRADLE" -p "$ROOT" --console=plain :launcher:test :launcher:jar :account:test :account:jar
cp "$ROOT/launcher/build/libs/launcher-adapter.jar" "$ROOT/account/build/libs/account.jar" "$DIST/lib/"

step "[5/6] Plugins · Lua"
cp -R "$ROOT/plugins/builtin/." "$DIST/plugins/"

step "[6/6] GUI 宿主 · C#"
dotnet publish "$ROOT/desktop/StarRaft.Desktop.csproj" -c Release -o "$DIST" --nologo

step "完成 → $DIST"
echo "运行: $DIST/StarRaftLauncher$EXE"
