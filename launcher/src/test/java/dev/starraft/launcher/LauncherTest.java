package dev.starraft.launcher;

import com.google.gson.JsonArray;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import org.junit.jupiter.api.Test;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

class LauncherTest {
    private static final Platform MAC_ARM = new Platform("osx", "arm64", "14.0");
    private static final Platform WIN_X64 = new Platform("windows", "x86_64", "10.0");

    private static JsonArray arr(String json) {
        return JsonParser.parseString(json).getAsJsonArray();
    }

    @Test
    void rulesFollowLastMatchWins() {
        JsonArray rules = arr("[{\"action\":\"allow\"},{\"action\":\"disallow\",\"os\":{\"name\":\"osx\"}}]");
        assertFalse(Rules.allowed(rules, MAC_ARM, Map.of()));
        assertTrue(Rules.allowed(rules, WIN_X64, Map.of()));
        assertTrue(Rules.allowed(new JsonArray(), MAC_ARM, Map.of()));
    }

    @Test
    void featureRules() {
        JsonArray rules = arr("[{\"action\":\"allow\",\"features\":{\"has_custom_resolution\":true}}]");
        assertTrue(Rules.allowed(rules, WIN_X64, Map.of("has_custom_resolution", true)));
        assertFalse(Rules.allowed(rules, WIN_X64, Map.of()));
    }

    @Test
    void mavenCoordinates() {
        var c = Libraries.Coord.parse("org.lwjgl:lwjgl:3.3.3:natives-macos-arm64");
        assertEquals("org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-macos-arm64.jar", c.path());
        assertTrue(c.nativesClassifier());
        assertEquals("org.lwjgl:lwjgl:natives-macos-arm64", c.key());
    }

    @Test
    void legacyNativesUseArchPlaceholder() {
        JsonObject version = JsonParser.parseString("""
                {"libraries":[{"name":"tv.twitch:twitch-platform:5.16","natives":{"windows":"natives-windows-${arch}"},
                  "downloads":{"classifiers":{"natives-windows-64":{"path":"tv/twitch/x-64.jar","url":"https://x/64.jar","sha1":"aa","size":1}}}}]}
                """).getAsJsonObject();
        var collected = Libraries.collect(version, WIN_X64);
        assertEquals(0, collected.classpath().size());
        assertEquals(1, collected.natives().size());
        assertEquals("tv/twitch/x-64.jar", collected.natives().get(0).path());
    }

    @Test
    void childLibrariesWinOnMerge() throws Exception {
        JsonObject parent = JsonParser.parseString(
                "{\"id\":\"1.21\",\"libraries\":[{\"name\":\"org.ow2.asm:asm:9.3\"}],\"arguments\":{\"game\":[\"--a\"]}}").getAsJsonObject();
        JsonObject child = JsonParser.parseString(
                "{\"id\":\"fabric\",\"inheritsFrom\":\"1.21\",\"libraries\":[{\"name\":\"org.ow2.asm:asm:9.7\",\"url\":\"https://maven.fabricmc.net/\"}],\"arguments\":{\"game\":[\"--b\"]}}").getAsJsonObject();
        JsonObject merged = VersionLoader.merge(parent, child);
        assertEquals("1.21", VersionLoader.jarId(merged));
        assertTrue(merged.has(VersionLoader.INHERITED_JAR));
        assertEquals(java.nio.file.Path.of("/mc/versions/fabric/fabric.jar"),
                VersionLoader.launchJar(java.nio.file.Path.of("/mc"), "fabric", merged));
        assertFalse(VersionLoader.merge(parent, JsonParser.parseString("{\"id\":\"x\",\"jar\":\"1.21\"}").getAsJsonObject())
                .has(VersionLoader.INHERITED_JAR));
        assertEquals(2, merged.getAsJsonObject("arguments").getAsJsonArray("game").size());
        var libs = Libraries.collect(merged, WIN_X64).classpath();
        assertEquals(1, libs.size());
        assertEquals("https://maven.fabricmc.net/org/ow2/asm/asm/9.7/asm-9.7.jar", libs.get(0).url());
    }

    @Test
    void forgeProcessorArguments() throws Exception {
        java.nio.file.Path lib = java.nio.file.Path.of("/mc/libraries");
        assertEquals(lib.resolve("de/oceanlabs/mcp/mcp_config/1.20.1/mcp_config-1.20.1-mappings.txt").toString(),
                ForgeInstaller.arg("[de.oceanlabs.mcp:mcp_config:1.20.1:mappings@txt]", Map.of(), lib));
        assertEquals("--input=/a.jar", ForgeInstaller.arg("--input={MINECRAFT_JAR}", Map.of("MINECRAFT_JAR", "/a.jar"), lib));
        assertEquals("{UNKNOWN}", ForgeInstaller.arg("{UNKNOWN}", Map.of(), lib));
        assertEquals("abc", ForgeInstaller.dataValue("'abc'", lib, null, null));
        assertTrue(ForgeInstaller.forClient(JsonParser.parseString("{\"sides\":[\"client\"]}").getAsJsonObject()));
        assertFalse(ForgeInstaller.forClient(JsonParser.parseString("{\"sides\":[\"server\"]}").getAsJsonObject()));
        assertTrue(ForgeInstaller.forClient(new JsonObject()));
    }

    @Test
    void argumentsSubstitution() {
        List<String> out = new ArrayList<>();
        JsonArray args = arr("[\"--username\",\"${auth_player_name}\",{\"rules\":[{\"action\":\"allow\",\"os\":{\"name\":\"osx\"}}],\"value\":[\"-XstartOnFirstThread\"]}]");
        CommandBuilder.appendArgs(out, args, Map.of("auth_player_name", "Steve"), WIN_X64, Map.of());
        assertEquals(List.of("--username", "Steve"), out);
        assertEquals("a$b\\c", CommandBuilder.substitute("${x}", Map.of("x", "a$b\\c")));
    }
}
