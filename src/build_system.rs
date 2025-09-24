// Zenlang Build System Implementation
// Provides project building, dependency management, and compilation orchestration

use crate::error::{CompileError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Build configuration for a Zen project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    pub name: String,
    pub version: String,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub license: Option<String>,

    /// Main entry point
    pub main: Option<String>,

    /// Library entry point
    pub lib: Option<String>,

    /// Dependencies
    pub dependencies: HashMap<String, Dependency>,

    /// Build dependencies
    pub build_dependencies: HashMap<String, Dependency>,

    /// Target configurations
    pub targets: HashMap<String, TargetConfig>,

    /// Build scripts
    pub build_script: Option<String>,

    /// Compiler flags
    pub compiler_flags: Vec<String>,

    /// Linker flags
    pub linker_flags: Vec<String>,

    /// Features
    pub features: HashMap<String, Vec<String>>,
    pub default_features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub version: String,
    pub path: Option<String>,
    pub git: Option<String>,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub features: Vec<String>,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetConfig {
    pub platform: Platform,
    pub arch: Architecture,
    pub optimization: OptimizationLevel,
    pub debug: bool,
    pub strip: bool,
    pub lto: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    Linux,
    MacOS,
    Windows,
    Wasm,
    Custom(u32), // For custom platform IDs
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Architecture {
    X86,
    X86_64,
    ARM,
    ARM64,
    RISCV32,
    RISCV64,
    Wasm32,
    Wasm64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationLevel {
    None,           // -O0
    Basic,          // -O1
    Standard,       // -O2
    Aggressive,     // -O3
    Size,           // -Os
    SizeAggressive, // -Oz
}

/// Build context managing the compilation process
pub struct BuildContext {
    pub config: BuildConfig,
    pub project_root: PathBuf,
    pub build_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub target: TargetConfig,
    pub verbose: bool,
    pub parallel_jobs: usize,

    /// Resolved dependencies
    dependencies: HashMap<String, ResolvedDependency>,

    /// Build graph for incremental compilation
    build_graph: BuildGraph,
}

#[derive(Debug)]
struct ResolvedDependency {
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub is_local: bool,
}

/// Build graph for tracking dependencies and incremental compilation
struct BuildGraph {
    nodes: HashMap<String, BuildNode>,
    edges: HashMap<String, HashSet<String>>,
}

struct BuildNode {
    pub path: PathBuf,
    pub hash: u64,
    pub last_modified: std::time::SystemTime,
    pub dependencies: Vec<String>,
    pub is_dirty: bool,
}

impl BuildContext {
    /// Create a new build context from a project directory
    pub fn new(project_root: impl AsRef<Path>) -> Result<Self> {
        let project_root = project_root.as_ref().to_path_buf();
        let config_path = project_root.join("zen.toml");

        if !config_path.exists() {
            return Err(CompileError::BuildError(format!(
                "No zen.toml found in {}",
                project_root.display()
            )));
        }

        let config_str = fs::read_to_string(&config_path)
            .map_err(|e| CompileError::BuildError(format!("Failed to read zen.toml: {}", e)))?;

        let config: BuildConfig = toml::from_str(&config_str)
            .map_err(|e| CompileError::BuildError(format!("Failed to parse zen.toml: {}", e)))?;

        let build_dir = project_root.join("build");
        let cache_dir = project_root.join(".zen-cache");

        // Create directories if they don't exist
        fs::create_dir_all(&build_dir).ok();
        fs::create_dir_all(&cache_dir).ok();

        // Detect target platform
        let target = Self::detect_target();

        Ok(Self {
            config,
            project_root,
            build_dir,
            cache_dir,
            target,
            verbose: false,
            parallel_jobs: num_cpus::get(),
            dependencies: HashMap::new(),
            build_graph: BuildGraph {
                nodes: HashMap::new(),
                edges: HashMap::new(),
            },
        })
    }

    /// Detect the current target platform
    fn detect_target() -> TargetConfig {
        let platform = if cfg!(target_os = "linux") {
            Platform::Linux
        } else if cfg!(target_os = "macos") {
            Platform::MacOS
        } else if cfg!(target_os = "windows") {
            Platform::Windows
        } else {
            Platform::Linux // Default
        };

        let arch = if cfg!(target_arch = "x86_64") {
            Architecture::X86_64
        } else if cfg!(target_arch = "x86") {
            Architecture::X86
        } else if cfg!(target_arch = "aarch64") {
            Architecture::ARM64
        } else if cfg!(target_arch = "arm") {
            Architecture::ARM
        } else {
            Architecture::X86_64 // Default
        };

        TargetConfig {
            platform,
            arch,
            optimization: OptimizationLevel::Standard,
            debug: cfg!(debug_assertions),
            strip: false,
            lto: false,
        }
    }

    /// Resolve all dependencies
    pub fn resolve_dependencies(&mut self) -> Result<()> {
        let deps: Vec<(String, Dependency)> = self
            .config
            .dependencies
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for (name, dep) in deps {
            self.resolve_dependency(&name, &dep)?;
        }
        Ok(())
    }

    /// Resolve a single dependency
    fn resolve_dependency(&mut self, name: &str, dep: &Dependency) -> Result<()> {
        let resolved = if let Some(path) = &dep.path {
            // Local path dependency
            let dep_path = self.project_root.join(path);
            if !dep_path.exists() {
                return Err(CompileError::BuildError(format!(
                    "Local dependency '{}' not found at {}",
                    name,
                    dep_path.display()
                )));
            }
            ResolvedDependency {
                name: name.to_string(),
                version: dep.version.clone(),
                path: dep_path,
                is_local: true,
            }
        } else if let Some(git_url) = &dep.git {
            // Git dependency
            self.fetch_git_dependency(name, git_url, dep)?
        } else {
            // Registry dependency (would connect to package registry)
            self.fetch_registry_dependency(name, &dep.version)?
        };

        self.dependencies.insert(name.to_string(), resolved);
        Ok(())
    }

    /// Fetch a dependency from git
    fn fetch_git_dependency(
        &self,
        name: &str,
        url: &str,
        dep: &Dependency,
    ) -> Result<ResolvedDependency> {
        let deps_dir = self.cache_dir.join("deps");
        fs::create_dir_all(&deps_dir).ok();

        let dep_dir = deps_dir.join(name);

        if !dep_dir.exists() {
            // Clone the repository
            let mut cmd = Command::new("git");
            cmd.arg("clone").arg(url).arg(&dep_dir);

            if self.verbose {
                println!("Cloning {} from {}", name, url);
            }

            let output = cmd.output().map_err(|e| {
                CompileError::BuildError(format!("Failed to clone {}: {}", name, e))
            })?;

            if !output.status.success() {
                return Err(CompileError::BuildError(format!(
                    "Failed to clone {}: {}",
                    name,
                    String::from_utf8_lossy(&output.stderr)
                )));
            }

            // Checkout specific branch or tag if specified
            if let Some(branch) = &dep.branch {
                Command::new("git")
                    .current_dir(&dep_dir)
                    .args(&["checkout", branch])
                    .output()
                    .map_err(|e| {
                        CompileError::BuildError(format!("Failed to checkout branch: {}", e))
                    })?;
            } else if let Some(tag) = &dep.tag {
                Command::new("git")
                    .current_dir(&dep_dir)
                    .args(&["checkout", &format!("tags/{}", tag)])
                    .output()
                    .map_err(|e| {
                        CompileError::BuildError(format!("Failed to checkout tag: {}", e))
                    })?;
            }
        }

        Ok(ResolvedDependency {
            name: name.to_string(),
            version: dep.version.clone(),
            path: dep_dir,
            is_local: false,
        })
    }

    /// Fetch a dependency from the package registry
    fn fetch_registry_dependency(&self, name: &str, version: &str) -> Result<ResolvedDependency> {
        // This would connect to a package registry server
        // For now, we'll simulate it with a local cache
        let registry_dir = self.cache_dir.join("registry");
        fs::create_dir_all(&registry_dir).ok();

        let dep_dir = registry_dir.join(format!("{}-{}", name, version));

        if !dep_dir.exists() {
            // In a real implementation, this would download from a registry
            return Err(CompileError::BuildError(format!(
                "Package '{}' version '{}' not found in registry",
                name, version
            )));
        }

        Ok(ResolvedDependency {
            name: name.to_string(),
            version: version.to_string(),
            path: dep_dir,
            is_local: false,
        })
    }

    /// Build the project
    pub fn build(&mut self) -> Result<PathBuf> {
        // Resolve dependencies first
        self.resolve_dependencies()?;

        // Run build script if present
        if let Some(script) = &self.config.build_script {
            self.run_build_script(script)?;
        }

        // Collect all source files
        let source_files = self.collect_source_files()?;

        // Build dependency graph
        self.build_dependency_graph(&source_files)?;

        // Compile in dependency order
        let output_path = if let Some(main) = &self.config.main {
            self.build_executable(main, &source_files)?
        } else if let Some(lib) = &self.config.lib {
            self.build_library(lib, &source_files)?
        } else {
            return Err(CompileError::BuildError(
                "No main or lib entry point specified in zen.toml".to_string(),
            ));
        };

        Ok(output_path)
    }

    /// Run a build script
    fn run_build_script(&self, script: &str) -> Result<()> {
        let script_path = self.project_root.join(script);

        if !script_path.exists() {
            return Err(CompileError::BuildError(format!(
                "Build script '{}' not found",
                script
            )));
        }

        if self.verbose {
            println!("Running build script: {}", script);
        }

        // Compile and run the build script
        // This would use the Zen compiler to compile and execute the script

        Ok(())
    }

    /// Collect all source files in the project
    fn collect_source_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let src_dir = self.project_root.join("src");

        if src_dir.exists() {
            self.collect_zen_files(&src_dir, &mut files)?;
        }

        // Also collect from dependencies
        for dep in self.dependencies.values() {
            let dep_src = dep.path.join("src");
            if dep_src.exists() {
                self.collect_zen_files(&dep_src, &mut files)?;
            }
        }

        Ok(files)
    }

    /// Recursively collect .zen files
    fn collect_zen_files(&self, dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        for entry in fs::read_dir(dir)
            .map_err(|e| CompileError::BuildError(format!("Failed to read directory: {}", e)))?
        {
            let entry = entry
                .map_err(|e| CompileError::BuildError(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();

            if path.is_dir() {
                self.collect_zen_files(&path, files)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("zen") {
                files.push(path);
            }
        }

        Ok(())
    }

    /// Build the dependency graph for incremental compilation
    fn build_dependency_graph(&mut self, files: &[PathBuf]) -> Result<()> {
        for file in files {
            let content = fs::read_to_string(file)
                .map_err(|e| CompileError::BuildError(format!("Failed to read file: {}", e)))?;

            // Parse imports from the file
            let imports = self.extract_imports(&content);

            // Calculate file hash for change detection
            let hash = self.calculate_file_hash(&content);

            let metadata = fs::metadata(file).map_err(|e| {
                CompileError::BuildError(format!("Failed to get file metadata: {}", e))
            })?;

            let node = BuildNode {
                path: file.clone(),
                hash,
                last_modified: metadata.modified().unwrap_or(std::time::SystemTime::now()),
                dependencies: imports,
                is_dirty: false,
            };

            let file_str = file.to_string_lossy().to_string();
            self.build_graph.nodes.insert(file_str.clone(), node);
        }

        // Mark dirty nodes (files that have changed)
        self.mark_dirty_nodes()?;

        Ok(())
    }

    /// Extract import statements from a Zen file
    fn extract_imports(&self, content: &str) -> Vec<String> {
        let mut imports = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("@std.build.import") || trimmed.starts_with("import ") {
                // Extract the module name
                if let Some(start) = trimmed.find('"') {
                    if let Some(end) = trimmed[start + 1..].find('"') {
                        let module = &trimmed[start + 1..start + 1 + end];
                        imports.push(module.to_string());
                    }
                }
            }
        }

        imports
    }

    /// Calculate a hash for file content
    fn calculate_file_hash(&self, content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    /// Mark nodes as dirty if they've changed since last build
    fn mark_dirty_nodes(&mut self) -> Result<()> {
        let cache_file = self.cache_dir.join("build_cache.json");

        if cache_file.exists() {
            // Load previous build state
            let cache_str = fs::read_to_string(&cache_file)
                .map_err(|e| CompileError::BuildError(format!("Failed to read cache: {}", e)))?;

            if let Ok(cache) = serde_json::from_str::<HashMap<String, u64>>(&cache_str) {
                for (file, node) in &mut self.build_graph.nodes {
                    if let Some(prev_hash) = cache.get(file) {
                        if *prev_hash != node.hash {
                            node.is_dirty = true;
                        }
                    } else {
                        // New file
                        node.is_dirty = true;
                    }
                }
            }
        } else {
            // First build, all nodes are dirty
            for node in self.build_graph.nodes.values_mut() {
                node.is_dirty = true;
            }
        }

        // Propagate dirty flag to dependents
        self.propagate_dirty_flag();

        Ok(())
    }

    /// Propagate dirty flag to all dependents
    fn propagate_dirty_flag(&mut self) {
        let mut changed = true;
        while changed {
            changed = false;
            let dirty_files: Vec<String> = self
                .build_graph
                .nodes
                .iter()
                .filter(|(_, node)| node.is_dirty)
                .map(|(file, _)| file.clone())
                .collect();

            for dirty_file in dirty_files {
                // Find all files that depend on this dirty file
                for (_file, node) in &mut self.build_graph.nodes {
                    if !node.is_dirty && node.dependencies.contains(&dirty_file) {
                        node.is_dirty = true;
                        changed = true;
                    }
                }
            }
        }
    }

    /// Build an executable
    fn build_executable(&self, main: &str, _source_files: &[PathBuf]) -> Result<PathBuf> {
        let main_path = self.project_root.join("src").join(main);

        if !main_path.exists() {
            return Err(CompileError::BuildError(format!(
                "Main file '{}' not found",
                main
            )));
        }

        let output_name = self.config.name.clone();
        let output_path = self.build_dir.join(&output_name);

        if self.verbose {
            println!("Building executable: {}", output_name);
        }

        // Here we would invoke the Zen compiler
        // For now, we'll simulate it

        // Save build cache
        self.save_build_cache()?;

        Ok(output_path)
    }

    /// Build a library
    fn build_library(&self, lib: &str, _source_files: &[PathBuf]) -> Result<PathBuf> {
        let lib_path = self.project_root.join("src").join(lib);

        if !lib_path.exists() {
            return Err(CompileError::BuildError(format!(
                "Library file '{}' not found",
                lib
            )));
        }

        let output_name = format!("lib{}.a", self.config.name);
        let output_path = self.build_dir.join(&output_name);

        if self.verbose {
            println!("Building library: {}", output_name);
        }

        // Here we would invoke the Zen compiler to build a library

        // Save build cache
        self.save_build_cache()?;

        Ok(output_path)
    }

    /// Save the build cache for incremental compilation
    fn save_build_cache(&self) -> Result<()> {
        let cache_file = self.cache_dir.join("build_cache.json");

        let cache: HashMap<String, u64> = self
            .build_graph
            .nodes
            .iter()
            .map(|(file, node)| (file.clone(), node.hash))
            .collect();

        let cache_str = serde_json::to_string_pretty(&cache)
            .map_err(|e| CompileError::BuildError(format!("Failed to serialize cache: {}", e)))?;

        fs::write(&cache_file, cache_str)
            .map_err(|e| CompileError::BuildError(format!("Failed to write cache: {}", e)))?;

        Ok(())
    }

    /// Clean build artifacts
    pub fn clean(&self) -> Result<()> {
        if self.build_dir.exists() {
            fs::remove_dir_all(&self.build_dir).map_err(|e| {
                CompileError::BuildError(format!("Failed to clean build directory: {}", e))
            })?;
        }

        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir).map_err(|e| {
                CompileError::BuildError(format!("Failed to clean cache directory: {}", e))
            })?;
        }

        println!("Build artifacts cleaned");
        Ok(())
    }

    /// Run tests
    pub fn test(&self, filter: Option<&str>) -> Result<()> {
        let test_dir = self.project_root.join("tests");

        if !test_dir.exists() {
            println!("No tests directory found");
            return Ok(());
        }

        let mut test_files = Vec::new();
        self.collect_zen_files(&test_dir, &mut test_files)?;

        if let Some(filter) = filter {
            test_files.retain(|f| f.to_string_lossy().contains(filter));
        }

        println!("Running {} tests", test_files.len());

        for test_file in test_files {
            println!("  Testing: {}", test_file.display());
            // Here we would compile and run the test file
        }

        Ok(())
    }
}

/// Package manager for Zen
pub struct PackageManager {
    registry_url: String,
    cache_dir: PathBuf,
}

impl PackageManager {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let cache_dir = PathBuf::from(home).join(".zen").join("packages");

        Self {
            registry_url: "https://packages.zenlang.org".to_string(),
            cache_dir,
        }
    }

    /// Install a package
    pub fn install(&self, name: &str, version: Option<&str>) -> Result<()> {
        let version = version.unwrap_or("latest");

        println!("Installing {} v{}", name, version);

        // This would download from the registry
        // For now, we'll simulate it

        Ok(())
    }

    /// Publish a package to the registry
    pub fn publish(&self, config: &BuildConfig) -> Result<()> {
        println!("Publishing {} v{}", config.name, config.version);

        // This would upload to the registry
        // For now, we'll simulate it

        Ok(())
    }

    /// Search for packages
    pub fn search(&self, query: &str) -> Result<Vec<PackageInfo>> {
        println!("Searching for: {}", query);

        // This would query the registry
        // For now, return empty results

        Ok(Vec::new())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub authors: Vec<String>,
    pub downloads: u64,
}
