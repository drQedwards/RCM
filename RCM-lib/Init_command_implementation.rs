//! Init command implementation
//! 
//! Initializes RCM workspace with specified package managers and templates

use anyhow::{anyhow, Result};
use console::style;
use dialoguer::{Confirm, Input, MultiSelect, Select};
use std::collections::HashMap;
use crate::workspace::Workspace;

/// Initialize RCM workspace
pub async fn run(
    workspace: &Workspace, 
    managers: Option<Vec<String>>, 
    template: &str
) -> Result<()> {
    println!("{}", style("🚀 Initializing RCM workspace...").cyan().bold());
    
    // Check if workspace is already initialized
    let rcm_dir = workspace.root().join(".rcm");
    if rcm_dir.exists() {
        let overwrite = Confirm::new()
            .with_prompt("RCM workspace already exists. Overwrite?")
            .default(false)
            .interact()?;
        
        if !overwrite {
            println!("{}", style("✋ Initialization cancelled.").yellow());
            return Ok(());
        }
    }
    
    // Interactive setup if no managers specified
    let selected_managers = if let Some(mgrs) = managers {
        mgrs
    } else {
        interactive_manager_selection().await?
    };
    
    // Validate template
    let templates = vec!["rust", "node", "php", "polyglot"];
    if !templates.contains(&template) {
        return Err(anyhow!("Invalid template '{}'. Available: {}", template, templates.join(", ")));
    }
    
    println!("{}", style(format!("📋 Using template: {}", template)).green());
    println!("{}", style(format!("📦 Selected managers: {}", selected_managers.join(", "))).green());
    
    // Create workspace clone for modification
    let mut workspace_mut = workspace.clone();
    
    // Initialize workspace
    workspace_mut.initialize(Some(selected_managers.clone()), template).await?;
    
    // Create initial files based on template
    create_template_files(workspace, template, &selected_managers).await?;
    
    // Create .gitignore if it doesn't exist
    create_gitignore(workspace).await?;
    
    // Create README if it doesn't exist
    create_readme(workspace, template, &selected_managers).await?;
    
    println!("{}", style("✅ RCM workspace initialized successfully!").green().bold());
    println!();
    println!("{}", style("Next steps:").bold());
    println!("  • Run {} to ensure all dependencies are installed", style("rcm ensure").cyan());
    println!("  • Run {} to add new packages", style("rcm add <package>").cyan());
    println!("  • Run {} to see available commands", style("rcm --help").cyan());
    
    Ok(())
}

/// Interactive manager selection
async fn interactive_manager_selection() -> Result<Vec<String>> {
    println!("{}", style("🔧 Select package managers to enable:").bold());
    
    let available_managers = vec![
        ("cargo", "Rust package manager"),
        ("npm", "Node.js package manager"),
        ("composer", "PHP package manager"),
        ("system", "System package manager (apt, yum, brew, etc.)"),
    ];
    
    let selections = MultiSelect::new()
        .with_prompt("Package managers")
        .items(&available_managers.iter().map(|(name, desc)| format!("{} - {}", name, desc)).collect::<Vec<_>>())
        .defaults(&[false, false, false, true]) // System manager enabled by default
        .interact()?;
    
    let selected: Vec<String> = selections
        .into_iter()
        .map(|i| available_managers[i].0.to_string())
        .collect();
    
    if selected.is_empty() {
        return Err(anyhow!("At least one package manager must be selected"));
    }
    
    Ok(selected)
}

/// Create template-specific files
async fn create_template_files(
    workspace: &Workspace, 
    template: &str, 
    managers: &[String]
) -> Result<()> {
    match template {
        "rust" => create_rust_files(workspace).await?,
        "node" => create_node_files(workspace).await?,
        "php" => create_php_files(workspace).await?,
        "polyglot" => {
            if managers.contains(&"cargo".to_string()) {
                create_rust_files(workspace).await?;
            }
            if managers.contains(&"npm".to_string()) {
                create_node_files(workspace).await?;
            }
            if managers.contains(&"composer".to_string()) {
                create_php_files(workspace).await?;
            }
            create_polyglot_files(workspace).await?;
        }
        _ => return Err(anyhow!("Unknown template: {}", template)),
    }
    
    Ok(())
}

