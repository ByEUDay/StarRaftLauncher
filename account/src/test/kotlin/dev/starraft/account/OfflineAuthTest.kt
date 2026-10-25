package dev.starraft.account

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class OfflineAuthTest {
    @Test
    fun `offline uuid matches vanilla server`() {
        // 与原版服务端 offline-mode 生成的 UUID 一致
        assertEquals("b50ad385829d3141a2167e7d7539ba7f", OfflineAuth.uuidFor("Notch"))
    }

    @Test
    fun `rejects invalid usernames`() {
        assertFailsWith<AuthException> { OfflineAuth.create("ab") }
        assertFailsWith<AuthException> { OfflineAuth.create("名字") }
        assertEquals("offline", OfflineAuth.create("Steve_01").kind)
    }
}
