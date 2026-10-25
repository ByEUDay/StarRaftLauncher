-- 根据分配内存与 Java 版本追加经过验证的 G1GC 参数（参考 Aikar's flags）

local function has_flag(args, prefix)
  for _, a in ipairs(args) do
    if a:sub(1, #prefix) == prefix then return true end
  end
  return false
end

local function append(args, list)
  for _, a in ipairs(list) do table.insert(args, a) end
end

---@type StarRaftPlugin
return {
  id = "jvm-optimizer",
  name = "JVM 参数优化",
  version = "1.0.0",
  author = "StarRaft",
  description = "为未手动指定 GC 的实例自动追加 G1GC 调优参数，并设置合理的初始堆大小。",

  setup = function(sr)
    sr.on("launch.before", function(ctx)
      local args = ctx.jvmArgs
      -- 用户已自定义 GC 时不干预
      if has_flag(args, "-XX:+Use") then
        sr.log("检测到自定义 GC 参数，跳过优化")
        return
      end

      local mem = ctx.memoryMb or 2048
      local large = mem >= 12288

      if not has_flag(args, "-Xms") then
        table.insert(args, "-Xms" .. math.min(mem, 1024) .. "m")
      end

      append(args, {
        "-XX:+UseG1GC",
        "-XX:+ParallelRefProcEnabled",
        "-XX:MaxGCPauseMillis=200",
        "-XX:+UnlockExperimentalVMOptions",
        "-XX:+DisableExplicitGC",
        "-XX:+AlwaysPreTouch",
        "-XX:G1NewSizePercent=" .. (large and 40 or 30),
        "-XX:G1MaxNewSizePercent=" .. (large and 50 or 40),
        "-XX:G1HeapRegionSize=" .. (large and "16M" or "8M"),
        "-XX:G1ReservePercent=" .. (large and 15 or 20),
        "-XX:G1HeapWastePercent=5",
        "-XX:G1MixedGCCountTarget=4",
        "-XX:InitiatingHeapOccupancyPercent=" .. (large and 20 or 15),
        "-XX:G1MixedGCLiveThresholdPercent=90",
        "-XX:G1RSetUpdatingPauseTimePercent=5",
        "-XX:SurvivorRatio=32",
        "-XX:+PerfDisableSharedMem",
        "-XX:MaxTenuringThreshold=1",
      })
      sr.log(string.format("已为 %s 追加 G1GC 参数（%d MB）", ctx.versionId, mem))
    end)
  end,
}
