package io.github.locadani.whichride.server

import android.content.SharedPreferences
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import androidx.core.content.edit
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/** Where the install's bearer token lives between launches. */
interface TokenStore {
    fun load(): String?
    fun save(token: String)
    fun clear()
}

/**
 * The bearer token encrypted with an AndroidKeyStore AES-256-GCM key (decision 32, R10). If the
 * key is gone (a restored or migrated phone) the token can't be read and the app registers again.
 */
class KeystoreTokenStore(private val prefs: SharedPreferences) : TokenStore {
    override fun load(): String? = runCatching {
        val stored = prefs.getString(KEY, null) ?: return null
        val (iv, data) = stored.split(':').map { Base64.decode(it, Base64.NO_WRAP) }
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(128, iv))
        String(cipher.doFinal(data), Charsets.UTF_8)
    }.getOrElse { clear(); null }

    override fun save(token: String) {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, key())
        val data = cipher.doFinal(token.toByteArray(Charsets.UTF_8))
        val encoded = listOf(cipher.iv, data).joinToString(":") { Base64.encodeToString(it, Base64.NO_WRAP) }
        prefs.edit { putString(KEY, encoded) }
    }

    override fun clear() = prefs.edit { remove(KEY) }

    private fun key(): SecretKey {
        val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
        (keyStore.getKey(ALIAS, null) as? SecretKey)?.let { return it }
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEYSTORE).apply {
            init(
                KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                    .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                    .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                    .setKeySize(256)
                    .build(),
            )
        }.generateKey()
    }

    private companion object {
        const val ANDROID_KEYSTORE = "AndroidKeyStore"
        const val ALIAS = "whichride.install-token"
        const val TRANSFORMATION = "AES/GCM/NoPadding"
        const val KEY = "install_token"
    }
}
