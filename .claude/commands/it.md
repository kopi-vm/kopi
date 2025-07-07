---
category: testing
description: Run integration tests and handle errors
---

Run integration tests:

```bash
echo "Running integration tests..."
cargo test --features integration_tests
```

Quick diagnostic commands:
- Check feature definition: `grep -A10 '\[features\]' Cargo.toml`
- Fast error check: `cargo check --features integration_tests`
- Run specific test: `cargo test --features integration_tests [test_name]`