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
