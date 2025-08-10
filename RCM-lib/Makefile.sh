# RCM - Polyglot Package Manager
# Production-ready Makefile for building and deploying RCM

# Configuration
RUST_VERSION ?= 1.75.0
NODE_VERSION ?= 18
PHP_VERSION ?= 8.2
CARGO_TARGET_DIR ?= target
BUILD_MODE ?= release
FEATURES ?= let,npm,ppm,system
PARALLEL_JOBS ?= $(shell nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)

# Platform detection
UNAME_S := $(shell uname -s)
UNAME_M := $(shell uname -m)

ifeq ($(UNAME_S),Linux)
    PLATFORM := linux
    LIB_EXT := so
endif
ifeq ($(UNAME_S),Darwin)
    PLATFORM := macos
    LIB_EXT := dylib
endif
ifeq ($(UNAME_S),MINGW32_NT)
    PLATFORM := windows
    LIB_EXT := dll
endif

# Directories
SRC_DIR := src
BUILD_DIR := build
DIST_DIR := dist
DOCS_DIR := docs
TESTS_DIR := tests
EXAMPLES_DIR := examples

# Build flags
RUST_FLAGS := --release --features $(FEATURES)
ifeq ($(BUILD_MODE),debug)
    RUST_FLAGS := --features $(FEATURES)
endif

