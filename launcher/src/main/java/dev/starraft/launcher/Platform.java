package dev.starraft.launcher;

import java.io.File;
import java.util.Locale;

/** 以 Mojang 版本 JSON 的命名描述当前平台。 */
record Platform(String osName, String arch, String osVersion) {

    static Platform current() {
        String os = System.getProperty("os.name", "").toLowerCase(Locale.ROOT);
        String name = os.contains("win") ? "windows" : os.contains("mac") || os.contains("darwin") ? "osx" : "linux";
        String rawArch = System.getProperty("os.arch", "").toLowerCase(Locale.ROOT);
        String arch = switch (rawArch) {
            case "aarch64", "arm64" -> "arm64";
            case "x86", "i386", "i486", "i586", "i686" -> "x86";
            case "amd64", "x86_64" -> "x86_64";
            default -> rawArch;
        };
        return new Platform(name, arch, System.getProperty("os.version", ""));
    }

    /** 旧版 natives 分类器中的 ${arch} 占位符：32 或 64 */
    String bits() {
        return "x86".equals(arch) ? "32" : "64";
    }

    String classpathSeparator() {
        return File.pathSeparator;
    }
}
