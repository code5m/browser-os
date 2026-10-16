//! Real table-driver tests against mysql-runtime-e2e.yml's disposable service.
//! Never discover workstation profiles, credentials, hosts or database names.
//! The existing mysql_real_e2e filter runs these explicitly; ordinary unit tests
//! leave them ignored when the service is absent. These are not GUI tests.
use super::database::{DbPool, QueryCancel};
use super::table_data::{self, TableChange, TableRequest, TableState};
use crate::domain::DbConnectionConfig;
use mysql::prelude::Queryable;
use serde_json::{json, Value};
use std::collections::BTreeMap;

struct Fixture {
    pool: DbPool,
    observer: DbPool,
    cfg: DbConnectionConfig,
    state: TableState,
    table: String,
}

fn connection(pool: &mut DbPool) -> &mut mysql::Conn {
    match pool {
        DbPool::MySql(conn, _) => conn,
        _ => panic!("fixture must use the real MySQL driver"),
    }
}

impl Fixture {
    fn new() -> Self {
        let password = std::env::var("BROWSEROS_MYSQL_E2E_PASSWORD")
            .expect("requires the disposable CI MySQL service");
        let cfg: DbConnectionConfig = serde_json::from_value(json!({
            "id":"mysql-studio-e2e", "name":"isolated CI test", "kind":"mysql",
            "host":"127.0.0.1", "port":3306, "database":"browseros_e2e",
            "username":"browseros_ci", "ssl_mode":"disable", "allow_write":true,
            "production_hint":false, "enabled":true,
            "created_at":"2026-10-16T00:00:00Z", "updated_at":"2026-10-16T00:00:00Z"
        }))
        .unwrap();
        let mut pool = DbPool::connect(&cfg, Some(&password), &[]).expect("real MySQL writer");
        let observer = DbPool::connect(&cfg, Some(&password), &[]).expect("independent observer");
        drop(password);
        let table = format!("browseros_studio_e2e_{}", uuid::Uuid::new_v4().simple());
        connection(&mut pool)
            .query_drop(format!(
                "CREATE TABLE `{table}` (id INT PRIMARY KEY, value TEXT NOT NULL) ENGINE=InnoDB"
            ))
            .unwrap();
        connection(&mut pool)
            .query_drop(format!(
                "INSERT INTO `{table}` VALUES (1,'original one'),(2,'original two')"
            ))
            .unwrap();
        Self {
            pool,
            observer,
            cfg,
            state: TableState::default(),
            table,
        }
    }

    fn read(&mut self) -> Value {
        table_data::read(
            &mut self.pool,
            &self.cfg,
            &mut self.state,
            &TableRequest {
                name: self.table.clone(),
                ..Default::default()
            },
            &QueryCancel::new(),
        )
        .unwrap()
    }

    fn preview(&mut self, changes: Vec<TableChange>) -> TableRequest {
        let mut request = TableRequest {
            name: self.table.clone(),
            changes,
            ..Default::default()
        };
        let reply = table_data::preview(
            &mut self.pool,
            &self.cfg,
            &mut self.state,
            &request,
            &QueryCancel::new(),
        )
        .unwrap();
        assert_eq!(reply["requires_separate_commit"], true);
        request.ticket = Some(reply["ticket"].as_str().unwrap().to_owned());
        request
    }

    fn apply(&mut self, request: &TableRequest, confirmed: bool) -> Result<Value, String> {
        table_data::apply(
            &mut self.pool,
            &self.cfg,
            &mut self.state,
            request,
            confirmed,
            &QueryCancel::new(),
        )
    }

    fn observed(&mut self) -> Vec<(i64, String)> {
        connection(&mut self.observer)
            .query(format!("SELECT id,value FROM `{}` ORDER BY id", self.table))
            .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Test-only cleanup: never commit unverified changes, even on panic.
        let _ = connection(&mut self.pool).query_drop("ROLLBACK");
        let _ = connection(&mut self.observer).query_drop("ROLLBACK");
        let _ = connection(&mut self.observer)
            .query_drop(format!("DROP TABLE IF EXISTS `{}`", self.table));
    }
}

