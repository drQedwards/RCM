#!/bin/bash
# GPT-lib Complete Usage Example
# Demonstrates AI model management through RCM LET imperatives

set -e

CYAN='\033[36m'
GREEN='\033[32m'
YELLOW='\033[33m'
RED='\033[31m'
RESET='\033[0m'
BOLD='\033[1m'

echo -e "${CYAN}${BOLD}🤖 RCM GPT-lib Complete Demo${RESET}"
echo -e "${CYAN}══════════════════════════════════════${RESET}"

# Create demo project
DEMO_DIR="rcm-gpt-demo"
echo -e "${YELLOW}📁 Creating GPT-enabled project: ${DEMO_DIR}${RESET}"
mkdir -p "$DEMO_DIR"
cd "$DEMO_DIR"

# Initialize RCM workspace with GPT support
echo -e "${CYAN}🔧 Initializing RCM workspace with GPT support...${RESET}"
rcm init --managers cargo,npm,system --template polyglot --features gpt

echo ""

# Demonstrate GPT LET command syntax
echo -e "${CYAN}🚀 GPT LET Command Demonstrations${RESET}"
echo -e "${CYAN}═══════════════════════════════════════${RESET}"

# Install popular models using LET syntax
echo -e "${YELLOW}📦 Installing AI models with LET commands:${RESET}"

# LET imperative for model installation and serving
rcm let gpt install llama2
rcm let gpt install codellama
rcm let gpt install mistral

echo ""

# Serve models using LET imperatives
echo -e "${YELLOW}⚡ Serving models with LET imperatives:${RESET}"

# Basic model serving
rcm let gpt serve llama2 --deploy --creativity 0.7
echo "✅ LLaMA 2 deployed on default port (11434)"

# Code generation model with low creativity
rcm let gpt serve codellama --deploy --port 11435 --creativity 0.2
echo "✅ CodeLLaMA deployed on port 11435 for code generation"

# Creative writing model with high creativity
rcm let gpt serve mistral --deploy --port 11436 --creativity 0.9
echo "✅ Mistral deployed on port 11436 for creative tasks"

echo ""

# Direct model serving with simplified syntax
echo -e "${YELLOW}🎯 Direct model serving (simplified LET syntax):${RESET}"

# These are equivalent to the above but more concise
rcm let llama2 --deploy --test
rcm let codellama --deploy --port 8080 --creativity 0.1
rcm let mistral --deploy --port 8081 --creativity 0.95

echo ""

# Show running models
echo -e "${CYAN}📊 Currently Running Models${RESET}"
echo -e "${CYAN}═══════════════════════════${RESET}"
rcm gpt list --running --format table

echo ""

# Demonstrate text generation
echo -e "${CYAN}🤖 AI Text Generation Examples${RESET}"
echo -e "${CYAN}════════════════════════════════${RESET}"

echo -e "${YELLOW}💬 Chat with LLaMA 2 (General purpose):${RESET}"
rcm gpt generate llama2 "Explain quantum computing in simple terms" --max-tokens 150 --temperature 0.7

echo ""

echo -e "${YELLOW}💻 Code generation with CodeLLaMA:${RESET}"
rcm gpt generate codellama "Write a Rust function to implement binary search" --max-tokens 200 --temperature 0.2

echo ""

echo -e "${YELLOW}✨ Creative writing with Mistral:${RESET}"
rcm gpt generate mistral "Write a short poem about artificial intelligence and humanity" --max-tokens 100 --temperature 0.9

echo ""

# Integration with existing RCM features
echo -e "${CYAN}🔗 RCM Integration Features${RESET}"
echo -e "${CYAN}═══════════════════════════${RESET}"

# Package management with AI integration
echo -e "${YELLOW}📦 AI-enhanced package management:${RESET}"

# Install AI/ML packages alongside model serving
rcm add transformers  # Python transformers library
rcm add candle-core   # Rust ML framework
rcm add ort          # ONNX Runtime bindings

# Build project with GPT features enabled
rcm let cargo --build --features gpt --release

echo ""

# Workspace management with GPT models
echo -e "${YELLOW}🏗️ Workspace management:${RESET}"
rcm workspace check --include-gpt-models
rcm workspace list --format table

