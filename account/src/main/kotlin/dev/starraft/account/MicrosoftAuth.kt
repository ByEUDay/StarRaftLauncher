package dev.starraft.account

import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.add
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.int
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.long
import kotlinx.serialization.json.put
import kotlinx.serialization.json.putJsonArray
import kotlinx.serialization.json.putJsonObject

/**
 * 微软账户登录链路（设备码流程）：
 *
 *   Microsoft OAuth (device code) → Xbox Live → XSTS → Minecraft Services → Profile
 *
 * 需要一个在 Azure 注册、允许“个人 Microsoft 账户”与公共客户端流的应用 Client ID，
 * 且该应用需要通过 Minecraft 的 API 访问审核。
 */
class MicrosoftAuth(
    private val clientId: String,
    private val notify: (String, JsonObject) -> Unit,
) {
    companion object {
        private const val AUTHORITY = "https://login.microsoftonline.com/consumers/oauth2/v2.0"
        private const val SCOPE = "XboxLive.signin offline_access"
        private const val XBL_AUTH = "https://user.auth.xboxlive.com/user/authenticate"
        private const val XSTS_AUTH = "https://xsts.auth.xboxlive.com/xsts/authorize"
        private const val MC_LOGIN = "https://api.minecraftservices.com/authentication/login_with_xbox"
        private const val MC_PROFILE = "https://api.minecraftservices.com/minecraft/profile"
    }

    fun login(): AccountResult {
        val dc = Http.postForm("$AUTHORITY/devicecode", mapOf("client_id" to clientId, "scope" to SCOPE))
        if (!dc.ok) throw AuthException("获取设备码失败: ${dc.json.str("error_description") ?: dc.status}")

        val deviceCode = dc.json.str("device_code")!!
        val expiresIn = dc.json["expires_in"]?.jsonPrimitive?.int ?: 900
        var interval = dc.json["interval"]?.jsonPrimitive?.int ?: 5

        notify("account.deviceCode", buildJsonObject {
            put("userCode", dc.json.str("user_code"))
            put("verificationUri", dc.json.str("verification_uri"))
            put("expiresIn", expiresIn)
            put("message", dc.json.str("message"))
        })

        val deadline = System.currentTimeMillis() + expiresIn * 1000L
        while (System.currentTimeMillis() < deadline) {
            Thread.sleep(interval * 1000L)
            val resp = Http.postForm(
                "$AUTHORITY/token",
                mapOf(
                    "grant_type" to "urn:ietf:params:oauth:grant-type:device_code",
                    "client_id" to clientId,
                    "device_code" to deviceCode,
                )
            )
            if (resp.ok) return completeChain(resp.json)
            when (resp.json.str("error")) {
                "authorization_pending" -> continue
                "slow_down" -> interval += 5
                "authorization_declined" -> throw AuthException("用户拒绝了授权")
                "expired_token" -> throw AuthException("设备码已过期，请重试")
                else -> throw AuthException("微软登录失败: ${resp.json.str("error_description") ?: resp.status}")
            }
        }
        throw AuthException("登录超时，请重试")
    }

    fun refresh(refreshToken: String): AccountResult {
        val resp = Http.postForm(
            "$AUTHORITY/token",
            mapOf(
                "grant_type" to "refresh_token",
                "client_id" to clientId,
                "refresh_token" to refreshToken,
                "scope" to SCOPE,
            )
        )
        if (!resp.ok) throw AuthException("刷新登录失败，请重新登录: ${resp.json.str("error_description") ?: resp.status}")
        return completeChain(resp.json)
    }

    private fun completeChain(msToken: JsonObject): AccountResult {
        val msAccess = msToken.str("access_token") ?: throw AuthException("微软令牌缺少 access_token")
        val msRefresh = msToken.str("refresh_token")

        // 1. Xbox Live
        val xbl = Http.postJson(XBL_AUTH, buildJsonObject {
            putJsonObject("Properties") {
                put("AuthMethod", "RPS")
                put("SiteName", "user.auth.xboxlive.com")
                put("RpsTicket", "d=$msAccess")
            }
            put("RelyingParty", "http://auth.xboxlive.com")
            put("TokenType", "JWT")
        })
        if (!xbl.ok) throw AuthException("Xbox Live 认证失败 (HTTP ${xbl.status})")
        val xblToken = xbl.json.str("Token")!!
        val userHash = xbl.json["DisplayClaims"]!!.jsonObject["xui"]!!.jsonArray[0].jsonObject.str("uhs")!!

        // 2. XSTS
        val xsts = Http.postJson(XSTS_AUTH, buildJsonObject {
            putJsonObject("Properties") {
                put("SandboxId", "RETAIL")
                putJsonArray("UserTokens") { add(xblToken) }
            }
            put("RelyingParty", "rp://api.minecraftservices.com/")
            put("TokenType", "JWT")
        })
        if (!xsts.ok) {
            val code = xsts.json["XErr"]?.jsonPrimitive?.long
            throw AuthException(
                when (code) {
                    2148916233L -> "该微软账户尚未创建 Xbox 档案，请先访问 xbox.com 登录一次"
                    2148916235L -> "Xbox Live 在当前国家/地区不可用"
                    2148916236L, 2148916237L -> "该账户需要完成成人验证（韩国地区）"
                    2148916238L -> "未成年账户需要由家长添加到家庭组后才能登录"
                    else -> "XSTS 认证失败 (HTTP ${xsts.status}, XErr $code)"
                }
            )
        }
        val xstsToken = xsts.json.str("Token")!!

        // 3. Minecraft Services
        val mc = Http.postJson(MC_LOGIN, buildJsonObject { put("identityToken", "XBL3.0 x=$userHash;$xstsToken") })
        if (!mc.ok) throw AuthException("Minecraft 服务登录失败 (HTTP ${mc.status})，Client ID 可能未通过 Minecraft API 审核")
        val mcToken = mc.json.str("access_token")!!
        val mcExpires = mc.json["expires_in"]?.jsonPrimitive?.long ?: 86400L

        // 4. 角色档案（404 表示账户未购买游戏）
        val profile = Http.get(MC_PROFILE, mcToken)
        if (profile.status == 404) throw AuthException("该账户未拥有 Minecraft: Java Edition")
        if (!profile.ok) throw AuthException("获取角色档案失败 (HTTP ${profile.status})")

        val skin = profile.json["skins"]?.jsonArray
            ?.map { it.jsonObject }
            ?.firstOrNull { it.str("state") == "ACTIVE" }
            ?.str("url")

        return AccountResult(
            kind = "microsoft",
            username = profile.json.str("name")!!,
            uuid = profile.json.str("id")!!,
            accessToken = mcToken,
            refreshToken = msRefresh,
            expiresAt = System.currentTimeMillis() / 1000 + mcExpires,
            skinUrl = skin,
        )
    }
}

internal fun JsonObject.str(key: String): String? = this[key]?.jsonPrimitive?.content
