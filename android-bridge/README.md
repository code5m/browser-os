# Android 原生桥示意（同一 BridgeCall 契约）

本目录是 **Android 端如何复用桌面同一套「受控桥」契约** 的最小示意，
**不是可直接编译的完整 App**，而是给出关键类的结构与安全落点，方便后续接入
（Tauri v2 Mobile 或纯原生 WebView 皆可套用）。

## 为什么能复用
Bridge 的本质是「前端只发意图，原生层做特权操作」。这套契约与平台无关：

| 能力 | 桌面（Rust/Tauri） | Android（Kotlin） |
|------|-------------------|-------------------|
| 命令白名单 | `#[tauri::command]` | `@JavascriptInterface` 显式方法 |
| 凭据隔离 | 系统密钥库 keyring | Android Keystore + EncryptedSharedPreferences |
| 确认闸门 | `request_sync`→`confirm_sync` | `requestSync()`→`confirmSync()` |
| 全程审计 | `audit.json` | Room / 追加式 `audit.log` |
| 出网推送 | `git2` | JGit |

## 四大安全红线在 Android 的落地
1. **凭据隔离**：Token 用 `EncryptedSharedPreferences`（主密钥存 Android Keystore，
   `AES256_GCM`），WebView 的 JS 永远拿不到明文；见 `KeystoreCredentialStore.kt`。
2. **确认闸门**：`requestSync` 只生成 Pending 任务，必须 `confirmSync` 才真正 push；
   见 `SyncManager.kt`。
3. **全程审计**：每个命令追加审计记录；见 `NativeBridge.kt` 的 `audit(...)`。
4. **最小 IPC 面**：只把 `@JavascriptInterface` 标注的方法暴露给 WebView，
   且仅对受信任的浏览子 WebView `addJavascriptInterface`。

## 文件
- `BridgeContract.kt`：与前端约定的命令/数据结构（对应桌面 `domain.rs` + `bridge.ts`）。
- `NativeBridge.kt`：`@JavascriptInterface` 命令实现（对应 `bridge.rs`）。
- `KeystoreCredentialStore.kt`：凭据隔离（对应 `keyring_store.rs`）。
- `SyncManager.kt`：确认闸门 + JGit 推送 + 三方合并占位（对应 `sync.rs`）。
- `collect_android.js`：注入到 Android WebView 的采集脚本（对应 `injected/collect.js`）。

## 前端零改动
前端 `bridge.ts` 里调用的是 `invoke("collect_selection", …)` 这类「意图」。
在 Android 上，Tauri Mobile 会把它路由到原生命令；若用纯 WebView，则由
`collect_android.js` 通过 `AndroidBridge.collectSelection(json)` 调到 `NativeBridge`。
**命令名与参数结构保持一致**，因此前端代码可跨端复用。

## 依赖（示意，接入时加到 app/build.gradle）
```gradle
implementation "androidx.security:security-crypto:1.1.0-alpha06" // EncryptedSharedPreferences
implementation "org.eclipse.jgit:org.eclipse.jgit:6.9.0.202403050737-r" // git push/merge
implementation "androidx.room:room-runtime:2.6.1" // 审计（可选）
```