# Generate SBOM including GPT models
rcm sbom --out sbom-with-gpt.json --include-gpt-models

# Create snapshot including GPT model state
rcm snapshot --name "gpt-enabled-v1" --include-gpt-state

echo ""

# Advanced use cases
echo -e "${CYAN}🚀 Advanced Use Cases${RESET}"
echo -e "${CYAN}═══════════════════════${RESET}"

# Development assistant workflow
echo -e "${YELLOW}👨‍💻 Development Assistant Workflow:${RESET}"
cat > dev_assistant.sh << 'EOF'
#!/bin/bash
# AI-Powered Development Assistant

echo "🤖 Starting AI Development Assistant"

# Code review assistant
rcm let codellama --deploy --port 9001 --creativity 0.1
echo "Code review assistant ready on port 9001"

# Documentation generator  
rcm let llama2 --deploy --port 9002 --creativity 0.5
echo "Documentation assistant ready on port 9002"

# Bug analysis assistant
rcm let mistral --deploy --port 9003 --creativity 0.3  
echo "Bug analysis assistant ready on port 9003"

echo "✅ Development assistant suite deployed"
EOF

chmod +x dev_assistant.sh
echo "Created dev_assistant.sh - AI development suite"

echo ""

# Content creation workflow
echo -e "${YELLOW}📝 Content Creation Workflow:${RESET}"
cat > content_creator.sh << 'EOF'
#!/bin/bash
# AI Content Creation Pipeline

echo "✍️ Starting AI Content Creation Pipeline"

# Technical writing
rcm let llama2 --deploy --port 9010 --creativity 0.4

# Creative writing
rcm let mistral --deploy --port 9011 --creativity 0.9

# Code documentation
rcm let codellama --deploy --port 9012 --creativity 0.2

# Generate content examples
rcm gpt generate llama2 "Write technical documentation for a REST API" --max-tokens 300
rcm gpt generate mistral "Create a marketing copy for a new tech product" --max-tokens 200
rcm gpt generate codellama "Generate comprehensive code comments for this function" --max-tokens 150

echo "✅ Content creation pipeline active"
EOF

chmod +x content_creator.sh
echo "Created content_creator.sh - AI content creation suite"

echo ""

# Performance optimization examples
echo -e "${CYAN}⚡ Performance Optimization${RESET}"
echo -e "${CYAN}══════════════════════════${RESET}"

echo -e "${YELLOW}🔧 GPU-accelerated serving:${RESET}"
# Enable GPU acceleration if available
rcm let llama2 --deploy --gpu-layers 32 --threads 8 --port 9020

echo -e "${YELLOW}🚀 ARM register optimization for AI:${RESET}"
# Combine ARM and GPT features for maximum performance
rcm arm let simd --deploy --computation llm --vector-size 512
rcm let codellama --deploy --arm-accelerated --port 9021

echo ""

# Multi-model serving orchestration
echo -e "${CYAN}🎼 Multi-Model Orchestration${RESET}"
echo -e "${CYAN}═══════════════════════════════${RESET}"

cat > multi_model_orchestrator.sh << 'EOF'
#!/bin/bash
# Multi-Model AI Orchestrator

echo "🎯 Starting Multi-Model AI Orchestrator"

# Specialized model deployment
declare -A models=(
    ["chat"]="llama2:11434:0.7"
    ["code"]="codellama:11435:0.2"  
    ["creative"]="mistral:11436:0.9"
    ["analysis"]="llama2:11437:0.4"
    ["translate"]="mistral:11438:0.5"
)

for purpose in "${!models[@]}"; do
    IFS=':' read -r model port creativity <<< "${models[$purpose]}"
    echo "Deploying $purpose model: $model on port $port"
    rcm let "$model" --deploy --port "$port" --creativity "$creativity" &
done

wait
echo "✅ All specialized models deployed"

# Health check all models
for purpose in "${!models[@]}"; do
    IFS=':' read -r model port creativity <<< "${models[$purpose]}"
    echo "Testing $purpose model ($model):"
    case $purpose in
        "chat")
            rcm gpt generate "$model" "Hello, how are you?" --max-tokens 50
            ;;
        "code") 
            rcm gpt generate "$model" "def fibonacci(n):" --max-tokens 100
            ;;
        "creative")
            rcm gpt generate "$model" "Once upon a time" --max-tokens 80
            ;;
    esac
    echo "---"
