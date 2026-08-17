# RCM — Rust Cargo Manager

**Polyglot package manager** with Cargo, NPM, Composer (PPM), and system packages — powered by the **LET** imperative system.

| | |
|---|---|
| **crates.io** | [`rcm-rs`](https://crates.io/crates/rcm-rs) |
| **Binary** | `rcm` |
| **Version** | 0.8.0 |
| **License** | MIT |

> **Note:** The crate is published as **`rcm-rs`** because the name `rcm` was already taken on crates.io. The installed command remains **`rcm`**.

---

## Install

```bash
cargo install rcm-rs
```

Then:

```bash
rcm --help
rcm let --help
```

From source:

```bash
git clone https://github.com/drQedwards/RCM.git
cd RCM
cargo build --release
./target/release/rcm --help
```

---

## Quick start

```bash
# LET is the prime imperative
rcm let init
rcm let cargo --dry-run
rcm let cargo --dry-run --json

# Examples (when full managers are enabled)
rcm let ffmpeg --deploy --arg input="video.mp4" --arg codec="h264"
rcm let cargo --build --test --parallel 4
rcm let npm --deploy --env production
```

---

## Features

### Multi-language package management

- **Rust (Cargo)** — native integration
- **Node.js (NPM / Yarn / PNPM)** — `package.json` lifecycle
- **PHP (Composer / PPM)** — `composer.json` support
- **System packages** — apt, yum, dnf, brew, chocolatey, winget

### LET imperative system

LET is the bonded command spine of RCM:

```bash
rcm let <target> [--dry-run] [--json] [--workspace <path>]
rcm let init
rcm let run <target>
```

- Conditional execution (files, platform, env)
- Parallel jobs
- Environment-specific configs
- Structured JSON outcomes

### Architecture highlights

- Async-friendly design
- Progress / terminal UX (indicatif, console, dialoguer)
- Workspace-oriented configuration
- Optional features: `let` (default), `npm`, `ppm`, `system`, `full`

```toml
[features]
default = ["let"]
let = []
npm = []
ppm = []
system = []
full = ["let", "npm", "ppm", "system"]
```

---

## Usage examples

```bash
# Initialize / probe LET
rcm let init
rcm let cargo --dry-run

# Polyglot-style workflows (feature-dependent)
rcm add serde                 # Rust
rcm add express               # NPM
rcm add symfony/console       # Composer
rcm add ffmpeg                # system

rcm let cargo --build --test --deploy --parallel 8
rcm workspace sync
rcm workspace health
rcm ensure
```

---

## C / FFI integration

RCM can be built as a native binary **or** as a shared library for C (and other languages).

### Layout

| Path | Purpose |
|------|---------|
| `RCM-cli/Rcm-cli.h` | C header (`rcm_run`, `rcm_version`) |
| `RCM-cli/Rcm-cli.c` | Minimal C front-end |
| `RCM-cli/Rcm-cli.rs` | Rust ↔ C bridge |

### Build the Rust library

```bash
cargo build --release
# with LET (default):
cargo build --release --features let
```

Artifacts:

- Linux: `target/release/librcm.so` (lib name follows `[lib] name = "rcm"`)
- macOS: `target/release/librcm.dylib`
- Windows: `target/release/rcm.dll`

### Build a C front-end

```bash
cc -O2 -o rcm-cli RCM-cli/Rcm-cli.c -L target/release -lrcm
./rcm-cli --help
```

### Embed from C

```c
#include "RCM-cli/Rcm-cli.h"

int main(void) {
    printf("Using RCM %s\n", rcm_version());
    const char *argv[] = {"rcm", "let", "init"};
    return rcm_run(3, argv);
}
```

**Notes**

- `rcm_version()` returns a process-lifetime pointer — do not free it.
- Not designed for concurrent multi-threaded CLI calls without external synchronization.

---

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo build --verbose
cargo test --verbose

# LET smoke
cargo run --quiet -- let init
cargo run --quiet -- let cargo --dry-run
cargo run --quiet -- let cargo --dry-run --json
```

CI (`.github/workflows/rust.yml`) runs fmt, clippy, build, test, and LET smoke on `main`, and publishes to crates.io on `v*` tags.

---

## Publish / release

```bash
git tag v0.8.0
git push origin v0.8.0
```

Or locally (with `CARGO_REGISTRY_TOKEN` set):

```bash
cargo publish --locked
```

---

## License

MIT — see [LICENSE](LICENSE).

---

**RCM** turns package management into a single polyglot surface. **LET** is the prime bonded imperative.
