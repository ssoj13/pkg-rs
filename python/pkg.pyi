"""Type stubs for packager - Rust package manager with Python bindings."""

from typing import Optional, Dict, List, Any, Union, Literal
from enum import IntEnum

class SolveStatus(IntEnum):
    """Status of package dependency resolution."""
    NotSolved = 0
    Solved = 1
    Failed = 2
    
    def is_ok(self) -> bool:
        """Check if status is Solved."""
        ...
    
    def is_error(self) -> bool:
        """Check if status is Failed."""
        ...
    
    def was_attempted(self) -> bool:
        """Check if resolution was attempted."""
        ...

class Action:
    """Environment variable action."""
    Set: "Action"
    Append: "Action"
    Insert: "Action"

class Evar:
    """Environment variable with action semantics."""
    
    name: str
    value: str
    action: Action
    
    def __init__(
        self,
        name: str,
        value: str,
        action: str = "set"  # "set", "append", "insert"
    ) -> None: ...
    
    def solve(self, env: "Env", max_depth: int = 10) -> "Evar":
        """Expand {TOKEN} references against environment."""
        ...
    
    def to_dict(self) -> Dict[str, Any]: ...
    
    @staticmethod
    def from_dict(d: Dict[str, Any]) -> "Evar": ...
    
    def to_json(self) -> str: ...
    
    @staticmethod
    def from_json(s: str) -> "Evar": ...

class Env:
    """Named collection of environment variables."""
    
    name: str
    evars: List[Evar]
    
    def __init__(self, name: str) -> None: ...
    
    def add(self, evar: Evar) -> None:
        """Add an environment variable."""
        ...
    
    def get(self, name: str) -> Optional[Evar]:
        """Get evar by name."""
        ...
    
    def merge(self, other: "Env") -> "Env":
        """Merge with another env."""
        ...
    
    def compress(self) -> "Env":
        """Collapse same-name evars using action semantics."""
        ...
    
    def solve(self, max_depth: int = 10, compress: bool = True) -> "Env":
        """Expand all {TOKEN} references."""
        ...
    
    def commit(self) -> None:
        """Apply to current process environment."""
        ...
    
    def to_dict(self) -> Dict[str, Any]: ...
    
    @staticmethod
    def from_dict(d: Dict[str, Any]) -> "Env": ...
    
    def to_json(self) -> str: ...
    
    @staticmethod
    def from_json(s: str) -> "Env": ...
    
    def __add__(self, other: "Env") -> "Env": ...

class App:
    """Application definition."""
    
    name: str
    path: Optional[str]
    env_name: Optional[str]
    args: List[str]
    cwd: Optional[str]
    properties: Dict[str, str]
    
    def __init__(
        self,
        name: str,
        path: Optional[str] = None,
        env_name: Optional[str] = None,
        args: Optional[List[str]] = None,
        cwd: Optional[str] = None,
        properties: Optional[Dict[str, str]] = None,
    ) -> None: ...
    
    def with_path(self, path: str) -> "App":
        """Builder: set executable path."""
        ...
    
    def with_env(self, env_name: str) -> "App":
        """Builder: set environment name."""
        ...
    
    def with_cwd(self, cwd: str) -> "App":
        """Builder: set working directory."""
        ...
    
    def with_arg(self, arg: str) -> "App":
        """Builder: add argument."""
        ...
    
    def with_property(self, key: str, value: str) -> "App":
        """Builder: set property."""
        ...
    
    def get_prop(self, key: str) -> Optional[str]: ...
    def set_prop(self, key: str, value: str) -> None: ...
    def has_prop(self, key: str) -> bool: ...
    def remove_prop(self, key: str) -> Optional[str]: ...
    
    def effective_cwd(self) -> Optional[str]:
        """Get working directory (explicit or from path)."""
        ...
    
    def path_exists(self) -> bool:
        """Check if executable exists."""
        ...
    
    def build_args(self, extra_args: Optional[List[str]] = None) -> List[str]:
        """Build complete argument list."""
        ...
    
    def is_hidden(self) -> bool: ...
    def icon(self) -> Optional[str]: ...
    def engine(self) -> Optional[str]: ...
    
    def launch(
        self,
        env: Optional[Union["Env", Dict[str, str]]] = None,
        extra_args: Optional[List[str]] = None,
        wait: bool = False,
    ) -> int:
        """Launch the application.
        
        Args:
            env: Environment - Env object or dict {"VAR": "value"}
            extra_args: Additional command-line arguments
            wait: Wait for process to complete
            
        Returns:
            Exit code if wait=True, else 0
        """
        ...
    
    def to_dict(self) -> Dict[str, Any]: ...
    
    @staticmethod
    def from_dict(d: Dict[str, Any]) -> "App": ...
    
    def to_json(self) -> str: ...
    
    @staticmethod
    def from_json(s: str) -> "App": ...

