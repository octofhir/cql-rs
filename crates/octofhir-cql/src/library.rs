//! Compiling CQL libraries together with the libraries they include.
//!
//! ```
//! use octofhir_cql::{CompiledLibrary, types::CqlValue};
//!
//! let library = CompiledLibrary::compile(
//!     "library Example version '1.0.0'
//!      define Sum: 1 + 1
//!      define function Double(x Integer): x * 2
//!      define Doubled: Double(Sum)",
//! )?;
//! let results = library.evaluate(&mut library.context())?;
//! assert_eq!(results["Doubled"], CqlValue::Integer(4));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::diagnostics::{CQL0103, CqlError};
use crate::elm::{AstToElmConverter, Library as ElmLibrary};
use crate::eval::{CqlEngine, EvalResult, EvaluationContext};
use crate::types::CqlValue;
use indexmap::IndexMap;
use octofhir_cql_ast::Library as AstLibrary;
use octofhir_cql_eval::references::FHIR_HELPERS;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Supplies the CQL source of included libraries.
pub trait LibrarySource {
    /// Source text of library `name` (as written in the `include`), if available.
    fn load(&self, name: &str, version: Option<&str>) -> Option<String>;
}

impl<F> LibrarySource for F
where
    F: Fn(&str, Option<&str>) -> Option<String>,
{
    fn load(&self, name: &str, version: Option<&str>) -> Option<String> {
        self(name, version)
    }
}

/// A [`LibrarySource`] that never finds anything (no includes available).
pub struct NoLibraries;

impl LibrarySource for NoLibraries {
    fn load(&self, _name: &str, _version: Option<&str>) -> Option<String> {
        None
    }
}

/// Looks up `<Name>-<version>.cql`, `<Name>_<version>.cql` and `<Name>.cql`
/// in a list of directories, in order.
#[derive(Debug, Clone, Default)]
pub struct DirectoryLibrarySource {
    directories: Vec<PathBuf>,
}

impl DirectoryLibrarySource {
    /// Search the given directories.
    pub fn new(directories: impl IntoIterator<Item = impl Into<PathBuf>>) -> Self {
        Self {
            directories: directories.into_iter().map(Into::into).collect(),
        }
    }

    /// The path a library would be loaded from.
    pub fn find(&self, name: &str, version: Option<&str>) -> Option<PathBuf> {
        let mut candidates = Vec::new();
        if let Some(version) = version {
            candidates.push(format!("{name}-{version}.cql"));
            candidates.push(format!("{name}_{version}.cql"));
        }
        candidates.push(format!("{name}.cql"));
        self.directories.iter().find_map(|dir| {
            candidates
                .iter()
                .map(|file| dir.join(file))
                .find(|path| path.is_file())
        })
    }
}

impl LibrarySource for DirectoryLibrarySource {
    fn load(&self, name: &str, version: Option<&str>) -> Option<String> {
        self.find(name, version)
            .and_then(|path| fs::read_to_string(path).ok())
    }
}

/// A translated library plus every library it (transitively) includes.
#[derive(Debug, Clone)]
pub struct CompiledLibrary {
    main: Arc<ElmLibrary>,
    included: Vec<(String, Arc<ElmLibrary>)>,
    diagnostics: Vec<CqlError>,
    unresolved_fhir_helpers: bool,
}

impl CompiledLibrary {
    /// Compile a library that includes nothing but (optionally) FHIRHelpers.
    pub fn compile(source: &str) -> crate::Result<Self> {
        Self::compile_with(source, &NoLibraries)
    }

    /// Parse and translate `source`, loading included libraries from `libraries`.
    ///
    /// `FHIRHelpers` may be missing: its conversion functions are built into
    /// the evaluator. Any other missing include is an error. Name-resolution
    /// problems do not fail compilation; they are reported by
    /// [`diagnostics`](Self::diagnostics).
    pub fn compile_with(source: &str, libraries: &dyn LibrarySource) -> crate::Result<Self> {
        let main_ast = crate::parse(source)?;
        let mut included: Vec<(String, Arc<ElmLibrary>)> = Vec::new();
        let mut diagnostics = Vec::new();
        let mut unresolved_fhir_helpers = false;
        let mut visiting = HashSet::new();
        let main = compile_tree(
            &main_ast,
            source,
            libraries,
            &mut included,
            &mut diagnostics,
            &mut unresolved_fhir_helpers,
            &mut visiting,
        )?;
        Ok(Self {
            main: Arc::new(main),
            included,
            diagnostics,
            unresolved_fhir_helpers,
        })
    }