fn edit(row: &Value, text: &str) -> TableChange {
    TableChange {
        action: "update".into(),
        row_id: Some(row["row_id"].as_str().unwrap().to_owned()),
        values: BTreeMap::from([("value".into(), Some(text.into()))]),
    }
}

#[test]
#[ignore = "requires disposable MySQL service"]
fn mysql_real_e2e_table_commit_rollback_and_ticket_ownership() {
    let mut fixture = Fixture::new();
    let original = fixture.observed();
    let page = fixture.read();
    assert_eq!(page["editable"], true);
    assert_eq!(page["stable_order"], true);
    let literal = "quoted ' value; DROP TABLE not_a_statement; -- literal data";
    let request = fixture.preview(vec![edit(&page["rows"][0], literal)]);
    assert_eq!(fixture.observed(), original, "preview must not mutate rows");
    assert_eq!(
        fixture.apply(&request, false).unwrap_err(),
        "DB_TABLE_CONFIRMATION_REQUIRED"
    );
    let mut other_session_state = TableState::default();
    assert_eq!(
        table_data::apply(
            &mut fixture.pool,
            &fixture.cfg,
            &mut other_session_state,
            &request,
            true,
            &QueryCancel::new()
        )
        .unwrap_err(),
        "DB_TABLE_PREVIEW_EXPIRED"
    );
    fixture.apply(&request, true).unwrap();
    assert_eq!(fixture.observed(), original, "Apply is not Commit");
    let written: String = connection(&mut fixture.pool)
        .query_first(format!("SELECT value FROM `{}` WHERE id=1", fixture.table))
        .unwrap()
        .unwrap();
    assert_eq!(written, literal, "parameters must stay literal data");
    connection(&mut fixture.pool)
        .query_drop("ROLLBACK")
        .unwrap();
    assert_eq!(fixture.observed(), original);

    let page = fixture.read();
    let request = fixture.preview(vec![edit(&page["rows"][0], literal)]);
    fixture.apply(&request, true).unwrap();
    connection(&mut fixture.pool).query_drop("COMMIT").unwrap();
    assert_eq!(fixture.observed()[0].1, literal);
    assert_eq!(
        fixture.apply(&request, true).unwrap_err(),
        "DB_TABLE_PREVIEW_EXPIRED",
        "the same approved write cannot be replayed"
    );
}

#[test]
#[ignore = "requires disposable MySQL service"]
fn mysql_real_e2e_table_late_conflict_rolls_back_entire_batch() {
    let mut fixture = Fixture::new();
    let page = fixture.read();
    let request = fixture.preview(vec![
        edit(&page["rows"][0], "must be rolled back"),
        edit(&page["rows"][1], "stale second edit"),
    ]);
    connection(&mut fixture.observer)
        .query_drop(format!(
            "UPDATE `{}` SET value='concurrent edit' WHERE id=2",
            fixture.table
        ))
        .unwrap();
    let expected = fixture.observed();
    assert_eq!(
        fixture.apply(&request, true).unwrap_err(),
        "DB_TABLE_ROW_CHANGED_BATCH_ROLLED_BACK"
    );
    assert_eq!(
        fixture.observed(),
        expected,
        "failure on the second row must undo the first mutation"
    );
    assert_eq!(expected[0].1, "original one");
    assert_eq!(expected[1].1, "concurrent edit");
    // An ordinary MVCC read can see old data while locks are still held. A
    // second session's NOWAIT locking read also proves the writer released them.
    connection(&mut fixture.observer)
        .query_drop("START TRANSACTION")
        .unwrap();
    let locked: Vec<(i64, String)> = connection(&mut fixture.observer)
        .query(format!(
            "SELECT id,value FROM `{}` ORDER BY id FOR UPDATE NOWAIT",
            fixture.table
        ))
        .expect("rollback must release every row lock");
    assert_eq!(locked, expected);
    connection(&mut fixture.observer)
        .query_drop("ROLLBACK")
        .unwrap();
}
