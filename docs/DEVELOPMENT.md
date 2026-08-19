# Development Guide

This document contains information for developers contributing to Terminalist.

## Development Setup

This project is set up with modern Rust development tooling.

### Quick Start

```bash
# Install Rust components
rustup component add rustfmt clippy

# Development workflow
cargo fmt && cargo clippy --fix --allow-dirty && cargo check  # Format + lint + check
```

### Available Commands

```bash
cargo fmt         # Format code with rustfmt
cargo clippy      # Run clippy linter
cargo clippy --fix --allow-dirty  # Auto-fix clippy issues
cargo check       # Check code without building
cargo test        # Run tests
cargo build       # Build the project
cargo run         # Run the main application
cargo clean       # Clean build artifacts
cargo clippy -- -W clippy::all -W clippy::pedantic  # Run all clippy lints (strict)
cargo doc --open --no-deps  # Generate and open documentation
```

### Configuration Files

- `rustfmt.toml` - Code formatting rules
- `clippy.toml` - Linting rules

### Development Workflow

1. `cargo fmt` - Format your code
2. `cargo clippy --fix --allow-dirty` - Auto-fix linting issues
3. `cargo test` - Run tests
4. `cargo check` - Quick compile check
5. `git commit` - Commit your changes

### README screenshots

README screenshots are generated from the real Ratatui application renderer with deterministic,
sanitized fixture data. ImageMagick must be installed and available as `magick`. The generator
uses Monaco on macOS or DejaVu Sans Mono on Linux; set `TERMINALIST_SCREENSHOT_FONT` to override
the font-file path.

```bash
TERMINALIST_SCREENSHOT_DIR=docs/images \
  cargo test generate_readme_screenshots -- --ignored --nocapture
```

The generator is an ignored test, so normal test runs never rewrite documentation assets. Review
both PNG files visually before committing them.

## CI/CD

GitHub Actions workflow is configured in `.github/workflows/ci.yml` with:
- Format checking with rustfmt
- Linting with clippy
- Testing on multiple Rust versions and OSes
- MSRV 1.92 build job
- Smoke tests for `--help` and `--version`
- Security auditing

## Contributing

This is a fully-featured TUI application for Todoist. You can extend it by:

- Adding more keyboard shortcuts
- Implementing additional task filters
- Extending the configuration system
- Enhancing the badge system
- Adding more dialog types

## Dependencies

This project uses the following Rust crates (see `Cargo.toml` for exact versions):

- `todoist-api = "1.0.0-alpha.2"` - Unofficial Todoist API client
- `ratatui = "0.30"` - Terminal UI framework
- `crossterm = "0.29"` - Cross-platform terminal handling
- `tokio = "1.x"` - Async runtime
- `sea-orm = "1.1"` - ORM and SQLite persistence layer
- `serde` - Serialization/deserialization
- `chrono = "0.4"` - Date and time handling
- `anyhow = "1.0"` - Error handling
- `toml = "1.0"` - Configuration file parsing
- `dirs = "6.0"` - Platform-specific directory paths
