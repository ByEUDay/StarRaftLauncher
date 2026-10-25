package dev.starraft.launcher;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.NoSuchFileException;
import java.nio.file.Path;
import java.util.Map;

/** 读取 versions/&lt;id&gt;/&lt;id&gt;.json，并按 inheritsFrom 合并父版本（Fabric/Quilt/Forge 配置都依赖此机制）。 */
final class VersionLoader {
    private VersionLoader() {}

    static Path versionDir(Path root, String id) {
        return root.resolve("versions").resolve(id);
    }

    static JsonObject load(Path root, String id) throws IOException {
        return load(root, id, 0);
    }

    private static JsonObject load(Path root, String id, int depth) throws IOException {
        if (depth > 8) throw new IOException("inheritsFrom chain too deep at " + id);
        Path file = versionDir(root, id).resolve(id + ".json");
        JsonObject version;
        try {
            version = JsonParser.parseString(Files.readString(file, StandardCharsets.UTF_8)).getAsJsonObject();
        } catch (NoSuchFileException e) {
            throw new IOException("版本文件不存在: " + file);
        }
        if (!version.has("inheritsFrom")) return version;
        JsonObject parent = load(root, version.get("inheritsFrom").getAsString(), depth + 1);
        return merge(parent, version);
    }

    /** 子版本覆盖父版本；libraries 子在前（去重时优先），arguments 追加。 */
    static JsonObject merge(JsonObject parent, JsonObject child) {
        JsonObject out = parent.deepCopy();
        for (Map.Entry<String, JsonElement> e : child.entrySet()) {
            switch (e.getKey()) {
                case "inheritsFrom" -> { }
                case "libraries" -> {
                    JsonArray libs = e.getValue().getAsJsonArray().deepCopy();
                    libs.addAll(Params.arr(parent, "libraries"));
                    out.add("libraries", libs);
                }
                case "arguments" -> {
                    JsonObject merged = Params.obj(parent, "arguments") != null
                            ? parent.getAsJsonObject("arguments").deepCopy() : new JsonObject();
                    for (Map.Entry<String, JsonElement> a : e.getValue().getAsJsonObject().entrySet()) {
                        JsonArray list = Params.arr(merged, a.getKey()).deepCopy();
                        list.addAll(a.getValue().getAsJsonArray());
                        merged.add(a.getKey(), list);
                    }
                    out.add("arguments", merged);
                }
                default -> out.add(e.getKey(), e.getValue().deepCopy());
            }
        }
        if (!child.has("jar")) {
            out.addProperty("jar", parent.has("jar") ? parent.get("jar").getAsString() : parent.get("id").getAsString());
            out.addProperty(INHERITED_JAR, true);
        }
        return out;
    }

    /** 标记：jar 字段由合并时从父版本继承而来（子版本自身未声明） */
    static final String INHERITED_JAR = "_starraftInheritedJar";

    /**
     * 放进 classpath 的客户端 jar。与官方启动器一致：继承版本使用 versions/&lt;子版本&gt;/&lt;子版本&gt;.jar
     * （内容为父版本 jar 的副本）。Forge/NeoForge 通过 -DignoreList=${version_name}.jar 忽略它，
     * 否则原版 jar 会与补丁后的 minecraft 模块冲突。
     */
    static Path launchJar(Path root, String versionId, JsonObject version) throws IOException {
        Path source = clientJar(root, version);
        if (!version.has(INHERITED_JAR)) return source;
        Path target = versionDir(root, versionId).resolve(versionId + ".jar");
        if (Files.exists(source) && (!Files.exists(target) || Files.size(target) != Files.size(source))) {
            Files.createDirectories(target.getParent());
            Files.copy(source, target, java.nio.file.StandardCopyOption.REPLACE_EXISTING);
        }
        return target;
    }

    /** 客户端 jar 所属的版本 ID（继承版本使用父版本的 jar） */
    static String jarId(JsonObject version) {
        return version.has("jar") ? version.get("jar").getAsString() : version.get("id").getAsString();
    }

    static Path clientJar(Path root, JsonObject version) {
        String jar = jarId(version);
        return versionDir(root, jar).resolve(jar + ".jar");
    }
}
