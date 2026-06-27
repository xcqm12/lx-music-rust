plugins {
    id("com.android.application") version "8.2.0" apply false
    id("com.android.library") version "8.2.0" apply false
    id("org.jetbrains.kotlin.android") version "1.9.20" apply false
    id("org.jetbrains.kotlin.plugin.serialization") version "1.9.20" apply false
    id("com.google.devtools.ksp") version "1.9.20-1.0.14" apply false
}

buildscript {
    dependencies {
        classpath("org.mozilla.rust-android-gradle:plugin:0.9.3")
    }
}

allprojects {
    repositories {
        google()
        mavenCentral()
    }
}
