# Download 下载引擎（Go）

`srdl` 是一个本地 HTTP 守护进程，由 BackEnd 启动并管理。

- 启动：`srdl --port 0`，环境变量 `STARRAFT_DL_TOKEN` 设定访问令牌
- 握手：stdout 输出 `LISTEN <port>`
- 退出：stdin 关闭时自动退出（`--watch-stdin=false` 可关闭此行为，便于单独调试）

## API

所有请求需带 `X-StarRaft-Token` 头。

```http
POST /v1/batches
{"label":"1.21 资源","mirror":"bmclapi","concurrency":16,
 "tasks":[{"url":"https://...","path":"/abs/path/file.jar","sha1":"...","size":123}]}
→ 201 {"id":"b1729850000-1"}

GET    /v1/batches/{id}  → {"id","label","state":"running|done|failed|cancelled","total","completed","failed","bytesDone","bytesTotal","errors":[]}
GET    /v1/batches       → [...]
DELETE /v1/batches/{id}  → 202（取消）
```

## 行为

- 工作池并发（默认 16，上限 64），按目标路径去重，只接受绝对路径
- 已存在且 SHA-1（或大小）匹配的文件直接跳过
- 先写 `.part` 临时文件，校验大小与 SHA-1 后原子重命名
- 每个源最多重试 3 次（指数退避）；4xx 直接切换下一个源
- `mirror=bmclapi` 时优先 BMCLAPI，失败回退 Mojang 官方源

```bash
go test ./...
```
