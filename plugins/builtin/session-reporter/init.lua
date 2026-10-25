-- 游戏退出后汇报本次游戏时长，并对异常退出给出排查提示

local HINTS = {
  [1] = "游戏异常退出，请查看日志中的 Exception 信息",
  [-1] = "游戏进程被强制终止",
  [137] = "进程被系统终止（可能内存不足）",
  [-805306369] = "Java 虚拟机崩溃（0xCFFFFFFF），尝试更换 Java 版本或显卡驱动",
}

local function format_duration(secs)
  local h = secs // 3600
  local m = (secs % 3600) // 60
  local s = secs % 60
  if h > 0 then return string.format("%d 小时 %d 分", h, m) end
  if m > 0 then return string.format("%d 分 %d 秒", m, s) end
  return string.format("%d 秒", s)
end

---@type StarRaftPlugin
return {
  id = "session-reporter",
  name = "游戏会话报告",
  version = "1.0.0",
  author = "StarRaft",
  description = "游戏结束后报告游戏时长，并为常见的异常退出码提供排查建议。",

  setup = function(sr)
    sr.on("launch.after", function(ctx)
      sr.log("本次游戏时长：" .. format_duration(ctx.durationSecs or 0))
      -- JSON null 在 Lua 中是 light userdata 而非 nil，因此用 type 判断
      local code = ctx.exitCode
      if type(code) == "number" and code ~= 0 then
        sr.log(string.format("退出码 %d：%s", code, HINTS[code] or "未知原因，请查看游戏日志"))
      end
    end)
  end,
}
