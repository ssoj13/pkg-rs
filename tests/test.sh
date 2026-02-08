#!/usr/bin/env zsh
# test.sh - Package manager test script
# Usage: ./test.sh [command]
# Commands: gen, basic, conflict

set -e

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PKG="$ROOT/target/release/pkg"
TEST_DIR="$(dirname "$0")"
REPO="$TEST_DIR/repo"
REPO_BAD="$TEST_DIR/repo_bad"

# Check if pkg exists
if [ ! -f "$PKG" ]; then
    PKG="$ROOT/target/debug/pkg"
    if [ ! -f "$PKG" ]; then
        echo "ERROR: pkg not found. Run 'cargo build --release' first."
        exit 1
    fi
fi

show_help() {
    echo "test.sh - Package manager test script"
    echo ""
    echo "Usage: ./test.sh <command>"
    echo ""
    echo "Commands:"
    echo "  gen      Generate test repositories (repo/ and repo_bad/)"
    echo "  basic    Run basic operations test (list, info, env)"
    echo "  conflict Run dependency conflict resolution test"
    echo ""
    echo "Examples:"
    echo "  ./test.sh gen"
    echo "  ./test.sh basic"
    echo "  ./test.sh conflict"
}

run_gen() {
    echo "=== GENERATING TEST REPOSITORIES ==="
    echo ""
    
    echo "1. Generating main repo ($REPO)..."
    "$PKG" gen-repo -n 200 -V 5 --dep-rate 0.4 --seed 42 -o "$REPO"
    
    echo ""
    echo "Done."
}

run_basic() {
    export REZ_PACKAGES_PATH="$REPO"
    
    echo "=== BASIC OPERATIONS TEST ==="
    echo "Using: $REZ_PACKAGES_PATH"
    echo ""
    
    echo "1. Scan packages:"
    "$PKG" scan
    echo ""
    
    echo "2. List packages:"
    "$PKG" list
    echo ""
    
    # Find first maya package dynamically (JSON returns array of strings)
    MAYA_PKG=$("$PKG" list maya --json 2>/dev/null | jq -r '.[0] // "maya"')
    echo "Selected package: $MAYA_PKG"
    echo ""
    
    echo "3. Package info:"
    "$PKG" info "$MAYA_PKG"
    echo ""
    
    echo "4. Environment variables:"
    "$PKG" env "$MAYA_PKG"
    echo ""
    
    echo "=== TEST COMPLETE ==="
}

run_conflict() {
    export REZ_PACKAGES_PATH="$REPO_BAD"
    
    echo "=== DEPENDENCY RESOLUTION TEST ==="
    echo "Using: $REZ_PACKAGES_PATH"
    echo ""
    
    echo "1. Available packages:"
    "$PKG" list
    echo ""
    
    echo "=================================================="
    echo "2. vfx_legacy-1.0.0 (UNSOLVABLE CONFLICT):"
    "$PKG" solve vfx_legacy-1.0.0
    echo ""
    
    echo "=================================================="
    echo "3. vfx_project-1.0.0 (SOLVER FINDS SOLUTION):"
    "$PKG" solve vfx_project-1.0.0
    echo ""
    
    echo "=================================================="
    echo "4. vfx_project-2.0.0 (NO CONFLICT):"
    "$PKG" solve vfx_project-2.0.0
    echo ""
    
    echo "=== TEST COMPLETE ==="
}

# Main
case "$1" in
    gen)      run_gen ;;
    basic)    run_basic ;;
    conflict) run_conflict ;;
    *)        show_help ;;
esac