    /// Compile the library in `path`, resolving includes from its directory
    /// and then from `library_paths`.
    pub fn compile_file(path: &Path, library_paths: &[PathBuf]) -> crate::Result<Self> {
        let source = fs::read_to_string(path).map_err(|e| {
            CqlError::system(
                crate::diagnostics::CQL0401,
                format!("cannot read {}: {e}", path.display()),
            )
        })?;
        let mut dirs: Vec<PathBuf> = path.parent().map(Path::to_path_buf).into_iter().collect();
        dirs.extend(library_paths.iter().cloned());
        Self::compile_with(&source, &DirectoryLibrarySource::new(dirs))
    }

    /// The main library's ELM.
    pub fn elm(&self) -> &ElmLibrary {
        &self.main
    }

    /// Included libraries' ELM, keyed by library name.
    pub fn included(&self) -> impl Iterator<Item = (&str, &ElmLibrary)> {
        self.included
            .iter()
            .map(|(name, lib)| (name.as_str(), lib.as_ref()))
    }

    /// Name-resolution problems found while translating (all libraries).
    pub fn diagnostics(&self) -> &[CqlError] {
        &self.diagnostics
    }

    /// Whether `FHIRHelpers` was included but its source was not available
    /// (its conversions then use the evaluator's built-in implementation).
    pub fn uses_builtin_fhir_helpers(&self) -> bool {
        self.unresolved_fhir_helpers
    }

    /// Value set URLs declared by the main and included libraries.
    pub fn value_set_ids(&self) -> Vec<String> {
        std::iter::once(&self.main)
            .chain(self.included.iter().map(|(_, l)| l))
            .flat_map(|lib| lib.value_sets.iter().flat_map(|v| v.defs.iter()))
            .map(|def| def.id.clone())
            .collect()
    }

    /// A fresh evaluation context with the main and included libraries registered.
    pub fn context(&self) -> EvaluationContext {
        let mut ctx = EvaluationContext::new();
        self.register(&mut ctx);
        ctx
    }

    /// Register the main and included libraries on an existing context.
    pub fn register(&self, ctx: &mut EvaluationContext) {
        ctx.set_main_library(self.main.clone());
        for (name, library) in &self.included {
            ctx.add_library_arc(name.clone(), library.clone());
        }
    }

    /// Evaluate every public expression definition of the main library.
    pub fn evaluate(&self, ctx: &mut EvaluationContext) -> EvalResult<IndexMap<String, CqlValue>> {
        CqlEngine::new().evaluate_library(&self.main, ctx)
    }

    /// Evaluate one expression definition of the main library.
    pub fn evaluate_definition(
        &self,
        name: &str,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        CqlEngine::new().evaluate_expression(&self.main, name, ctx)
    }
}

/// Translate `ast` after compiling (depth first) the libraries it includes.
fn compile_tree(
    ast: &AstLibrary,
    source: &str,
    libraries: &dyn LibrarySource,
    compiled: &mut Vec<(String, Arc<ElmLibrary>)>,
    diagnostics: &mut Vec<CqlError>,
    unresolved_fhir_helpers: &mut bool,
    visiting: &mut HashSet<String>,
) -> crate::Result<ElmLibrary> {
    let name = ast
        .definition
        .as_ref()
        .map_or_else(|| "Anonymous".to_string(), |d| d.name.name.name.clone());
    visiting.insert(name.clone());

    let mut converter = AstToElmConverter::new().with_source(source);
    for include in &ast.includes {
        let include = &include.inner;
        let included_name = include.library.name.name.clone();
        let version = include.version.as_ref().map(|v| v.version.as_str());
        if visiting.contains(&included_name) {
            return Err(CqlError::semantic(
                CQL0103,
                format!("circular include: `{name}` includes `{included_name}`"),
            ));
        }

        let included_source = libraries.load(&included_name, version);
        let Some(included_source) = included_source else {
            if included_name == FHIR_HELPERS {
                *unresolved_fhir_helpers = true;
                continue;
            }
            return Err(CqlError::semantic(
                CQL0103,
                format!(
                    "included library `{included_name}`{} not found",
                    version
                        .map(|v| format!(" version '{v}'"))
                        .unwrap_or_default()
                ),
            ));
        };
        let included_ast = crate::parse(&included_source)?;
        converter.add_included_library(&included_name, &included_ast);

        if compiled.iter().any(|(n, _)| *n == included_name) {
            continue;
        }
        let elm = compile_tree(
            &included_ast,
            &included_source,
            libraries,
            compiled,
            diagnostics,
            unresolved_fhir_helpers,
            visiting,
        )?;
        compiled.push((included_name, Arc::new(elm)));
    }

    let elm = converter.convert_library(ast);
    diagnostics.extend(converter.diagnostics());
    visiting.remove(&name);
    Ok(elm)
}