/// Create Rust-specific files
async fn create_rust_files(workspace: &Workspace) -> Result<()> {
    let cargo_toml = workspace.root().join("Cargo.toml");
    if !cargo_toml.exists() {
        let workspace_name = workspace.root()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("rust-project");
        
        let content = format!(r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"
authors = []
description = "A Rust project managed by RCM"
license = "MIT"

[dependencies]
anyhow = "1.0"
tokio = {{ version = "1.0", features = ["full"] }}

[dev-dependencies]
"#, workspace_name);
        
        tokio::fs::write(cargo_toml, content).await?;
        println!("{}", style("📄 Created Cargo.toml").green());
    }
    
    // Create src/main.rs
    let src_dir = workspace.root().join("src");
    if !src_dir.exists() {
        tokio::fs::create_dir_all(&src_dir).await?;
        
        let main_rs = src_dir.join("main.rs");
        let content = r#"fn main() {
    println!("Hello from RCM Rust project!");
}
"#;
        tokio::fs::write(main_rs, content).await?;
        println!("{}", style("📄 Created src/main.rs").green());
    }
    
    Ok(())
}

/// Create Node.js-specific files
async fn create_node_files(workspace: &Workspace) -> Result<()> {
    let package_json = workspace.root().join("package.json");
    if !package_json.exists() {
        let workspace_name = workspace.root()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("node-project");
        
        let content = format!(r#"{{
  "name": "{}",
  "version": "1.0.0",
  "description": "A Node.js project managed by RCM",
  "main": "index.js",
  "scripts": {{
    "start": "node index.js",
    "test": "echo \"Error: no test specified\" && exit 1",
    "dev": "node --watch index.js"
  }},
  "keywords": ["rcm", "nodejs"],
  "author": "",
  "license": "MIT",
  "dependencies": {{}},
  "devDependencies": {{}}
}}
"#, workspace_name);
        
        tokio::fs::write(package_json, content).await?;
        println!("{}", style("📄 Created package.json").green());
    }
    
    // Create index.js
    let index_js = workspace.root().join("index.js");
    if !index_js.exists() {
        let content = r#"#!/usr/bin/env node

console.log('Hello from RCM Node.js project!');

// Example async function
async function main() {
    try {
        console.log('Starting application...');
        // Your application logic here
    } catch (error) {
        console.error('Error:', error);
        process.exit(1);
    }
}

if (require.main === module) {
    main();
}

module.exports = { main };
"#;
        tokio::fs::write(index_js, content).await?;
        println!("{}", style("📄 Created index.js").green());
    }
    
    Ok(())
}

/// Create PHP-specific files
async fn create_php_files(workspace: &Workspace) -> Result<()> {
    let composer_json = workspace.root().join("composer.json");
    if !composer_json.exists() {
        let workspace_name = workspace.root()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("php-project");
        
        let content = format!(r#"{{
    "name": "vendor/{}",
    "description": "A PHP project managed by RCM",
    "type": "project",
    "license": "MIT",
    "authors": [
        {{
            "name": "Developer",
            "email": "dev@example.com"
        }}
    ],
    "require": {{
        "php": "^8.1"
    }},
    "require-dev": {{
        "phpunit/phpunit": "^10.0"
    }},
    "autoload": {{
        "psr-4": {{
            "App\\\\": "src/"
        }}
    }},
    "autoload-dev": {{
        "psr-4": {{
            "App\\\\Tests\\\\": "tests/"
        }}
    }},
    "scripts": {{
        "test": "phpunit",
        "start": "php -S localhost:8000 -t public"
    }}
}}
"#, workspace_name);
        
        tokio::fs::write(composer_json, content).await?;
        println!("{}", style("📄 Created composer.json").green());
    }
    
    // Create src directory structure
    let src_dir = workspace.root().join("src");
    if !src_dir.exists() {
        tokio::fs::create_dir_all(&src_dir).await?;
        
        let main_php = src_dir.join("App.php");
        let content = r#"<?php

namespace App;

class App
{
    public function run(): void
    {
        echo "Hello from RCM PHP project!\n";
    }
}
"#;
        tokio::fs::write(main_php, content).await?;
        println!("{}", style("📄 Created src/App.php").green());
    }
    
    // Create public directory
    let public_dir = workspace.root().join("public");
    if !public_dir.exists() {
        tokio::fs::create_dir_all(&public_dir).await?;
        
        let index_php = public_dir.join("index.php");
        let content = r#"<?php

require_once __DIR__ . '/../vendor/autoload.php';

use App\App;

$app = new App();
$app->run();
"#;
        tokio::fs::write(index_php, content).await?;
        println!("{}", style("📄 Created public/index.php").green());
    }
    
    Ok(())
}

