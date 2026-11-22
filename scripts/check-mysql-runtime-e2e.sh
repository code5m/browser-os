#!/usr/bin/env bash
# Isolated real-service tests. No workstation profiles, credentials or databases.
set -euo pipefail
export CARGO_TERM_COLOR=never

mysql_e2e_tmp=$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/browseros-mysql-e2e.XXXXXX")
mysql_e2e_ca_owned=0
mysql_e2e_hosts_added=0
mysql_e2e_ca=""
mysql_e2e_marker=""

cleanup() {
  local status=$?
  trap - EXIT
  if (( mysql_e2e_ca_owned )); then
    sudo rm -f -- "$mysql_e2e_ca" || status=1
    sudo update-ca-certificates >/dev/null || status=1
  fi
  if (( mysql_e2e_hosts_added )); then
    sudo python3 - "$mysql_e2e_marker" <<'PY' || status=1
from pathlib import Path
import sys
path = Path("/etc/hosts")
marker = " # " + sys.argv[1]
lines = path.read_text().splitlines(keepends=True)
path.write_text("".join(line for line in lines if not line.rstrip("\n").endswith(marker)))
PY
  fi
  rm -rf -- "$mysql_e2e_tmp" || status=1
  exit "$status"
}
trap cleanup EXIT

run_checked() {
  local filter=$1
  shift
  local log="$mysql_e2e_tmp/results.log"
  local exact=()
  if (( $# == 1 )); then exact=(--exact); fi
  cargo test --manifest-path src-tauri/Cargo.toml --locked --bin mvp-browser-os "$filter" \
    -- --ignored --show-output --color never --test-threads=1 "${exact[@]}" 2>&1 | tee "$log"
  python3 - "$log" "$@" <<'PY'
from pathlib import Path
import re
import sys

text = Path(sys.argv[1]).read_text()
expected = sys.argv[2:]
actual = re.findall(r"^test (\S+) \.\.\. (\w+)\s*$", text, re.MULTILINE)
if len(actual) != len(expected) or set(actual) != {(name, "ok") for name in expected}:
    raise SystemExit(f"MYSQL_E2E_TEST_SET_MISMATCH expected={expected!r} actual={actual!r}")
summaries = re.findall(r"^test result: ok\. (\d+) passed; 0 failed; 0 ignored;", text, re.MULTILINE)
if summaries != [str(len(expected))] or not expected:
    raise SystemExit(f"MYSQL_E2E_NONZERO_RESULT_REQUIRED summaries={summaries!r}")
print(f"MYSQL_E2E_EXECUTED={len(expected)} " + ", ".join(expected))
PY
}

case "${1:-}" in
  drivers)
    run_checked mysql_real_e2e \
      capabilities::database::database::tests::mysql_real_e2e \
      capabilities::database::mysql_studio_e2e::mysql_real_e2e_table_constraint_error_rolls_back_entire_batch \
      capabilities::database::mysql_studio_e2e::mysql_real_e2e_table_late_conflict_rolls_back_entire_batch \
      capabilities::database::mysql_studio_e2e::mysql_real_e2e_table_commit_rollback_and_ticket_ownership
    exit 0
    ;;
  postgres)
    run_checked capabilities::database::database::tests::postgres_real_e2e_bigint_binary_roundtrip \
      capabilities::database::database::tests::postgres_real_e2e_bigint_binary_roundtrip
    exit 0
    ;;
  tls) ;;
  *) printf '%s\n' 'usage: check-mysql-runtime-e2e.sh drivers|postgres|tls' >&2; exit 2 ;;
esac

# The only mutable services/trust entries belong to this disposable Actions job.
[[ "${GITHUB_ACTIONS:-}" == true && "${RUNNER_OS:-}" == Linux ]]
[[ "${GITHUB_RUN_ID:-}" =~ ^[0-9]+$ && "${GITHUB_RUN_ATTEMPT:-}" =~ ^[0-9]+$ ]]
[[ "${BROWSEROS_MYSQL_TLS_CONTAINER:-}" =~ ^[a-f0-9]{64}$ ]]
[[ "${BROWSEROS_MYSQL_TLS_E2E_PASSWORD:-}" =~ ^[A-Za-z0-9_-]{20,100}$ ]]
: "${BROWSEROS_MYSQL_ROOT_PASSWORD:?disposable service root password is required}"
mysql_e2e_ca="/usr/local/share/ca-certificates/browseros-mysql-ci-${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}.crt"
mysql_e2e_marker="browseros-mysql-ci-${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}"
[[ ! -e "$mysql_e2e_ca" ]]
umask 077

# Both leaf certificates have identical valid SANs; only their signing CA differs.
for authority in trusted untrusted; do
  openssl req -x509 -newkey rsa:2048 -nodes -sha256 -days 1 \
    -subj "/CN=BrowserOS MySQL CI $authority" \
    -addext 'basicConstraints=critical,CA:TRUE' \
    -addext 'keyUsage=critical,keyCertSign,cRLSign' \
    -keyout "$mysql_e2e_tmp/$authority-ca.key" \
    -out "$mysql_e2e_tmp/$authority-ca.crt" >/dev/null 2>&1
