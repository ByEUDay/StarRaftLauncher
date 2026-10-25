package dev.starraft.launcher;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;

import java.util.Map;
import java.util.regex.Pattern;

/**
 * 版本 JSON 中 rules 的求值。
 *
 * <p>规则：存在 rules 时默认禁止；按顺序检查每条规则，匹配的规则用它的 action 覆盖当前结果。
 */
final class Rules {
    private Rules() {}

    static boolean allowed(JsonArray rules, Platform p, Map<String, Boolean> features) {
        if (rules == null || rules.isEmpty()) return true;
        boolean allow = false;
        for (JsonElement el : rules) {
            JsonObject rule = el.getAsJsonObject();
            if (matches(rule, p, features)) {
                allow = "allow".equals(rule.get("action").getAsString());
            }
        }
        return allow;
    }

    private static boolean matches(JsonObject rule, Platform p, Map<String, Boolean> features) {
        JsonObject os = Params.obj(rule, "os");
        if (os != null) {
            if (os.has("name") && !os.get("name").getAsString().equals(p.osName())) return false;
            if (os.has("arch") && !os.get("arch").getAsString().equals(p.arch())) return false;
            if (os.has("version") && !Pattern.compile(os.get("version").getAsString()).matcher(p.osVersion()).find()) {
                return false;
            }
        }
        JsonObject required = Params.obj(rule, "features");
        if (required != null) {
            for (Map.Entry<String, JsonElement> e : required.entrySet()) {
                if (features.getOrDefault(e.getKey(), false) != e.getValue().getAsBoolean()) return false;
            }
        }
        return true;
    }
}
