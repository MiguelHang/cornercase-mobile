package dev.usecornercase.secrets

import android.app.Activity
import android.content.Context
import android.content.SharedPreferences
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.security.GeneralSecurityException
import java.security.KeyStore
import javax.crypto.AEADBadTagException
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

private const val KEYSTORE = "AndroidKeyStore"
private const val KEY_ALIAS = "cornercase-secrets"
private const val KEY_BITS = 256
private const val PREFERENCES = "cornercase-secrets"
private const val TRANSFORMATION = "AES/GCM/NoPadding"
private const val IV_BYTES = 12
private const val TAG_BITS = 128

private const val MALFORMED = "the request to the keystore plugin is malformed"
private const val KEY_GONE = "the keystore key for the secrets is gone"
private const val DAMAGED = "a stored secret is damaged"
private const val WRONG_KEY = "the keystore key cannot open a stored secret"
private const val KEYSTORE_FAILED = "the keystore refused the operation"
private const val NOT_SAVED = "the secrets could not be saved"
private const val UNEXPECTED = "the keystore plugin failed"

@InvokeArg
class NameArgs {
    lateinit var name: String
}

@InvokeArg
class EntryArgs {
    lateinit var name: String
    lateinit var value: String
}

private class Refusal(message: String) : Exception(message)

@TauriPlugin
class SecretsPlugin(activity: Activity) : Plugin(activity) {
    private val preferences: SharedPreferences =
        activity.applicationContext.getSharedPreferences(PREFERENCES, Context.MODE_PRIVATE)
    private var key: SecretKey? = null

    @Command
    fun get(invoke: Invoke) = answer(invoke) {
        val name = parsed(invoke, NameArgs::class.java).name
        val value = preferences.getString(name, null)?.let { opened(name, it) }
        invoke.resolve(JSObject().put("value", value))
    }

    @Command
    fun set(invoke: Invoke) = answer(invoke) {
        val entry = parsed(invoke, EntryArgs::class.java)
        saved(preferences.edit().putString(entry.name, sealed(entry.name, entry.value)))
        invoke.resolve()
    }

    @Command
    fun delete(invoke: Invoke) = answer(invoke) {
        saved(preferences.edit().remove(parsed(invoke, NameArgs::class.java).name))
        invoke.resolve()
    }

    private fun answer(invoke: Invoke, work: () -> Unit) {
        try {
            work()
        } catch (refusal: Refusal) {
            invoke.reject(refusal.message)
        } catch (wrongKey: AEADBadTagException) {
            invoke.reject(WRONG_KEY)
        } catch (refused: GeneralSecurityException) {
            invoke.reject(KEYSTORE_FAILED)
        } catch (damaged: IllegalArgumentException) {
            invoke.reject(DAMAGED)
        } catch (failed: Exception) {
            invoke.reject(UNEXPECTED)
        }
    }

    private fun <T> parsed(invoke: Invoke, type: Class<T>): T {
        try {
            return invoke.parseArgs(type)
        } catch (malformed: Exception) {
            throw Refusal(MALFORMED)
        }
    }

    private fun saved(editor: SharedPreferences.Editor) {
        if (!editor.commit()) {
            throw Refusal(NOT_SAVED)
        }
    }

    private fun sealed(name: String, value: String): String {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, keyFor(creating = true))
        cipher.updateAAD(name.toByteArray(Charsets.UTF_8))
        val text = cipher.doFinal(value.toByteArray(Charsets.UTF_8))
        if (cipher.iv.size != IV_BYTES) {
            throw Refusal(KEYSTORE_FAILED)
        }
        return Base64.encodeToString(cipher.iv + text, Base64.NO_WRAP)
    }

    private fun opened(name: String, stored: String): String {
        val bytes = Base64.decode(stored, Base64.NO_WRAP)
        if (bytes.size <= IV_BYTES) {
            throw Refusal(DAMAGED)
        }
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.DECRYPT_MODE, keyFor(creating = false), GCMParameterSpec(TAG_BITS, bytes, 0, IV_BYTES))
        cipher.updateAAD(name.toByteArray(Charsets.UTF_8))
        return String(cipher.doFinal(bytes, IV_BYTES, bytes.size - IV_BYTES), Charsets.UTF_8)
    }

    @Synchronized
    private fun keyFor(creating: Boolean): SecretKey {
        val known = key ?: stored() ?: if (creating) generated() else throw Refusal(KEY_GONE)
        key = known
        return known
    }

    private fun stored(): SecretKey? {
        val store = KeyStore.getInstance(KEYSTORE)
        store.load(null)
        return store.getKey(KEY_ALIAS, null) as? SecretKey
    }

    private fun generated(): SecretKey {
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, KEYSTORE)
        generator.init(
            KeyGenParameterSpec.Builder(KEY_ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(KEY_BITS)
                .build()
        )
        return generator.generateKey()
    }
}
