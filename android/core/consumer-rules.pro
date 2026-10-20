# JNA reflects over its own classes and over the generated binding interfaces.
-keep class com.sun.jna.** { *; }
-keepclassmembers class * extends com.sun.jna.** { public *; }
-keep class io.github.locadani.whichride.core.** { *; }
-dontwarn java.awt.**