class Package:
    """Software package definition."""
    
    name: str
    base: str
    version: str
    description: Optional[str]
    envs: List[Env]
    apps: List[App]
    reqs: List[str]
    build_requires: List[str]
    private_build_requires: List[str]
    requires_rez_version: Optional[str]
    has_plugins: Optional[bool]
    plugin_for: List[str]
    build_system: Optional[str]
    build_command: Optional[Union[Literal[False], str, List[str]]]
    build_directory: Optional[str]
    build_args: List[str]
    pre_build_commands: Optional[str]
    pre_commands: Optional[str]
    commands: Optional[str]
    post_commands: Optional[str]
    pre_test_commands: Optional[str]
    config: Optional[Any]
    variants: List[List[str]]
    hashed_variants: bool
    relocatable: Optional[bool]
    cachable: Optional[bool]
    deps: List["Package"]
    tags: List[str]
    uuid: Optional[str]
    icon: Optional[str]
    pip_name: Optional[str]
    from_pip: bool
    is_pure_python: bool
    help: Optional[Any]
    tests: Optional[Any]
    authors: List[str]
    tools: List[str]
    timestamp: Optional[int]
    revision: Optional[Any]
    changelog: Optional[str]
    release_message: Optional[str]
    previous_version: Optional[str]
    previous_revision: Optional[Any]
    vcs: Optional[str]
    solve_status: SolveStatus
    solve_error: Optional[str]
    package_source: Optional[str]
    
    def __init__(self, base: str, version: str) -> None: ...
    
    def add_env(self, env: Env) -> None: ...
    def add_app(self, app: App) -> None: ...
    def add_req(self, req: str) -> None: ...
    def add_build_req(self, req: str) -> None: ...
    def add_tag(self, tag: str) -> None: ...
    
    def get_env(self, name: str) -> Optional[Env]: ...
    def get_app(self, name: str) -> Optional[App]: ...
    
    def has_req(self, base_name: str) -> bool: ...
    def has_tag(self, tag: str) -> bool: ...
    
    def default_env(self) -> Optional[Env]: ...
    def default_app(self) -> Optional[App]: ...
    
    def app_names(self) -> List[str]: ...
    def env_names(self) -> List[str]: ...
    
    def effective_env(self, app_name: Optional[str] = None) -> Optional[Env]:
        """Get solved environment for an app."""
        ...
    
    def semver(self) -> str:
        """Validate and return SemVer string."""
        ...
    
    def satisfies(self, constraint: str) -> bool:
        """Check if version matches constraint."""
        ...
    
    def solve(self, available: List["Package"]) -> None:
        """Resolve dependencies and fill deps field."""
        ...
    
    def is_solved(self) -> bool:
        """Check if dependencies are solved."""
        ...
    
    def status(self) -> SolveStatus:
        """Get detailed solve status."""
        ...
    
    def to_dict(self) -> Dict[str, Any]: ...
    
    @staticmethod
    def from_dict(d: Dict[str, Any]) -> "Package": ...
    
    def to_json(self) -> str: ...
    def to_json_pretty(self) -> str: ...
    
    @staticmethod
    def from_json(s: str) -> "Package": ...

class Loader:
    """Package.py file loader."""
    
    def __init__(self, use_cache: Optional[bool] = None) -> None: ...
    
    def load(self, path: str, **kwargs: Any) -> Package:
        """Load package from package.py file.
        
        Args:
            path: Path to package.py file
            **kwargs: Keyword arguments for get_package()
        """
        ...
    
    def clear_cache(self) -> None:
        """Clear the package cache."""
        ...
    
    def cache_size(self) -> int:
        """Get cache size."""
        ...
    
    def is_cached(self, path: str) -> bool:
        """Check if a path is cached."""
        ...

class Solver:
    """Dependency resolver using PubGrub."""
    
    def __init__(self, packages: List[Package]) -> None: ...
    
    def solve(self, root: str) -> List[str]:
        """Resolve dependencies for a package."""
        ...
    
    def solve_reqs(self, reqs: List[str]) -> List[str]:
        """Resolve a list of requirements."""
        ...

class Storage:
    """Package scanner and registry."""
    
    packages: List[Package]
    locations: List[str]
    warnings: List[str]
    
    def __init__(self) -> None: ...
    
    @staticmethod
    def scan() -> "Storage":
        """Scan default rezconfig packages_path (including REZ_PACKAGES_PATH)."""
        ...
    
    @staticmethod
    def from_paths(paths: List[str]) -> "Storage":
        """Scan specific paths."""
        ...
    
    def get(self, name: str) -> Optional[Package]:
        """Get package by full name."""
        ...
    
    def versions(self, base: str) -> List[str]:
        """Get all versions of a package (newest first)."""
        ...
    
    def bases(self) -> List[str]:
        """Get all base package names."""
        ...
    
    def has(self, name: str) -> bool: ...
    def has_base(self, base: str) -> bool: ...
    
    def list(self, tags: Optional[List[str]] = None) -> List[Package]:
        """List packages, optionally filtered by tags."""
        ...
    
    def find(self, pattern: str) -> List[str]:
        """Find packages matching glob pattern."""
        ...
    
    def latest(self, base: str) -> Optional[Package]:
        """Get latest version of a package."""
        ...
    
    def add(self, pkg: Package) -> None:
        """Manually add a package."""
        ...
    
    def refresh(self) -> "Storage":
        """Rescan locations."""
        ...
    
    def __len__(self) -> int: ...
    def __contains__(self, name: str) -> bool: ...
