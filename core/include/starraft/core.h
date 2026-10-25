/*
 * StarRaftLauncher Core — 平台相关的原生能力，以 C ABI 暴露给 BackEnd (Rust FFI)。
 *
 * 内存约定：返回 `char*` 的函数由调用方使用 sr_free() 释放；
 * 返回 `const char*` 的函数指向静态存储，不得释放。
 */
#ifndef STARRAFT_CORE_H
#define STARRAFT_CORE_H

#include <stddef.h>
#include <stdint.h>

#if defined(SR_CORE_SHARED) && defined(_WIN32)
#  ifdef SR_CORE_EXPORTS
#    define SR_API __declspec(dllexport)
#  else
#    define SR_API __declspec(dllimport)
#  endif
#elif defined(SR_CORE_SHARED)
#  define SR_API __attribute__((visibility("default")))
#else
#  define SR_API
#endif

#ifdef __cplusplus
extern "C" {
#endif

/* 核心库版本，如 "0.1.0" */
SR_API const char* sr_core_version(void);

/* 释放由本库分配的字符串 */
SR_API void sr_free(char* ptr);

/* {"os":"osx|windows|linux","arch":"x86_64|arm64|x86"} — 采用 Mojang 版本 JSON 的命名 */
SR_API char* sr_os_info(void);

/* 扫描本机 Java 安装，返回 JSON 数组：[{"path":"...","version":"17.0.9","major":17}, ...] */
SR_API char* sr_detect_java(void);

/* 读取指定 java 可执行文件的版本；失败返回 NULL */
SR_API char* sr_java_version(const char* java_path);

/* 计算文件 SHA-1，写入 41 字节缓冲区（含结尾 0）。成功返回 0 */
SR_API int sr_sha1_file(const char* path, char out_hex[41]);

/* 计算内存数据 SHA-1 */
SR_API void sr_sha1_bytes(const uint8_t* data, size_t len, char out_hex[41]);

/* 物理内存（字节）。成功返回 0 */
SR_API int sr_system_memory(uint64_t* total, uint64_t* available);

/* 根据物理内存给出推荐的游戏最大堆（MB） */
SR_API uint32_t sr_recommended_memory_mb(void);

#ifdef __cplusplus
}
#endif

#endif /* STARRAFT_CORE_H */
