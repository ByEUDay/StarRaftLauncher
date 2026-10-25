package dev.starraft.launcher;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;

import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.util.ArrayList;
import java.util.Comparator;
import java.util.HashMap;
import java.util.HexFormat;
import java.util.List;
import java.util.Map;
import java.util.jar.JarFile;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import java.util.stream.Stream;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;

/**
 * Forge / NeoForge 安装器的无界面实现（客户端侧）。
 *
 * <p>分两步，中间由 BackEnd 通过 Go 下载引擎下载依赖：
 * <ol>
 *   <li>{@link #extract}：读取 install_profile.json，写出版本配置，解出内嵌 maven/ 文件，返回需要下载的库</li>
 *   <li>{@link #runProcessors}：按顺序执行客户端处理器（二进制补丁、映射合并等）</li>
 * </ol>
 * 同时兼容旧格式（install + versionInfo）。
 */
final class ForgeInstaller {
    private ForgeInstaller() {}

    private static final Pattern TOKEN = Pattern.compile("\\{([A-Za-z0-9_]+)}");

    static JsonObject readProfile(ZipFile zip) throws IOException {
        ZipEntry entry = zip.getEntry("install_profile.json");
        if (entry == null) throw new IOException("不是有效的 Forge/NeoForge 安装器：缺少 install_profile.json");
        try (InputStream in = zip.getInputStream(entry)) {
            return JsonParser.parseString(new String(in.readAllBytes(), StandardCharsets.UTF_8)).getAsJsonObject();
        }
    }

    static JsonObject extract(Path root, Path installer) throws IOException {
        Path libDir = root.resolve("libraries");
        try (ZipFile zip = new ZipFile(installer.toFile())) {
            JsonObject profile = readProfile(zip);
            JsonObject result = new JsonObject();
            JsonArray downloads = new JsonArray();

            if (profile.has("versionInfo")) {
                // 旧格式：versionInfo 即版本 JSON，install.filePath 为需要放入 libraries 的通用 jar
                JsonObject versionInfo = profile.getAsJsonObject("versionInfo");
                JsonObject install = profile.getAsJsonObject("install");
                String id = versionInfo.get("id").getAsString();
                writeVersion(root, id, versionInfo.toString());
                ZipEntry universal = zip.getEntry(install.get("filePath").getAsString());
                if (universal != null) {
                    Path target = libDir.resolve(Libraries.Coord.parse(install.get("path").getAsString()).path());
                    copyEntry(zip, universal, target);
                }
                result.addProperty("profileId", id);
                result.addProperty("processors", 0);
                result.add("downloads", downloads);
                return result;
            }

            String id = profile.get("version").getAsString();
            ZipEntry versionEntry = zip.getEntry(Params.str(profile, "json", "/version.json").replaceFirst("^/", ""));
            if (versionEntry == null) throw new IOException("安装器中缺少版本配置");
            try (InputStream in = zip.getInputStream(versionEntry)) {
                writeVersion(root, id, new String(in.readAllBytes(), StandardCharsets.UTF_8));
            }

            // 内嵌在安装器 maven/ 目录下的构件（如 universal / shim jar）
            var entries = zip.entries();
            while (entries.hasMoreElements()) {
                ZipEntry e = entries.nextElement();
                if (e.isDirectory() || !e.getName().startsWith("maven/")) continue;
                Path target = libDir.resolve(e.getName().substring("maven/".length())).normalize();
                if (target.startsWith(libDir)) copyEntry(zip, e, target);
            }

            // 处理器依赖的库（有 url 的才需要下载）
            for (JsonElement el : Params.arr(profile, "libraries")) {
                JsonObject lib = el.getAsJsonObject();
                JsonObject art = Params.obj(Params.obj(lib, "downloads"), "artifact");
                if (art == null) continue;
                String url = Params.str(art, "url", "");
                if (url.isEmpty()) continue;
                String path = Params.str(art, "path", Libraries.Coord.parse(lib.get("name").getAsString()).path());
                downloads.add(Params.task(url, libDir.resolve(path), Params.str(art, "sha1", null), Params.number(art, "size", 0)));
            }

            int clientProcessors = 0;
            for (JsonElement el : Params.arr(profile, "processors")) {
                if (forClient(el.getAsJsonObject())) clientProcessors++;
            }

            result.addProperty("profileId", id);
            result.addProperty("minecraft", Params.str(profile, "minecraft", ""));
            result.addProperty("processors", clientProcessors);
            result.add("downloads", downloads);
            return result;
        }
    }

