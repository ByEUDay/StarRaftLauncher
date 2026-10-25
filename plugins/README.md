# Plugins 插件系统（Lua 5.4）

插件由 BackEnd 通过 [mlua](https://github.com/mlua-rs/mlua) 加载，每个插件运行在**独立且受限**的 Lua 状态中：
只开放 `table` / `string` / `math` / `utf8` 标准库，没有 `io`、`os`、`require`、`load` 文件访问。

## 目录

| 位置 | 说明 |
| --- | --- |
| `plugins/builtin/<id>/init.lua` | 随启动器发布的内置插件（打包到 `dist/plugins/`） |
| `<数据目录>/plugins/<id>/init.lua` | 用户插件 |
| `plugins/types/starraft.lua` | 给 lua-language-server 用的 API 类型注解 |

## 插件结构

```lua
return {
  id = "my-plugin",            -- 必填，全局唯一
  name = "我的插件",
  version = "1.0.0",
  author = "you",
  description = "做点什么",

  setup = function(sr)
    sr.on("launch.before", function(ctx)
      sr.log("即将启动 " .. ctx.versionId)
      table.insert(ctx.jvmArgs, "-Dfoo=bar")
    end)
  end,
}
```

## API（`sr` / 全局 `starraft`）

| 成员 | 说明 |
| --- | --- |
| `sr.on(event, fn)` | 注册事件处理器。`fn(ctx)` 可原地修改 `ctx`，或返回新表替换它 |
| `sr.log(msg)` | 写日志，并推送到前端「插件」页 |
| `sr.has(event)` | 是否注册过某事件 |
| `sr.version` | 启动器版本 |
| `sr.config` | 该插件在数据库 `plugins.config` 中的配置（JSON → table） |

## 事件

| 事件 | ctx 字段 | 可修改 |
| --- | --- | --- |
| `app.ready` | `version` | — |
| `launch.before` | `instance`, `versionId`, `javaMajor`, `account{username,uuid,kind}`, `javaPath`, `memoryMb`, `jvmArgs[]`, `gameArgs[]` | `javaPath` `memoryMb` `jvmArgs` `gameArgs`，设置 `cancel = true` + `cancelReason` 可阻止启动 |
| `launch.after` | `instanceId`, `exitCode`, `durationSecs` | — |

处理器抛出的错误会被隔离：记录日志、推送 `plugin.error` 事件，`ctx` 保持调用前的值，其它插件继续执行。