done
openssl req -new -newkey rsa:2048 -nodes -sha256 -subj '/CN=localhost' \
  -keyout "$mysql_e2e_tmp/server.key" -out "$mysql_e2e_tmp/server.csr" >/dev/null 2>&1
cat >"$mysql_e2e_tmp/server.ext" <<'EOF'
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
subjectAltName=DNS:localhost,IP:127.0.0.1
EOF
for authority in trusted untrusted; do
  openssl x509 -req -sha256 -days 1 -in "$mysql_e2e_tmp/server.csr" \
    -CA "$mysql_e2e_tmp/$authority-ca.crt" -CAkey "$mysql_e2e_tmp/$authority-ca.key" \
    -CAcreateserial -extfile "$mysql_e2e_tmp/server.ext" \
    -out "$mysql_e2e_tmp/$authority-server.crt" >/dev/null 2>&1
done
openssl verify -CAfile "$mysql_e2e_tmp/trusted-ca.crt" -verify_hostname localhost "$mysql_e2e_tmp/trusted-server.crt"
openssl verify -CAfile "$mysql_e2e_tmp/trusted-ca.crt" -verify_ip 127.0.0.1 "$mysql_e2e_tmp/trusted-server.crt"

mysql_e2e_ca_owned=1
sudo install -m 0644 "$mysql_e2e_tmp/trusted-ca.crt" "$mysql_e2e_ca"
sudo update-ca-certificates >/dev/null
sudo python3 - "$mysql_e2e_marker" <<'PY'
from pathlib import Path
import sys
path = Path("/etc/hosts")
text = path.read_text()
alias = "browseros-mysql-mismatch.invalid"
if any(alias in line.split("#", 1)[0].split() for line in text.splitlines()):
    raise SystemExit("MYSQL_TLS_ALIAS_ALREADY_PRESENT")
with path.open("a") as output:
    output.write("\n127.0.0.1 " + alias + " # " + sys.argv[1] + "\n")
PY
mysql_e2e_hosts_added=1

mysql_e2e_remote="/var/lib/mysql/browseros-ci-tls"
docker exec --user root "$BROWSEROS_MYSQL_TLS_CONTAINER" mkdir -m 0700 "$mysql_e2e_remote"
docker cp "$mysql_e2e_tmp/server.key" "$BROWSEROS_MYSQL_TLS_CONTAINER:$mysql_e2e_remote/server.key"
for authority in trusted untrusted; do
  docker cp "$mysql_e2e_tmp/$authority-ca.crt" "$BROWSEROS_MYSQL_TLS_CONTAINER:$mysql_e2e_remote/$authority-ca.crt"
  docker cp "$mysql_e2e_tmp/$authority-server.crt" "$BROWSEROS_MYSQL_TLS_CONTAINER:$mysql_e2e_remote/$authority-server.crt"
done
docker exec --user root "$BROWSEROS_MYSQL_TLS_CONTAINER" chown -R mysql:mysql "$mysql_e2e_remote"

mysql_admin() {
  docker exec -i --env "MYSQL_PWD=$BROWSEROS_MYSQL_ROOT_PASSWORD" "$BROWSEROS_MYSQL_TLS_CONTAINER" \
    mysql --protocol=SOCKET --user=root --batch --skip-column-names
}
mysql_admin <<SQL
SET GLOBAL ssl_ca = '$mysql_e2e_remote/trusted-ca.crt';
SET GLOBAL ssl_cert = '$mysql_e2e_remote/trusted-server.crt';
SET GLOBAL ssl_key = '$mysql_e2e_remote/server.key';
ALTER INSTANCE RELOAD TLS;
CREATE TABLE browseros_e2e.browseros_tls_readonly_fixture (id INT PRIMARY KEY, label VARCHAR(32) NOT NULL);
INSERT INTO browseros_e2e.browseros_tls_readonly_fixture VALUES (1, 'original');
CREATE USER 'browseros_tls_readonly'@'%' IDENTIFIED BY '$BROWSEROS_MYSQL_TLS_E2E_PASSWORD' REQUIRE SSL;
GRANT SELECT ON browseros_e2e.browseros_tls_readonly_fixture TO 'browseros_tls_readonly'@'%';
SQL

export BROWSEROS_MYSQL_TLS_E2E=1
run_checked capabilities::database::mysql_tls_e2e::mysql_tls_e2e_prefer_and_require_encrypt_and_enforce_readonly \
  capabilities::database::mysql_tls_e2e::mysql_tls_e2e_prefer_and_require_encrypt_and_enforce_readonly
run_checked capabilities::database::mysql_tls_e2e::mysql_tls_e2e_hostname_mismatch_is_rejected \
  capabilities::database::mysql_tls_e2e::mysql_tls_e2e_hostname_mismatch_is_rejected

# No verification is disabled: swap only the server chain to an untrusted CA.
mysql_admin <<SQL
SET GLOBAL ssl_ca = '$mysql_e2e_remote/untrusted-ca.crt';
SET GLOBAL ssl_cert = '$mysql_e2e_remote/untrusted-server.crt';
ALTER INSTANCE RELOAD TLS;
SQL
run_checked capabilities::database::mysql_tls_e2e::mysql_tls_e2e_untrusted_ca_is_rejected \
  capabilities::database::mysql_tls_e2e::mysql_tls_e2e_untrusted_ca_is_rejected
