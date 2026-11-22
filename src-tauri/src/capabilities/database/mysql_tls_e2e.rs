//! Real TLS and read-only boundaries against the disposable MySQL CI service.
//! Uses production connection/query policy; never loads workstation profiles or keys.
use super::database::{DbPool, DbValue, QueryCancel};
use crate::domain::{DbConnectionConfig, DbErrorCode, DbSslMode};
use crate::security_policy::evaluate_db_query_gate;
use serde_json::json;

fn fixture(host: &str, mode: DbSslMode) -> (DbConnectionConfig, String) {
    assert_eq!(
        std::env::var("BROWSEROS_MYSQL_TLS_E2E").as_deref(),
        Ok("1"),
        "requires the explicitly prepared disposable TLS service"
    );
    assert!(matches!(
        host,
        "127.0.0.1" | "localhost" | "browseros-mysql-mismatch.invalid"
    ));
    let password = std::env::var("BROWSEROS_MYSQL_TLS_E2E_PASSWORD")
        .expect("disposable read-only account password is required");
    let mut cfg: DbConnectionConfig = serde_json::from_value(json!({
        "id": "mysql-tls-ci",
        "name": "isolated TLS read-only test",
        "kind": "mysql",
        "host": host,
        "port": 3306,
        "database": "browseros_e2e",
        "username": "browseros_tls_readonly",
        "ssl_mode": "prefer",
        "allow_write": false,
        "production_hint": false,
        "enabled": true,
        "created_at": "2026-11-22T00:00:00Z",
        "updated_at": "2026-11-22T00:00:00Z"
    }))
    .unwrap();
    cfg.ssl_mode = mode;
    (cfg, password)
}

#[test]
#[ignore = "requires the disposable MySQL service with a trusted localhost certificate"]
fn mysql_tls_e2e_prefer_and_require_encrypt_and_enforce_readonly() {
    for (host, mode) in [
        ("127.0.0.1", DbSslMode::Prefer),
        ("localhost", DbSslMode::Require),
    ] {
        let (cfg, password) = fixture(host, mode);
        let mut pool = DbPool::connect(&cfg, Some(&password), &[])
            .expect("production TLS connection must succeed");
        assert_eq!(pool.encrypted(), Some(true));
        let cipher = pool
            .query(
                "SHOW SESSION STATUS LIKE 'Ssl_cipher'",
                &QueryCancel::new(),
                Some(10),
                "tls-cipher",
            )
            .unwrap();
        assert!(
            matches!(
                cipher.rows.first().and_then(|row| row.get(1)),
                Some(DbValue::Text(value)) if !value.is_empty()
            ),
            "server must independently report an active TLS cipher"
        );
        let select = "SELECT label FROM browseros_tls_readonly_fixture WHERE id = 1";
        evaluate_db_query_gate(select, &cfg, pool.encrypted(), false).unwrap();
        let before = pool
            .query(select, &QueryCancel::new(), Some(10), "tls-read")
            .unwrap();
        assert_eq!(
            before.rows,
            vec![vec![DbValue::Text("original".to_string())]]
        );
        let update = "UPDATE browseros_tls_readonly_fixture SET label = 'changed' WHERE id = 1";
        assert_eq!(
            evaluate_db_query_gate(update, &cfg, pool.encrypted(), true),
            Err(DbErrorCode::WriteDenied),
            "explicit confirmation cannot override a read-only profile"
        );
        // Exercise the server's SELECT-only grant independently of the UI gate,
        // through the same production connection and query implementation.
        let denied = pool
            .query(update, &QueryCancel::new(), Some(10), "tls-server-denial")
            .expect_err("the read-only server account must independently reject UPDATE");
        assert_eq!(denied.code, DbErrorCode::QueryFailed);
        let after = pool
            .query(select, &QueryCancel::new(), Some(10), "tls-read-after")
            .unwrap();
        assert_eq!(
            after.rows, before.rows,
            "the fixture row must remain unchanged"
        );
    }
}

fn rejected_certificate(host: &str, reasons: &[&str]) {
    for mode in [DbSslMode::Prefer, DbSslMode::Require] {
        let (cfg, password) = fixture(host, mode);
        let error = DbPool::connect(&cfg, Some(&password), &[])
            .expect_err("certificate failure must not establish or downgrade the connection");
        assert_eq!(error.code, DbErrorCode::ConnectFailed);
        assert!(
            !error.message.contains(&password),
            "password must not escape"
        );
        let message = error.message.to_ascii_lowercase();
        assert!(
            message.contains("certificate verify failed")
                || message.contains("certificate verification failed"),
            "expected a TLS verification failure, got: {message}"
        );
        assert!(
            reasons.iter().any(|reason| message.contains(reason)),
            "wrong TLS rejection reason: {message}"
        );
    }
}

#[test]
#[ignore = "requires the trusted MySQL TLS service and a loopback hostname alias"]
fn mysql_tls_e2e_hostname_mismatch_is_rejected() {
    rejected_certificate(
        "browseros-mysql-mismatch.invalid",
        &["hostname mismatch", "ip address mismatch"],
    );
}

#[test]
#[ignore = "requires the MySQL service switched to a separately signed untrusted certificate"]
fn mysql_tls_e2e_untrusted_ca_is_rejected() {
    rejected_certificate(
        "127.0.0.1",
        &[
            "unable to get local issuer certificate",
            "self-signed certificate",
            "unable to verify the first certificate",
        ],
    );
}
