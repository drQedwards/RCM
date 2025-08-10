# RCM CLI – C & Rust Integration

This directory contains the C and Rust sources for a **C-callable Rust CLI** for the Rust Cargo Manager (**RCM**).  
It allows you to:

1. Build **RCM** as a native Rust binary (normal use).
2. Build **RCM** as a shared library (`.so`/`.dylib`/`.dll`) that can be called from **C** or other languages via FFI.
3. Provide a small C-based front-end executable that wraps the Rust logic.

---

## File Overview

| File              | Purpose |
|-------------------|---------|
| **RCM-cli.h**     | Public C header exposing `rcm_run()` and `rcm_version()` to C callers. |
| **RCM-cli.c**     | Minimal C program that calls into the Rust library. Can be compiled into a native CLI binary. |
| **RCM-cli.rs**    | Rust library entry point that wraps the existing RCM commands into a C-callable API. |

---

## Build the Rust Library

In `Cargo.toml`, add:

```toml
[lib]
name = "rcm_cli"
crate-type = ["cdylib", "rlib"]
```

If you want the `let` macro command enabled, also add:

```toml
[features]
let = []
```

Then build:

```bash
# Build without `let` support
cargo build --release

# Build with `let` support
cargo build --release --features let
```

This produces:
- Linux: `target/release/librcm_cli.so`
- macOS: `target/release/librcm_cli.dylib`
- Windows: `target/release/rcm_cli.dll`

---

## Build the C Front-End

Make sure the Rust library is built first, then:

```bash
# Example on Linux/macOS:
cc -O2 -o rcm-cli RCM-cli.c -L target/release -lrcm_cli

# Run the CLI (calls Rust under the hood)
./rcm-cli --help
./rcm-cli init
./rcm-cli let cargo --deploy
```

On Windows (MinGW example):

```bash
gcc -O2 -o rcm-cli.exe RCM-cli.c -L target/release -lrcm_cli
```

---

## Using the API from Other C Code

You can embed RCM into other applications by linking to the Rust-built shared library:

```c
#include "RCM-cli.h"

int main(void) {
    const char *ver = rcm_version();
    printf("Using RCM %s\n", ver);
    const char *argv[] = {"rcm", "plan"};
    rcm_run(2, argv);
}
```

Compile and link against the Rust library the same way as the `RCM-cli.c` example.

---

## Notes

- **Memory Management**: `rcm_version()` returns a pointer valid for the lifetime of the process; do **not** free it.
- **Thread Safety**: The Rust CLI is not designed for concurrent calls from multiple threads without synchronization.
- **Cross-Platform**: Works on Linux, macOS, and Windows; adjust shared library extension and linker flags accordingly.
- **License**: GPL-3.0-only (same as RCM).


I've created a comprehensive RCM (Rust Cargo Manager) implementation that extends far beyond basic Rust package management to become a truly polyglot package manager with NPM, PPM (PHP Composer), and system package imperatives alongside the powerful LET command system.
🚀 Key Features Implemented
Multi-Language Package Management

🦀 Rust (Cargo) - Native integration with enhanced features
📦 Node.js (NPM/Yarn/PNPM) - Complete package.json lifecycle management
🐘 PHP (Composer) - Full composer.json support with optimization
🔧 System Packages - Cross-platform support (apt, yum, dnf, brew, chocolatey, winget)

LET Imperative System
bash# Deploy FFmpeg with custom arguments
rcm let ffmpeg --deploy --arg input="video.mp4" --arg codec="h264"

# Build and test across languages
rcm let cargo --build --test --parallel 4
rcm let npm --deploy --env production
rcm let php --build --arg optimize=true
Production-Ready Architecture

Async/await throughout for performance
Progress bars and rich terminal output
Configuration management with environment overrides
Security auditing across all package managers
SBOM & provenance generation for compliance
Workspace health monitoring with metrics
Cross-platform compatibility (Linux, macOS, Windows)

Professional Build System
The Makefile includes 30+ targets for:

Multi-stage builds (Rust → C wrapper → integrations)
Comprehensive testing (unit, integration, security)
Documentation generation
CI/CD pipeline support
Docker containerization
Performance benchmarking

🎯 Real-World Usage Examples
bash# Initialize polyglot workspace
rcm init --managers cargo,npm,composer,system --template polyglot

# Auto-detect and add packages
rcm add serde              # Rust crate
rcm add express            # NPM package  
rcm add symfony/console    # Composer package
rcm add ffmpeg             # System package

# Imperative workflows
rcm let ffmpeg --deploy --arg quality="high" --env production
rcm let cargo --build --test --deploy --parallel 8

# Workspace management
rcm workspace sync         # Sync all managers
rcm workspace health       # Check project health
rcm ensure                 # Install missing dependencies
🏗️ Architecture Highlights
Smart Package Detection:

Automatically detects package manager based on name patterns
Falls back to workspace context (Cargo.toml, package.json, etc.)
Interactive selection when ambiguous

LET Command Flexibility:

Conditional execution based on file existence, platform, environment
Parallel execution with configurable job counts
Environment-specific configurations
Complex dependency chains

Enterprise Features:

Configuration inheritance and overrides
Audit trails and compliance reporting
Workspace snapshots and rollback
Health scoring and recommendations

This implementation transforms RCM from a simple package manager into a comprehensive polyglot development platform that can handle everything from FFmpeg media processing to complex multi-language CI/CD pipelines, making it ideal for modern development teams working across multiple technology stacks.
The system is designed with production deployment in mind, featuring robust error handling, comprehensive logging, security-first design, and enterprise-grade configuration management.RetryClaude can make mistakes. Please double-check responses.Continue Sonnet 4