done

echo "🎉 Multi-model orchestration complete!"
EOF

chmod +x multi_model_orchestrator.sh
echo "Created multi_model_orchestrator.sh"

echo ""

# Industrial AI automation
echo -e "${CYAN}🏭 Industrial AI Automation${RESET}"
echo -e "${CYAN}═══════════════════════════${RESET}"

echo -e "${YELLOW}🤖 Utility company AI integration:${RESET}"
cat > industrial_ai.sh << 'EOF'
#!/bin/bash
# Industrial AI Integration

echo "🏭 Starting Industrial AI Systems"

# Log analysis AI
rcm let codellama --deploy --port 9030 --creativity 0.1
echo "Log analysis AI ready"

# Predictive maintenance AI  
rcm let llama2 --deploy --port 9031 --creativity 0.3
echo "Predictive maintenance AI ready"

# Safety monitoring AI
rcm let mistral --deploy --port 9032 --creativity 0.2
echo "Safety monitoring AI ready"

# Integration with robot systems
rcm robot analyze-logs --model codellama --creativity 0.1
rcm robot predict-maintenance --model llama2 --creativity 0.3
rcm robot safety-monitor --model mistral --creativity 0.2

echo "✅ Industrial AI systems integrated with robot controllers"
EOF

chmod +x industrial_ai.sh
echo "Created industrial_ai.sh - Industrial AI automation"

echo ""

# Configuration and management
echo -e "${CYAN}⚙️ Configuration & Management${RESET}"
echo -e "${CYAN}═══════════════════════════════${RESET}"

echo -e "${YELLOW}🔧 GPT-specific configuration:${RESET}"
rcm config set gpt.default_model llama2
rcm config set gpt.serving_defaults.creativity 0.7
rcm config set gpt.serving_defaults.context_length 4096
rcm config set gpt.serving_defaults.max_tokens 512

# Model-specific configuration
rcm gpt config llama2 --set temperature=0.7,context_length=4096,max_tokens=256
rcm gpt config codellama --set temperature=0.2,context_length=8192,max_tokens=1024

echo -e "${YELLOW}📊 Show current configuration:${RESET}"
rcm config show | grep gpt

echo ""

# Performance benchmarking
echo -e "${CYAN}📈 Performance Benchmarking${RESET}"
echo -e "${CYAN}══════════════════════════════${RESET}"

cat > benchmark_models.sh << 'EOF'
#!/bin/bash
# AI Model Performance Benchmarking

echo "📊 Benchmarking AI Model Performance"

models=("llama2" "codellama" "mistral")
test_prompt="Explain machine learning in one paragraph"

for model in "${models[@]}"; do
    echo "Testing $model..."
    start_time=$(date +%s.%N)
    
    rcm gpt generate "$model" "$test_prompt" --max-tokens 100 --temperature 0.5 > /dev/null
    
    end_time=$(date +%s.%N)
    duration=$(echo "$end_time - $start_time" | bc)
    
    echo "$model: ${duration}s"
done

# Memory and GPU usage monitoring
rcm gpt status --detailed

echo "✅ Benchmarking complete"
EOF

chmod +x benchmark_models.sh
echo "Created benchmark_models.sh"

echo ""

# Interactive demos
echo -e "${CYAN}🎮 Interactive Demos${RESET}"
echo -e "${CYAN}═══════════════════════${RESET}"

echo -e "${YELLOW}💬 Interactive chat example:${RESET}"
echo "Starting interactive chat with LLaMA 2..."
echo "Type 'exit' to quit"

# Interactive chat session (simulated)
echo "User: What are the benefits of using RCM for AI model management?"
rcm gpt generate llama2 "What are the benefits of using RCM for AI model management?" --max-tokens 200

echo ""
echo "User: How does the LET imperative work with GPT models?"
rcm gpt generate llama2 "Explain how LET imperative syntax works for AI model deployment" --max-tokens 150

echo ""

