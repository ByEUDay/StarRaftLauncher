// 独立的 SHA-1 实现，用于校验 Minecraft 资源文件（Mojang 元数据统一使用 SHA-1）
#include <algorithm>
#include <cstdio>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <vector>

#include "starraft/core.h"

namespace {

class Sha1 {
public:
    void update(const uint8_t* p, size_t n) {
        total_ += n;
        while (n > 0) {
            const size_t take = std::min(n, sizeof(buf_) - len_);
            std::memcpy(buf_ + len_, p, take);
            len_ += take;
            p += take;
            n -= take;
            if (len_ == sizeof(buf_)) {
                block(buf_);
                len_ = 0;
            }
        }
    }

    void finish(char out_hex[41]) {
        const uint64_t bits = total_ * 8;
        const uint8_t one = 0x80, zero = 0x00;
        update(&one, 1);
        while (len_ != 56) update(&zero, 1);
        uint8_t length[8];
        for (int i = 0; i < 8; ++i) length[i] = static_cast<uint8_t>(bits >> (56 - 8 * i));
        update(length, 8);
        for (int i = 0; i < 5; ++i) std::snprintf(out_hex + i * 8, 9, "%08x", h_[i]);
        out_hex[40] = '\0';
    }

private:
    static uint32_t rol(uint32_t v, int b) { return (v << b) | (v >> (32 - b)); }

    void block(const uint8_t* p) {
        uint32_t w[80];
        for (int i = 0; i < 16; ++i) {
            w[i] = static_cast<uint32_t>(p[4 * i]) << 24 | static_cast<uint32_t>(p[4 * i + 1]) << 16 |
                   static_cast<uint32_t>(p[4 * i + 2]) << 8 | static_cast<uint32_t>(p[4 * i + 3]);
        }
        for (int i = 16; i < 80; ++i) w[i] = rol(w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16], 1);

        uint32_t a = h_[0], b = h_[1], c = h_[2], d = h_[3], e = h_[4];
        for (int i = 0; i < 80; ++i) {
            uint32_t f, k;
            if (i < 20) {
                f = (b & c) | (~b & d);
                k = 0x5A827999;
            } else if (i < 40) {
                f = b ^ c ^ d;
                k = 0x6ED9EBA1;
            } else if (i < 60) {
                f = (b & c) | (b & d) | (c & d);
                k = 0x8F1BBCDC;
            } else {
                f = b ^ c ^ d;
                k = 0xCA62C1D6;
            }
            const uint32_t t = rol(a, 5) + f + e + k + w[i];
            e = d;
            d = c;
            c = rol(b, 30);
            b = a;
            a = t;
        }
        h_[0] += a;
        h_[1] += b;
        h_[2] += c;
        h_[3] += d;
        h_[4] += e;
    }

    uint32_t h_[5] = {0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0};
    uint8_t buf_[64] = {};
    size_t len_ = 0;
    uint64_t total_ = 0;
};

}  // namespace

extern "C" {

void sr_sha1_bytes(const uint8_t* data, size_t len, char out_hex[41]) {
    Sha1 s;
    s.update(data, len);
    s.finish(out_hex);
}

int sr_sha1_file(const char* path, char out_hex[41]) {
    if (!path || !out_hex) return -1;
#ifdef _WIN32
    // 兼容非 ASCII 路径：UTF-8 → UTF-16
    std::ifstream in(std::filesystem::u8path(path), std::ios::binary);
#else
    std::ifstream in(path, std::ios::binary);
#endif
    if (!in) return -2;
    Sha1 s;
    std::vector<char> buf(1 << 16);
    while (in) {
        in.read(buf.data(), static_cast<std::streamsize>(buf.size()));
        const std::streamsize n = in.gcount();
        if (n > 0) s.update(reinterpret_cast<const uint8_t*>(buf.data()), static_cast<size_t>(n));
    }
    if (in.bad()) return -3;
    s.finish(out_hex);
    return 0;
}

}  // extern "C"
