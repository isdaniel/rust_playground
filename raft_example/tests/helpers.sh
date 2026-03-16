#!/usr/bin/env bash
# helpers.sh -- shared utilities for Raft integration tests
set -euo pipefail

# --- Cluster addresses ---
NODE1="172.20.0.11:9001"
NODE2="172.20.0.12:9002"
NODE3="172.20.0.13:9003"
ALL_NODES=("$NODE1" "$NODE2" "$NODE3")
NODE_CONTAINERS=("raft-node1" "raft-node2" "raft-node3")

# --- Colours ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Colour

# --- Counters ---
TESTS_PASSED=0
TESTS_FAILED=0

# ---------------------------------------------------------------------------
# Logging helpers
# ---------------------------------------------------------------------------
log_info()  { echo -e "${CYAN}[INFO]${NC}  $*"; }
log_pass()  { echo -e "${GREEN}[PASS]${NC}  $*"; TESTS_PASSED=$((TESTS_PASSED + 1)); }
log_fail()  { echo -e "${RED}[FAIL]${NC}  $*";  TESTS_FAILED=$((TESTS_FAILED + 1)); }
log_warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
log_section() { echo -e "\n${CYAN}========================================${NC}"; echo -e "${CYAN}  $*${NC}"; echo -e "${CYAN}========================================${NC}"; }

# ---------------------------------------------------------------------------
# assert_eq <actual> <expected> <test_name>
# ---------------------------------------------------------------------------
assert_eq() {
    local actual="$1"
    local expected="$2"
    local name="$3"
    if [[ "$actual" == "$expected" ]]; then
        log_pass "$name (got: '$actual')"
    else
        log_fail "$name -- expected '$expected', got '$actual'"
    fi
}

# ---------------------------------------------------------------------------
# assert_contains <haystack> <needle> <test_name>
# ---------------------------------------------------------------------------
assert_contains() {
    local haystack="$1"
    local needle="$2"
    local name="$3"
    if echo "$haystack" | grep -q "$needle"; then
        log_pass "$name"
    else
        log_fail "$name -- '$haystack' does not contain '$needle'"
    fi
}

# ---------------------------------------------------------------------------
# assert_not_empty <value> <test_name>
# ---------------------------------------------------------------------------
assert_not_empty() {
    local value="$1"
    local name="$2"
    if [[ -n "$value" ]]; then
        log_pass "$name (value: '$value')"
    else
        log_fail "$name -- value is empty"
    fi
}

# ---------------------------------------------------------------------------
# kv_set <addr> <key> <value> -- returns output, exit 0 on success
# ---------------------------------------------------------------------------
kv_set() {
    local addr="$1" key="$2" value="$3"
    local out
    out=$(timeout 10 raft-test-client "$addr" set "$key" "$value" 2>&1) || true
    echo "$out"
}

# ---------------------------------------------------------------------------
# kv_get <addr> <key> -- returns output
# ---------------------------------------------------------------------------
kv_get() {
    local addr="$1" key="$2"
    local out
    out=$(timeout 10 raft-test-client "$addr" get "$key" 2>&1) || true
    echo "$out"
}

# ---------------------------------------------------------------------------
# kv_delete <addr> <key> -- returns output
# ---------------------------------------------------------------------------
kv_delete() {
    local addr="$1" key="$2"
    local out
    out=$(timeout 10 raft-test-client "$addr" delete "$key" 2>&1) || true
    echo "$out"
}

# ---------------------------------------------------------------------------
# wait_for_cluster <timeout_secs> -- wait until at least one node answers
# ---------------------------------------------------------------------------
wait_for_cluster() {
    local timeout="${1:-30}"
    local deadline=$((SECONDS + timeout))
    log_info "Waiting for cluster to be ready (timeout: ${timeout}s)..."
    while [[ $SECONDS -lt $deadline ]]; do
        for node in "${ALL_NODES[@]}"; do
            if result=$(timeout 3 raft-test-client "$node" get "__probe__" 2>/dev/null); then
                log_info "Cluster is ready (node $node responded)"
                return 0
            fi
        done
        sleep 1
    done
    log_warn "Cluster may not be fully ready after ${timeout}s"
    return 1
}