# Final status and cleanup options
echo -e "${CYAN}🎉 GPT Integration Demo Complete!${RESET}"
echo ""
echo -e "${GREEN}${BOLD}✅ Successfully demonstrated:${RESET}"
echo "• 🤖 AI model installation and serving with LET imperatives"
echo "• ⚡ Direct model serving with simplified syntax"
echo "• 🔗 Integration with existing RCM package management"
echo "• 🏗️ Workspace management with GPT model tracking"
echo "• 🚀 Advanced multi-model orchestration"
echo "• 🏭 Industrial AI automation integration"
echo "• ⚙️ Configuration management and optimization"
echo "• 📊 Performance monitoring and benchmarking"

echo ""
echo -e "${CYAN}Key GPT commands demonstrated:${RESET}"
echo "• rcm let gpt serve <model> --deploy --creativity <level>"
echo "• rcm let <model> --deploy --port <port> --test"
echo "• rcm gpt install <model> --source <source>"
echo "• rcm gpt generate <model> <prompt> --max-tokens <n>"
echo "• rcm gpt list --running --format table"
echo "• rcm gpt status <model> --detailed"

echo ""
echo -e "${CYAN}Key integration points:${RESET}"
echo "• rcm workspace check --include-gpt-models"
echo "• rcm sbom --include-gpt-models"
echo "• rcm snapshot --include-gpt-state"
echo "• rcm let cargo --build --features gpt"
echo "• rcm robot <command> --model <ai-model>"

echo ""
echo -e "${YELLOW}📚 Available scripts:${RESET}"
echo "• ./dev_assistant.sh - AI development assistant suite"
echo "• ./content_creator.sh - AI content creation pipeline"
echo "• ./multi_model_orchestrator.sh - Multi-model serving"
echo "• ./industrial_ai.sh - Industrial AI automation"
echo "• ./benchmark_models.sh - Performance benchmarking"

echo ""
echo -e "${YELLOW}📖 Model serving endpoints:${RESET}"
echo "• LLaMA 2 (General): http://localhost:11434"
echo "• CodeLLaMA (Code): http://localhost:11435"  
echo "• Mistral (Creative): http://localhost:11436"
echo "• API documentation: http://localhost:11434/docs"

echo ""
echo -e "${YELLOW}🔧 Configuration:${RESET}"
echo "• Default model: llama2"
echo "• Default creativity: 0.7"
echo "• Context length: 4096 tokens"
echo "• Models directory: .rcm/models/"

# Create final summary report
cat > gpt-integration-report.md << EOF
# RCM GPT-lib Integration Report

Generated: $(date)
Project: $DEMO_DIR

## AI Model Management Overview
- **RCM GPT-lib**: AI model serving and management through LET imperatives
- **Supported Models**: LLaMA 2, CodeLLaMA, Mistral, and custom models
- **Backends**: Ollama, LlamaCpp, ONNX, Candle, TensorFlow Serving

## Deployed Models
$(rcm gpt list --running --format table)

## Integration Features
1. **LET Imperatives**: \`rcm let <model> --deploy --creativity <level>\`
2. **Package Integration**: AI models managed alongside traditional packages
3. **Workspace Management**: GPT models included in workspace health and snapshots
4. **Industrial Automation**: AI integration with utility robot systems

## Use Cases Demonstrated
- Development assistant (code generation, review, documentation)
- Content creation (technical writing, creative writing, marketing)
- Industrial automation (log analysis, predictive maintenance, safety monitoring)
- Multi-model orchestration (specialized models for different tasks)

## Performance Features
- GPU acceleration with \`--gpu-layers\` parameter
- ARM register optimization for LLM inference
- Multi-threaded serving with configurable thread counts
- Real-time performance monitoring and benchmarking

## Configuration
- Models stored in: .rcm/models/
- Configurations in: .rcm/gpt-configs/
- Default serving port: 11434
- API compatibility: OpenAI-style endpoints

## Files Generated
- dev_assistant.sh - AI development suite
- content_creator.sh - Content creation pipeline
- multi_model_orchestrator.sh - Multi-model serving
- industrial_ai.sh - Industrial AI integration
- benchmark_models.sh - Performance testing
- gpt-integration-report.md - This report

---
Generated by RCM GPT-lib Integration Demo
EOF

echo -e "${GREEN}📄 Integration report saved to gpt-integration-report.md${RESET}"
echo ""
echo -e "${CYAN}🚀 Ready to revolutionize AI model management with RCM!${RESET}"
