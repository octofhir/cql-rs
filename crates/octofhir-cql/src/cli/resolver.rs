//! Library resolution utilities

use anyhow::{anyhow, Context, Result};
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Library resolver that handles file lookups and caching
pub struct LibraryResolver {
    /// Search paths for libraries
    search_paths: Vec<PathBuf>,
    /// Cached libraries (path -> content)
    cache: Arc<RwLock<HashMap<PathBuf, String>>>,
    /// Track loading dependencies to detect cycles
    loading: Arc<RwLock<HashSet<PathBuf>>>,
}

impl LibraryResolver {
    /// Create a new library resolver
    pub fn new(search_paths: Vec<PathBuf>) -> Self {
        let mut paths = search_paths;

        // Add CQL_LIBRARY_PATH environment variable
        if let Ok(env_path) = std::env::var("CQL_LIBRARY_PATH") {
            for path in env_path.split(':') {
                if !path.is_empty() {
                    paths.push(PathBuf::from(path));
                }
            }
        }

        // Add current directory if not already included
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        if !paths.contains(&current_dir) {
            paths.insert(0, current_dir);
        }

        Self {
            search_paths: paths,
            cache: Arc::new(RwLock::new(HashMap::new())),
            loading: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Resolve a library by name and version
    pub fn resolve(&self, name: &str, version: Option<&str>) -> Result<String> {
        // Try different file patterns
        let patterns = if let Some(ver) = version {
            vec![
                format!("{}-{}.cql", name, ver),
                format!("{}_{}.cql", name, ver),
                format!("{}.{}.cql", name, ver),
                format!("{}.cql", name),
            ]
        } else {
            vec![format!("{}.cql", name)]
        };

        for pattern in patterns {
            if let Some(path) = self.find_file(&pattern) {
                return self.load_file(&path);
            }
        }

        Err(anyhow!(
            "Library not found: {} version {}",
            name,
            version.unwrap_or("(any)")
        ))
    }

    /// Resolve a library from a specific file path
    pub fn resolve_path(&self, path: &Path) -> Result<String> {
        self.load_file(path)
    }

    /// Find a file in the search paths
    fn find_file(&self, filename: &str) -> Option<PathBuf> {
        for search_path in &self.search_paths {
            let candidate = search_path.join(filename);
            if candidate.exists() && candidate.is_file() {
                return Some(candidate);
            }
        }
        None
    }

    /// Load a file with caching and cycle detection
    fn load_file(&self, path: &Path) -> Result<String> {
        let canonical_path = path
            .canonicalize()
            .with_context(|| format!("Failed to resolve path: {}", path.display()))?;

        // Check cache first
        {
            let cache = self.cache.read();
            if let Some(content) = cache.get(&canonical_path) {
                return Ok(content.clone());
            }
        }

        // Check for circular dependencies
        {
            let mut loading = self.loading.write();
            if loading.contains(&canonical_path) {
                return Err(anyhow!(
                    "Circular dependency detected: {}",
                    canonical_path.display()
                ));
            }
            loading.insert(canonical_path.clone());
        }

        // Load file
        let content = fs::read_to_string(&canonical_path)
            .with_context(|| format!("Failed to read file: {}", canonical_path.display()))?;

        // Cache the content
        {
            let mut cache = self.cache.write();
            cache.insert(canonical_path.clone(), content.clone());
        }

        // Remove from loading set
        {
            let mut loading = self.loading.write();
            loading.remove(&canonical_path);
        }

        Ok(content)
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write();
        cache.clear();
    }

    /// Get the search paths
    pub fn search_paths(&self) -> &[PathBuf] {
        &self.search_paths
    }
}

impl Default for LibraryResolver {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("Test.cql");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "library Test version '1.0.0'").unwrap();

        let resolver = LibraryResolver::new(vec![temp_dir.path().to_path_buf()]);
        let content = resolver.resolve("Test", Some("1.0.0")).unwrap();
        assert!(content.contains("library Test"));
    }

    #[test]
    fn test_cache() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("Test.cql");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "library Test version '1.0.0'").unwrap();

        let resolver = LibraryResolver::new(vec![temp_dir.path().to_path_buf()]);

        // First load
        let content1 = resolver.resolve("Test", Some("1.0.0")).unwrap();

        // Modify file
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "library Test version '2.0.0'").unwrap();

        // Second load should return cached content
        let content2 = resolver.resolve("Test", Some("1.0.0")).unwrap();
        assert_eq!(content1, content2);

        // Clear cache
        resolver.clear_cache();

        // Third load should get new content
        let content3 = resolver.resolve("Test", Some("1.0.0")).unwrap();
        assert!(content3.contains("2.0.0"));
    }
}
