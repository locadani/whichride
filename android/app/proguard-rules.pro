# kotlinx.serialization: keep generated serializers of the server DTOs.
-keepattributes *Annotation*, InnerClasses
-keepclassmembers @kotlinx.serialization.Serializable class io.github.locadani.whichride.** {
    *** Companion;
    kotlinx.serialization.KSerializer serializer(...);
}