/// Create polyglot-specific files
async fn create_polyglot_files(workspace: &Workspace) -> Result<()> {
    // Create Makefile for unified commands
    let makefile = workspace.root().join("Makefile");
    if !makefile.exists() {
        let content = r#".PHONY: install build test clean run dev

# Install all dependencies
install:
	@echo "Installing dependencies..."
	@rcm ensure

# Build all projects
build:
	@echo "Building all projects..."
	@if [ -f "Cargo.toml" ]; then cargo build; fi
	@if [ -f "package.json" ]; then npm run build 2>/dev/null || true; fi
	@if [ -f "composer.json" ]; then composer install --optimize-autoloader; fi

# Run tests for all projects
test:
	@echo "Running tests..."
	@if [ -f "Cargo.toml" ]; then cargo test; fi
	@if [ -f "package.json" ]; then npm test 2>/dev/null || true; fi
	@if [ -f "composer.json" ]; then composer run test 2>/dev/null || true; fi

# Clean all build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@if [ -d "target" ]; then rm -rf target; fi
	@if [ -d "node_modules" ]; then rm -rf node_modules; fi
	@if [ -d "vendor" ]; then rm -rf vendor; fi

# Run development server (detects project type)
dev:
	@if [ -f "Cargo.toml" ]; then cargo run; \
	elif [ -f "package.json" ]; then npm run dev 2>/dev/null || npm start; \
	elif [ -f "composer.json" ]; then composer run start 2>/dev/null || php -S localhost:8000 -t public; \
	else echo "No recognized project type found"; fi

# Show project status
status:
	@echo "=== RCM Project Status ==="
	@rcm workspace list
	@echo
	@if [ -f "Cargo.toml" ]; then echo "🦀 Rust project detected"; fi
	@if [ -f "package.json" ]; then echo "📦 Node.js project detected"; fi
	@if [ -f "composer.json" ]; then echo "🐘 PHP project detected"; fi
"#;
        tokio::fs::write(makefile, content).await?;
        println!("{}", style("📄 Created Makefile").green());
    }
    
    // Create docker-compose.yml for development
    let docker_compose = workspace.root().join("docker-compose.yml");
    if !docker_compose.exists() {
        let content = r#"version: '3.8'

services:
  # Uncomment services as needed for your project
  
  # postgres:
  #   image: postgres:15
  #   environment:
  #     POSTGRES_DB: development
  #     POSTGRES_USER: dev
  #     POSTGRES_PASSWORD: password
  #   ports:
  #     - "5432:5432"
  #   volumes:
  #     - postgres_data:/var/lib/postgresql/data
  
  # redis:
  #   image: redis:7-alpine
  #   ports:
  #     - "6379:6379"
  
  # mysql:
  #   image: mysql:8
  #   environment:
  #     MYSQL_DATABASE: development
  #     MYSQL_USER: dev
  #     MYSQL_PASSWORD: password
  #     MYSQL_ROOT_PASSWORD: rootpassword
  #   ports:
  #     - "3306:3306"
  #   volumes:
  #     - mysql_data:/var/lib/mysql

volumes:
  postgres_data:
  mysql_data:
"#;
        tokio::fs::write(docker_compose, content).await?;
        println!("{}", style("📄 Created docker-compose.yml").green());
    }
    
    Ok(())
}

