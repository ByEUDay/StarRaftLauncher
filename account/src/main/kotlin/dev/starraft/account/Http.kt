package dev.starraft.account

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import java.net.URI
import java.net.URLEncoder
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import java.time.Duration

internal object Http {
    private val client: HttpClient = HttpClient.newBuilder()
        .connectTimeout(Duration.ofSeconds(15))
        .followRedirects(HttpClient.Redirect.NORMAL)
        .build()

    class Response(val status: Int, val body: String) {
        val ok get() = status in 200..299
        val json: JsonObject by lazy {
            runCatching { Json.parseToJsonElement(body) as JsonObject }.getOrDefault(JsonObject(emptyMap()))
        }
    }

    fun postForm(url: String, form: Map<String, String>): Response {
        val body = form.entries.joinToString("&") { (k, v) ->
            "${URLEncoder.encode(k, Charsets.UTF_8)}=${URLEncoder.encode(v, Charsets.UTF_8)}"
        }
        return send(
            request(url)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .POST(HttpRequest.BodyPublishers.ofString(body))
        )
    }

    fun postJson(url: String, body: JsonObject): Response = send(
        request(url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .POST(HttpRequest.BodyPublishers.ofString(body.toString()))
    )

    fun get(url: String, bearer: String): Response = send(
        request(url).header("Authorization", "Bearer $bearer").GET()
    )

    private fun request(url: String) = HttpRequest.newBuilder(URI.create(url))
        .timeout(Duration.ofSeconds(30))
        .header("User-Agent", "StarRaftLauncher/0.1")

    private fun send(builder: HttpRequest.Builder): Response {
        val resp = client.send(builder.build(), HttpResponse.BodyHandlers.ofString())
        return Response(resp.statusCode(), resp.body())
    }
}
