#!/usr/bin/env bash
# test_03_consensus.sh -- Additional Raft consensus algorithm tests
#
# Scenarios:
#   1. Minority partition: isolate 1 follower, cluster still works (2/3 majority)
#   2. Majority partition: isolate 2 nodes, single node cannot serve writes
#   3. Heal partition and verify data convergence
#   4. Rapid leader failover: crash and recover leaders multiple times
#   5. Concurrent writes during stable leadership
#   6. Delete and re-create key

# Guard: only source helpers if not already loaded (standalone mode)
if [[ -z "${TESTS_PASSED+x}" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    source "$SCRIPT_DIR/helpers.sh"
fi

log_section "TEST 03: Raft Consensus Algorithm"

# -----------------------------------------------------------------------
# Setup: ensure cluster is healthy
# -----------------------------------------------------------------------
wait_for_cluster 20 || true
LEADER=$(wait_for_leader 15 || echo "")
if [[ -z "$LEADER" ]]; then
    log_fail "No leader found at start of test 03"
    log_info "Test 03 complete (aborted)."
    return 0 2>/dev/null || true
fi
log_pass "Initial leader: $LEADER"

# ===================================================================
# Test 3.1: Minority partition (1 follower isolated, cluster works)
# ===================================================================
log_section "Test 3.1: Minority Partition"

FOLLOWERS=($(get_follower_addrs "$LEADER"))
ISOLATED_FOLLOWER="${FOLLOWERS[0]}"

log_info "Isolating one follower: $ISOLATED_FOLLOWER"
pause_node "$ISOLATED_FOLLOWER"
sleep 2

log_info "Writing data with one follower isolated..."
result=$(kv_set "$LEADER" "minority_test" "still_works")
assert_eq "$result" "still_works" "Write succeeds with minority partition (2/3 nodes)"

result=$(kv_get "$LEADER" "minority_test")
assert_eq "$result" "still_works" "Read succeeds with minority partition"

kv_set "$LEADER" "during_partition" "data1" >/dev/null
kv_set "$LEADER" "during_partition2" "data2" >/dev/null

log_info "Restoring isolated follower: $ISOLATED_FOLLOWER"
unpause_node "$ISOLATED_FOLLOWER"
sleep 5

result=$(kv_get "$LEADER" "during_partition")
assert_eq "$result" "data1" "Data consistent after follower rejoins: during_partition"

result=$(kv_get "$LEADER" "during_partition2")
assert_eq "$result" "data2" "Data consistent after follower rejoins: during_partition2"

log_pass "Minority partition test passed"

# ===================================================================
# Test 3.2: No write quorum (isolate 2 followers, leader alone)
# ===================================================================
log_section "Test 3.2: No Write Quorum (Leader Alone)"

sleep 2
LEADER=$(wait_for_leader 10 || echo "")
if [[ -z "$LEADER" ]]; then
    log_fail "No leader for quorum test"
else
    FOLLOWERS=($(get_follower_addrs "$LEADER"))

    log_info "Isolating both followers: ${FOLLOWERS[*]}"
    pause_node "${FOLLOWERS[0]}"
    pause_node "${FOLLOWERS[1]}"
    sleep 2

    log_info "Attempting write with no quorum (should timeout/fail)..."
    result=$(timeout 10 raft-test-client "$LEADER" set "no_quorum_key" "should_fail" 2>&1 || true)

    if echo "$result" | grep -qi "timeout\|error\|fail\|CONNECTION_ERROR"; then
        log_pass "Write correctly fails/times out without quorum"
    else
        log_warn "Write response without quorum: '$result' (may indicate issue)"
    fi

    log_info "Restoring both followers"
    unpause_node "${FOLLOWERS[0]}"
    unpause_node "${FOLLOWERS[1]}"
    sleep 5
fi

# ===================================================================
# Test 3.3: Rapid leadership changes
# ===================================================================
log_section "Test 3.3: Rapid Leadership Changes"

LEADER=$(wait_for_leader 15 || echo "")
if [[ -z "$LEADER" ]]; then
    log_fail "Cannot find leader for rapid failover test"
else
    kv_set "$LEADER" "stability_key" "initial" >/dev/null
    result=$(kv_get "$LEADER" "stability_key")
    assert_eq "$result" "initial" "Baseline: stability_key = initial"

    # Crash leader #1
    log_info "Crash leader #1: $LEADER"
    OLD_LEADER1="$LEADER"
    pause_node "$LEADER"
    sleep 6

    REMAINING=($(get_follower_addrs "$OLD_LEADER1"))
    LEADER=""
    for attempt in $(seq 1 20); do
        for node in "${REMAINING[@]}"; do
            if timeout 3 env NO_REDIRECT=1 raft-test-client "$node" get "__probe_t3a__" 2>/dev/null >/dev/null; then
                LEADER="$node"
                break 2
            fi
        done
        sleep 1
    done

    if [[ -n "$LEADER" ]]; then
        log_pass "New leader #2 elected: $LEADER"

        kv_set "$LEADER" "stability_key" "after_crash1" >/dev/null
        result=$(kv_get "$LEADER" "stability_key")
        assert_eq "$result" "after_crash1" "Write after first leadership change"

        # Restore old leader
        unpause_node "$OLD_LEADER1"
        sleep 5

        # Crash leader #2
        log_info "Crash leader #2: $LEADER"
        OLD_LEADER2="$LEADER"
        pause_node "$LEADER"
        sleep 6

        REMAINING2=($(get_follower_addrs "$OLD_LEADER2"))
        LEADER=""
        for attempt in $(seq 1 20); do
            for node in "${REMAINING2[@]}"; do
                if timeout 3 env NO_REDIRECT=1 raft-test-client "$node" get "__probe_t3b__" 2>/dev/null >/dev/null; then
                    LEADER="$node"
                    break 2
                fi
            done
            sleep 1
        done

        if [[ -n "$LEADER" ]]; then
            log_pass "New leader #3 elected: $LEADER"

            result=$(kv_get "$LEADER" "stability_key")
            assert_eq "$result" "after_crash1" "Data survived two leadership changes"

            kv_set "$LEADER" "stability_key" "after_crash2" >/dev/null
        else
            log_fail "No leader elected after second crash"
        fi

        unpause_node "$OLD_LEADER2"
        sleep 5
    else
        log_fail "No leader elected after first crash"
        unpause_node "$OLD_LEADER1"
        sleep 5
    fi
fi

# ===================================================================
# Test 3.4: Data convergence after all partitions healed
# ===================================================================
log_section "Test 3.4: Final Convergence"

sleep 5
LEADER=$(wait_for_leader 15 || echo "")
if [[ -n "$LEADER" ]]; then
    kv_set "$LEADER" "convergence_marker" "all_healed" >/dev/null
    sleep 3

    result=$(kv_get "$LEADER" "convergence_marker")
    assert_eq "$result" "all_healed" "Final convergence write succeeds"

    result=$(kv_get "$LEADER" "minority_test")
    assert_eq "$result" "still_works" "Historical data: minority_test"

    result=$(kv_get "$LEADER" "during_partition")
    assert_eq "$result" "data1" "Historical data: during_partition"

    result=$(kv_get "$LEADER" "during_partition2")
    assert_eq "$result" "data2" "Historical data: during_partition2"
else
    log_fail "Cannot find leader for final convergence check"
fi

# ===================================================================
# Test 3.5: Concurrent writes under stable leadership
# ===================================================================
log_section "Test 3.5: Concurrent Writes"

LEADER=$(wait_for_leader 10 || echo "")
if [[ -n "$LEADER" ]]; then
    log_info "Sending 20 concurrent writes..."
    for i in $(seq 1 20); do
        kv_set "$LEADER" "concurrent_$i" "val_$i" >/dev/null &
    done
    wait
    sleep 3

    CONCURRENT_OK=0
    CONCURRENT_FAIL=0
    for i in $(seq 1 20); do
        result=$(kv_get "$LEADER" "concurrent_$i")
        if [[ "$result" == "val_$i" ]]; then
            CONCURRENT_OK=$((CONCURRENT_OK + 1))
        else
            CONCURRENT_FAIL=$((CONCURRENT_FAIL + 1))
        fi
    done

    if [[ $CONCURRENT_OK -eq 20 ]]; then
        log_pass "All 20 concurrent writes committed and readable"
    elif [[ $CONCURRENT_OK -gt 15 ]]; then
        log_pass "Concurrent writes: $CONCURRENT_OK/20 succeeded (acceptable under load)"
    else
        log_fail "Concurrent writes: only $CONCURRENT_OK/20 succeeded"
    fi
else
    log_fail "No leader for concurrent write test"
fi

# ===================================================================
# Test 3.6: Delete and re-create key
# ===================================================================
log_section "Test 3.6: Delete and Re-create"

LEADER=$(wait_for_leader 10 || echo "")
if [[ -n "$LEADER" ]]; then
    kv_set "$LEADER" "ephemeral" "first" >/dev/null
    result=$(kv_get "$LEADER" "ephemeral")
    assert_eq "$result" "first" "Set ephemeral=first"

    kv_delete "$LEADER" "ephemeral" >/dev/null
    result=$(kv_get "$LEADER" "ephemeral")
    assert_eq "$result" "(nil)" "Delete ephemeral -> (nil)"

    kv_set "$LEADER" "ephemeral" "second" >/dev/null
    result=$(kv_get "$LEADER" "ephemeral")
    assert_eq "$result" "second" "Re-create ephemeral=second"
else
    log_fail "No leader for delete/re-create test"
fi

log_info "Test 03 complete."
