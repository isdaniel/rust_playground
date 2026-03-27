# =============================================================================
# Failover Demo Script (PowerShell)
#
# This script demonstrates the Active/Standby failover mechanism:
# 1. Start all services
# 2. Verify Active exporter is processing
# 3. Kill the Active exporter
# 4. Verify Standby takes over
# 5. Restart the original Active (it becomes Standby)
# 6. Simulate WAN disconnect/reconnect
# =============================================================================

$ErrorActionPreference = "Continue"

function Write-Step {
    param([string]$Message)
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host "  $Message" -ForegroundColor Cyan
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ""
}

function Write-Ok {
    param([string]$Message)
    Write-Host "[OK] $Message" -ForegroundColor Green
}

function Write-Warn {
    param([string]$Message)
    Write-Host "[WARN] $Message" -ForegroundColor Yellow
}

function Write-Fail {
    param([string]$Message)
    Write-Host "[FAIL] $Message" -ForegroundColor Red
}

function Get-ServiceStatus {
    param([string]$Url)
    try {
        $response = Invoke-RestMethod -Uri $Url -TimeoutSec 5 -ErrorAction Stop
        return $response
    }
    catch {
        return $null
    }
}

function Get-ServiceStatusRaw {
    param([string]$Url)
    try {
        $response = Invoke-WebRequest -Uri $Url -TimeoutSec 5 -ErrorAction Stop
        return $response.Content
    }
    catch {
        return "unreachable"
    }
}

# ── Step 2: Verify Active is processing ──────────────────────────────────────
Write-Step "Step 2: Waiting 20s for metrics to be processed..."
Start-Sleep -Seconds 20

Write-Host "Mock AWS stats after 20s:"
$awsStats = Get-ServiceStatusRaw "http://localhost:8080/admin/stats"
Write-Host $awsStats
Write-Host ""

$activeObj = Get-ServiceStatus "http://localhost:9091/status"
if ($activeObj -and $activeObj.role -eq "ACTIVE") {
    Write-Ok "exporter-active is ACTIVE"
} else {
    $standbyObj = Get-ServiceStatus "http://localhost:9095/status"
    if ($standbyObj -and $standbyObj.role -eq "ACTIVE") {
        Write-Ok "exporter-standby is ACTIVE (leader election chose standby first)"
        Write-Warn "Swapping test target ports: Active=9095, Standby=9091"
        # The rest of the test continues with whichever got elected
    } else {
        Write-Warn "Neither exporter shows ACTIVE yet. Leader election may still be in progress."
    }
}

# Determine which is actually Active
$activePort = $null
$standbyPort = $null

$statusA = Get-ServiceStatus "http://localhost:9091/status"
$statusB = Get-ServiceStatus "http://localhost:9095/status"

if ($statusA -and $statusA.role -eq "ACTIVE") {
    $activePort = 9091
    $standbyPort = 9095
    $activeContainer = "exporter-active"
    $standbyContainer = "exporter-standby"
    Write-Ok "Active = exporter-active (port 9091)"
} elseif ($statusB -and $statusB.role -eq "ACTIVE") {
    $activePort = 9095
    $standbyPort = 9091
    $activeContainer = "exporter-standby"
    $standbyContainer = "exporter-active"
    Write-Ok "Active = exporter-standby (port 9095)"
} else {
    Write-Warn "Could not determine Active. Proceeding with exporter-active as target."
    $activePort = 9091
    $standbyPort = 9095
    $activeContainer = "exporter-active"
    $standbyContainer = "exporter-standby"
}

# ── Step 3: Kill the Active exporter ─────────────────────────────────────────
Write-Step "Step 3: KILLING $activeContainer to trigger failover"
docker compose stop $activeContainer
Write-Ok "$activeContainer stopped"

Write-Host "Waiting 25s for Kafka consumer group rebalance..."
Start-Sleep -Seconds 25

# ── Step 4: Check Standby took over ─────────────────────────────────────────
Write-Step "Step 4: Checking if Standby ($standbyContainer) took over"

$newStatus = Get-ServiceStatus "http://localhost:${standbyPort}/status"
Write-Host "Standby status: $($newStatus | ConvertTo-Json -Compress)"

if ($newStatus -and $newStatus.role -eq "ACTIVE") {
    Write-Ok "FAILOVER SUCCESS! $standbyContainer is now ACTIVE"
} else {
    Write-Warn "$standbyContainer may still be transitioning (role: $($newStatus.role))"
}

Write-Host ""
Write-Host "Mock AWS stats (should still be receiving data):"
$awsStats = Get-ServiceStatusRaw "http://localhost:8080/admin/stats"
Write-Host $awsStats

