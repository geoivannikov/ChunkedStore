# HTTP Object Storage Server

A high-performance HTTP object storage server built in Rust with streaming capabilities, designed for video streaming and general file storage.

## Architecture

```
src/
├── main.rs      # Entry point and initialization
├── models.rs    # Data structures (ChunkedObject, SharedState)
├── handlers.rs  # HTTP request handlers
└── server.rs    # Server setup and configuration
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
# Run all tests (unit + integration + coverage)
./tools/run_all.sh

# Individual test suites
./tools/run_unit_tests.sh
./tools/run_integration_tests.shscripts
# Generate coverage report
./tools/run_coverage.sh        
```
### Coverage
- Reports generated in `test_coverage/` directory

## Project Structure

```
ChunkedStore/
├── chunked_store/           # Rust project
│   ├── src/
│   │   ├── main.rs         # Entry point + unit tests
│   │   ├── models.rs       # Data structures + unit tests
│   │   ├── handlers.rs     # HTTP handlers + unit tests
│   │   └── server.rs       # Server setup + unit tests
│   └── Cargo.toml
├── tests/                  # Integration tests
│   ├── http_methods.sh     # Core HTTP operations
│   ├── cors.sh            # CORS testing
│   ├── streaming.sh       # Streaming features
│   └── video.sh           # Video streaming
├── tools/                  # Build/test scripts
│   ├── run_server.sh      # Start server
│   ├── run_unit_tests.sh  # Unit tests
│   ├── run_integration_tests.sh  # Integration tests
│   ├── run_coverage.sh    # Coverage report
│   └── run_all.sh         # Run everything
├── sample/                 # Test data
│   └── sample.mp4         # Video file for testing
└── test_coverage/         # Coverage reports (gitignored)
```
