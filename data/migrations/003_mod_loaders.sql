-- 模组加载器支持：instances.loader 取值 vanilla | fabric | quilt | forge | neoforge
-- loader_version：加载器版本（Forge 为完整构件版本，如 1.20.1-47.4.28）
-- profile_id：安装后生成的版本配置 ID（versions/<profile_id>/<profile_id>.json），启动时使用
ALTER TABLE instances ADD COLUMN loader_version TEXT;
ALTER TABLE instances ADD COLUMN profile_id TEXT;
