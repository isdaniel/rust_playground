# Redis Async vs Sync Performance Test Suite for Windows PowerShell

Write-Host "🚀 Redis Async vs Sync Performance Test Suite" -ForegroundColor Green
Write-Host "=============================================" -ForegroundColor Green

# Build the project
Write-Host "Building project..." -ForegroundColor Yellow
cargo build --release

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Build failed!" -ForegroundColor Red
    exit 1
}

Write-Host "✅ Build successful!" -ForegroundColor Green
Write-Host ""

# Check if Redis is running
Write-Host "Checking Redis connection..." -ForegroundColor Yellow
try {
    $result = redis-cli ping 2>$null
    if ($result -ne "PONG") {
        throw "Redis not responding"
    }
    Write-Host "✅ Redis server is running!" -ForegroundColor Green
} catch {
    Write-Host "❌ Redis server is not running!" -ForegroundColor Red
    Write-Host "Please start Redis server. On Windows:" -ForegroundColor Yellow
    Write-Host "  - Install: choco install redis-64" -ForegroundColor Yellow
    Write-Host "  - Or download from: https://github.com/microsoftarchive/redis/releases" -ForegroundColor Yellow
    Write-Host "  - Then run: redis-server" -ForegroundColor Yellow
    exit 1
}
Write-Host ""

# Run comprehensive benchmark
Write-Host "1. Running comprehensive benchmark..." -ForegroundColor Cyan
cargo run --release -- benchmark
Write-Host ""

# Run stress test
Write-Host "2. Running stress test..." -ForegroundColor Cyan
cargo run --release -- stress --operations 2000 --max-concurrent 25
Write-Host ""

# Run memory test
Write-Host "3. Running memory test..." -ForegroundColor Cyan
cargo run --release -- memory --value-size-kb 512 --num-keys 50
Write-Host ""

# Run comparison with different concurrency levels
Write-Host "4. Testing different concurrency levels..." -ForegroundColor Cyan
$concurrencyLevels = @(1, 5, 10, 25, 50)
foreach ($concurrent in $concurrencyLevels) {
    Write-Host "   Testing with $concurrent concurrent tasks..." -ForegroundColor Yellow
    cargo run --release -- async --operations 500 --concurrent $concurrent
}
Write-Host ""

Write-Host "🎉 All tests completed!" -ForegroundColor Green
Write-Host ""
Write-Host "💡 Try these commands manually:" -ForegroundColor Blue
Write-Host "   cargo run --release -- compare --operations 2000" -ForegroundColor Gray
Write-Host "   cargo run --release -- sync --operations 1000 --threads 8" -ForegroundColor Gray
Write-Host "   cargo run --release -- async --operations 1000 --concurrent 20" -ForegroundColor Gray
