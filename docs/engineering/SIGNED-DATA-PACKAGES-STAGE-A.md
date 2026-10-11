# BrowserOS 离线签名资源包 · 安全交付 Stage-A

## 能力边界

本工具 **真正执行离线、数据型资源包的 Ed25519 签名验证、SHA-256 文件校验、原子状态更新、安装、停用指针、历史版本回滚与依赖拒绝**。只有严格白名单的 `md/txt/json/csv/png/jpg/jpeg/webp` 数据资源可以进入存储，禁止 HTML、JS、Wasm、CSS、SVG、可执行文件、脚本和任意 IPC 权限。只接受**目录包**；不解析 ZIP、TAR 等任意归档格式，从源头避开解压目录穿越与压缩炸弹。最大 32 个资源、单资源 4MiB、合计 16MiB。

**这不是 HP3 的外部可执行能力安装，不注册 Capability、不改 Manifest Owner、不启用插件运行沙箱。** 现有 `PLUGIN_NO_EXEC_SURFACE` 必须保持，插件管理器的「安装」仍是 Stage-I manifest 元数据操作，不能把两者混为一谈。工具由 Node CLI 在受信任的本地构建/运维环境运行，**不是直接给陌生网页开放的 Tauri 命令**。

## manifest 与信任库

输入目录包含 `manifest.json` 和资源文件，例如：

```json
{
  "schema":"browseros-signed-data-v1",
  "id":"org.example.guide",
  "version":"1.0.0",
  "kind":"data-only",
  "host_api":"1",
  "dependencies":[],
  "files":[{"path":"assets/guide.md","bytes":16,"sha256":"<64 lowercase hex>"}],
  "signature":{"algorithm":"Ed25519","key_id":"publisher-1","value":"<base64 64-byte signature>"}
}
```

信任库必须由本机运维独立提供，而不是来自资源包：

```json
{"schema":"browseros-trusted-keys-v1","keys":{"publisher-1":{"spki_base64":"<DER SPKI public key, base64>","revoked":false}}}
```

签名消息为 UTF-8 `browseros-signed-data-v1\n` + 对 manifest 移除 `signature` 字段后的对象按**字典序排序 key 的递归 canonical JSON**。签名算法 Ed25519，签名原文字节经严格 Base64 解析。私钥**绝不能**进入 manifest/日志/仓库。重签或更换密钥需运维侧审核，撤销后的包无法重新验证安装。

## 使用

```bash
node scripts/secure-resource-packages.mjs --self-test
node scripts/secure-resource-packages.mjs install --store /secure/browseros-data --trust /secure/trusted-keys.json --source /local/signed-package
node scripts/secure-resource-packages.mjs list --store /secure/browseros-data --trust /secure/trusted-keys.json
node scripts/secure-resource-packages.mjs uninstall --store /secure/browseros-data --trust /secure/trusted-keys.json --id org.example.guide
node scripts/secure-resource-packages.mjs rollback --store /secure/browseros-data --trust /secure/trusted-keys.json --id org.example.guide
```

安装先把资源读入内存验证签名/散列/限制，再写入私有临时目录、重命名为不可变内容目录，最后通过原子重命名切换 `state.json` 指针。卸载仅删除激活指针，保留历史版本和数据以支持回滚；签名无效、密钥已撤销、依赖未满足、路径穿越、符号链接及篡改全部拒绝。崩溃后可能留下未引用 staging 目录或保守的锁文件，默认 **拒绝继续写入、不自动清除未知目录**，需要经过审计再恢复。

## 后续才能申报 HP3 的前置证据

- 正式将可信公钥存储引入 Native/OS Keyring，并在授权机器做 E2E。
- 真正的可运行模块需另外解决**沙箱、权限白名单、签名过期和撤销、热插拔事务、全生命周期卸载、进程隔离、攻击模拟与 GUI 回归**。上述能力未有证据前不得移除 `PLUGIN_NO_EXEC_SURFACE` 或宣称 HP3。
- 与核心 Browser、Files、Grid 隔离；内核构建资源新增量、安装包体积和运行内存需分别测量，不得假设拆分源代码=降低安装体积。

