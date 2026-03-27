#!/bin/bash
# =============================================================================
# Failover Demo Script
#
# This script demonstrates the Active/Standby failover mechanism:
# 1. Start all services
# 2. Verify Active exporter is processing
# 3. Kill the Active exporter
# 4. Verify Standby takes over
# 5. Restart the original Active (it becomes Standby)
# 6. Simulate WAN disconnect/reconnect
# =============================================================================

set -e

COMPOSE="docker compose"
CYAN='\033[0;36m'
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_step() {
    echo -e "\n${CYAN}========================================${NC}"
    echo -e "${CYAN}  $1${NC}"
    echo -e "${CYAN}========================================${NC}\n"
}

log_ok() {
    echo -e "${GREEN}[OK] $1${NC}"
}

log_warn() {
    echo -e "${YELLOW}[WARN] $1${NC}"
}

log_fail() {
    echo -e "${RED}[FAIL] $1${NC}"
}

check_status() {
    local name=$1
    local port=$2
    local result
    result=$(curl -s "http://localhost:${port}/status" 2>/dev/null || echo "unreachable")
    echo "$result"
}

# ── Step 0: Build and start ──────────────────────────────────────────────────
log_step "Step 0: Building and starting all services"
$COMPOSE down -v 2>/dev/null || true
$COMPOSE build
$COMPOSE up -d

echo "Waiting for services to start (30s)..."
sleep 30

# ── Step 1: Check initial state ─────────────────────────────────────────────
log_step "Step 1: Checking initial state"

echo "Exporter Active status:"
check_status "exporter-active" 9091
echo ""

echo "Exporter Standby status:"
check_status "exporter-standby" 9092
echo ""

echo "Mock AWS stats:"
curl -s http://localhost:8080/admin/stats 2>/dev/null || echo "unreachable"
echo ""

# ── Step 2: Verify Active is processing ──────────────────────────────────────
log_step "Step 2: Waiting 15s for some metrics to be processed..."
sleep 15

echo "Mock AWS stats after 15s:"
curl -s http://localhost:8080/admin/stats 2>/dev/null || echo "unreachable"
echo ""

ACTIVE_STATUS=$(check_status "exporter-active" 9091)
echo "Active exporter: $ACTIVE_STATUS"

if echo "$ACTIVE_STATUS" | grep -q "ACTIVE"; then
    log_ok "exporter-active is ACTIVE"
else
    log_warn "exporter-active may not be ACTIVE yet (leader election may still be in progress)"
fi

# ── Step 3: Kill the Active exporter ─────────────────────────────────────────
log_step "Step 3: KILLING exporter-active to trigger failover"
$COMPOSE stop exporter-active
log_ok "exporter-active stopped"

echo "Waiting 20s for Kafka consumer group rebalance..."
sleep 20

# ── Step 4: Check Standby took over ─────────────────────────────────────────
log_step "Step 4: Checking if Standby took over"

STANDBY_STATUS=$(check_status "exporter-standby" 9092)
echo "Standby exporter: $STANDBY_STATUS"

if echo "$STANDBY_STATUS" | grep -q "ACTIVE"; then
    log_ok "FAILOVER SUCCESS! exporter-standby is now ACTIVE"
else
    log_warn "exporter-standby may still be transitioning"
fi

echo ""
echo "Mock AWS stats (should still be receiving data):"
curl -s http://localhost:8080/admin/stats 2>/dev/null || echo "unreachable"
echo ""

sleep 10
echo "Mock AWS stats after 10 more seconds:"
curl -s http://localhost:8080/admin/stats 2>/dev/null || echo "unreachable"
echo ""

# ── Step 5: Restart original Active (becomes Standby) ───────────────────────
log_step "Step 5: Restarting exporter-active (should become Standby)"
$COMPOSE start exporter-active

echo "Waiting 15s for restarted instance..."
sleep 15

echo "Exporter Active status (should be STANDBY):"
check_status "exporter-active" 9091
echo ""

echo "Exporter Standby status (should still be ACTIVE):"
check_status "exporter-standby" 9092
echo ""

# ── Step 6: Simulate WAN disconnect ─────────────────────────────────────────
log_step "Step 6: Simulating WAN disconnect"

echo "Sending disconnect signal to mock-aws..."
curl -s -X POST http://localhost:8080/admin/disconnect
echo ""

echo "Waiting 20s for health monitor to detect disconnect..."
sleep 20

echo "Current Active exporter status:"
CURRENT_ACTIVE=$(curl -s "http://localhost:9092/status" 2>/dev/null || echo "unreachable")
echo "$CURRENT_ACTIVE"
echo ""

# ── Step 7: Simulate WAN reconnect ──────────────────────────────────────────
log_step "Step 7: Simulating WAN reconnect"

echo "Sending connect signal to mock-aws..."
curl -s -X POST http://localhost:8080/admin/connect
echo ""

echo "Waiting 20s for backfill to complete..."
sleep 20

echo "Current Active exporter status (should be CONNECTED):"
curl -s "http://localhost:9092/status" 2>/dev/null || echo "unreachable"
echo ""

echo "Mock AWS final stats:"
curl -s http://localhost:8080/admin/stats 2>/dev/null || echo "unreachable"
echo ""

# ── Summary ──────────────────────────────────────────────────────────────────
log_step "Demo Complete!"
echo -e "${GREEN}Summary:${NC}"
echo "  1. exporter-active started as Active, processed metrics"
echo "  2. exporter-active was killed -> exporter-standby took over via Kafka rebalance"
echo "  3. exporter-active restarted -> became Standby (exporter-standby remains Active)"
echo "  4. WAN disconnect simulated -> exporter paused, Kafka buffered"
echo "  5. WAN reconnect -> backfill engine activated, data recovered"
echo ""
echo "To clean up: $COMPOSE down -v"
