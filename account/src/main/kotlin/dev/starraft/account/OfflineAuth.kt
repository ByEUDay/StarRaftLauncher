package dev.starraft.account

import java.util.UUID

object OfflineAuth {
    private val VALID_NAME = Regex("^[A-Za-z0-9_]{3,16}$")

    /** 与原版服务器离线模式一致：UUID v3("OfflinePlayer:<name>") */
    fun uuidFor(username: String): String =
        UUID.nameUUIDFromBytes("OfflinePlayer:$username".toByteArray(Charsets.UTF_8)).toString().replace("-", "")

    fun create(username: String): AccountResult {
        if (!VALID_NAME.matches(username)) {
            throw AuthException("用户名需为 3-16 位字母、数字或下划线")
        }
        return AccountResult(
            kind = "offline",
            username = username,
            uuid = uuidFor(username),
            accessToken = "0",
        )
    }
}
