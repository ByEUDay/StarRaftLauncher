package dev.starraft.account

import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put

/** 返回给 BackEnd 的账户信息，字段与 Rust `AuthResult` 对应 */
data class AccountResult(
    val kind: String,
    val username: String,
    val uuid: String,
    val accessToken: String,
    val refreshToken: String? = null,
    val expiresAt: Long? = null,
    val skinUrl: String? = null,
) {
    fun toJson(): JsonObject = buildJsonObject {
        put("kind", kind)
        put("username", username)
        put("uuid", uuid)
        put("accessToken", accessToken)
        refreshToken?.let { put("refreshToken", it) }
        expiresAt?.let { put("expiresAt", it) }
        skinUrl?.let { put("skinUrl", it) }
    }
}

class AuthException(message: String) : Exception(message)
