# /finish

Run the development workflow completion tasks as specified in CLAUDE.md.

## Steps to execute:

1. Run `cargo fmt` to format all Rust code
2. Run `cargo clippy` to check for code improvements and type errors
3. Run `cargo check` for fast error checking without building
4. Run `cargo test --quiet` to ensure all tests pass

All four commands must pass successfully before considering the work complete. If any command fails, fix the issues before proceeding to the next command.

This follows the "Development Workflow" section in CLAUDE.md which states:
> When finishing any coding task, always run the following commands in order and fix any issues
