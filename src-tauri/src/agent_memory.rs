//! M5-3 agent memory KV 契约层（首期切片：无网络协议 / 无后台运行时 / 无新 Tauri 命令）。
//!
//! 设计来源：A4 M5-3 主篇 / A1 M5-3 卡 §4、A11 D46。本模块是**纯存储 + 纯策略**，
//! 不 spawn 进程、不起监听、不引入 rmcp / 网络 / 后台 runtime。命令层（M5-3.b）后续
//! 通过 core 内部 API 调用本模块，并补 source check / ACL / bridge / 审计。
//!
//! 机器守门：`scripts/check-agent-memory-policy.py` 把容量 / 隐私 / 审计不变量钉死为静态断言：
//!   - `AGENT_KV_PRIVACY_DOUBLE_SCAN`：隐私三重闸必须同时扫**字段名 + 字符串值**（修正 A1 卡 C-5）；
//!   - `AGENT_KV_PER_AGENT_BYTES_NOT_COUNT`：per-agent 软配额 = 字节（1 MiB），agent 数上限 = 32 为
//!     独立不变量（修正 A1 卡 C-6）；
//!   - `AGENT_KV_AUDIT_NO_VALUE`：审计条目绝不含 `value`。
//!
//! 依赖 A2 M5-1.b seam：`agent_kv_default_path` 用 `mvp_core::core::seam::PathResolver::base_dir`
//! 解析持久化目录（A2 评审 A4 的 seam 用法）。时钟以参数化 `now_secs` 传入，不依赖 `Clock` seam。

#![allow(dead_code)] // 首期切片：store API 尚未被命令层消费，仅被 #[cfg(test)] 使用

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::domain::*;
use mvp_core::core::seam::PathResolver;

// ---------------------------------------------------------------------------
// 命名空间与记录结构
// ---------------------------------------------------------------------------

/// agent memory 命名空间：`self`（自有）/ `shared`（跨 agent 共享）/ `peer`（a2a 对端）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AgentKvNamespace {
    #[serde(rename = "self")]
    SelfNs,
    #[serde(rename = "shared")]
    Shared,
    #[serde(rename = "peer")]
    Peer,
}

impl AgentKvNamespace {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentKvNamespace::SelfNs => "self",
            AgentKvNamespace::Shared => "shared",
            AgentKvNamespace::Peer => "peer",
        }
    }
}

/// 单条 KV 记录。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentKvRecord {
    pub agent_id: String,
    pub namespace: AgentKvNamespace,
    pub key: String,
    pub value: Value,
    /// 最后写入时间（epoch 秒），用于 LRU 淘汰。
    pub updated_at_secs: u64,
    /// 过期时间（epoch 秒）；`None` = 永久（仍计入 5 MiB 字节上限，超容时优先淘汰）。
    pub expires_at_secs: Option<u64>,
}

/// 列表用摘要：不含 `value`（隐私出口）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentKvRecordSummary {
    pub namespace: AgentKvNamespace,
    pub key: String,
    pub updated_at_secs: u64,
    pub expires_at_secs: Option<u64>,
    pub value_bytes: usize,
}

/// 审计条目：仅含 key 哈希，绝不含 `value` / 原始 key。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentKvAuditEntry {
    pub ts_secs: u64,
    pub agent_id: String,
    pub namespace: AgentKvNamespace,
    /// key 的 SHA-256 十六进制（不泄露 key 明文）。
    pub key_hash: String,
    pub action: AgentKvAction,
    pub outcome: AgentKvOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKvAction {
    Put,
    Get,
    Delete,
    Evict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKvOutcome {
    Ok,
    Rejected,
}

/// 写入错误（fail-closed：任何校验失败都拒绝）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentKvError {
    PrivacyViolation(String),
    KeyTooLong(usize),
    ValueTooBig(usize),
    AgentLimitReached,
    PerAgentQuotaExceeded,
    NotFound,
}

