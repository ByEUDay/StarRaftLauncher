package dev.starraft.launcher;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;

import java.io.IOException;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/** 生成最终的游戏启动命令行。 */
final class CommandBuilder {
    private CommandBuilder() {}

    private static final Pattern PLACEHOLDER = Pattern.compile("\\$\\{([^}]+)}");

    static JsonObject build(JsonObject p) throws IOException {
        Path root = Params.root(p);
        String versionId = Params.str(p, "versionId");
        Path gameDir = Params.path(p, "gameDir");
        JsonObject account = Params.obj(p, "account");
        if (account == null) throw new IllegalArgumentException("missing parameter: account");
        int memory = Params.integer(p, "memoryMb", 2048);
        int width = Params.integer(p, "width", 0);
        int height = Params.integer(p, "height", 0);

        JsonObject version = VersionLoader.load(root, versionId);
        Platform platform = Platform.current();
        Libraries.Collected libs = Libraries.collect(version, platform);

        List<String> cp = new ArrayList<>();
        Path libDir = root.resolve("libraries");
        for (Libraries.Library lib : libs.classpath()) cp.add(libDir.resolve(lib.path()).toString());
        cp.add(VersionLoader.launchJar(root, versionId, version).toString());
        String classpath = String.join(platform.classpathSeparator(), cp);

        JsonObject index = Assets.readIndex(root, version);
        Path natives = Preparer.nativesDir(root, versionId);

        Map<String, String> vars = new HashMap<>();
        vars.put("auth_player_name", Params.str(account, "username"));
        vars.put("auth_uuid", Params.str(account, "uuid"));
        vars.put("auth_access_token", Params.str(account, "accessToken"));
        vars.put("auth_session", Params.str(account, "accessToken"));
        vars.put("auth_xuid", "");
        vars.put("clientid", "");
        vars.put("user_type", Params.str(account, "userType", "msa"));
        vars.put("user_properties", "{}");
        vars.put("version_name", versionId);
        vars.put("version_type", Params.str(p, "launcherName", "StarRaftLauncher"));
        vars.put("game_directory", gameDir.toString());
        vars.put("assets_root", root.resolve("assets").toString());
        vars.put("game_assets", Assets.gameAssetsDir(root, gameDir, version, index).toString());
        vars.put("assets_index_name", Assets.indexId(version));
        vars.put("natives_directory", natives.toString());
        vars.put("library_directory", libDir.toString());
        vars.put("classpath_separator", platform.classpathSeparator());
        vars.put("classpath", classpath);
        vars.put("launcher_name", Params.str(p, "launcherName", "StarRaftLauncher"));
        vars.put("launcher_version", Params.str(p, "launcherVersion", "0.1.0"));
        vars.put("resolution_width", String.valueOf(width));
        vars.put("resolution_height", String.valueOf(height));

        Map<String, Boolean> features = new HashMap<>();
        features.put("has_custom_resolution", width > 0 && height > 0);
        features.put("is_demo_user", false);

        List<String> cmd = new ArrayList<>();
        cmd.add(Params.str(p, "javaPath"));
        cmd.add("-Xmx" + memory + "m");
        cmd.addAll(Params.strings(p, "jvmArgs"));

        JsonObject arguments = Params.obj(version, "arguments");
        if (arguments != null && arguments.has("jvm")) {
            appendArgs(cmd, arguments.getAsJsonArray("jvm"), vars, platform, features);
        } else {
            // 1.12.2 及更早版本没有 arguments.jvm
            if ("osx".equals(platform.osName())) cmd.add("-XstartOnFirstThread");
            cmd.add("-Djava.library.path=" + natives);
            cmd.add("-Dminecraft.launcher.brand=" + vars.get("launcher_name"));
            cmd.add("-Dminecraft.launcher.version=" + vars.get("launcher_version"));
            cmd.add("-cp");
            cmd.add(classpath);
        }

        cmd.add(Params.str(version, "mainClass"));

        if (arguments != null && arguments.has("game")) {
            appendArgs(cmd, arguments.getAsJsonArray("game"), vars, platform, features);
        } else {
            for (String token : Params.str(version, "minecraftArguments", "").split(" ")) {
                if (!token.isEmpty()) cmd.add(substitute(token, vars));
            }
            if (features.get("has_custom_resolution")) {
                cmd.addAll(List.of("--width", String.valueOf(width), "--height", String.valueOf(height)));
            }
        }
        cmd.addAll(Params.strings(p, "gameArgs"));

        JsonObject result = new JsonObject();
        JsonArray command = new JsonArray();
        cmd.forEach(command::add);
        result.add("command", command);
        result.addProperty("workingDir", gameDir.toString());
        result.addProperty("nativesDir", natives.toString());
        return result;
    }

    static void appendArgs(List<String> out, JsonArray args, Map<String, String> vars, Platform p, Map<String, Boolean> features) {
        for (JsonElement el : args) {
            if (el.isJsonPrimitive()) {
                out.add(substitute(el.getAsString(), vars));
                continue;
            }
            JsonObject arg = el.getAsJsonObject();
            if (!Rules.allowed(Params.arr(arg, "rules"), p, features)) continue;
            JsonElement value = arg.get("value");
            if (value.isJsonArray()) {
                for (JsonElement v : value.getAsJsonArray()) out.add(substitute(v.getAsString(), vars));
            } else {
                out.add(substitute(value.getAsString(), vars));
            }
        }
    }

    static String substitute(String s, Map<String, String> vars) {
        Matcher m = PLACEHOLDER.matcher(s);
        StringBuilder sb = new StringBuilder();
        while (m.find()) {
            String v = vars.getOrDefault(m.group(1), m.group(0));
            m.appendReplacement(sb, Matcher.quoteReplacement(v));
        }
        m.appendTail(sb);
        return sb.toString();
    }
}
