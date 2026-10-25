---@meta
-- StarRaftLauncher 插件 API 类型注解（供 lua-language-server 使用，运行时不加载）

---@class StarRaftAccount
---@field username string
---@field uuid string
---@field kind "microsoft"|"offline"

---@class StarRaftLaunchContext
---@field instance table          实例信息（id, name, versionId, memoryMb, ...）
---@field versionId string
---@field javaMajor integer       版本要求的 Java 主版本
---@field account StarRaftAccount
---@field javaPath string
---@field memoryMb integer
---@field jvmArgs string[]
---@field gameArgs string[]
---@field cancel boolean?
---@field cancelReason string?

---@class StarRaftExitContext
---@field instanceId string
---@field exitCode integer?
---@field durationSecs integer

---@class StarRaftApi
---@field version string
---@field config table
starraft = {}

---@overload fun(event: "launch.before", fn: fun(ctx: StarRaftLaunchContext): StarRaftLaunchContext?)
---@overload fun(event: "launch.after", fn: fun(ctx: StarRaftExitContext))
---@overload fun(event: "app.ready", fn: fun(ctx: {version: string}))
---@param event string
---@param fn fun(ctx: table): table?
function starraft.on(event, fn) end

---@param message string
function starraft.log(message) end

---@param event string
---@return boolean
function starraft.has(event) end

---@class StarRaftPlugin
---@field id string
---@field name string?
---@field version string?
---@field author string?
---@field description string?
---@field setup fun(sr: StarRaftApi)?
