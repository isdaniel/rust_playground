#!/usr/bin/env bash
# run_all_tests.sh -- Master test runner for Raft integration tests
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Fix Windows line endings if present
for f in "$SCRIPT_DIR"/*.sh; do
    sed -i 's/\r$//' "$f" 2>/dev/null || true
done

source "$SCRIPT_DIR/helpers.sh"

echo ""
echo "============================================================"
echo "  RAFT INTEGRATION TEST SUITE"
echo "  Cluster: 3 nodes (172.20.0.11-13 : 9001-9003)"
echo "============================================================"
echo ""

# -----------------------------------------------------------------------
# Wait for the cluster to fully boot
# -----------------------------------------------------------------------
log_info "Waiting for cluster to initialize..."
sleep 5
wait_for_cluster 45 || true

# -----------------------------------------------------------------------
# Run test suites sequentially, capturing results via temp files
# -----------------------------------------------------------------------
TOTAL_PASSED=0
TOTAL_FAILED=0

run_test_suite() {
    local name="$1"
    local script="$2"
    local tmpfile
    tmpfile=$(mktemp /tmp/raft_test_XXXXXX)

    log_section "Running $name"

    # Run in a subshell with errors allowed so we always capture counters
    (
        set +e  # Allow errors so we can always write counters
        set +u  # Allow unset variables
        source "$SCRIPT_DIR/helpers.sh"
        source "$script"
        echo "PASSED=$TESTS_PASSED" > "$tmpfile"
        echo "FAILED=$TESTS_FAILED" >> "$tmpfile"
    )
    local exit_code=$?

    local suite_passed=0
    local suite_failed=0
    if [[ -f "$tmpfile" ]]; then
        suite_passed=$(grep "^PASSED=" "$tmpfile" 2>/dev/null | cut -d= -f2 || echo "0")
        suite_failed=$(grep "^FAILED=" "$tmpfile" 2>/dev/null | cut -d= -f2 || echo "0")
        rm -f "$tmpfile"
    fi

    # If suite_passed and suite_failed are empty, set defaults
    suite_passed=${suite_passed:-0}
    suite_failed=${suite_failed:-0}

    if [[ $exit_code -ne 0 && "$suite_failed" -eq 0 && "$suite_passed" -eq 0 ]]; then
        suite_failed=1
    fi

    TOTAL_PASSED=$((TOTAL_PASSED + suite_passed))
    TOTAL_FAILED=$((TOTAL_FAILED + suite_failed))

    echo ""
    echo "  >>> $name: Passed=$suite_passed  Failed=$suite_failed"
    echo ""
}

run_test_suite "Test 01: Normal Workflow" "$SCRIPT_DIR/test_01_normal_workflow.sh"
run_test_suite "Test 02: Leader Crash & Data Consistency" "$SCRIPT_DIR/test_02_leader_crash.sh"
run_test_suite "Test 03: Raft Consensus Algorithm" "$SCRIPT_DIR/test_03_consensus.sh"

# -----------------------------------------------------------------------
# Final summary
# -----------------------------------------------------------------------
TESTS_PASSED=$TOTAL_PASSED
TESTS_FAILED=$TOTAL_FAILED

echo ""
echo "============================================================"
echo "  FINAL RESULTS"
echo "============================================================"

print_summary || true
if [[ $TESTS_FAILED -gt 0 ]]; then
    exit 1
else
    exit 0
fi