    static JsonObject runProcessors(Path root, Path installer, String tag) throws Exception {
        Path libDir = root.resolve("libraries");
        Path temp = Files.createTempDirectory("starraft-installer");
        try (ZipFile zip = new ZipFile(installer.toFile())) {
            JsonObject profile = readProfile(zip);
            String mc = profile.get("minecraft").getAsString();

            Map<String, String> vars = new HashMap<>();
            vars.put("SIDE", "client");
            vars.put("MINECRAFT_VERSION", mc);
            vars.put("MINECRAFT_JAR", VersionLoader.versionDir(root, mc).resolve(mc + ".jar").toString());
            vars.put("ROOT", root.toString());
            vars.put("INSTALLER", installer.toString());
            vars.put("LIBRARY_DIR", libDir.toString());
            JsonObject data = Params.obj(profile, "data");
            if (data != null) {
                for (Map.Entry<String, JsonElement> e : data.entrySet()) {
                    String value = Params.str(e.getValue().getAsJsonObject(), "client", null);
                    if (value != null) vars.put(e.getKey(), dataValue(value, libDir, zip, temp));
                }
            }

            List<JsonObject> processors = new ArrayList<>();
            for (JsonElement el : Params.arr(profile, "processors")) {
                if (forClient(el.getAsJsonObject())) processors.add(el.getAsJsonObject());
            }

            String javaExe = ProcessHandle.current().info().command().orElse("java");
            int ran = 0;
            for (int i = 0; i < processors.size(); i++) {
                JsonObject p = processors.get(i);
                Map<String, String> outputs = new HashMap<>();
                JsonObject outs = Params.obj(p, "outputs");
                if (outs != null) {
                    for (Map.Entry<String, JsonElement> e : outs.entrySet()) {
                        outputs.put(arg(e.getKey(), vars, libDir), arg(e.getValue().getAsString(), vars, libDir));
                    }
                }
                String name = p.get("jar").getAsString();
                notifyProgress(tag, i + 1, processors.size(), name);
                if (!outputs.isEmpty() && outputsValid(outputs)) continue;

                Path jar = libDir.resolve(Libraries.Coord.parse(name).path());
                List<String> cp = new ArrayList<>();
                cp.add(jar.toString());
                for (String c : Params.strings(p, "classpath")) cp.add(libDir.resolve(Libraries.Coord.parse(c).path()).toString());

                List<String> cmd = new ArrayList<>(List.of(javaExe, "-cp", String.join(java.io.File.pathSeparator, cp), mainClass(jar)));
                for (String a : Params.strings(p, "args")) cmd.add(arg(a, vars, libDir));
                run(cmd, name);
                ran++;

                if (!outputs.isEmpty() && !outputsValid(outputs)) {
                    throw new IOException("处理器 " + name + " 的输出校验失败");
                }
            }

            JsonObject result = new JsonObject();
            result.addProperty("ran", ran);
            result.addProperty("total", processors.size());
            return result;
        } finally {
            deleteTree(temp);
        }
    }

    static boolean forClient(JsonObject processor) {
        JsonArray sides = Params.arr(processor, "sides");
        if (sides.isEmpty()) return true;
        for (JsonElement s : sides) if ("client".equals(s.getAsString())) return true;
        return false;
    }

    /** data 中的取值：[构件] → 库路径，'文本' → 字面量，/路径 → 从安装器解出的文件 */
    static String dataValue(String v, Path libDir, ZipFile zip, Path temp) throws IOException {
        if (v.startsWith("[") && v.endsWith("]")) {
            return libDir.resolve(Libraries.Coord.parse(v.substring(1, v.length() - 1)).path()).toString();
        }
        if (v.startsWith("'") && v.endsWith("'")) return v.substring(1, v.length() - 1);
        if (v.startsWith("/") && zip != null) {
            ZipEntry entry = zip.getEntry(v.substring(1));
            if (entry == null) throw new IOException("安装器中缺少 " + v);
            Path out = temp.resolve(v.substring(1)).normalize();
            copyEntry(zip, entry, out);
            return out.toString();
        }
        return v;
    }

