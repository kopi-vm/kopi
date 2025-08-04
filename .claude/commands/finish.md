# /finish

Run the development workflow completion tasks.

## Steps to execute:

1. Run `cargo fmt` to format all Rust code
2. Run `cargo clippy --all-targets -- -D warnings` for error checking
3. Run `cargo build --all-targets` for build all targets
4. Run `cargo test --quiet --features integration_tests` to ensure all tests pass

All four commands must pass successfully before considering the work complete. If any command fails, fix the issues before proceeding to the next command.