# ---------------------------------------------------------------------------
# find_leader -- prints the addr of the current leader (tries all nodes)
#   Uses NO_REDIRECT=1 GET to directly identify the leader node.
# ---------------------------------------------------------------------------
find_leader() {
    for node in "${ALL_NODES[@]}"; do
        local result exit_code
        result=$(timeout 3 env NO_REDIRECT=1 raft-test-client "$node" get "__leader_probe__" 2>/dev/null)
        exit_code=$?
        # Exit code 0 means the node IS the leader (served the read directly)
        if [[ $exit_code -eq 0 ]]; then
            echo "$node"
            return 0
        fi
        # Exit code 2 means NOT_LEADER with possible redirect hint
        if [[ $exit_code -eq 2 ]]; then
            local leader_addr
            leader_addr=$(echo "$result" | grep -o 'NOT_LEADER:[^ ]*' | sed 's/NOT_LEADER://')
            if [[ -n "$leader_addr" && "$leader_addr" != "unknown" ]]; then
                echo "$leader_addr"
                return 0
            fi
        fi
    done
    echo ""
    return 1
}

# ---------------------------------------------------------------------------
# wait_for_leader <timeout_secs> -- wait until a leader is elected
# ---------------------------------------------------------------------------
wait_for_leader() {
    local timeout="${1:-30}"
    local deadline=$((SECONDS + timeout))
    log_info "Waiting for leader election (timeout: ${timeout}s)..." >&2
    while [[ $SECONDS -lt $deadline ]]; do
        local leader
        leader=$(find_leader 2>/dev/null) || true
        if [[ -n "$leader" ]]; then
            log_info "Leader found: $leader" >&2
            echo "$leader"
            return 0
        fi
        sleep 1
    done
    log_warn "No leader found after ${timeout}s" >&2
    echo ""
    return 1
}

# ---------------------------------------------------------------------------
# node_addr_to_container <addr> -- maps address to container name
# ---------------------------------------------------------------------------
node_addr_to_container() {
    local addr="$1"
    case "$addr" in
        *9001*) echo "raft-node1" ;;
        *9002*) echo "raft-node2" ;;
        *9003*) echo "raft-node3" ;;
        *) echo "unknown" ;;
    esac
}

# ---------------------------------------------------------------------------
# node_addr_to_index <addr> -- maps address to index (0,1,2)
# ---------------------------------------------------------------------------
node_addr_to_index() {
    local addr="$1"
    case "$addr" in
        *9001*) echo "0" ;;
        *9002*) echo "1" ;;
        *9003*) echo "2" ;;
        *) echo "-1" ;;
    esac
}

# ---------------------------------------------------------------------------
# get_follower_addrs <leader_addr> -- prints follower addresses
# ---------------------------------------------------------------------------
get_follower_addrs() {
    local leader="$1"
    for node in "${ALL_NODES[@]}"; do
        if [[ "$node" != "$leader" ]]; then
            echo "$node"
        fi
    done
}

# ---------------------------------------------------------------------------
# pause_node <node_addr> -- pauses the Docker container (freezes all processes)
#   This fully simulates a crash: no heartbeats, no elections, no responses.
# ---------------------------------------------------------------------------
pause_node() {
    local addr="$1"
    local container
    container=$(node_addr_to_container "$addr")
    log_info "Pausing container $container (simulating crash of $addr)"
    curl -s --unix-socket /var/run/docker.sock -X POST "http://localhost/containers/${container}/pause" >/dev/null 2>&1 || true
}

unpause_node() {
    local addr="$1"
    local container
    container=$(node_addr_to_container "$addr")
    log_info "Unpausing container $container (recovering $addr)"
    curl -s --unix-socket /var/run/docker.sock -X POST "http://localhost/containers/${container}/unpause" >/dev/null 2>&1 || true
}

# Keep iptables-based functions as alternatives for network partition tests
block_node_traffic() {
    local node_ip="${1%%:*}"
    log_info "Blocking traffic to/from $node_ip"
    iptables -A INPUT  -s "$node_ip" -j DROP 2>/dev/null || true
    iptables -A OUTPUT -d "$node_ip" -j DROP 2>/dev/null || true
}

unblock_node_traffic() {
    local node_ip="${1%%:*}"
    log_info "Unblocking traffic to/from $node_ip"
    iptables -D INPUT  -s "$node_ip" -j DROP 2>/dev/null || true
    iptables -D OUTPUT -d "$node_ip" -j DROP 2>/dev/null || true
}

# ---------------------------------------------------------------------------
# print_summary
# ---------------------------------------------------------------------------
print_summary() {
    echo ""
    log_section "TEST SUMMARY"
    echo -e "  ${GREEN}Passed: ${TESTS_PASSED}${NC}"
    echo -e "  ${RED}Failed: ${TESTS_FAILED}${NC}"
    echo -e "  Total:  $((TESTS_PASSED + TESTS_FAILED))"
    echo ""
    if [[ $TESTS_FAILED -gt 0 ]]; then
        echo -e "  ${RED}SOME TESTS FAILED${NC}"
        return 1
    else
        echo -e "  ${GREEN}ALL TESTS PASSED${NC}"
        return 0
    fi
}
