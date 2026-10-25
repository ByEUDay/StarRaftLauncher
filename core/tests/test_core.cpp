#include <cstdio>
#include <cstring>

#include "starraft/core.h"

static int failures = 0;

#define CHECK(cond)                                              \
    do {                                                         \
        if (!(cond)) {                                           \
            std::fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond); \
            ++failures;                                          \
        }                                                        \
    } while (0)

int main() {
    char hex[41];

    sr_sha1_bytes(reinterpret_cast<const uint8_t*>(""), 0, hex);
    CHECK(std::strcmp(hex, "da39a3ee5e6b4b0d3255bfef95601890afd80709") == 0);

    sr_sha1_bytes(reinterpret_cast<const uint8_t*>("abc"), 3, hex);
    CHECK(std::strcmp(hex, "a9993e364706816aba3e25717850c26c9cd0d89d") == 0);

    const char* long_msg = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
    sr_sha1_bytes(reinterpret_cast<const uint8_t*>(long_msg), std::strlen(long_msg), hex);
    CHECK(std::strcmp(hex, "84983e441c3bd26ebaae4aa1f95129e5e54670f1") == 0);

    uint64_t total = 0, avail = 0;
    CHECK(sr_system_memory(&total, &avail) == 0);
    CHECK(total > 0);
    CHECK(sr_recommended_memory_mb() >= 1024);

    char* os = sr_os_info();
    CHECK(os != nullptr && std::strstr(os, "\"os\"") != nullptr);
    std::printf("os: %s\n", os);
    sr_free(os);

    char* javas = sr_detect_java();
    CHECK(javas != nullptr && javas[0] == '[');
    std::printf("java: %s\n", javas);
    sr_free(javas);

    if (failures == 0) std::printf("all core tests passed\n");
    return failures == 0 ? 0 : 1;
}
