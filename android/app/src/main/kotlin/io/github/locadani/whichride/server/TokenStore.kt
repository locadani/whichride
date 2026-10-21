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

/** This install on the server: its ID (part of every signed request, decision 44) and its bearer token. */
data class Install(val id: String, val token: String)

/** Where the install lives between launches. */
interface TokenStore {
    fun load(): Install?
    fun save(install: Install)
    fun clear()
}

/**
 * The install ID and bearer token, encrypted with an AndroidKeyStore AES-256-GCM key (decision 32, R10). If the
 * key is gone (a restored or migrated phone) the token can't be read and the app registers again.
 */
class KeystoreTokenStore(private val prefs: SharedPreferences) : TokenStore {
    override fun load(): Install? = runCatching {
        val stored = prefs.getString(KEY, null) ?: return null
        val (iv, data) = stored.split(':').map { Base64.decode(it, Base64.NO_WRAP) }
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(128, iv))
        val parts = String(cipher.doFinal(data), Charsets.UTF_8).split('\n')
        // A 5a value holds only a token: it reads as missing, so the install registers again (R15).
        if (parts.size != 2) {
            clear()
            return null
        }
        Install(parts[0], parts[1])
    }.getOrElse { clear(); null }

    override fun save(install: Install) {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, key())
        val data = cipher.doFinal("${install.id}\n${install.token}".toByteArray(Charsets.UTF_8))
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
