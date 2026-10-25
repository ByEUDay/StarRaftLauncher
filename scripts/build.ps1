# 构建全部 9 个模块并组装到 dist\（Windows PowerShell 7+）
$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Dist = Join-Path $Root 'dist'

function Step($msg) { Write-Host "`n==> $msg" -ForegroundColor Cyan }
function Need($cmd, $hint) {
    if (-not (Get-Command $cmd -ErrorAction SilentlyContinue)) { throw "缺少依赖: $cmd（$hint）" }
}
function Run($exe, [string[]]$argv) {
    & $exe @argv
    if ($LASTEXITCODE -ne 0) { throw "$exe 失败（退出码 $LASTEXITCODE）" }
}

Need node 'FrontEnd 需要 Node.js 18+'
Need go 'Download 需要 Go 1.22+'
Need cargo 'BackEnd 需要 Rust 1.75+'
Need cmake 'Core 需要 CMake 3.16+ 与 MSVC'
Need dotnet 'GUI 宿主需要 .NET 8 SDK'
Need java 'JVM 模块需要 JDK 17+'
$Gradle = if (Test-Path (Join-Path $Root 'gradlew.bat')) { Join-Path $Root 'gradlew.bat' } else { 'gradle' }
Need $Gradle 'JVM 模块需要 Gradle 8+'

if (Test-Path $Dist) { Remove-Item -Recurse -Force $Dist }
New-Item -ItemType Directory -Force -Path "$Dist\bin", "$Dist\lib", "$Dist\web", "$Dist\plugins" | Out-Null

Step '[1/6] FrontEnd · TypeScript + React'
Push-Location "$Root\frontend"
try {
    if (Test-Path package-lock.json) { Run npm @('ci') } else { Run npm @('install') }
    Run npm @('run', 'build')
} finally { Pop-Location }
Copy-Item -Recurse "$Root\frontend\dist\*" "$Dist\web\"

Step '[2/6] Download · Go'
Push-Location "$Root\download"
try {
    Run go @('test', './...')
    Run go @('build', '-trimpath', '-ldflags', '-s -w', '-o', "$Dist\bin\srdl.exe", './cmd/srdl')
} finally { Pop-Location }

Step '[3/6] Core · C++  +  BackEnd · Rust  +  Data · SQL'
Push-Location "$Root\backend"
try {
    Run cargo @('test', '--release')
    Run cargo @('build', '--release')
} finally { Pop-Location }
Copy-Item "$Root\backend\target\release\srbackend.exe" "$Dist\bin\"

Step '[4/6] Launcher · Java  +  Account · Kotlin'
Run $Gradle @('-p', $Root, '--console=plain', ':launcher:test', ':launcher:jar', ':account:test', ':account:jar')
Copy-Item "$Root\launcher\build\libs\launcher-adapter.jar", "$Root\account\build\libs\account.jar" "$Dist\lib\"

Step '[5/6] Plugins · Lua'
Copy-Item -Recurse "$Root\plugins\builtin\*" "$Dist\plugins\"

Step '[6/6] GUI 宿主 · C#'
Run dotnet @('publish', "$Root\desktop\StarRaft.Desktop.csproj", '-c', 'Release', '-o', $Dist, '--nologo')

Step "完成 → $Dist"
