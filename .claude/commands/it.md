---
category: testing
description: Run integration tests and handle errors
---

Run integration tests with error handling:

```bash
echo "Running integration tests..."
cargo test --features integration_tests 2>&1 | tee /tmp/test_output.txt

if [ ${PIPESTATUS[0]} -ne 0 ]; then
    echo -e "\n❌ Integration tests failed. Analyzing errors...\n"
    
    # Check for common error patterns
    if grep -q "error\[E0433\]: failed to resolve" /tmp/test_output.txt; then
        echo "📦 Missing imports detected. Fixing..."
        grep "error\[E0433\]" /tmp/test_output.txt | head -5
    fi
    
    if grep -q "error\[E0425\]: cannot find" /tmp/test_output.txt; then
        echo "🔍 Undefined items found. Details:"
        grep "error\[E0425\]" /tmp/test_output.txt | head -5
    fi
    
    if grep -q "error\[E0599\]: no method named" /tmp/test_output.txt; then
        echo "🔧 Method not found errors:"
        grep "error\[E0599\]" /tmp/test_output.txt | head -5
    fi
    
    if grep -q "error\[E0277\]: the trait bound" /tmp/test_output.txt; then
        echo "🧩 Trait implementation errors:"
        grep "error\[E0277\]" /tmp/test_output.txt | head -5
    fi
    
    if grep -q "error: could not compile" /tmp/test_output.txt; then
        echo -e "\n💡 Compilation failed. Running quick checks:"
        echo "  - Checking Cargo.toml for feature flags..."
        grep -A5 -B5 "integration_tests" Cargo.toml || echo "    ⚠️  'integration_tests' feature not found in Cargo.toml"
        
        echo -e "\n  - Checking for test modules..."
        find tests -name "*.rs" -type f | head -5
    fi
    
    # Summary
    echo -e "\n📊 Error Summary:"
    grep "error\[E[0-9]\+\]" /tmp/test_output.txt | cut -d: -f1 | sort | uniq -c | sort -nr
    
    echo -e "\n💬 To fix these errors, consider:"
    echo "  1. Check if all required dependencies are in Cargo.toml"
    echo "  2. Ensure test modules have proper imports"
    echo "  3. Verify feature flags are correctly defined"
    echo "  4. Run 'cargo check --features integration_tests' for faster iteration"
else
    echo -e "\n✅ All integration tests passed!"
fi

# Clean up
rm -f /tmp/test_output.txt
```

Quick diagnostic commands:
- Check feature definition: `grep -A10 '\[features\]' Cargo.toml`
- Fast error check: `cargo check --features integration_tests`
- Run specific test: `cargo test --features integration_tests [test_name]`