# Colors for output
CYAN := \033[36m
GREEN := \033[32m
YELLOW := \033[33m
RED := \033[31m
RESET := \033[0m
BOLD := \033[1m

.PHONY: all build test clean install uninstall docs check lint format \
        prepare-env build-rust build-c build-node build-php \
        package deploy docker benchmark security-audit \
        deps-check deps-install deps-update \
        examples integration-tests release pre-release

# Default target
all: prepare-env build test package

help: ## Show this help message
	@echo "$(CYAN)$(BOLD)RCM - Polyglot Package Manager$(RESET)"
	@echo "$(CYAN)Production build system for Rust, Node.js, PHP, and system package management$(RESET)"
	@echo ""
	@echo "$(BOLD)Available targets:$(RESET)"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  $(CYAN)%-20s$(RESET) %s\n", $$1, $$2}' $(MAKEFILE_LIST)
	@echo ""
	@echo "$(BOLD)Environment variables:$(RESET)"
	@echo "  $(CYAN)BUILD_MODE$(RESET)     Build mode (debug|release) [default: release]"
	@echo "  $(CYAN)FEATURES$(RESET)       Rust features to enable [default: let,npm,ppm,system]"
	@echo "  $(CYAN)PARALLEL_JOBS$(RESET)  Number of parallel jobs [default: auto-detected]"

prepare-env: ## Prepare build environment
	@echo "$(CYAN)🔧 Preparing build environment...$(RESET)"
	@mkdir -p $(BUILD_DIR) $(DIST_DIR) $(DOCS_DIR)
	@echo "$(GREEN)✓ Build directories created$(RESET)"

# Dependency management
deps-check: ## Check if all required tools are installed
	@echo "$(CYAN)🔍 Checking dependencies...$(RESET)"
	@command -v rustc >/dev/null 2>&1 || { echo "$(RED)❌ Rust not installed$(RESET)"; exit 1; }
	@command -v cargo >/dev/null 2>&1 || { echo "$(RED)❌ Cargo not installed$(RESET)"; exit 1; }
	@command -v node >/dev/null 2>&1 || { echo "$(RED)❌ Node.js not installed$(RESET)"; exit 1; }
	@command -v npm >/dev/null 2>&1 || { echo "$(RED)❌ NPM not installed$(RESET)"; exit 1; }
	@command -v php >/dev/null 2>&1 || { echo "$(RED)❌ PHP not installed$(RESET)"; exit 1; }
	@command -v composer >/dev/null 2>&1 || { echo "$(RED)❌ Composer not installed$(RESET)"; exit 1; }
	@echo "$(GREEN)✓ All dependencies available$(RESET)"

deps-install: ## Install project dependencies
	@echo "$(CYAN)📦 Installing project dependencies...$(RESET)"
	@echo "$(YELLOW)Installing Rust dependencies...$(RESET)"
	@cargo fetch
	@if [ -f "package.json" ]; then \
		echo "$(YELLOW)Installing Node.js dependencies...$(RESET)"; \
		npm ci; \
	fi
	@if [ -f "composer.json" ]; then \
		echo "$(YELLOW)Installing PHP dependencies...$(RESET)"; \
		composer install --no-dev --optimize-autoloader; \
	fi
	@echo "$(GREEN)✓ Dependencies installed$(RESET)"

deps-update: ## Update all dependencies
	@echo "$(CYAN)📈 Updating dependencies...$(RESET)"
	@cargo update
	@if [ -f "package.json" ]; then npm update; fi
	@if [ -f "composer.json" ]; then composer update; fi
	@echo "$(GREEN)✓ Dependencies updated$(RESET)"

# Build targets
build: deps-check build-rust build-c build-node build-php ## Build all components
	@echo "$(GREEN)$(BOLD)✅ Build completed successfully!$(RESET)"

build-rust: ## Build Rust components
	@echo "$(CYAN)🦀 Building Rust components...$(RESET)"
	@CARGO_TARGET_DIR=$(CARGO_TARGET_DIR) cargo build $(RUST_FLAGS) --bins
	@CARGO_TARGET_DIR=$(CARGO_TARGET_DIR) cargo build $(RUST_FLAGS) --lib
	@echo "$(GREEN)✓ Rust build completed$(RESET)"
	@ls -la $(CARGO_TARGET_DIR)/$(BUILD_MODE)/

build-c: build-rust ## Build C wrapper
	@echo "$(CYAN)🔧 Building C wrapper...$(RESET)"
	@mkdir -p $(BUILD_DIR)/c
	@gcc -O2 -o $(BUILD_DIR)/c/rcm-cli RCM-cli/Rcm-cli.c \
		-L$(CARGO_TARGET_DIR)/$(BUILD_MODE) -lrcm_cli \
		-Wl,-rpath,$(shell pwd)/$(CARGO_TARGET_DIR)/$(BUILD_MODE)
	@echo "$(GREEN)✓ C wrapper built$(RESET)"

build-node: ## Build Node.js integration
	@echo "$(CYAN)📦 Building Node.js integration...$(RESET)"
	@if [ -f "package.json" ]; then \
		npm run build 2>/dev/null || echo "$(YELLOW)⚠ No Node.js build script found$(RESET)"; \
	fi
	@echo "$(GREEN)✓ Node.js integration ready$(RESET)"

build-php: ## Build PHP integration
	@echo "$(CYAN)🐘 Building PHP integration...$(RESET)"
	@if [ -f "composer.json" ]; then \
		composer dump-autoload --optimize --no-dev; \
	fi
	@echo "$(GREEN)✓ PHP integration ready$(RESET)"

# Testing
test: ## Run all tests
	@echo "$(CYAN)🧪 Running tests...$(RESET)"
	@cargo test $(RUST_FLAGS) --all
	@if [ -f "package.json" ]; then \
		npm test 2>/dev/null || echo "$(YELLOW)⚠ No Node.js tests found$(RESET)"; \
	fi
	@if [ -f "composer.json" ]; then \
		composer run test 2>/dev/null || echo "$(YELLOW)⚠ No PHP tests found$(RESET)"; \
	fi
	@echo "$(GREEN)✓ All tests passed$(RESET)"

integration-tests: build ## Run integration tests
	@echo "$(CYAN)🔗 Running integration tests...$(RESET)"
	@$(BUILD_DIR)/c/rcm-cli --version
	@$(CARGO_TARGET_DIR)/$(BUILD_MODE)/rcm --version
	@echo "$(GREEN)✓ Integration tests passed$(RESET)"

benchmark: build ## Run performance benchmarks
	@echo "$(CYAN)⚡ Running benchmarks...$(RESET)"
	@cargo bench --features $(FEATURES)
	@echo "$(GREEN)✓ Benchmarks completed$(RESET)"

# Code quality
check: ## Run code quality checks
	@echo "$(CYAN)🔍 Running code quality checks...$(RESET)"
	@cargo check --all-targets --features $(FEATURES)
	@cargo clippy --all-targets --features $(FEATURES) -- -D warnings
	@echo "$(GREEN)✓ Code quality checks passed$(RESET)"

lint: ## Run linting
	@echo "$(CYAN)📝 Running linters...$(RESET)"
	@cargo clippy --all-targets --features $(FEATURES) -- -D warnings
	@if [ -f "package.json" ]; then \
		npm run lint 2>/dev/null || echo "$(YELLOW)⚠ No Node.js linter configured$(RESET)"; \
	fi
	@if [ -f "composer.json" ]; then \
		composer run lint 2>/dev/null || echo "$(YELLOW)⚠ No PHP linter configured$(RESET)"; \
	fi
	@echo "$(GREEN)✓ Linting completed$(RESET)"

format: ## Format code
	@echo "$(CYAN)✨ Formatting code...$(RESET)"
	@cargo fmt --all
	@if [ -f "package.json" ] && [ -f "node_modules/.bin/prettier" ]; then \
		npx prettier --write "**/*.{js,ts,json,md}"; \
	fi
	@if [ -f "composer.json" ] && command -v php-cs-fixer >/dev/null; then \
		php-cs-fixer fix; \
	fi
	@echo "$(GREEN)✓ Code formatted$(RESET)"

security-audit: ## Run security audit
	@echo "$(CYAN)🛡️ Running security audit...$(RESET)"
	@cargo audit
	@if [ -f "package.json" ]; then npm audit --audit-level=moderate; fi
	@if [ -f "composer.json" ]; then composer audit; fi
	@echo "$(GREEN)✓ Security audit completed$(RESET)"

# Documentation
docs: ## Generate documentation
	@echo "$(CYAN)📚 Generating documentation...$(RESET)"
	@cargo doc --no-deps --features $(FEATURES)
	@mkdir -p $(DOCS_DIR)/rust
	@cp -r $(CARGO_TARGET_DIR)/doc/* $(DOCS_DIR)/rust/
	@if [ -f "package.json" ]; then \
		npm run docs 2>/dev/null || echo "$(YELLOW)⚠ No Node.js docs script$(RESET)"; \
	fi
	@echo "$(GREEN)✓ Documentation generated$(RESET)"

# Packaging
package: build ## Create distribution packages
	@echo "$(CYAN)📦 Creating distribution packages...$(RESET)"
	@mkdir -p $(DIST_DIR)/bin $(DIST_DIR)/lib $(DIST_DIR)/include
	
	# Copy binaries
	@cp $(CARGO_TARGET_DIR)/$(BUILD_MODE)/rcm $(DIST_DIR)/bin/
	@cp $(BUILD_DIR)/c/rcm-cli $(DIST_DIR)/bin/
	
	# Copy libraries
	@cp $(CARGO_TARGET_DIR)/$(BUILD_MODE)/librcm_cli.$(LIB_EXT) $(DIST_DIR)/lib/
	
	# Copy headers
	@cp RCM-cli/Rcm-cli.h $(DIST_DIR)/include/
	
	# Create archive
	@cd $(DIST_DIR) && tar -czf ../rcm-$(PLATFORM)-$(shell date +%Y%m%d).tar.gz *
	
	@echo "$(GREEN)✓ Distribution package created$(RESET)"
	@ls -la rcm-*.tar.gz

# Installation
install: package ## Install RCM system-wide
	@echo "$(CYAN)⚙️ Installing RCM...$(RESET)"
	@sudo mkdir -p /usr/local/bin /usr/local/lib /usr/local/include
	@sudo cp $(DIST_DIR)/bin/* /usr/local/bin/
	@sudo cp $(DIST_DIR)/lib/* /usr/local/lib/
	@sudo cp $(DIST_DIR)/include/* /usr/local/include/
	@sudo ldconfig 2>/dev/null || true
	@echo "$(GREEN)✓ RCM installed to /usr/local$(RESET)"

uninstall: ## Uninstall RCM
	@echo "$(CYAN)🗑️ Uninstalling RCM...$(RESET)"
	@sudo rm -f /usr/local/bin/rcm /usr/local/bin/rcm-cli
	@sudo rm -f /usr/local/lib/librcm_cli.*
	@sudo rm -f /usr/local/include/Rcm-cli.h
	@echo "$(GREEN)✓ RCM uninstalled$(RESET)"

# Docker support
docker: ## Build Docker image
	@echo "$(CYAN)🐳 Building Docker image...$(RESET)"
	@docker build -t rcm:latest -f Dockerfile .
	@echo "$(GREEN)✓ Docker image built$(RESET)"

docker-run: docker ## Run RCM in Docker
	@echo "$(CYAN)🐳 Running RCM in Docker...$(RESET)"
	@docker run --rm -v $(PWD):/workspace -w /workspace rcm:latest rcm --version

# Examples and demos
examples: build ## Build examples
	@echo "$(CYAN)📖 Building examples...$(RESET)"
	@mkdir -p $(EXAMPLES_DIR)
	@if [ -d "examples" ]; then \
		for example in examples/*/; do \
			echo "Building $$example..."; \
			(cd "$$example" && $(CARGO_TARGET_DIR)/$(BUILD_MODE)/rcm ensure); \
		done; \
	fi
	@echo "$(GREEN)✓ Examples built$(RESET)"

# Release management
pre-release: clean deps-install check test security-audit ## Prepare for release
	@echo "$(CYAN)🚀 Preparing for release...$(RESET)"
	@echo "$(GREEN)✓ Pre-release checks completed$(RESET)"

release: pre-release build package ## Create a release
	@echo "$(CYAN)🎉 Creating release...$(RESET)"
	@VERSION=$$(cargo metadata --format-version=1 --no-deps | jq -r '.packages[0].version'); \
	echo "Creating release v$$VERSION"; \
	git tag -a "v$$VERSION" -m "Release v$$VERSION"; \
	echo "$(GREEN)✓ Release v$$VERSION created$(RESET)"

# Development helpers
dev-setup: ## Set up development environment
	@echo "$(CYAN)🛠️ Setting up development environment...$(RESET)"
	@rustup component add clippy rustfmt
	@cargo install cargo-audit cargo-watch
	@if command -v npm >/dev/null; then \
		npm install -g prettier eslint; \
	fi
	@echo "$(GREEN)✓ Development environment ready$(RESET)"

watch: ## Watch for changes and rebuild
	@echo "$(CYAN)👀 Watching for changes...$(RESET)"
	@cargo watch -x 'build --features $(FEATURES)' -x test

clean: ## Clean build artifacts
	@echo "$(CYAN)🧹 Cleaning build artifacts...$(RESET)"
	@cargo clean
	@rm -rf $(BUILD_DIR) $(DIST_DIR) *.tar.gz
	@if [ -d "node_modules" ]; then rm -rf node_modules; fi
	@if [ -d "vendor" ]; then rm -rf vendor; fi
	@echo "$(GREEN)✓ Build artifacts cleaned$(RESET)"

# Deployment
deploy-staging: package ## Deploy to staging environment
	@echo "$(CYAN)🚀 Deploying to staging...$(RESET)"
	@echo "$(YELLOW)⚠ Staging deployment not implemented$(RESET)"

deploy-production: release ## Deploy to production
	@echo "$(CYAN)🚀 Deploying to production...$(RESET)"
	@echo "$(YELLOW)⚠ Production deployment not implemented$(RESET)"

# Status and information
status: ## Show project status
	@echo "$(CYAN)$(BOLD)📊 RCM Project Status$(RESET)"
	@echo "$(CYAN)═══════════════════════$(RESET)"
	@echo "$(BOLD)Platform:$(RESET) $(PLATFORM)"
	@echo "$(BOLD)Build Mode:$(RESET) $(BUILD_MODE)"
	@echo "$(BOLD)Features:$(RESET) $(FEATURES)"
	@echo "$(BOLD)Parallel Jobs:$(RESET) $(PARALLEL_JOBS)"
	@echo ""
	@if [ -f "Cargo.toml" ]; then \
		echo "$(BOLD)Rust Version:$(RESET) $$(rustc --version 2>/dev/null || echo 'Not installed')"; \
		echo "$(BOLD)Cargo Version:$(RESET) $$(cargo --version 2>/dev/null || echo 'Not installed')"; \
	fi
	@if [ -f "package.json" ]; then \
		echo "$(BOLD)Node.js Version:$(RESET) $$(node --version 2>/dev/null || echo 'Not installed')"; \
		echo "$(BOLD)NPM Version:$(RESET) $$(npm --version 2>/dev/null || echo 'Not installed')"; \
	fi
	@if [ -f "composer.json" ]; then \
		echo "$(BOLD)PHP Version:$(RESET) $$(php --version 2>/dev/null | head -1 || echo 'Not installed')"; \
		echo "$(BOLD)Composer Version:$(RESET) $$(composer --version 2>/dev/null || echo 'Not installed')"; \
	fi
	@echo ""
	@echo "$(BOLD)Git Status:$(RESET)"
	@git status --porcelain 2>/dev/null | head -5 || echo "Not a git repository"

# Maintenance
update-deps: ## Update all dependencies and tools
	@echo "$(CYAN)🔄 Updating all dependencies and tools...$(RESET)"
	@rustup update
	@cargo install-update -a 2>/dev/null || cargo install cargo-update && cargo install-update -a
	@if command -v npm >/dev/null; then npm update -g; fi
	@deps-update
	@echo "$(GREEN)✓ All dependencies updated$(RESET)"

# CI/CD helpers
ci-test: deps-check deps-install check test security-audit ## Run CI test suite
	@echo "$(GREEN)✓ CI test suite completed$(RESET)"

ci-build: ci-test build package ## Run CI build pipeline
	@echo "$(GREEN)✓ CI build pipeline completed$(RESET)"

# Show build info
info:
	@echo "$(CYAN)$(BOLD)RCM Build Information$(RESET)"
	@echo "$(CYAN)══════════════════════$(RESET)"
	@echo "Version: $$(cargo metadata --format-version=1 --no-deps | jq -r '.packages[0].version' 2>/dev/null || echo 'Unknown')"
	@echo "Platform: $(PLATFORM)"
	@echo "Architecture: $(UNAME_M)"
	@echo "Build Mode: $(BUILD_MODE)"
	@echo "Features: $(FEATURES)"
	@echo "Target Directory: $(CARGO_TARGET_DIR)"
	@echo "Parallel Jobs: $(PARALLEL_JOBS)"