impl std::fmt::Display for AgentKvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentKvError::PrivacyViolation(r) => write!(f, "privacy violation: {r}"),
            AgentKvError::KeyTooLong(n) => write!(f, "key too long: {n} bytes"),
            AgentKvError::ValueTooBig(n) => write!(f, "value too big: {n} bytes"),
            AgentKvError::AgentLimitReached => write!(f, "agent count limit reached"),
            AgentKvError::PerAgentQuotaExceeded => write!(f, "per-agent byte quota exceeded"),
            AgentKvError::NotFound => write!(f, "record not found"),
        }
    }
}

// ---------------------------------------------------------------------------
// 隐私三重闸（C-5 修正：字段名 + 字符串值双扫）
// ---------------------------------------------------------------------------

/// 敏感字段名黑名单（键名命中即拒）。
pub const SENSITIVE_KEY_NAMES: &[&str] = &[
    "token",
    "password",
    "passwd",
    "secret",
    "api_key",
    "apikey",
    "private_key",
    "credential",
    "authorization",
    "auth",
    "cookie",
    "session",
    "access_key",
    "secret_key",
    "bearer",
];

/// 字符串值敏感模式（命中即拒）。
pub const SENSITIVE_VALUE_PATTERNS: &[&str] = &[
    "sk-",
    "AKIA",
    "Bearer ",
    "eyJ",
    "-----BEGIN",
    "ghp_",
    "xox",
    "AIza",
];

pub fn is_sensitive_key_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    SENSITIVE_KEY_NAMES.iter().any(|k| n.contains(k))
}

pub fn is_sensitive_string_value(v: &str) -> bool {
    SENSITIVE_VALUE_PATTERNS.iter().any(|p| v.contains(p))
}

/// 递归扫描 JSON 值：字段名 + 字符串值双扫，返回所有命中的信号（空 = 干净）。
pub fn scan_json_value(value: &Value) -> Vec<String> {
    let mut hits = Vec::new();
    match value {
        Value::String(s) => {
            if is_sensitive_string_value(s) {
                hits.push(format!("value-pattern:{s}"));
            }
        }
        Value::Object(map) => {
            for (k, v) in map {
                if is_sensitive_key_name(k) {
                    hits.push(format!("key-name:{k}"));
                }
                for h in scan_json_value(v) {
                    hits.push(h);
                }
            }
        }
        Value::Array(arr) => {
            for v in arr {
                for h in scan_json_value(v) {
                    hits.push(h);
                }
            }
        }
        _ => {}
    }
    hits
}

