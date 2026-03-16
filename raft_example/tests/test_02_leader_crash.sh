#!/usr/bin/env bash
# test_02_leader_crash.sh -- Leader crash, re-election, and data consistency
#
# Scenario:
#   1. Write data to the leader
#   2. Simulate leader crash (network isolation via iptables)
#   3. Wait for a new leader to be elected
#   4. Verify previously committed data is still available
#   5. Write new data to the new leader
#   6. Restore the old leader
#   7. Verify all data converges (no split-brain)

# Guard: only source helpers if not already loaded (standalone mode)
if [[ -z "${TESTS_PASSED+x}" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    source "$SCRIPT_DIR/helpers.sh"
fi

log_section "TEST 02: Leader Crash & Data Consistency"

# -----------------------------------------------------------------------
# 1. Ensure cluster is ready and find the leader
# -----------------------------------------------------------------------
log_info "Step 1: Find current leader"
wait_for_cluster 20 || true

LEADER=$(wait_for_leader 15 || echo "")
if [[ -z "$LEADER" ]]; then
    log_fail "No leader found at start of test 02"
    log_info "Test 02 complete (aborted)."
    return 0 2>/dev/null || true
fi
log_pass "Initial leader: $LEADER"

# -----------------------------------------------------------------------
# 2. Write data to the leader (pre-crash data)
# -----------------------------------------------------------------------
log_info "Step 2: Write pre-crash data"

kv_set "$LEADER" "precrash_key1" "value1" >/dev/null
kv_set "$LEADER" "precrash_key2" "value2" >/dev/null
kv_set "$LEADER" "precrash_key3" "value3" >/dev/null

result=$(kv_get "$LEADER" "precrash_key1")
assert_eq "$result" "value1" "Pre-crash SET/GET precrash_key1"
result=$(kv_get "$LEADER" "precrash_key2")
assert_eq "$result" "value2" "Pre-crash SET/GET precrash_key2"
result=$(kv_get "$LEADER" "precrash_key3")
assert_eq "$result" "value3" "Pre-crash SET/GET precrash_key3"

# Wait for replication to propagate
sleep 3

# -----------------------------------------------------------------------
# 3. Crash the leader (network isolation)
# -----------------------------------------------------------------------
log_info "Step 3: Isolating leader $LEADER from the network"
OLD_LEADER="$LEADER"
pause_node "$OLD_LEADER"

# -----------------------------------------------------------------------
# 4. Wait for new leader election
# -----------------------------------------------------------------------
log_info "Step 4: Waiting for new leader election among remaining nodes"
sleep 5

FOLLOWERS=($(get_follower_addrs "$OLD_LEADER"))
NEW_LEADER=""
for attempt in $(seq 1 20); do
    for node in "${FOLLOWERS[@]}"; do
        if result=$(timeout 3 env NO_REDIRECT=1 raft-test-client "$node" get "__election_probe_t2__" 2>/dev/null); then
            NEW_LEADER="$node"
            break 2
        fi
    done
    sleep 1
done

if [[ -n "$NEW_LEADER" ]]; then
    log_pass "New leader elected: $NEW_LEADER (old was: $OLD_LEADER)"
else
    log_fail "No new leader elected after old leader crashed"
    unpause_node "$OLD_LEADER"
    log_info "Test 02 complete (aborted at step 4)."
    return 0 2>/dev/null || true
fi

if [[ "$NEW_LEADER" != "$OLD_LEADER" ]]; then
    log_pass "New leader is different from crashed leader"
else
    log_fail "New leader is the same as crashed leader"
fi

# -----------------------------------------------------------------------
# 5. Verify pre-crash data survived on the new leader
# -----------------------------------------------------------------------
log_info "Step 5: Verify pre-crash data on new leader"

result=$(kv_get "$NEW_LEADER" "precrash_key1")
assert_eq "$result" "value1" "Post-crash GET precrash_key1 from new leader"

result=$(kv_get "$NEW_LEADER" "precrash_key2")
assert_eq "$result" "value2" "Post-crash GET precrash_key2 from new leader"

result=$(kv_get "$NEW_LEADER" "precrash_key3")
assert_eq "$result" "value3" "Post-crash GET precrash_key3 from new leader"

# -----------------------------------------------------------------------
# 6. Write new data while old leader is down
# -----------------------------------------------------------------------
log_info "Step 6: Write new data to new leader while old leader is down"

kv_set "$NEW_LEADER" "postcrash_key1" "new_value1" >/dev/null
kv_set "$NEW_LEADER" "postcrash_key2" "new_value2" >/dev/null

result=$(kv_get "$NEW_LEADER" "postcrash_key1")
assert_eq "$result" "new_value1" "Post-crash SET/GET postcrash_key1"

result=$(kv_get "$NEW_LEADER" "postcrash_key2")
assert_eq "$result" "new_value2" "Post-crash SET/GET postcrash_key2"

# -----------------------------------------------------------------------
# 7. Update a pre-crash key on the new leader
# -----------------------------------------------------------------------
log_info "Step 7: Update pre-crash key on new leader"

kv_set "$NEW_LEADER" "precrash_key1" "updated_value1" >/dev/null

result=$(kv_get "$NEW_LEADER" "precrash_key1")
assert_eq "$result" "updated_value1" "Updated precrash_key1 on new leader"

# -----------------------------------------------------------------------
# 8. Restore old leader and verify convergence
# -----------------------------------------------------------------------
log_info "Step 8: Restoring old leader and checking convergence"
unpause_node "$OLD_LEADER"

sleep 8

CURRENT_LEADER=$(wait_for_leader 10 || echo "$NEW_LEADER")

result=$(kv_get "$CURRENT_LEADER" "precrash_key1")
assert_eq "$result" "updated_value1" "Convergence: precrash_key1 = updated_value1"

result=$(kv_get "$CURRENT_LEADER" "precrash_key2")
assert_eq "$result" "value2" "Convergence: precrash_key2 = value2"

result=$(kv_get "$CURRENT_LEADER" "precrash_key3")
assert_eq "$result" "value3" "Convergence: precrash_key3 = value3"

result=$(kv_get "$CURRENT_LEADER" "postcrash_key1")
assert_eq "$result" "new_value1" "Convergence: postcrash_key1 = new_value1"

result=$(kv_get "$CURRENT_LEADER" "postcrash_key2")
assert_eq "$result" "new_value2" "Convergence: postcrash_key2 = new_value2"

# -----------------------------------------------------------------------
# 9. Verify cluster is fully operational after recovery
# -----------------------------------------------------------------------
log_info "Step 9: Verify cluster operational after recovery"

kv_set "$CURRENT_LEADER" "recovery_key" "recovered" >/dev/null
sleep 2
result=$(kv_get "$CURRENT_LEADER" "recovery_key")
assert_eq "$result" "recovered" "Post-recovery write works: recovery_key"

log_info "Test 02 complete."
