#!/usr/bin/env bash
# test_01_normal_workflow.sh -- Tests basic Raft operations
#
# Scenario: Normal cluster operation
#   1. Cluster starts, a leader is elected
#   2. Set/Get/Delete operations work correctly
#   3. Data is consistent across follower reads (via leader redirect)
#   4. Multiple keys can be stored and retrieved

# Guard: only source helpers if not already loaded (standalone mode)
if [[ -z "${TESTS_PASSED+x}" ]]; then
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    source "$SCRIPT_DIR/helpers.sh"
fi

log_section "TEST 01: Normal Workflow"

# -----------------------------------------------------------------------
# 1. Wait for cluster and leader election
# -----------------------------------------------------------------------
log_info "Step 1: Verify cluster startup and leader election"
wait_for_cluster 30 || true

LEADER=$(wait_for_leader 15 || echo "")
if [[ -z "$LEADER" ]]; then
    log_fail "No leader elected"
else
    log_pass "Leader elected: $LEADER"

    # -------------------------------------------------------------------
    # 2. Basic Set and Get
    # -------------------------------------------------------------------
    log_info "Step 2: Basic Set and Get operations"

    result=$(kv_set "$LEADER" "name" "alice")
    assert_eq "$result" "alice" "SET name=alice"

    result=$(kv_get "$LEADER" "name")
    assert_eq "$result" "alice" "GET name -> alice"

    # -------------------------------------------------------------------
    # 3. Update existing key
    # -------------------------------------------------------------------
    log_info "Step 3: Update existing key"

    result=$(kv_set "$LEADER" "name" "bob")
    assert_eq "$result" "bob" "SET name=bob (update)"

    result=$(kv_get "$LEADER" "name")
    assert_eq "$result" "bob" "GET name -> bob (after update)"

    # -------------------------------------------------------------------
    # 4. Multiple keys
    # -------------------------------------------------------------------
    log_info "Step 4: Multiple keys"

    kv_set "$LEADER" "city" "tokyo" >/dev/null
    kv_set "$LEADER" "country" "japan" >/dev/null
    kv_set "$LEADER" "language" "rust" >/dev/null

    result=$(kv_get "$LEADER" "city")
    assert_eq "$result" "tokyo" "GET city -> tokyo"

    result=$(kv_get "$LEADER" "country")
    assert_eq "$result" "japan" "GET country -> japan"

    result=$(kv_get "$LEADER" "language")
    assert_eq "$result" "rust" "GET language -> rust"

    # -------------------------------------------------------------------
    # 5. Delete operation
    # -------------------------------------------------------------------
    log_info "Step 5: Delete operation"

    result=$(kv_delete "$LEADER" "language")
    assert_eq "$result" "rust" "DELETE language -> rust (old value)"

    result=$(kv_get "$LEADER" "language")
    assert_eq "$result" "(nil)" "GET language -> (nil) after delete"

    # -------------------------------------------------------------------
    # 6. Get non-existent key
    # -------------------------------------------------------------------
    log_info "Step 6: Get non-existent key"

    result=$(kv_get "$LEADER" "nonexistent_key_12345")
    assert_eq "$result" "(nil)" "GET nonexistent key -> (nil)"

    # -------------------------------------------------------------------
    # 7. Follower redirect
    # -------------------------------------------------------------------
    log_info "Step 7: Follower redirect (client follows leader redirect)"

    FOLLOWERS=($(get_follower_addrs "$LEADER"))
    if [[ ${#FOLLOWERS[@]} -gt 0 ]]; then
        FOLLOWER="${FOLLOWERS[0]}"
        log_info "Sending request to follower: $FOLLOWER"

        result=$(kv_set "$FOLLOWER" "redirect_test" "works")
        assert_eq "$result" "works" "SET via follower redirect"

        result=$(kv_get "$LEADER" "redirect_test")
        assert_eq "$result" "works" "GET redirect_test from leader"
    else
        log_warn "No followers found, skipping redirect test"
    fi

    # -------------------------------------------------------------------
    # 8. Bulk write and read consistency
    # -------------------------------------------------------------------
    log_info "Step 8: Bulk write and read consistency"

    for i in $(seq 1 10); do
        kv_set "$LEADER" "bulk_key_$i" "value_$i" >/dev/null
    done

    sleep 2

    ALL_MATCH=true
    for i in $(seq 1 10); do
        result=$(kv_get "$LEADER" "bulk_key_$i")
        if [[ "$result" != "value_$i" ]]; then
            log_fail "Bulk read bulk_key_$i: expected 'value_$i', got '$result'"
            ALL_MATCH=false
        fi
    done

    if [[ "$ALL_MATCH" == "true" ]]; then
        log_pass "All 10 bulk keys read back correctly"
    fi
fi

log_info "Test 01 complete."
