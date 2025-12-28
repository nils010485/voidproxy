# VoidProxy

<div align="center">

<img src="https://cdn.angelkarlsson.eu/persist/voidproxy/logo.png" alt="VoidProxy Banner" width="800">

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![Version](https://img.shields.io/badge/version-0.1.0-blue.svg?style=for-the-badge)
![License](https://img.shields.io/badge/license-MIT-green.svg?style=for-the-badge)

**High-performance TCP/UDP proxy with modern web management interface**

[Features](#-features) • [Installation](#-installation) • [Usage](#-usage) • [Web UI](#-web-ui) • [Configuration](#-configuration) • [API](#-api)

</div>

## Features

- **High Performance**: Built with Rust and Tokio for maximum throughput
- **Dual Protocol**: Support for both TCP and UDP proxying
- **Web Management**: Intuitive web interface with modern design
- **IP Filtering**: Flexible allow/deny lists for access control
- **Persistent Storage**: Automatic configuration persistence
- **Real-time Monitoring**: Live statistics and connection tracking
- **REST API**: Complete automation interface

## Installation

### From Source

```bash
git clone https://github.com/nils010485/voidproxy.ggit
cd voidproxy
cargo build --release
```

### Requirements

- Rust 1.70+
- Tokio runtime

## Usage

### Basic Usage

```bash
# Start with default settings
./voidproxy

# Custom web interface
./voidproxy --web-listen-ip 0.0.0.0 --web-listen-port 9000

# Enable verbose logging
./voidproxy --verbose

# Custom config file
./voidproxy --config-path /path/to/instances.toml
```

### Command Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `--web-listen-ip` | Web UI listen IP | `127.0.0.1` |
| `--web-listen-port` | Web UI listen port | `8080` |
| `--config-path` | Configuration file path | `instances.toml` |
| `--verbose` | Enable verbose logging | `false` |

## Web UI

Access the web interface at `http://localhost:8080` (or your custom port):

### Demo Interface

<img src="https://cdn.angelkarlsson.eu/persist/voidproxy/webui.png" alt="VoidProxy Web UI" width="800">

## Configuration

### Example Configuration

```toml
[proxy]
listen_ip = "127.0.0.1"
listen_port = 8080
dst_ip = "192.168.1.100"
dst_port = 80
protocol = "tcp"

[ip_filter]
allow_list = ["192.168.1.10", "192.168.1.20"]
# deny_list = ["10.0.0.1", "10.0.0.2"]
```

### Configuration Parameters

#### Proxy Settings
- **listen_ip**: IP address to listen on
- **listen_port**: Port to listen on
- **dst_ip**: Destination IP address
- **dst_port**: Destination port
- **protocol**: Protocol type (`tcp` or `udp`)

#### IP Filtering
- **allow_list**: List of allowed IP addresses (optional)
- **deny_list**: List of blocked IP addresses (optional)

## API Endpoints

### Instances

- `GET /api/instances` - List all instances
- `POST /api/instances` - Create new instance
- `GET /api/instances/{id}` - Get instance details
- `PUT /api/instances/{id}` - Update instance
- `DELETE /api/instances/{id}` - Delete instance
- `POST /api/instances/{id}/start` - Start instance
- `POST /api/instances/{id}/stop` - Stop instance

### Statistics

- `GET /api/stats` - Get system statistics
- `GET /api/instances/{id}/stats` - Get instance statistics

### API Example

```bash
curl -X POST http://127.0.0.1:8080/api/instances \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Web Proxy",
    "listen_ip": "127.0.0.1",
    "listen_port": 8081,
    "dst_ip": "93.184.216.34",
    "dst_port": 80,
    "protocol": "tcp",
    "auto_start": true,
    "allow_list": ["192.168.1.0/24"]
  }'
```

## Project Structure

```
voidProxy/
├── src/
│   ├── lib.rs                 # Main library entry point
│   ├── main.rs                # Application entry point
│   ├── config.rs              # Configuration management (15 unit tests)
│   ├── instance.rs            # Proxy instance implementation
│   ├── instance_manager.rs    # Instance lifecycle management
│   ├── tcp_proxy.rs           # TCP proxy implementation
│   ├── udp_proxy.rs           # UDP proxy implementation
│   ├── buffer_pool.rs         # Memory management with three-tier buffer system
│   ├── ip_cache.rs            # IP filtering with TTL and LRU eviction
│   ├── storage.rs             # Configuration persistence (3 unit tests)
│   ├── metrics.rs             # Statistics collection and monitoring
│   ├── web_api.rs             # REST API endpoints
│   └── web_ui.rs              # Web UI server with embedded static files
├── tests/                     # Integration tests (10 tests total)
│   ├── config_tests.rs        # Configuration validation (2 tests)
│   ├── instance_tests.rs      # Instance lifecycle management (2 tests)
│   ├── metrics_tests.rs       # Performance metrics (2 tests)
│   ├── ip_cache_tests.rs      # IP caching functionality (2 tests)
│   └── buffer_pool_tests.rs   # Buffer pool operations (2 tests)
├── static/                    # Web assets (embedded in binary)
│   ├── html/
│   │   └── index.html        # Main UI page
│   ├── js/
│   │   ├── app.js            # Application logic
│   │   ├── core.js           # Core utilities
│   │   └── icons.js          # SVG icon definitions
│   └── css/
│       ├── style.css         # Main styles
│       └── ui.css            # UI component styles
├── Cargo.toml                # Project dependencies
├── instances.toml            # Default configuration
└── README.md                 # This file
```

## Testing

The project includes comprehensive test coverage with **27 total tests**:

### Test Coverage
- **17 unit tests** embedded in source files
  - `src/config.rs`: 15 tests for configuration validation and IP filtering
  - `src/storage.rs`: 3 tests for persistence operations
- **10 integration tests** organized by module
  - Configuration validation and creation
  - Proxy instance lifecycle management
  - Performance metrics with overflow protection
  - IP caching with TTL and LRU eviction
  - Buffer pool memory management

### Running Tests

```bash
# Run all tests (27 total)
cargo test

# Run specific test modules
cargo test config_tests     # Configuration tests
cargo test instance_tests   # Instance management tests
cargo test metrics_tests    # Performance metrics tests
cargo test ip_cache_tests   # IP caching tests
cargo test buffer_pool_tests # Buffer management tests

# Run with verbose output
cargo test -- --nocapture

# Run specific test function
cargo test test_config_creation
```

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Web Client    │    │   REST API      │    │  Instance Mgr   │
│   (Embedded)    │    │   (Axum)        │    │  (Arc<RwLock>)  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                    ┌─────────────────┐    ┌─────────────────┐
                    │   TCP/UDP       │    │   Metrics &     │
                    │   Proxy Core    │    │   Monitoring    │
                    │   (Tokio)       │    │   (Atomic)      │
                    └─────────────────┘    └─────────────────┘
                                 │                       │
                    ┌─────────────────┐    ┌─────────────────┐
                    │   Buffer Pool   │    │   IP Cache      │
                    │   (Memory Mgmt) │    │   (TTL/LRU)     │
                    └─────────────────┘    └─────────────────┘
                                 │
                    ┌─────────────────┐
                    │   Storage       │
                    │   (TOML)        │
                    └─────────────────┘
```

### Key Features
- **Embedded Assets**: All static files (HTML, CSS, JS) are embedded in the binary
- **Thread Safety**: Uses `Arc<RwLock<T>>` for concurrent instance management
- **Async Performance**: Built on Tokio for high-performance I/O operations
- **Smart Caching**: IP address filtering with TTL-based expiration and LRU eviction
- **Efficient Memory**: Three-tier buffer pool system for optimal memory usage

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Disclaimer

This software is provided "as is", without warranty of any kind, express or implied, including but not limited to the warranties of merchantability, fitness for a particular purpose and noninfringement. In no event shall the authors or copyright holders be liable for any claim, damages or other liability, whether in an action of contract, tort or otherwise, arising from, out of or in connection with the software or the use or other dealings in the software.

The author of this software cannot be held responsible for any damages or issues that may arise from its use. Use at your own risk and discretion.

## License

This project is licensed under the MIT License

---

<div align="center">

Made with passion by [Nils](https://nils.begou.dev)

</div>