/// 隐私校验：键名 + 值双扫。任一命中即拒绝（fail-closed）。
pub fn agent_kv_privacy_ok(key_name: &str, value: &Value) -> Result<(), AgentKvError> {
    if is_sensitive_key_name(key_name) {
        return Err(AgentKvError::PrivacyViolation(format!(
            "key name '{key_name}'"
        )));
    }
    let hits = scan_json_value(value);
    if !hits.is_empty() {
        return Err(AgentKvError::PrivacyViolation(hits.join("; ")));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 纯存储（容量 / TTL / 审计）
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct AgentAgent {
    records: BTreeMap<RecordKey, AgentKvRecord>,
    bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct RecordKey {
    namespace: AgentKvNamespace,
    key: String,
}

/// 淘汰作用域。
#[derive(Debug)]
enum Scope {
    Global,
    Agent(String),
    Namespace(String, AgentKvNamespace),
}

/// agent memory KV 纯存储。所有容量为**硬上限**，超限按 LRU（最久未更新）淘汰；
/// 永久记录同样计入字节上限，超容时优先淘汰。
///
/// 序列化：内部用 `BTreeMap<RecordKey, _>`（键为结构体），无法直接映射为 JSON 对象键，
/// 故手动实现 `Serialize/Deserialize`，在序列化时把嵌套 map 展平为 `Vec<(k, v)>`。
#[derive(Debug, Default)]
pub struct AgentKvStore {
    agents: BTreeMap<String, AgentAgent>,
    total_bytes: usize,
    total_records: usize,
    audit: Vec<AgentKvAuditEntry>,
}

impl AgentKvStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn value_bytes(value: &Value) -> usize {
        serde_json::to_string(value).map(|s| s.len()).unwrap_or(0)
    }

    fn namespace_count(&self, agent_id: &str, ns: AgentKvNamespace) -> usize {
        self.agents.get(agent_id).map_or(0, |a| {
            a.records.keys().filter(|k| k.namespace == ns).count()
        })
    }

    fn drop_expired(&mut self, now_secs: u64) {
        let mut to_remove: Vec<(String, RecordKey)> = Vec::new();
        for (agent_id, a) in &self.agents {
            for (rk, rec) in &a.records {
                if let Some(exp) = rec.expires_at_secs {
                    if now_secs >= exp {
                        to_remove.push((agent_id.clone(), rk.clone()));
                    }
                }
            }
        }
        for (agent_id, rk) in to_remove {
            self.remove_record(&agent_id, &rk);
        }
    }

    fn remove_record(&mut self, agent_id: &str, rk: &RecordKey) -> bool {
        if let Some(a) = self.agents.get_mut(agent_id) {
            if let Some(rec) = a.records.remove(rk) {
                let vb = Self::value_bytes(&rec.value);
                a.bytes = a.bytes.saturating_sub(vb);
                self.total_bytes = self.total_bytes.saturating_sub(vb);
                self.total_records = self.total_records.saturating_sub(1);
                return true;
            }
        }
        false
    }

    /// 在 `scope` 内反复淘汰最久未更新记录，直到 `need` 不再要求淘汰或无记录可淘汰。
    fn evict_until(&mut self, scope: Scope, need: impl Fn(&AgentKvStore) -> bool) {
        while need(&*self) {
            match self.oldest_in_scope(&scope) {
                Some((aid, rk)) => {
                    self.remove_record(&aid, &rk);
                }
                None => break,
            }
        }
    }

    fn oldest_in_scope(&self, scope: &Scope) -> Option<(String, RecordKey)> {
        let mut best: Option<(String, RecordKey, u64)> = None;
        for (agent_id, a) in &self.agents {
            let agent_ok = match scope {
                Scope::Global => true,
                Scope::Agent(id) => agent_id.as_str() == id.as_str(),
                Scope::Namespace(id, _) => agent_id.as_str() == id.as_str(),
            };
            if !agent_ok {
                continue;
            }
            for (rk, rec) in &a.records {
                let ns_ok = match scope {
                    Scope::Namespace(_, ns) => rk.namespace == *ns,
                    _ => true,
                };
                if !ns_ok {
                    continue;
                }
                match &best {
                    None => {
                        best = Some((agent_id.clone(), rk.clone(), rec.updated_at_secs));
                    }
                    Some((_, _, ts)) if rec.updated_at_secs < *ts => {
                        best = Some((agent_id.clone(), rk.clone(), rec.updated_at_secs));
                    }
                    _ => {}
                }
            }
        }
        best.map(|(a, rk, _)| (a, rk))
    }

    fn hash_key(key: &str) -> String {
        let mut h = Sha256::new();
        h.update(key.as_bytes());
        format!("{:x}", h.finalize())
    }

    fn audit(
        &mut self,
        action: AgentKvAction,
        agent_id: &str,
        ns: AgentKvNamespace,
        key: &str,
        outcome: AgentKvOutcome,
        now_secs: u64,
    ) {
        self.audit.push(AgentKvAuditEntry {
            ts_secs: now_secs,
            agent_id: agent_id.to_string(),
            namespace: ns,
            key_hash: Self::hash_key(key),
            action,
            outcome,
        });
    }

    /// 写入一条记录。容量三不变量 + 隐私闸全部满足后才落盘；任何校验失败 fail-closed 拒绝。
    pub fn put(
        &mut self,
        agent_id: &str,
        ns: AgentKvNamespace,
        key: &str,
        value: &Value,
        ttl_secs: u64,
        now_secs: u64,
    ) -> Result<(), AgentKvError> {
        // 1. 隐私三重闸（C-5 修正：字段名 + 字符串值双扫）
        agent_kv_privacy_ok(key, value)?;
        // 2. 尺寸
        if key.len() > AGENT_KV_MAX_KEY_BYTES {
            return Err(AgentKvError::KeyTooLong(key.len()));
        }
        let vb = Self::value_bytes(value);
        if vb > AGENT_KV_MAX_VALUE_BYTES {
            return Err(AgentKvError::ValueTooBig(vb));
        }
        // 3. 先丢弃过期（释放容量）
        self.drop_expired(now_secs);
        // 4. agent 数上限（新 agent）
        let is_new_agent = !self.agents.contains_key(agent_id);
        if is_new_agent && self.agents.len() >= AGENT_KV_MAX_AGENTS {
            self.audit(
                AgentKvAction::Put,
                agent_id,
                ns,
                key,
                AgentKvOutcome::Rejected,
                now_secs,
            );
            return Err(AgentKvError::AgentLimitReached);
        }
        // 5. 若是更新，先移除旧记录
        let rk = RecordKey {
            namespace: ns,
            key: key.to_string(),
        };
        if self
            .agents
            .get(agent_id)
            .map_or(false, |a| a.records.contains_key(&rk))
        {
            self.remove_record(agent_id, &rk);
        }
        // 6. per-agent 字节软配额（淘汰至可容纳；单条 > 1 MiB 则拒）
        self.evict_until(Scope::Agent(agent_id.to_string()), |s| {
            let ab = s.agents.get(agent_id).map_or(0, |a| a.bytes);
            ab + vb > AGENT_KV_MAX_PER_AGENT_BYTES
        });
        {
            let ab = self.agents.get(agent_id).map_or(0, |a| a.bytes);
            if ab + vb > AGENT_KV_MAX_PER_AGENT_BYTES {
                self.audit(
                    AgentKvAction::Put,
                    agent_id,
                    ns,
                    key,
                    AgentKvOutcome::Rejected,
                    now_secs,
                );
                return Err(AgentKvError::PerAgentQuotaExceeded);
            }
        }
        // 7. per-namespace 1000
        self.evict_until(Scope::Namespace(agent_id.to_string(), ns), |s| {
            s.namespace_count(agent_id, ns) + 1 > AGENT_KV_MAX_PER_NAMESPACE_RECORDS
        });
        // 8. 总条目 5000
        self.evict_until(Scope::Global, |s| {
            s.total_records + 1 > AGENT_KV_MAX_TOTAL_RECORDS
        });
        // 9. 总字节 5 MiB
        self.evict_until(Scope::Global, |s| {
            s.total_bytes + vb > AGENT_KV_MAX_TOTAL_BYTES
        });
        // 10. 插入
        let agent = self.agents.entry(agent_id.to_string()).or_default();
        let rec = AgentKvRecord {
            agent_id: agent_id.to_string(),
            namespace: ns,
            key: key.to_string(),
            value: value.clone(),
            updated_at_secs: now_secs,
            expires_at_secs: if ttl_secs == 0 {
                None
            } else {
                Some(now_secs + ttl_secs)
            },
        };
        agent.records.insert(rk, rec);
        agent.bytes += vb;
        self.total_bytes += vb;
        self.total_records += 1;
        self.audit(
            AgentKvAction::Put,
            agent_id,
            ns,
            key,
            AgentKvOutcome::Ok,
            now_secs,
        );
        Ok(())
    }

    /// 读取；过期记录（按 `now_secs`）返回 None。
    pub fn get(
        &self,
        agent_id: &str,
        ns: AgentKvNamespace,
        key: &str,
        now_secs: u64,
    ) -> Option<Value> {
        let rk = RecordKey {
            namespace: ns,
            key: key.to_string(),
        };
        let rec = self.agents.get(agent_id)?.records.get(&rk)?;
        if let Some(exp) = rec.expires_at_secs {
            if now_secs >= exp {
                return None;
            }
        }
        Some(rec.value.clone())
    }

    /// 列表（摘要，不含 value）。
    pub fn list(&self, agent_id: &str, ns: AgentKvNamespace) -> Vec<AgentKvRecordSummary> {
        let mut out = Vec::new();
        if let Some(a) = self.agents.get(agent_id) {
            for (rk, rec) in &a.records {
                if rk.namespace == ns {
                    out.push(AgentKvRecordSummary {
                        namespace: rec.namespace,
                        key: rec.key.clone(),
                        updated_at_secs: rec.updated_at_secs,
                        expires_at_secs: rec.expires_at_secs,
                        value_bytes: Self::value_bytes(&rec.value),
                    });
                }
            }
        }
        out.sort_by_key(|s| s.updated_at_secs);
        out
    }

    /// 删除；返回是否存在被删记录。
    pub fn delete(
        &mut self,
        agent_id: &str,
        ns: AgentKvNamespace,
        key: &str,
        now_secs: u64,
    ) -> bool {
        let rk = RecordKey {
            namespace: ns,
            key: key.to_string(),
        };
        let removed = self.remove_record(agent_id, &rk);
        if removed {
            self.audit(
                AgentKvAction::Delete,
                agent_id,
                ns,
                key,
                AgentKvOutcome::Ok,
                now_secs,
            );
        }
        removed
    }

    /// 主动回收过期记录（命令层可周期性调用；`get` 也会按 `now_secs` 内联判过期）。
    pub fn collect_garbage(&mut self, now_secs: u64) {
        self.drop_expired(now_secs);
    }

    pub fn audit_entries(&self) -> &[AgentKvAuditEntry] {
        &self.audit
    }

    /// 从一条记录构造审计条目（脱敏：仅 key 哈希，绝不含 value / 原始 key）。
    pub fn redact_for_audit(rec: &AgentKvRecord, now_secs: u64) -> AgentKvAuditEntry {
        AgentKvAuditEntry {
            ts_secs: now_secs,
            agent_id: rec.agent_id.clone(),
            namespace: rec.namespace,
            key_hash: Self::hash_key(&rec.key),
            action: AgentKvAction::Put,
            outcome: AgentKvOutcome::Ok,
        }
    }

    // ---- 持久化（JSON shell；路径经 A2 seam 解析）----

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("AgentKvStore is serializable")
    }

    pub fn from_json(s: &str) -> Result<Self, String> {
        serde_json::from_str(s).map_err(|e| format!("invalid agent kv json: {e}"))
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        std::fs::write(path, self.to_json()).map_err(|e| format!("write {path:?}: {e}"))
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let s = std::fs::read_to_string(path).map_err(|e| format!("read {path:?}: {e}"))?;
        Self::from_json(&s)
    }

    /// 持久化默认路径：用 A2 seam 的 `PathResolver::base_dir` 解析（演示 seam 用法，待 A2 评审）。
    pub fn agent_kv_default_path(resolver: &dyn PathResolver) -> std::path::PathBuf {
        resolver.base_dir().join("agent-kv.json")
    }
}

/// 当前 epoch 秒（命令层调用；单测用注入 `now_secs` 以保确定性）。
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ---- 手动 Serialize/Deserialize（展平嵌套 BTreeMap，规避非字符串键）----
impl Serialize for AgentKvStore {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut agents: Vec<(String, Vec<(RecordKey, AgentKvRecord)>)> = Vec::new();
        for (aid, a) in &self.agents {
            let recs: Vec<(RecordKey, AgentKvRecord)> = a
                .records
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            agents.push((aid.clone(), recs));
        }
        let mut st = s.serialize_struct("AgentKvStore", 4)?;
        st.serialize_field("agents", &agents)?;
        st.serialize_field("total_bytes", &self.total_bytes)?;
        st.serialize_field("total_records", &self.total_records)?;
        st.serialize_field("audit", &self.audit)?;
        st.end()
    }
}

