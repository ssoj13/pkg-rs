#!/usr/bin/env bash
# bootstrap.sh - Build script for packager-rs (Unix version)
#
# Commands:
#   build   - Build Rust CLI (release by default)
#   python  - Build Python module via maturin
#   test    - Run tests
#   bench   - Run benchmarks
#   docs    - Build documentation (mdbook + rustdoc)
#   clean   - Clean build artifacts
#   help    - Show this help

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEBUG_BUILD=false
INSTALL=false

# Colors (auto-detect terminal support)
if [[ -t 1 ]] && [[ -n "$TERM" ]] && [[ "$TERM" != "dumb" ]]; then
    CYAN='\033[0;36m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    RED='\033[0;31m'
    WHITE='\033[0;37m'
    NC='\033[0m' # No Color
else
    CYAN='' GREEN='' YELLOW='' RED='' WHITE='' NC=''
fi

# ============================================================
# HELPERS
# ============================================================

format_time() {
    local ms=$1
    if (( ms < 1000 )); then
        printf "%dms" "$ms"
    elif (( ms < 60000 )); then
        printf "%.1fs" "$(echo "scale=1; $ms / 1000" | bc)"
    else
        local min=$((ms / 60000))
        local sec=$(echo "scale=0; ($ms % 60000) / 1000" | bc)
        printf "%dm%ds" "$min" "$sec"
    fi
}

write_header() {
    local text=$1
    local line="=================================================="
    echo ""
    echo -e "${CYAN}${line}${NC}"
    echo -e "${CYAN} ${text}${NC}"
    echo -e "${CYAN}${line}${NC}"
}

# Get current time in milliseconds
get_time_ms() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS: use perl for milliseconds
        perl -MTime::HiRes=time -e 'printf "%.0f\n", time * 1000'
    else
        # Linux: use date with nanoseconds
        echo $(($(date +%s%N) / 1000000))
    fi
}

# ============================================================
# HELP
# ============================================================

show_help() {
    cat << 'EOF'

 PACKAGER-RS BUILD SCRIPT

 COMMANDS
   build     Build Rust CLI binary
   python    Build Python module (maturin develop)
   test      Run cargo tests
   bench     Run criterion benchmarks
   docs      Build documentation
   clean     Clean build artifacts
   help      Show this help

 OPTIONS
   -d, --debug     Build in debug mode (default: release)
   -i, --install   For python: install in current venv (default: build only)
   -h, --help      Show this help

 EXAMPLES
   ./bootstrap.sh build              # Build release CLI
   ./bootstrap.sh build -d           # Build debug CLI
   ./bootstrap.sh python             # Build wheel only (release)
   ./bootstrap.sh python -i          # Build and install in venv
   ./bootstrap.sh python -d          # Build wheel (debug)
   ./bootstrap.sh test               # Run all tests

 PYTHON USAGE (after python build)
   from packager import Package, Env, Evar, App, Storage, Solver

   pkg = Package("maya", "2026.1.0")
   pkg.add_req("redshift@>=3.5")
   
   storage = Storage.scan()
   pkg.solve(storage.packages)
   print(pkg.deps)

EOF
}

# ============================================================
# BUILD
# ============================================================

do_build() {
    local build_type="release"
    $DEBUG_BUILD && build_type="debug"
    
    write_header "BUILD ($build_type)"
    
    local start_time=$(get_time_ms)
    
    cd "$SCRIPT_DIR"
    
    if $DEBUG_BUILD; then
        echo -e "${YELLOW}  cargo build...${NC}"
        cargo build
    else
        echo -e "${YELLOW}  cargo build --release...${NC}"
        cargo build --release
    fi
    
    local end_time=$(get_time_ms)
    local elapsed=$((end_time - start_time))
    
    local exe_path
    if $DEBUG_BUILD; then
        exe_path="$SCRIPT_DIR/target/debug/pkg"
    else
        exe_path="$SCRIPT_DIR/target/release/pkg"
    fi
    
    echo ""
    echo -e "${GREEN}  Done! ($(format_time $elapsed))${NC}"
    echo -e "${CYAN}  Binary: $exe_path${NC}"
    echo ""
}

# ============================================================
# PYTHON
# ============================================================

