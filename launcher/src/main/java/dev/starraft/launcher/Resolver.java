package dev.starraft.launcher;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;

import java.io.IOException;
import java.nio.file.Path;
import java.util.HashSet;
import java.util.Set;

/** 计算安装一个版本需要下载的文件：客户端 jar、依赖库、natives、资源索引。 */
final class Resolver {
    private Resolver() {}

    static JsonObject resolve(Path root, String versionId) throws IOException {
        JsonObject version = VersionLoader.load(root, versionId);
        Platform platform = Platform.current();
        JsonArray downloads = new JsonArray();
        Set<String> seen = new HashSet<>();

        JsonObject client = Params.obj(Params.obj(version, "downloads"), "client");
        if (client != null) {
            downloads.add(Params.task(client.get("url").getAsString(), VersionLoader.clientJar(root, version),
                    Params.str(client, "sha1", null), Params.number(client, "size", 0)));
        }

        Libraries.Collected libs = Libraries.collect(version, platform);
        Path libDir = root.resolve("libraries");
        for (var list : java.util.List.of(libs.classpath(), libs.natives())) {
            for (Libraries.Library lib : list) {
                if (lib.url().isEmpty() || !seen.add(lib.path())) continue; // 无 url 的库需由安装器自行生成
                downloads.add(Params.task(lib.url(), libDir.resolve(lib.path()), lib.sha1(), lib.size()));
            }
        }

        JsonObject assetIndex = Params.obj(version, "assetIndex");
        if (assetIndex != null) {
            Path indexPath = root.resolve("assets").resolve("indexes").resolve(assetIndex.get("id").getAsString() + ".json");
            downloads.add(Params.task(assetIndex.get("url").getAsString(), indexPath,
                    Params.str(assetIndex, "sha1", null), Params.number(assetIndex, "size", 0)));
        }

        JsonObject result = new JsonObject();
        result.addProperty("versionId", versionId);
        result.addProperty("mainClass", Params.str(version, "mainClass"));
        result.addProperty("javaMajor", Params.integer(Params.obj(version, "javaVersion"), "majorVersion", 8));
        result.add("downloads", downloads);
        return result;
    }
}
