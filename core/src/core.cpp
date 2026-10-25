#include "starraft/core.h"

#include <cstdio>

#include "internal.hpp"

namespace starraft {

const char* mojang_os_name() {
#if defined(_WIN32)
    return "windows";
#elif defined(__APPLE__)
    return "osx";
#else
    return "linux";
#endif
}

const char* mojang_arch() {
#if defined(__aarch64__) || defined(_M_ARM64)
    return "arm64";
#elif defined(__x86_64__) || defined(_M_X64)
    return "x86_64";
#elif defined(__i386__) || defined(_M_IX86)
    return "x86";
#else
    return "unknown";
#endif
}

}  // namespace starraft

extern "C" {

const char* sr_core_version(void) { return "0.1.0"; }

void sr_free(char* ptr) { std::free(ptr); }

char* sr_os_info(void) {
    std::string json = std::string("{\"os\":\"") + starraft::mojang_os_name() + "\",\"arch\":\"" +
                       starraft::mojang_arch() + "\"}";
    return starraft::dup_cstr(json);
}

uint32_t sr_recommended_memory_mb(void) {
    uint64_t total = 0, avail = 0;
    if (sr_system_memory(&total, &avail) != 0 || total == 0) return 2048;
    const uint64_t gb = total / (1024ULL * 1024 * 1024);
    if (gb < 4) return 1024;
    if (gb < 8) return 2048;
    if (gb < 16) return 4096;
    if (gb < 32) return 6144;
    return 8192;
}

}  // extern "C"
