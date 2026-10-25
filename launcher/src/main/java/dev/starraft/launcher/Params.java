package dev.starraft.launcher;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;

import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

/** JSON 参数与对象读取的小工具。 */
final class Params {
    private Params() {}

    static String str(JsonObject o, String key) {
        JsonElement e = o.get(key);
        if (e == null || e.isJsonNull()) throw new IllegalArgumentException("missing parameter: " + key);
        return e.getAsString();
    }

    static String str(JsonObject o, String key, String fallback) {
        JsonElement e = o == null ? null : o.get(key);
        return e == null || e.isJsonNull() ? fallback : e.getAsString();
    }

    static int integer(JsonObject o, String key, int fallback) {
        JsonElement e = o == null ? null : o.get(key);
        return e == null || e.isJsonNull() ? fallback : e.getAsInt();
    }

    static long number(JsonObject o, String key, long fallback) {
        JsonElement e = o == null ? null : o.get(key);
        return e == null || e.isJsonNull() ? fallback : e.getAsLong();
    }

    static Path path(JsonObject o, String key) {
        return Path.of(str(o, key)).toAbsolutePath().normalize();
    }

    static Path root(JsonObject o) {
        return path(o, "root");
    }

    static JsonObject obj(JsonObject o, String key) {
        JsonElement e = o == null ? null : o.get(key);
        return e != null && e.isJsonObject() ? e.getAsJsonObject() : null;
    }

    static JsonArray arr(JsonObject o, String key) {
        JsonElement e = o == null ? null : o.get(key);
        return e != null && e.isJsonArray() ? e.getAsJsonArray() : new JsonArray();
    }

    static List<String> strings(JsonObject o, String key) {
        List<String> out = new ArrayList<>();
        for (JsonElement e : arr(o, key)) out.add(e.getAsString());
        return out;
    }

    /** 构造给 Go 下载引擎的任务对象 */
    static JsonObject task(String url, Path path, String sha1, long size) {
        JsonObject t = new JsonObject();
        t.addProperty("url", url);
        t.addProperty("path", path.toString());
        if (sha1 != null && !sha1.isEmpty()) t.addProperty("sha1", sha1);
        if (size > 0) t.addProperty("size", size);
        return t;
    }
}