    /** 处理器参数：[构件] → 库路径；{KEY} → 变量 */
    static String arg(String a, Map<String, String> vars, Path libDir) {
        if (a.startsWith("[") && a.endsWith("]")) {
            return libDir.resolve(Libraries.Coord.parse(a.substring(1, a.length() - 1)).path()).toString();
        }
        Matcher m = TOKEN.matcher(a);
        StringBuilder sb = new StringBuilder();
        while (m.find()) {
            String v = vars.get(m.group(1));
            m.appendReplacement(sb, Matcher.quoteReplacement(v != null ? v : m.group(0)));
        }
        m.appendTail(sb);
        return sb.toString();
    }

    private static boolean outputsValid(Map<String, String> outputs) throws IOException {
        for (Map.Entry<String, String> e : outputs.entrySet()) {
            Path file = Path.of(e.getKey());
            if (!Files.isRegularFile(file) || !sha1(file).equalsIgnoreCase(e.getValue())) return false;
        }
        return true;
    }

    private static String mainClass(Path jar) throws IOException {
        if (!Files.isRegularFile(jar)) throw new IOException("缺少处理器 " + jar.getFileName());
        try (JarFile jf = new JarFile(jar.toFile())) {
            String main = jf.getManifest() == null ? null : jf.getManifest().getMainAttributes().getValue("Main-Class");
            if (main == null) throw new IOException(jar.getFileName() + " 没有 Main-Class");
            return main;
        }
    }

    private static void run(List<String> cmd, String name) throws IOException, InterruptedException {
        System.err.println("[processor] " + name);
        Process proc = new ProcessBuilder(cmd).redirectErrorStream(true).start();
        StringBuilder tail = new StringBuilder();
        try (var reader = proc.inputReader(StandardCharsets.UTF_8)) {
            String line;
            while ((line = reader.readLine()) != null) {
                System.err.println("  " + line);
                tail.append(line).append('\n');
                if (tail.length() > 4000) tail.delete(0, tail.length() - 4000);
            }
        }
        int code = proc.waitFor();
        if (code != 0) {
            String last = tail.toString().strip();
            int cut = last.lastIndexOf('\n', Math.max(0, last.length() - 600));
            throw new IOException("处理器 " + name + " 失败（退出码 " + code + "）：" + (cut > 0 ? last.substring(cut + 1) : last));
        }
    }

    private static void notifyProgress(String tag, int index, int total, String name) {
        JsonObject p = new JsonObject();
        if (tag != null) p.addProperty("instanceId", tag);
        p.addProperty("index", index);
        p.addProperty("total", total);
        p.addProperty("processor", name);
        Main.notify("loader.progress", p);
    }

    private static void writeVersion(Path root, String id, String json) throws IOException {
        Path dir = VersionLoader.versionDir(root, id);
        Files.createDirectories(dir);
        Files.writeString(dir.resolve(id + ".json"), json, StandardCharsets.UTF_8);
    }

    private static void copyEntry(ZipFile zip, ZipEntry entry, Path target) throws IOException {
        Files.createDirectories(target.getParent());
        try (InputStream in = zip.getInputStream(entry)) {
            Files.copy(in, target, StandardCopyOption.REPLACE_EXISTING);
        }
    }

    static String sha1(Path file) throws IOException {
        try {
            MessageDigest md = MessageDigest.getInstance("SHA-1");
            try (InputStream in = Files.newInputStream(file)) {
                byte[] buf = new byte[1 << 16];
                for (int n; (n = in.read(buf)) > 0; ) md.update(buf, 0, n);
            }
            return HexFormat.of().formatHex(md.digest());
        } catch (NoSuchAlgorithmException e) {
            throw new IllegalStateException(e);
        }
    }

    private static void deleteTree(Path dir) {
        try (Stream<Path> walk = Files.walk(dir)) {
            for (Path p : walk.sorted(Comparator.reverseOrder()).toList()) Files.deleteIfExists(p);
        } catch (IOException ignored) {
            // 临时目录清理失败不影响安装结果
        }
    }
}
