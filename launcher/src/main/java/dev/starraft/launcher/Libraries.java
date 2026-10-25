package dev.starraft.launcher;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/** 依赖库解析：规则过滤、Maven 坐标、natives 分类器、去重。 */
final class Libraries {
    private Libraries() {}

    static final String DEFAULT_REPO = "https://libraries.minecraft.net/";

    /** 一个需要放进 libraries/ 的文件 */
    record Library(String name, String path, String url, String sha1, long size, boolean isNative, List<String> excludes) {}

    record Collected(List<Library> classpath, List<Library> natives) {}

    /** Maven 坐标 group:artifact:version[:classifier][@ext] */
    record Coord(String group, String artifact, String version, String classifier, String ext) {
        static Coord parse(String name) {
            String ext = "jar";
            int at = name.indexOf('@');
            if (at >= 0) {
                ext = name.substring(at + 1);
                name = name.substring(0, at);
            }
            String[] p = name.split(":");
            if (p.length < 3) throw new IllegalArgumentException("invalid library name: " + name);
            return new Coord(p[0], p[1], p[2], p.length > 3 ? p[3] : null, ext);
        }

        Coord withClassifier(String c) {
            return new Coord(group, artifact, version, c, ext);
        }

        String path() {
            String file = artifact + "-" + version + (classifier != null ? "-" + classifier : "") + "." + ext;
            return group.replace('.', '/') + "/" + artifact + "/" + version + "/" + file;
        }

        /** 去重键：同组件不同版本视为同一个，不同 classifier 视为不同 */
        String key() {
            return group + ":" + artifact + (classifier != null ? ":" + classifier : "");
        }

        boolean nativesClassifier() {
            return classifier != null && classifier.startsWith("natives-");
        }
    }

    static Collected collect(JsonObject version, Platform p) {
        Map<String, Library> classpath = new LinkedHashMap<>();
        List<Library> natives = new ArrayList<>();

        for (JsonElement el : Params.arr(version, "libraries")) {
            JsonObject lib = el.getAsJsonObject();
            if (lib.has("rules") && !Rules.allowed(lib.getAsJsonArray("rules"), p, Map.of())) continue;

            String name = lib.get("name").getAsString();
            Coord coord = Coord.parse(name);
            JsonObject downloads = Params.obj(lib, "downloads");
            List<String> excludes = Params.strings(Params.obj(lib, "extract"), "exclude");

            // 旧式 natives：通过 "natives": {"osx": "natives-osx"} 指定分类器
            JsonObject nativesMap = Params.obj(lib, "natives");
            if (nativesMap != null) {
                if (nativesMap.has(p.osName())) {
                    String classifier = nativesMap.get(p.osName()).getAsString().replace("${arch}", p.bits());
                    JsonObject classifiers = Params.obj(downloads, "classifiers");
                    JsonObject art = Params.obj(classifiers, classifier);
                    natives.add(art != null
                            ? fromArtifact(name, coord.withClassifier(classifier), art, true, excludes)
                            : fromMaven(name, coord.withClassifier(classifier), lib, true, excludes));
                }
                if (Params.obj(downloads, "artifact") == null) continue; // 纯 natives 条目
            }

            JsonObject artifact = Params.obj(downloads, "artifact");
            Library main = artifact != null
                    ? fromArtifact(name, coord, artifact, coord.nativesClassifier(), excludes)
                    : fromMaven(name, coord, lib, coord.nativesClassifier(), excludes);

            if (classpath.putIfAbsent(coord.key(), main) == null && main.isNative()) {
                natives.add(main); // 新式 natives（1.19+）：既在 classpath，也需要解压
            }
        }
        return new Collected(new ArrayList<>(classpath.values()), natives);
    }

    private static Library fromArtifact(String name, Coord coord, JsonObject art, boolean isNative, List<String> excludes) {
        String path = Params.str(art, "path", coord.path());
        return new Library(name, path, Params.str(art, "url", ""), Params.str(art, "sha1", null),
                Params.number(art, "size", 0), isNative, excludes);
    }

    /** Fabric 等加载器的库只给出 name + url（仓库根） */
    private static Library fromMaven(String name, Coord coord, JsonObject lib, boolean isNative, List<String> excludes) {
        String repo = Params.str(lib, "url", DEFAULT_REPO);
        if (!repo.endsWith("/")) repo += "/";
        return new Library(name, coord.path(), repo + coord.path(), Params.str(lib, "sha1", null),
                Params.number(lib, "size", 0), isNative, excludes);
    }
}
