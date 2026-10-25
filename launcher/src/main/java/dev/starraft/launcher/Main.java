package dev.starraft.launcher;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonElement;
import com.google.gson.JsonNull;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.google.gson.JsonPrimitive;

import java.io.BufferedReader;
import java.io.FileDescriptor;
import java.io.FileOutputStream;
import java.io.InputStreamReader;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;

/**
 * Minecraft 启动适配层入口：通过 stdin/stdout 上的行分隔 JSON-RPC 为 BackEnd 提供服务。
 *
 * <p>stdout 专用于协议输出，所有日志重定向到 stderr。
 */
public final class Main {
    static final Gson GSON = new GsonBuilder().disableHtmlEscaping().create();
    private static final PrintStream PROTOCOL =
            new PrintStream(new FileOutputStream(FileDescriptor.out), true, StandardCharsets.UTF_8);

    private Main() {}

    public static void main(String[] args) throws Exception {
        System.setOut(System.err);
        System.err.println("launcher-adapter ready (java " + System.getProperty("java.version") + ")");

        BufferedReader in = new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8));
        String line;
        while ((line = in.readLine()) != null) {
            if (line.isBlank()) continue;
            JsonObject req;
            try {
                req = JsonParser.parseString(line).getAsJsonObject();
            } catch (RuntimeException e) {
                System.err.println("invalid request: " + line);
                continue;
            }
            JsonElement id = req.has("id") ? req.get("id") : JsonNull.INSTANCE;
            try {
                String method = req.get("method").getAsString();
                JsonObject params = req.has("params") && req.get("params").isJsonObject()
                        ? req.getAsJsonObject("params") : new JsonObject();
                reply(id, dispatch(method, params), null);
            } catch (Exception e) {
                e.printStackTrace();
                reply(id, null, e.getMessage() != null ? e.getMessage() : e.toString());
            }
        }
    }

    static JsonElement dispatch(String method, JsonObject p) throws Exception {
        return switch (method) {
            case "ping" -> new JsonPrimitive("pong");
            case "resolve" -> Resolver.resolve(Params.root(p), Params.str(p, "versionId"));
            case "resolveAssets" -> Assets.resolve(Params.root(p), Params.str(p, "versionId"));
            case "prepare" -> Preparer.prepare(Params.root(p), Params.str(p, "versionId"), Params.path(p, "gameDir"));
            case "buildCommand" -> CommandBuilder.build(p);
            case "installer.extract" -> ForgeInstaller.extract(Params.root(p), Params.path(p, "installer"));
            case "installer.run" -> ForgeInstaller.runProcessors(Params.root(p), Params.path(p, "installer"), Params.str(p, "tag", null));
            default -> throw new IllegalArgumentException("unknown method: " + method);
        };
    }

    /** 不带 id 的通知，BackEnd 会将其转发为前端事件 */
    static void notify(String method, JsonObject params) {
        JsonObject msg = new JsonObject();
        msg.addProperty("jsonrpc", "2.0");
        msg.addProperty("method", method);
        msg.add("params", params);
        synchronized (PROTOCOL) {
            PROTOCOL.println(GSON.toJson(msg));
        }
    }

    private static void reply(JsonElement id, JsonElement result, String error) {
        JsonObject msg = new JsonObject();
        msg.addProperty("jsonrpc", "2.0");
        msg.add("id", id);
        if (error == null) {
            msg.add("result", result == null ? JsonNull.INSTANCE : result);
        } else {
            JsonObject err = new JsonObject();
            err.addProperty("code", -32000);
            err.addProperty("message", error);
            msg.add("error", err);
        }
        synchronized (PROTOCOL) {
            PROTOCOL.println(GSON.toJson(msg));
        }
    }
}
