#!/usr/bin/env pwsh

# Quick test script to verify the Redis benchmarking project works

Write-Host "🧪 Testing Redis Async vs Sync Project" -ForegroundColor Green
Write-Host "=====================================" -ForegroundColor Green

# Check if Rust is installed
Write-Host "Checking Rust installation..." -ForegroundColor Yellow
try {
    $rustVersion = cargo --version
    Write-Host "✅ Rust found: $rustVersion" -ForegroundColor Green
} catch {
    Write-Host "❌ Rust not found. Please install Rust first." -ForegroundColor Red
    exit 1
}

# Build the project
Write-Host "`nBuilding project..." -ForegroundColor Yellow
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Build failed!" -ForegroundColor Red
    exit 1
}
Write-Host "✅ Build successful!" -ForegroundColor Green

# Test the conceptual example (doesn't require Redis)
Write-Host "`n1. Testing conceptual example (no Redis required)..." -ForegroundColor Cyan
cargo run --example concepts --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Concepts example failed!" -ForegroundColor Red
    exit 1
}
Write-Host "✅ Concepts example completed!" -ForegroundColor Green

# Test help output
Write-Host "`n2. Testing CLI help..." -ForegroundColor Cyan
cargo run --release -- --help
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ CLI help failed!" -ForegroundColor Red
    exit 1
}
Write-Host "✅ CLI help working!" -ForegroundColor Green

# Check if Redis is available
Write-Host "`n3. Checking Redis availability..." -ForegroundColor Cyan
try {
    $redisCheck = redis-cli ping 2>$null
    if ($redisCheck -eq "PONG") {
        Write-Host "✅ Redis is running! Running benchmark..." -ForegroundColor Green
        cargo run --release -- benchmark
        Write-Host "✅ Full benchmark completed!" -ForegroundColor Green
    } else {
        throw "Redis not responding"
    }
} catch {
    Write-Host "⚠️  Redis not available. Skipping full benchmark." -ForegroundColor Yellow
    Write-Host "   To run full tests:" -ForegroundColor Gray
    Write-Host "   1. Install Redis: choco install redis-64" -ForegroundColor Gray
    Write-Host "   2. Start Redis: redis-server" -ForegroundColor Gray
    Write-Host "   3. Run: cargo run --release -- benchmark" -ForegroundColor Gray
}

Write-Host "`n🎉 Project verification completed!" -ForegroundColor Green
Write-Host "✨ Your Redis async vs sync comparison tool is ready!" -ForegroundColor Green
