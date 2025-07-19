#!/bin/bash

echo "🚀 Redis Async vs Sync Performance Test Suite"
echo "============================================="

# Build the project
echo "Building project..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "❌ Build failed!"
    exit 1
fi

echo "✅ Build successful!"
echo ""

# Check if Redis is running
echo "Checking Redis connection..."
if ! redis-cli ping > /dev/null 2>&1; then
    echo "❌ Redis server is not running!"
    echo "Please start Redis server with: redis-server"
    exit 1
fi

echo "✅ Redis server is running!"
echo ""

# Run comprehensive benchmark
echo "1. Running comprehensive benchmark..."
cargo run --release -- benchmark
echo ""

# Run stress test
echo "2. Running stress test..."
cargo run --release -- stress --operations 2000 --max-concurrent 25
echo ""

# Run memory test
echo "3. Running memory test..."
cargo run --release -- memory --value-size-kb 512 --num-keys 50
echo ""

# Run comparison with different concurrency levels
echo "4. Testing different concurrency levels..."
for concurrent in 1 5 10 25 50; do
    echo "   Testing with $concurrent concurrent tasks..."
    cargo run --release -- async --operations 500 --concurrent $concurrent
done
echo ""

echo "🎉 All tests completed!"
echo ""
echo "💡 Try these commands manually:"
echo "   cargo run --release -- compare --operations 2000"
echo "   cargo run --release -- sync --operations 1000 --threads 8"
echo "   cargo run --release -- async --operations 1000 --concurrent 20"
