package dev.starraft.launcher;

import com.google.gson.JsonObject;

import java.io.IOException;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.util.Comparator;
import java.util.Locale;
import java.util.stream.Stream;
import java.util.zip.ZipEntry;
import java.util.zip.ZipFile;

/** 启动前的文件准备：解压 natives、展开旧版资源。 */
final class Preparer {
    private Preparer() {}

    static Path nativesDir(Path root, String versionId) {
        return VersionLoader.versionDir(root, versionId).resolve("natives");
    }

    static JsonObject prepare(Path root, String versionId, Path gameDir) throws IOException {
        JsonObject version = VersionLoader.load(root, versionId);
        Path natives = nativesDir(root, versionId);
        clean(natives);
        Files.createDirectories(natives);

        int extracted = 0;
        Path libDir = root.resolve("libraries");
        for (Libraries.Library lib : Libraries.collect(version, Platform.current()).natives()) {
            Path jar = libDir.resolve(lib.path());
            if (Files.exists(jar)) extracted += extract(jar, natives, lib);
        }

        Files.createDirectories(gameDir);
        Assets.materializeLegacy(root, gameDir, version);

        JsonObject result = new JsonObject();
        result.addProperty("nativesDir", natives.toString());
        result.addProperty("extracted", extracted);
        return result;
    }

    /** 只提取动态库文件并平铺到 natives 根目录（LWJGL 从 java.library.path 根目录加载） */
    private static int extract(Path jar, Path target, Libraries.Library lib) throws IOException {
        int count = 0;
        try (ZipFile zip = new ZipFile(jar.toFile())) {
            var entries = zip.entries();
            while (entries.hasMoreElements()) {
                ZipEntry entry = entries.nextElement();
                String name = entry.getName();
                if (entry.isDirectory() || name.startsWith("META-INF/")) continue;
                if (lib.excludes().stream().anyMatch(name::startsWith)) continue;
                if (!isNativeLibrary(name)) continue;
                Path out = target.resolve(Path.of(name).getFileName().toString());
                try (InputStream in = zip.getInputStream(entry)) {
                    Files.copy(in, out, StandardCopyOption.REPLACE_EXISTING);
                }
                count++;
            }
        }
        return count;
    }

    private static boolean isNativeLibrary(String name) {
        String n = name.toLowerCase(Locale.ROOT);
        return n.endsWith(".dll") || n.endsWith(".so") || n.endsWith(".dylib") || n.endsWith(".jnilib");
    }

    private static void clean(Path dir) throws IOException {
        if (!Files.exists(dir)) return;
        try (Stream<Path> walk = Files.walk(dir)) {
            for (Path p : walk.sorted(Comparator.reverseOrder()).toList()) Files.deleteIfExists(p);
        }
    }
}
