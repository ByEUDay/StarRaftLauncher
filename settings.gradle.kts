// JVM 模块：Java 启动适配层 + Kotlin 账户模块
rootProject.name = "starraft-launcher"

include("launcher", "account")

dependencyResolutionManagement {
    repositories {
        mavenCentral()
    }
}