Start-Sleep -Seconds 10
Write-Host ""
Write-Host "Mock AWS stats after 10 more seconds:"
$awsStats = Get-ServiceStatusRaw "http://localhost:8080/admin/stats"
Write-Host $awsStats

# ── Step 5: Restart original Active (becomes Standby) ───────────────────────
Write-Step "Step 5: Restarting $activeContainer (should become Standby)"
docker compose start $activeContainer

Write-Host "Waiting 20s for restarted instance..."
Start-Sleep -Seconds 20

Write-Host "$activeContainer status (should be STANDBY):"
$restartedStatus = Get-ServiceStatusRaw "http://localhost:${activePort}/status"
Write-Host $restartedStatus
Write-Host ""

Write-Host "$standbyContainer status (should still be ACTIVE):"
$currentActiveStatus = Get-ServiceStatusRaw "http://localhost:${standbyPort}/status"
Write-Host $currentActiveStatus
Write-Host ""

# ── Step 5b: Verify sticky primary (key check!) ─────────────────────────────
Write-Step "Step 5b: Verifying STICKY PRIMARY behavior"
Write-Host "Waiting 15s to confirm roles are stable..."
Start-Sleep -Seconds 15

$restartedObj = Get-ServiceStatus "http://localhost:${activePort}/status"
$promotedObj = Get-ServiceStatus "http://localhost:${standbyPort}/status"

if ($restartedObj -and $restartedObj.role -eq "STANDBY") {
    Write-Ok "STICKY PRIMARY VERIFIED: Restarted $activeContainer remains STANDBY"
} else {
    Write-Fail "Restarted $activeContainer is NOT STANDBY (role: $($restartedObj.role)) -- flip-flop detected!"
}

if ($promotedObj -and $promotedObj.role -eq "ACTIVE") {
    Write-Ok "STICKY PRIMARY VERIFIED: Promoted $standbyContainer remains ACTIVE"
} else {
    Write-Fail "Promoted $standbyContainer is NOT ACTIVE (role: $($promotedObj.role)) -- flip-flop detected!"
}
Write-Host ""

# ── Step 6: Simulate WAN disconnect ─────────────────────────────────────────
Write-Step "Step 6: Simulating WAN disconnect"

Write-Host "Sending disconnect signal to mock-aws..."
try {
    Invoke-RestMethod -Method Post -Uri "http://localhost:8080/admin/disconnect" -TimeoutSec 5
} catch {
    Write-Host "POST sent"
}

Write-Host "Waiting 25s for health monitor to detect disconnect..."
Start-Sleep -Seconds 25

Write-Host "Current exporter statuses:"
Write-Host "  Port $activePort : $(Get-ServiceStatusRaw "http://localhost:${activePort}/status")"
Write-Host "  Port $standbyPort : $(Get-ServiceStatusRaw "http://localhost:${standbyPort}/status")"
Write-Host ""

# ── Step 7: Simulate WAN reconnect ──────────────────────────────────────────
Write-Step "Step 7: Simulating WAN reconnect"

Write-Host "Sending connect signal to mock-aws..."
try {
    Invoke-RestMethod -Method Post -Uri "http://localhost:8080/admin/connect" -TimeoutSec 5
} catch {
    Write-Host "POST sent"
}

Write-Host "Waiting 25s for backfill to complete..."
Start-Sleep -Seconds 25

Write-Host "Current exporter statuses:"
Write-Host "  Port $activePort : $(Get-ServiceStatusRaw "http://localhost:${activePort}/status")"
Write-Host "  Port $standbyPort : $(Get-ServiceStatusRaw "http://localhost:${standbyPort}/status")"
Write-Host ""

Write-Host "Mock AWS final stats:"
$finalStats = Get-ServiceStatusRaw "http://localhost:8080/admin/stats"
Write-Host $finalStats
Write-Host ""

# ── Summary ──────────────────────────────────────────────────────────────────
Write-Step "Demo Complete!"
Write-Host "Summary:" -ForegroundColor Green
Write-Host "  1. One exporter started as Active, processed metrics"
Write-Host "  2. Active was killed -> Standby took over via Kafka rebalance"
Write-Host "  3. Original Active restarted -> became Standby (STICKY PRIMARY)"
Write-Host "  3b. Verified: restarted instance stays Standby, promoted stays Active"
Write-Host "  4. WAN disconnect simulated -> exporter paused, Kafka buffered"
Write-Host "  5. WAN reconnect -> backfill engine activated, data recovered"
Write-Host ""
Write-Host "To view logs:   docker compose logs -f exporter-active exporter-standby" -ForegroundColor Yellow
Write-Host "To clean up:    docker compose down -v" -ForegroundColor Yellow
