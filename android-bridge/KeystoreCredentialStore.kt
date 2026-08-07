package com.jizhijiandan.mvp.bridge

import android.content.Context
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey

/**
 * 凭据隔离（对应桌面 keyring_store.rs）。
 *
 * Token 用 EncryptedSharedPreferences 加密落盘：主密钥存放在 Android Keystore
 * （硬件级 TEE/StrongBox 可用时由硬件保护），条目用 AES256-GCM 加密。
 * WebView 的 JavaScript 永远拿不到明文——它只能通过白名单命令“使用”凭据，
 * 而读取明文只发生在原生层 push 的一瞬间（见 SyncManager）。
 */
class KeystoreCredentialStore(context: Context) {

    private val masterKey = MasterKey.Builder(context)
        .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
        .build()

    private val prefs = EncryptedSharedPreferences.create(
        context,
        "jzjd_secure_tokens",           // 文件名
        masterKey,
        EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
        EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM,
    )

    fun saveToken(repoId: String, token: String) {
        prefs.edit().putString(key(repoId), token).apply()
    }

    /** 仅供原生 push 时短暂读取；调用方用完不得回传前端/写日志。 */
    fun getToken(repoId: String): String =
        prefs.getString(key(repoId), null)
            ?: throw IllegalStateException("凭据缺失（请重新配置仓库）")

    fun deleteToken(repoId: String) {
        prefs.edit().remove(key(repoId)).apply()
    }

    private fun key(repoId: String) = "token::$repoId"
}
