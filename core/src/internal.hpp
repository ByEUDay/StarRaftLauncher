#pragma once

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>

namespace starraft {

// 复制为 malloc 分配的 C 字符串，交由 sr_free 释放
inline char* dup_cstr(const std::string& s) {
    char* out = static_cast<char*>(std::malloc(s.size() + 1));
    if (out) std::memcpy(out, s.c_str(), s.size() + 1);
    return out;
}

inline std::string json_escape(const std::string& in) {
    std::string out;
    out.reserve(in.size() + 2);
    for (unsigned char c : in) {
        switch (c) {
            case '"': out += "\\\""; break;
            case '\\': out += "\\\\"; break;
            case '\n': out += "\\n"; break;
            case '\r': out += "\\r"; break;
            case '\t': out += "\\t"; break;
            default:
                if (c < 0x20) {
                    char buf[8];
                    std::snprintf(buf, sizeof buf, "\\u%04x", c);
                    out += buf;
                } else {
                    out += static_cast<char>(c);
                }
        }
    }
    return out;
}

const char* mojang_os_name();
const char* mojang_arch();

}  // namespace starraft
