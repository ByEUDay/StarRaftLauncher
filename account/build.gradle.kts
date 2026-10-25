plugins {
    kotlin("jvm") version "2.4.21"
}

group = "dev.starraft"
version = "0.1.0"

dependencies {
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.7.3")
    testImplementation(kotlin("test"))
}

// 用固定的 JDK 17 编译，不受运行 Gradle 的 JDK（可能是 27 等新版本）影响
kotlin {
    jvmToolchain(17)
}

tasks.test {
    useJUnitPlatform()
}

// 自包含的可执行 jar（含 Kotlin 标准库）
tasks.jar {
    archiveFileName.set("account.jar")
    manifest { attributes["Main-Class"] = "dev.starraft.account.MainKt" }
    duplicatesStrategy = DuplicatesStrategy.EXCLUDE
    from(configurations.runtimeClasspath.get().map { if (it.isDirectory) it else zipTree(it) })
    exclude("META-INF/*.SF", "META-INF/*.DSA", "META-INF/*.RSA", "META-INF/versions/9/module-info.class", "module-info.class")
}
