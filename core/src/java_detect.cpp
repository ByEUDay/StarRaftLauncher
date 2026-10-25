// Java 运行时探测：扫描常见安装位置 + JAVA_HOME + PATH，并通过 `java -version` 读取版本
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <set>
#include <sstream>
#include <string>
#include <vector>

#include "internal.hpp"
#include "starraft/core.h"

namespace fs = std::filesystem;

namespace {

#ifdef _WIN32
constexpr const char* kJavaExe = "java.exe";
constexpr char kPathSep = ';';
#  define popen _popen
#  define pclose _pclose
#else
constexpr const char* kJavaExe = "java";
constexpr char kPathSep = ':';
#endif

std::string env(const char* name) {
    const char* v = std::getenv(name);
    return v ? v : "";
}

// 把 <dir>/*/<suffix>/java 形式的候选加入列表
void scan_children(const fs::path& dir, const fs::path& suffix, std::vector<fs::path>& out) {
    std::error_code ec;
    if (!fs::is_directory(dir, ec)) return;
    for (const auto& entry : fs::directory_iterator(dir, fs::directory_options::skip_permission_denied, ec)) {
        out.push_back(entry.path() / suffix / kJavaExe);
    }
}

std::vector<fs::path> candidates() {
    std::vector<fs::path> c;

    if (auto home = env("JAVA_HOME"); !home.empty()) c.push_back(fs::path(home) / "bin" / kJavaExe);

    std::stringstream path(env("PATH"));
    for (std::string dir; std::getline(path, dir, kPathSep);) {
        if (dir.empty()) continue;
#ifdef __APPLE__
        // /usr/bin/java 是系统存根，未安装 JDK 时会弹窗，跳过
        if (dir == "/usr/bin") continue;
#endif
        c.push_back(fs::path(dir) / kJavaExe);
    }

#if defined(_WIN32)
    for (const char* var : {"ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"}) {
        const std::string base = env(var);
        if (base.empty()) continue;
        for (const char* vendor : {"Java", "Eclipse Adoptium", "Eclipse Foundation", "Microsoft", "Zulu",
                                   "BellSoft", "Amazon Corretto", "Semeru"}) {
            scan_children(fs::path(base) / vendor, "bin", c);
        }
    }
    // 官方启动器自带运行时
    if (auto appdata = env("APPDATA"); !appdata.empty()) {
        scan_children(fs::path(appdata) / ".minecraft" / "runtime", fs::path("windows-x64") / "bin", c);
    }
#elif defined(__APPLE__)
    scan_children("/Library/Java/JavaVirtualMachines", "Contents/Home/bin", c);
    if (auto home = env("HOME"); !home.empty()) {
        scan_children(fs::path(home) / "Library/Java/JavaVirtualMachines", "Contents/Home/bin", c);
        scan_children(fs::path(home) / ".sdkman/candidates/java", "bin", c);
    }
    for (const char* prefix : {"/opt/homebrew/opt", "/usr/local/opt"}) {
        std::error_code ec;
        if (!fs::is_directory(prefix, ec)) continue;
        for (const auto& e : fs::directory_iterator(prefix, fs::directory_options::skip_permission_denied, ec)) {
            if (e.path().filename().string().rfind("openjdk", 0) == 0) c.push_back(e.path() / "bin" / kJavaExe);
        }
    }
#else
    for (const char* dir : {"/usr/lib/jvm", "/usr/java", "/opt/java", "/opt/jdk"}) scan_children(dir, "bin", c);
    if (auto home = env("HOME"); !home.empty()) {
        scan_children(fs::path(home) / ".sdkman/candidates/java", "bin", c);
        scan_children(fs::path(home) / ".jdks", "bin", c);
    }
#endif
    return c;
}

std::string run_version(const std::string& java) {
#ifdef _WIN32
    const std::string cmd = "\"\"" + java + "\" -version 2>&1\"";
#else
    const std::string cmd = "'" + java + "' -version 2>&1";
#endif
    FILE* pipe = popen(cmd.c_str(), "r");
    if (!pipe) return "";
    std::string output;
    char buf[256];
    while (std::fgets(buf, sizeof buf, pipe)) output += buf;
    pclose(pipe);

    // 形如: openjdk version "17.0.9" 2023-10-17 / java version "1.8.0_392"
    const auto key = output.find("version \"");
    if (key == std::string::npos) return "";
    const auto start = key + 9;
    const auto end = output.find('"', start);
    return end == std::string::npos ? "" : output.substr(start, end - start);
}

int major_of(const std::string& version) {
    // "1.8.0_392" -> 8, "17.0.9" -> 17, "21" -> 21
    const int first = std::atoi(version.c_str());
    if (first == 1) {
        const auto dot = version.find('.');
        if (dot != std::string::npos) return std::atoi(version.c_str() + dot + 1);
    }
    return first;
}

}  // namespace

extern "C" {

char* sr_java_version(const char* java_path) {
    if (!java_path) return nullptr;
    const std::string v = run_version(java_path);
    return v.empty() ? nullptr : starraft::dup_cstr(v);
}

char* sr_detect_java(void) {
    std::set<std::string> seen;
    std::string json = "[";
    bool first = true;

    for (const auto& candidate : candidates()) {
        std::error_code ec;
        if (!fs::is_regular_file(candidate, ec)) continue;
        const fs::path real = fs::canonical(candidate, ec);
        const std::string path = (ec ? candidate : real).string();
        if (!seen.insert(path).second) continue;

        const std::string version = run_version(path);
        if (version.empty()) continue;

        if (!first) json += ",";
        first = false;
        json += "{\"path\":\"" + starraft::json_escape(path) + "\",\"version\":\"" + starraft::json_escape(version) +
                "\",\"major\":" + std::to_string(major_of(version)) + "}";
    }
    json += "]";
    return starraft::dup_cstr(json);
}

}  // extern "C"
