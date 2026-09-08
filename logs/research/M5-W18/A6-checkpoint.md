# A6 · M5-W18-R Lane Checkpoint

## Required lane output (per PARALLEL_COMMAND_BOARD.md §Required lane output)

```
LANE=A6
STATUS=PASS_WITH_DEBT
BASE=78d2cfb
HEAD=0d072de7fcd9e612a7ec7a7960c2ff096801a3e4
REFERENCE_EVIDENCE=/home/ainfinit/Documents/极智简单/V3/research/dbx-src (Apache-2.0, Cargo.lock SHA-256 c0a7be12c05d8dffe867f1a70d4b82e881dec2da3bb622b5d3e3410c3d10e3a7); /home/ainfinit/Documents/极智简单/V3/dbx-study/dbx-借鉴分析.md (V5); current product = mvp-browser-os-v3 worktree m5-w18-a6 @ 78d2cfb
FILES=logs/research/M5-W18/A6-security-lifecycle-audit.md, logs/research/M5-W18/A6-checkpoint.md
SOURCE_MAP=dbx: connection_secrets.rs:37/97/127, state_persistence.rs:476-547, production_safety.rs:98/110/184, sql_risk.rs:686/703, agent_tools.rs:19/28/31/34/78/90/111/699, models/connection.rs:206/296/961/1509/2138, query_result_export.rs:48/1261/1392, runtime_config.rs:51/66, storage.rs:332/1000/1029/1039/1098/926, export_download.rs:9/972, app_settings.rs:83; current: core/keyring_store.rs:5-27, security_policy.rs:1046/1100/1106/1146/1206/1240/1250/1269, database.rs:38-58/156-158/168/184-192/230/263/369/384/400-409/413/422/468/476, check-database-policy.py (15 codes), shutdown.rs, bridge.rs:780/6020/6069
CLASSIFICATION=COPY=3 (OS keyring; result limit constants+truncated flag; ShutdownCoordinator) | ADAPT=4 (verify_confirmed_target SQL-text match; cross-DB production verdict + production_databases; config persistence atomic + cred retain; encrypted fallback EncryptedPayload) | REIMPLEMENT_FROM_BEHAVIOR=3 (server-side kill_query cancel; connect/idle timeout + driver floor; DSN/debug redaction breadth) | DEFER=2 (Mongo $out/$merge verdict; DB_POOL_SHUTDOWN_CLEANUP on pool intro) | REJECT=2 (dbx plaintext FileSecretStore JSON; JDBC sidecar)
VERIFY=read-only source audit only (no product code executed); cross-checked current guards against dbx evidence; dbx Cargo.lock SHA-256 verified against pinned value
CHECKPOINT=logs/research/M5-W18/A6-checkpoint.md
MERGE_NOTES=depends on A4 (security_policy.rs/domain.rs) for G1/G2, A3 (database.rs) for G3/G5/G6/G7, A5 UI for G1 confirm UX; W19 mandatory codes G1-G10 handed to A10 ledger + A11 acceptance matrix; no product edits in this lane
NEXT=W19 implementation cards for G1-G10 (await A0 architecture freeze); open questions: Mongo/Spanner drivers, confirm-SQL UX, history storage medium, SQLite cancel feasibility
```

## Summary

A6 完成 M5-W18-R 安全与生命周期审计：将 dbx 的 10 个安全/生命周期主题逐条比对当前产品（M4 A3/A4 守卫），
形成精确源映射与 10 项强制 W19 策略/测试码位（G1–G10）。研究确认当前产品在凭据存储（OS keyring）、
结果上限、`ShutdownCoordinator` 上**优于** dbx 默认路径；但在「确认 SQL 文本比对、跨库生产判定、服务端取消、
连接配置/历史持久化脱敏、导出净化、超时下限、DSN 脱敏广度、加密兜底」上存在可补强差距。

全部为 RESEARCH ONLY：未修改任何产品代码，未 push，仅在 `logs/research/M5-W18/` 写入报告与 checkpoint。

## Debt / 遗留

- 本波只研究，不落地产品代码；G1–G10 为 W19 强制项，须由 A0 开档时转为实现卡 + 策略门禁。
- `DB_PERSIST_NOT_ATOMIC`（现有 PENDING）随 G5 落地后转 ACTIVE。
- 许可：dbx Apache-2.0；任何移植单元须保留 NOTICE 与修改声明（交 A10 ledger）。