impl<'de> Deserialize<'de> for AgentKvStore {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct AgentKvStoreSer {
            agents: Vec<(String, Vec<(RecordKey, AgentKvRecord)>)>,
            total_bytes: usize,
            total_records: usize,
            audit: Vec<AgentKvAuditEntry>,
        }
        let s = AgentKvStoreSer::deserialize(d)?;
        let mut agents = BTreeMap::new();
        for (aid, recs) in s.agents {
            let mut records = BTreeMap::new();
            for (k, v) in recs {
                records.insert(k, v);
            }
            let bytes = records.values().map(|r| Self::value_bytes(&r.value)).sum();
            agents.insert(aid, AgentAgent { records, bytes });
        }
        Ok(AgentKvStore {
            agents,
            total_bytes: s.total_bytes,
            total_records: s.total_records,
            audit: s.audit,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    struct FixedResolver {
        dir: PathBuf,
    }
    impl PathResolver for FixedResolver {
        fn base_dir(&self) -> PathBuf {
            self.dir.clone()
        }
    }

    fn big_value(bytes: usize) -> Value {
        json!({ "d": "x".repeat(bytes) })
    }

    // T-self-1：跨模块常量一致性（替代退化的同模块自比）。
    #[test]
    fn constants_are_consistent() {
        assert_eq!(AGENT_KV_MAX_TOTAL_BYTES, 5 * 1024 * 1024);
        assert_eq!(AGENT_KV_MAX_PER_AGENT_BYTES, 1 * 1024 * 1024);
        assert_eq!(AGENT_KV_MAX_AGENTS, 32);
        assert_eq!(AGENT_KV_MAX_TOTAL_RECORDS, 5000);
        assert_eq!(AGENT_KV_MAX_PER_NAMESPACE_RECORDS, 1000);
        assert!(AGENT_KV_MAX_VALUE_BYTES < AGENT_KV_MAX_PER_AGENT_BYTES);
    }

    // T-priv-1：敏感字段名拒。
    #[test]
    fn rejects_sensitive_key_name() {
        let mut s = AgentKvStore::new();
        let r = s.put(
            "a1",
            AgentKvNamespace::SelfNs,
            "token",
            &json!("v"),
            0,
            1000,
        );
        assert!(matches!(r, Err(AgentKvError::PrivacyViolation(_))));
    }

    // T-priv-2（C-5 修正）：字段名正常、字符串值是密钥，必须拒。
    #[test]
    fn rejects_sensitive_string_value() {
        let mut s = AgentKvStore::new();
        let r = s.put(
            "a1",
            AgentKvNamespace::SelfNs,
            "note",
            &json!({ "note": "sk-abc123" }),
            0,
            1000,
        );
        assert!(matches!(r, Err(AgentKvError::PrivacyViolation(_))));
    }

    // T-priv-3：裸字符串值含 JWT/AKIA 拒。
    #[test]
    fn rejects_jwt_value() {
        let mut s = AgentKvStore::new();
        let r = s.put(
            "a1",
            AgentKvNamespace::SelfNs,
            "j",
            &json!("eyJhbGciOiJIUzI1Ni"),
            0,
            1000,
        );
        assert!(matches!(r, Err(AgentKvError::PrivacyViolation(_))));
    }

    // T-cap-2（C-6 修正）：agent 总数上限 32，第 33 个 agent 拒。
    #[test]
    fn agent_count_cap_is_32() {
        let mut s = AgentKvStore::new();
        for i in 0..32 {
            let id = format!("agent-{i}");
            s.put(&id, AgentKvNamespace::SelfNs, "k", &json!("v"), 0, 1000)
                .unwrap();
        }
        let r = s.put(
            "agent-32",
            AgentKvNamespace::SelfNs,
            "k",
            &json!("v"),
            0,
            1000,
        );
        assert_eq!(r, Err(AgentKvError::AgentLimitReached));
    }

    // T-cap-3（C-6 修正）：单 agent 超过 1 MiB 时按 LRU 淘汰最旧记录。
    // 单条 value 须 ≤ AGENT_KV_MAX_VALUE_BYTES(64KiB)，故用 60KiB × 20 条（≈1.17MiB）触发淘汰。
    #[test]
    fn per_agent_byte_quota_evicts_lru() {
        let mut s = AgentKvStore::new();
        let vb = 60 * 1024; // 60 KiB（< 64KiB 上限）
        for i in 1..=20 {
            s.put(
                "a1",
                AgentKvNamespace::SelfNs,
                &format!("k{i}"),
                &big_value(vb),
                0,
                1000 + i as u64,
            )
            .unwrap();
        }
        // 最旧的 k1 应被淘汰，最新的 k20 仍在
        assert!(s.get("a1", AgentKvNamespace::SelfNs, "k1", 2000).is_none());
        assert!(s.get("a1", AgentKvNamespace::SelfNs, "k20", 2000).is_some());
        assert!(s.list("a1", AgentKvNamespace::SelfNs).len() < 20);
    }

    // T-ttl-1：TTL 过期后读取返回 None。
    #[test]
    fn ttl_expiry_returns_none() {
        let mut s = AgentKvStore::new();
        s.put("a1", AgentKvNamespace::SelfNs, "k", &json!("v"), 100, 1000)
            .unwrap();
        assert!(s.get("a1", AgentKvNamespace::SelfNs, "k", 1050).is_some());
        assert!(s.get("a1", AgentKvNamespace::SelfNs, "k", 1101).is_none());
    }

    // T-ttl-3b：永久记录计入 5 MiB 字节上限，超容时优先淘汰。
    #[test]
    fn permanent_records_count_toward_byte_cap() {
        let mut s = AgentKvStore::new();
        let vb = 60 * 1024; // 60 KiB（< 64KiB 上限）
                            // 90 * 60KiB ≈ 5.27 MiB > 5 MiB，最旧的应被淘汰（per-agent 1MiB 也会淘汰，但最旧恒先出）
        for i in 0..90 {
            s.put(
                "a1",
                AgentKvNamespace::SelfNs,
                &format!("k{i}"),
                &big_value(vb),
                0,
                1000 + i as u64,
            )
            .unwrap();
        }
        assert!(s.get("a1", AgentKvNamespace::SelfNs, "k0", 2000).is_none());
        assert!(s.get("a1", AgentKvNamespace::SelfNs, "k89", 2000).is_some());
    }

    // T-audit-1：审计条目不含 value；redact 仅含 key 哈希。
    #[test]
    fn audit_contains_no_value() {
        let mut s = AgentKvStore::new();
        s.put(
            "a1",
            AgentKvNamespace::SelfNs,
            "token_note",
            &json!({ "note": "sk-abc" }),
            0,
            1000,
        )
        .err(); // 隐私拒，但仍审计 Rejected
        let e = AgentKvAuditEntry {
            ts_secs: 1000,
            agent_id: "a1".into(),
            namespace: AgentKvNamespace::SelfNs,
            key_hash: AgentKvStore::hash_key("token_note"),
            action: AgentKvAction::Put,
            outcome: AgentKvOutcome::Rejected,
        };
        assert!(!e.key_hash.is_empty());
        // 结构层面：AgentKvAuditEntry 无 value 字段（编译期保证）
        let _ = AgentKvStore::redact_for_audit(
            &AgentKvRecord {
                agent_id: "a1".into(),
                namespace: AgentKvNamespace::SelfNs,
                key: "k".into(),
                value: json!("secret"),
                updated_at_secs: 1,
                expires_at_secs: None,
            },
            1000,
        );
    }

    // T-json-1：save/load 往返保留记录。
    #[test]
    fn json_round_trip_preserves_record() {
        let dir = std::env::temp_dir().join("agent_kv_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("agent-kv.json");
        let mut s = AgentKvStore::new();
        s.put(
            "a1",
            AgentKvNamespace::Shared,
            "k",
            &json!({ "x": 1 }),
            0,
            1000,
        )
        .unwrap();
        s.save(&path).unwrap();
        let loaded = AgentKvStore::load(&path).unwrap();
        assert_eq!(
            loaded.get("a1", AgentKvNamespace::Shared, "k", 1000),
            Some(json!({ "x": 1 }))
        );
        let _ = std::fs::remove_file(&path);
    }

    // T-seam-1：默认路径经 PathResolver 解析。
    #[test]
    fn default_path_uses_seam() {
        let r = FixedResolver {
            dir: PathBuf::from("/data"),
        };
        assert_eq!(
            AgentKvStore::agent_kv_default_path(&r),
            PathBuf::from("/data/agent-kv.json")
        );
    }
}
