package dev.starraft.account

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.put
import java.io.FileDescriptor
import java.io.FileOutputStream
import java.io.PrintStream
import java.util.concurrent.Executors

/**
 * 账户模块入口：stdin/stdout 行分隔 JSON-RPC。
 *
 * 方法：
 *  - offline.create      {username}
 *  - microsoft.login     {clientId}        （期间发送 account.deviceCode 通知）
 *  - microsoft.refresh   {clientId, refreshToken}
 */
private val protocol = PrintStream(FileOutputStream(FileDescriptor.out), true, Charsets.UTF_8)

// 微软登录会阻塞数分钟等待用户授权，请求并发处理
private val pool = Executors.newCachedThreadPool { r -> Thread(r, "account-rpc").apply { isDaemon = true } }

fun main() {
    System.setOut(System.err)
    System.err.println("account module ready (java ${System.getProperty("java.version")})")

    val reader = System.`in`.bufferedReader(Charsets.UTF_8)
    while (true) {
        val line = reader.readLine() ?: break
        if (line.isBlank()) continue
        val req = runCatching { Json.parseToJsonElement(line).jsonObject }.getOrNull() ?: continue
        val id = req["id"] ?: JsonNull
        val method = req["method"]?.jsonPrimitive?.content ?: continue
        val params = req["params"] as? JsonObject ?: JsonObject(emptyMap())

        pool.execute {
            try {
                reply(id, dispatch(method, params), null)
            } catch (e: Exception) {
                System.err.println("[$method] ${e.javaClass.simpleName}: ${e.message}")
                reply(id, null, e.message ?: e.toString())
            }
        }
    }
}

internal fun dispatch(method: String, params: JsonObject): JsonElement = when (method) {
    "ping" -> JsonPrimitive("pong")
    "offline.create" -> OfflineAuth.create(params.string("username")).toJson()
    "microsoft.login" -> MicrosoftAuth(params.string("clientId"), ::notify).login().toJson()
    "microsoft.refresh" -> MicrosoftAuth(params.string("clientId"), ::notify).refresh(params.string("refreshToken")).toJson()
    else -> throw IllegalArgumentException("unknown method: $method")
}

internal fun JsonObject.string(key: String): String =
    this[key]?.jsonPrimitive?.content?.takeIf { it.isNotEmpty() } ?: throw IllegalArgumentException("缺少参数: $key")

private fun send(msg: JsonObject) = synchronized(protocol) { protocol.println(msg.toString()) }

private fun reply(id: JsonElement, result: JsonElement?, error: String?) = send(buildJsonObject {
    put("jsonrpc", "2.0")
    put("id", id)
    if (error == null) {
        put("result", result ?: JsonNull)
    } else {
        put("error", buildJsonObject {
            put("code", -32000)
            put("message", error)
        })
    }
})

internal fun notify(method: String, params: JsonObject) = send(buildJsonObject {
    put("jsonrpc", "2.0")
    put("method", method)
    put("params", params)
})
