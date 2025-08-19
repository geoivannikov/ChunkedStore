# HTTP Object Storage Server

HTTP object storage server built in Rust with streaming capabilities, designed for video streaming and general file storage. Features clean, type-safe error handling and comprehensive test coverage.

## Architecture

```
src/
├── main.rs      # Entry point and initialization
├── models.rs    # Data structures (ChunkedObject, SharedState)
├── handlers.rs  # HTTP request handlers
├── server.rs    # Server setup and configuration
└── error.rs     # Custom error types and handling
```

## Quick Start

### Prerequisites

- Rust 1.8+
- `cargo-tarpaulin` (for test coverage)

### Installation

```bash
git clone <repository>
cd ChunkedStore
```

### Running the Server

```bash
# Start server (default port 8080)
./tools/run_server.sh

# Or with custom port
PORT=9090 ./tools/run_server.sh
```

### Testing

```bash
# Run all checks (lint + unit + integration + coverage)
./tools/run_all.sh

# Individual test suites
./tools/run_lint.sh              # Code quality checks (clippy, fmt, check)
./tools/run_unit_tests.sh        # Unit tests
./tools/run_integration_tests.sh # Integration tests
./tools/run_coverage.sh          # Coverage report
```
### Coverage
- Reports generated in `test_coverage/` directory

## Project Structure

```
ChunkedStore/
├── chunked_store/           # Rust project
│   ├── src/
│   │   ├── main.rs         # Entry point
│   │   ├── models.rs       # Data structures
│   │   ├── handlers.rs     # HTTP handlers
│   │   ├── server.rs       # Server setup
│   │   └── error.rs        # Custom error types
│   └── Cargo.toml
├── tests/                  # Integration tests
│   ├── http_methods.sh     # Core HTTP operations
│   ├── streaming.sh       # Streaming features
│   └── video.sh           # Video streaming
├── tools/                  # Build/test scripts
│   ├── run_server.sh      # Start server
│   ├── run_lint.sh        # Code quality checks
│   ├── run_unit_tests.sh  # Unit tests
│   ├── run_integration_tests.sh  # Integration tests
│   ├── run_coverage.sh    # Coverage report
│   └── run_all.sh         # Run everything
├── sample/                 # Test data
│   └── sample.mp4         # Video file for testing
└── test_coverage/         # Coverage reports (gitignored)
```