/// Create .gitignore file
async fn create_gitignore(workspace: &Workspace) -> Result<()> {
    let gitignore = workspace.root().join(".gitignore");
    if !gitignore.exists() {
        let content = r#"# RCM
.rcm/cache/
.rcm/temp/

# Rust
/target/
**/*.rs.bk
Cargo.lock

# Node.js
node_modules/
npm-debug.log*
yarn-debug.log*
yarn-error.log*
.npm
.eslintcache
package-lock.json
yarn.lock

# PHP
/vendor/
composer.lock
.phpunit.result.cache

# Python
__pycache__/
*.py[cod]
*$py.class
*.so
.Python
venv/
env/
.env
.venv

# Go
*.exe
*.exe~
*.dll
*.so
*.dylib
*.test
*.out
go.work

# IDEs and editors
.vscode/
.idea/
*.swp
*.swo
*~

# OS
.DS_Store
.DS_Store?
._*
.Spotlight-V100
.Trashes
ehthumbs.db
Thumbs.db

# Logs
*.log
logs/

# Runtime data
pids/
*.pid
*.seed
*.pid.lock

# Coverage directory used by tools like istanbul
coverage/
*.lcov

# Dependency directories
jspm_packages/

# Optional npm cache directory
.npm

# Optional eslint cache
.eslintcache

# Microbundle cache
.rpt2_cache/
.rts2_cache_cjs/
.rts2_cache_es/
.rts2_cache_umd/

# Optional REPL history
.node_repl_history

# Output of 'npm pack'
*.tgz

# Yarn Integrity file
.yarn-integrity

# dotenv environment variables file
.env
.env.test
.env.production
.env.local
.env.development.local
.env.test.local
.env.production.local

# Database
*.db
*.sqlite
*.sqlite3

# Temporary files
*.tmp
*.temp
.tmp/
.temp/
"#;
        tokio::fs::write(gitignore, content).await?;
        println!("{}", style("📄 Created .gitignore").green());
    }
    
    Ok(())
}

/// Create README.md file
async fn create_readme(workspace: &Workspace, template: &str, managers: &[String]) -> Result<()> {
    let readme = workspace.root().join("README.md");
    if !readme.exists() {
        let project_name = workspace.root()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("RCM Project");
        
        let mut content = format!(r#"# {}

A {} project managed by [RCM](https://github.com/drq-company/rcm) (Rust Cargo Manager).

## Package Managers

This project uses the following package managers:

"#, project_name, template);
        
        for manager in managers {
            let description = match manager.as_str() {
                "cargo" => "🦀 **Rust** - System programming language",
                "npm" => "📦 **Node.js** - JavaScript runtime and package ecosystem",
                "composer" => "🐘 **PHP** - Web development and scripting",
                "system" => "🔧 **System** - OS-level packages and dependencies",
                _ => &format!("📋 **{}** - Package manager", manager),
            };
            content.push_str(&format!("- {}\n", description));
        }
        
        content.push_str(r#"
## Quick Start

```bash
# Install all dependencies
rcm ensure

# Add a new package
rcm add <package-name>

# Show workspace status
rcm workspace list

# Run project-specific commands
"#);
        
        if managers.contains(&"cargo".to_string()) {
            content.push_str(r#"
# Rust commands
cargo build
cargo test
cargo run
"#);
        }
        
        if managers.contains(&"npm".to_string()) {
            content.push_str(r#"
# Node.js commands
npm start
npm test
npm run dev
"#);
        }
        
        if managers.contains(&"composer".to_string()) {
            content.push_str(r#"
# PHP commands
composer install
composer run test
composer run start
"#);
        }
        
        content.push_str(r#"```

## RCM Commands

- `rcm init` - Initialize workspace
- `rcm add <package>` - Add dependency
- `rcm remove <package>` - Remove dependency
- `rcm ensure` - Install all dependencies
- `rcm plan` - Show what would change
- `rcm apply` - Apply planned changes
- `rcm workspace list` - Show all packages
- `rcm let <target> --deploy` - Imperative package installation

## Project Structure

```
"#);
        
        if template == "polyglot" {
            content.push_str(r#".
├── .rcm/                # RCM configuration
├── src/                 # Source code (Rust/PHP)
├── public/              # Public web files (PHP)
├── tests/               # Test files
├── Cargo.toml          # Rust dependencies
├── package.json        # Node.js dependencies
├── composer.json       # PHP dependencies
├── Makefile            # Unified build commands
├── docker-compose.yml  # Development services
└── README.md           # This file
"#);
        } else {
            content.push_str(r#".
├── .rcm/               # RCM configuration
├── src/                # Source code
├── tests/              # Test files
"#);
            match template {
                "rust" => content.push_str("├── Cargo.toml         # Rust dependencies\n"),
                "node" => content.push_str("├── package.json       # Node.js dependencies\n"),
                "php" => content.push_str("├── composer.json      # PHP dependencies\n"),
                _ => {}
            }
            content.push_str("└── README.md          # This file\n");
        }
        
        content.push_str(r#"```

## Development

### Prerequisites

Make sure you have the required tools installed:

"#);
        
        if managers.contains(&"cargo".to_string()) {
            content.push_str("- [Rust](https://rustup.rs/) (latest stable)\n");
        }
        if managers.contains(&"npm".to_string()) {
            content.push_str("- [Node.js](https://nodejs.org/) (LTS version)\n");
        }
        if managers.contains(&"composer".to_string()) {
            content.push_str("- [PHP](https://php.net/) (8.1 or later)\n");
            content.push_str("- [Composer](https://getcomposer.org/)\n");
        }
        
        content.push_str(r#"
### Installation

1. Clone the repository
2. Run `rcm ensure` to install all dependencies
3. Follow platform-specific setup instructions below

### Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `rcm workspace check`
5. Submit a pull request

## License

This project is licensed under the MIT License - see the LICENSE file for details.

---

Generated by RCM - Polyglot Package Manager
"#);
        
        tokio::fs::write(readme, content).await?;
        println!("{}", style("📄 Created README.md").green());
    }
    
    Ok(())
}
