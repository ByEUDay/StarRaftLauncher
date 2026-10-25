package dev.starraft.launcher;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.Map;

/** 资源索引（assets/indexes/*.json）处理。 */
final class Assets {
    private Assets() {}

    static final String RESOURCES = "https://resources.download.minecraft.net/";

    static String indexId(JsonObject version) {
        JsonObject idx = Params.obj(version, "assetIndex");
        if (idx != null) return idx.get("id").getAsString();
        return Params.str(version, "assets", "legacy");
    }

    static JsonObject readIndex(Path root, JsonObject version) throws IOException {
        Path file = root.resolve("assets").resolve("indexes").resolve(indexId(version) + ".json");
        return JsonParser.parseString(Files.readString(file, StandardCharsets.UTF_8)).getAsJsonObject();
    }

    static JsonArray resolve(Path root, String versionId) throws IOException {
        JsonObject version = VersionLoader.load(root, versionId);
        JsonObject index = readIndex(root, version);
        Path objects = root.resolve("assets").resolve("objects");
        JsonArray tasks = new JsonArray();
        for (Map.Entry<String, JsonElement> e : Params.obj(index, "objects").entrySet()) {
            JsonObject o = e.getValue().getAsJsonObject();
            String hash = o.get("hash").getAsString();
            String sub = hash.substring(0, 2) + "/" + hash;
            tasks.add(Params.task(RESOURCES + sub, objects.resolve(sub), hash, Params.number(o, "size", 0)));
        }
        return tasks;
    }

    /** ${game_assets} 的取值：旧版本需要按文件名展开的目录 */
    static Path gameAssetsDir(Path root, Path gameDir, JsonObject version, JsonObject index) {
        if (index.has("map_to_resources") && index.get("map_to_resources").getAsBoolean()) return gameDir.resolve("resources");
        if (index.has("virtual") && index.get("virtual").getAsBoolean()) {
            return root.resolve("assets").resolve("virtual").resolve(indexId(version));
        }
        return root.resolve("assets");
    }

    /** 为 1.7 之前的版本把哈希命名的对象复制为原始文件名 */
    static void materializeLegacy(Path root, Path gameDir, JsonObject version) throws IOException {
        JsonObject index = readIndex(root, version);
        Path target = gameAssetsDir(root, gameDir, version, index);
        if (target.equals(root.resolve("assets"))) return;
        Path objects = root.resolve("assets").resolve("objects");
        for (Map.Entry<String, JsonElement> e : Params.obj(index, "objects").entrySet()) {
            String hash = e.getValue().getAsJsonObject().get("hash").getAsString();
            Path src = objects.resolve(hash.substring(0, 2)).resolve(hash);
            Path dst = target.resolve(e.getKey()).normalize();
            if (!dst.startsWith(target) || Files.exists(dst) || !Files.exists(src)) continue;
            Files.createDirectories(dst.getParent());
            Files.copy(src, dst, StandardCopyOption.REPLACE_EXISTING);
        }
    }
}
