#!/usr/bin/env python3
"""
bootstrap.py - Cross-platform build script for packager-rs

Commands:
    build   - Build Rust CLI (release by default)
    python  - Build Python module via maturin
    test    - Run tests
    bench   - Run benchmarks
    docs    - Build documentation (mdbook + rustdoc)
    clean   - Clean build artifacts
    help    - Show this help
"""

import argparse
import shutil
import subprocess
import sys
import time
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent.resolve()

# ANSI colors (disabled on Windows cmd without ANSI support)
def _supports_color() -> bool:
    if not sys.stdout.isatty():
        return False
    if sys.platform == "win32":
        # Windows 10+ supports ANSI in modern terminals
        import os
        return os.environ.get("TERM") or os.environ.get("WT_SESSION")
    return True

if _supports_color():
    CYAN, GREEN, YELLOW, RED, WHITE, NC = (
        "\033[0;36m", "\033[0;32m", "\033[0;33m",
        "\033[0;31m", "\033[0;37m", "\033[0m"
    )
else:
    CYAN = GREEN = YELLOW = RED = WHITE = NC = ""


# ============================================================
# HELPERS
# ============================================================

def fmt_time(ms: float) -> str:
    """Format milliseconds to human-readable string."""
    if ms < 1000:
        return f"{ms:.0f}ms"
    elif ms < 60000:
        return f"{ms / 1000:.1f}s"
    else:
        m, s = divmod(ms / 1000, 60)
        return f"{int(m)}m{int(s)}s"


def header(text: str) -> None:
    """Print a colored header."""
    line = "=" * 50
    print(f"\n{CYAN}{line}{NC}")
    print(f"{CYAN} {text}{NC}")
    print(f"{CYAN}{line}{NC}")


def run(*args: str, check: bool = True) -> int:
    """Run a command, return exit code."""
    print(f"{YELLOW}  {' '.join(args)}...{NC}")
    result = subprocess.run(args, cwd=SCRIPT_DIR)
    if check and result.returncode != 0:
        print(f"\n{RED}  Command failed!{NC}")
        sys.exit(1)
    return result.returncode


def cmd_exists(name: str) -> bool:
    """Check if command exists in PATH."""
    return shutil.which(name) is not None


# ============================================================
# COMMANDS
# ============================================================

def do_build(debug: bool = False) -> None:
    build_type = "debug" if debug else "release"
    header(f"BUILD ({build_type})")

    start = time.time()

    if debug:
        run("cargo", "build")
    else:
        run("cargo", "build", "--release")

    elapsed = (time.time() - start) * 1000

    exe_name = "pkg.exe" if sys.platform == "win32" else "pkg"
    exe_path = SCRIPT_DIR / "target" / build_type / exe_name

    print(f"\n{GREEN}  Done! ({fmt_time(elapsed)}){NC}")
    print(f"{CYAN}  Binary: {exe_path}{NC}\n")


def do_python(debug: bool = False, install: bool = False) -> None:
    build_type = "debug" if debug else "release"
    header(f"PYTHON BUILD ({build_type})")

    if not cmd_exists("maturin"):
        print(f"\n{RED}  ERROR: maturin not found{NC}")
        print(f"{YELLOW}  Install: pip install maturin{NC}\n")
        sys.exit(1)

    start = time.time()

    if install:
        if debug:
            run("maturin", "develop")
        else:
            run("maturin", "develop", "--release")
    else:
        if debug:
            run("maturin", "build")
        else:
            run("maturin", "build", "--release")

    elapsed = (time.time() - start) * 1000

    print(f"\n{GREEN}  Done! ({fmt_time(elapsed)}){NC}")

    if install:
        print(f"\n{CYAN}  Usage:{NC}")
        print(f'{WHITE}    python -c "from packager import Package; print(Package(\'test\', \'1.0.0\'))"{NC}')
    else:
        wheel_dir = SCRIPT_DIR / "target" / "wheels"
        if wheel_dir.exists():
            wheels = sorted(wheel_dir.glob("*.whl"), key=lambda p: p.stat().st_mtime, reverse=True)
            if wheels:
                print(f"{CYAN}  Wheel: {wheels[0]}{NC}")
                print(f"\n{WHITE}  Install with:{NC}")
                print(f"{YELLOW}    pip install {wheels[0]}{NC}")
    print()


def do_test() -> None:
    header("TEST")

    start = time.time()

    # Unit tests
    run("cargo", "test", "--lib")

    # Integration tests
    print()
    run("cargo", "test", "--test", "integration")

    elapsed = (time.time() - start) * 1000

    print(f"\n{GREEN}  All tests passed! ({fmt_time(elapsed)}){NC}\n")


def do_bench() -> None:
    header("BENCH")

    start = time.time()

    run("cargo", "bench", "--bench", "scan_bench")

    elapsed = (time.time() - start) * 1000

    print(f"\n{GREEN}  Benchmarks complete! ({fmt_time(elapsed)}){NC}")
    print(f"{CYAN}  Report: target/criterion/report/index.html{NC}\n")


def do_docs() -> None:
    header("DOCS")

    start = time.time()

    # Rustdoc
    run("cargo", "doc", "--no-deps")

    # mdbook
    if cmd_exists("mdbook"):
        print(f"{YELLOW}  mdbook build...{NC}")
        subprocess.run(["mdbook", "build"], cwd=SCRIPT_DIR / "docs")
    else:
        print(f"{YELLOW}  mdbook not found, skipping (cargo install mdbook){NC}")

    elapsed = (time.time() - start) * 1000

    print(f"\n{GREEN}  Done! ({fmt_time(elapsed)}){NC}")
    print(f"{CYAN}  Rustdoc: target/doc/packager/index.html{NC}")
    if cmd_exists("mdbook"):
        print(f"{CYAN}  Mdbook:  docs/book/index.html{NC}")
    print()


def do_clean() -> None:
    header("CLEAN")

    run("cargo", "clean")

    print(f"\n{GREEN}  Done!{NC}\n")


def show_help() -> None:
    print("""
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
   python bootstrap.py build              # Build release CLI
   python bootstrap.py build -d           # Build debug CLI
   python bootstrap.py python             # Build wheel only (release)
   python bootstrap.py python -i          # Build and install in venv
   python bootstrap.py python -d          # Build wheel (debug)
   python bootstrap.py test               # Run all tests

 PYTHON USAGE (after python build)
   from packager import Package, Env, Evar, App, Storage, Solver

   pkg = Package("maya", "2026.1.0")
   pkg.add_req("redshift@>=3.5")

   storage = Storage.scan()
   pkg.solve(storage.packages)
   print(pkg.deps)
""")


# ============================================================
# MAIN
# ============================================================

def main() -> None:
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("command", nargs="?", default="help",
                        choices=["build", "python", "test", "bench", "docs", "clean", "help"])
    parser.add_argument("-d", "--debug", action="store_true", help="Debug build")
    parser.add_argument("-i", "--install", action="store_true", help="Install in venv")
    parser.add_argument("-h", "--help", action="store_true", help="Show help")

    args = parser.parse_args()

    if args.help or args.command == "help":
        show_help()
        return

    match args.command:
        case "build":
            do_build(debug=args.debug)
        case "python":
            do_python(debug=args.debug, install=args.install)
        case "test":
            do_test()
        case "bench":
            do_bench()
        case "docs":
            do_docs()
        case "clean":
            do_clean()


if __name__ == "__main__":
    main()