do_python() {
    local build_type="release"
    $DEBUG_BUILD && build_type="debug"
    
    write_header "PYTHON BUILD ($build_type)"
    
    # Check maturin
    if ! command -v maturin &> /dev/null; then
        echo ""
        echo -e "${RED}  ERROR: maturin not found${NC}"
        echo -e "${YELLOW}  Install: pip install maturin${NC}"
        echo ""
        exit 1
    fi
    
    local start_time=$(get_time_ms)
    
    cd "$SCRIPT_DIR"
    
    if $INSTALL; then
        # Build and install in current venv
        if $DEBUG_BUILD; then
            echo -e "${YELLOW}  maturin develop...${NC}"
            maturin develop
        else
            echo -e "${YELLOW}  maturin develop --release...${NC}"
            maturin develop --release
        fi
    else
        # Build wheel only
        if $DEBUG_BUILD; then
            echo -e "${YELLOW}  maturin build...${NC}"
            maturin build
        else
            echo -e "${YELLOW}  maturin build --release...${NC}"
            maturin build --release
        fi
    fi
    
    local end_time=$(get_time_ms)
    local elapsed=$((end_time - start_time))
    
    echo ""
    echo -e "${GREEN}  Done! ($(format_time $elapsed))${NC}"
    
    if $INSTALL; then
        echo ""
        echo -e "${CYAN}  Usage:${NC}"
        echo -e "${WHITE}    python -c \"from packager import Package; print(Package('test', '1.0.0'))\"${NC}"
    else
        # Show wheel location
        local wheel_dir="$SCRIPT_DIR/target/wheels"
        if [[ -d "$wheel_dir" ]]; then
            local wheel=$(ls -t "$wheel_dir"/*.whl 2>/dev/null | head -1)
            if [[ -n "$wheel" ]]; then
                echo -e "${CYAN}  Wheel: $wheel${NC}"
                echo ""
                echo -e "${WHITE}  Install with:${NC}"
                echo -e "${YELLOW}    pip install $wheel${NC}"
            fi
        fi
    fi
    echo ""
}

# ============================================================
# TEST
# ============================================================

do_test() {
    write_header "TEST"
    
    local start_time=$(get_time_ms)
    
    cd "$SCRIPT_DIR"
    
    # Unit tests
    echo -e "${YELLOW}  cargo test --lib...${NC}"
    cargo test --lib
    
    # Integration tests
    echo ""
    echo -e "${YELLOW}  cargo test --test integration...${NC}"
    cargo test --test integration
    
    local end_time=$(get_time_ms)
    local elapsed=$((end_time - start_time))
    
    echo ""
    echo -e "${GREEN}  All tests passed! ($(format_time $elapsed))${NC}"
    echo ""
}

# ============================================================
# BENCH
# ============================================================

do_bench() {
    write_header "BENCH"
    
    local start_time=$(get_time_ms)
    
    cd "$SCRIPT_DIR"
    
    echo -e "${YELLOW}  cargo bench...${NC}"
    cargo bench --bench scan_bench
    
    local end_time=$(get_time_ms)
    local elapsed=$((end_time - start_time))
    
    echo ""
    echo -e "${GREEN}  Benchmarks complete! ($(format_time $elapsed))${NC}"
    echo -e "${CYAN}  Report: target/criterion/report/index.html${NC}"
    echo ""
}

# ============================================================
# DOCS
# ============================================================

do_docs() {
    write_header "DOCS"
    
    local start_time=$(get_time_ms)
    
    cd "$SCRIPT_DIR"
    
    # Build rustdoc
    echo -e "${YELLOW}  cargo doc...${NC}"
    cargo doc --no-deps
    
    # Build mdbook if installed
    if command -v mdbook &> /dev/null; then
        echo -e "${YELLOW}  mdbook build...${NC}"
        cd docs
        mdbook build
        cd "$SCRIPT_DIR"
    else
        echo -e "${YELLOW}  mdbook not found, skipping (cargo install mdbook)${NC}"
    fi
    
    local end_time=$(get_time_ms)
    local elapsed=$((end_time - start_time))
    
    echo ""
    echo -e "${GREEN}  Done! ($(format_time $elapsed))${NC}"
    echo -e "${CYAN}  Rustdoc: target/doc/packager/index.html${NC}"
    if command -v mdbook &> /dev/null; then
        echo -e "${CYAN}  Mdbook:  docs/book/index.html${NC}"
    fi
    echo ""
}

# ============================================================
# CLEAN
# ============================================================

do_clean() {
    write_header "CLEAN"
    
    cd "$SCRIPT_DIR"
    
    echo -e "${YELLOW}  cargo clean...${NC}"
    cargo clean
    
    echo ""
    echo -e "${GREEN}  Done!${NC}"
    echo ""
}

# ============================================================
# MAIN
# ============================================================

# Parse arguments
MODE=""
while [[ $# -gt 0 ]]; do
    case $1 in
        -d|--debug)
            DEBUG_BUILD=true
            shift
            ;;
        -i|--install)
            INSTALL=true
            shift
            ;;
        -h|--help|help)
            show_help
            exit 0
            ;;
        build|python|test|bench|docs|clean)
            MODE=$1
            shift
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            show_help
            exit 1
            ;;
    esac
done

# No command = help
if [[ -z "$MODE" ]]; then
    show_help
    exit 0
fi

# Execute command
case $MODE in
    build)  do_build ;;
    python) do_python ;;
    test)   do_test ;;
    bench)  do_bench ;;
    docs)   do_docs ;;
    clean)  do_clean ;;
esac
