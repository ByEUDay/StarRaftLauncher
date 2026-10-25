-- 内存保护：分配内存过低时自动提升到该版本可运行的下限，并提示过高的分配

local MIN_MB = 1024

---@type StarRaftPlugin
return {
  id = "java-guard",
  name = "内存保护",
  version = "1.0.0",
  author = "StarRaft",
  description = "防止因内存分配过低导致游戏无法启动；可在配置中设置 minMemoryMb。",

  setup = function(sr)
    local min_mb = type(sr.config) == "table" and tonumber(sr.config.minMemoryMb) or MIN_MB

    sr.on("launch.before", function(ctx)
      if ctx.memoryMb < min_mb then
        sr.log(string.format("内存 %d MB 过低，已提升至 %d MB", ctx.memoryMb, min_mb))
        ctx.memoryMb = min_mb
      elseif ctx.memoryMb > 16384 then
        sr.log("分配超过 16 GB 通常不会带来收益，反而可能增加 GC 停顿")
      end
    end)
  end,
}
