plugins {
    java
}

group = "dev.starraft"
version = "0.1.0"

// 用固定的 JDK 17 编译，不受运行 Gradle 的 JDK 影响
java {
    toolchain { languageVersion.set(JavaLanguageVersion.of(17)) }
}

dependencies {
    implementation("com.google.code.gson:gson:2.11.0")
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.3")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

tasks.withType<JavaCompile> {
    options.encoding = "UTF-8"
    options.release.set(17)
}

tasks.test {
    useJUnitPlatform()
}

// 自包含的可执行 jar
tasks.jar {
    archiveFileName.set("launcher-adapter.jar")
    manifest { attributes["Main-Class"] = "dev.starraft.launcher.Main" }
    duplicatesStrategy = DuplicatesStrategy.EXCLUDE
    from(configurations.runtimeClasspath.get().map { if (it.isDirectory) it else zipTree(it) })
    exclude("META-INF/*.SF", "META-INF/*.DSA", "META-INF/*.RSA", "module-info.class")
}
