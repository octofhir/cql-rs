//! Library-level reference resolution
//!
//! Evaluates `ExpressionRef`, `FunctionRef`, `ParameterRef` and the
//! terminology references (`ValueSetRef`, `CodeSystemRef`, `CodeRef`,
//! `ConceptRef`), including references into included libraries.
//!
//! - Unqualified references resolve against the library whose definition is
//!   being evaluated (see [`EvaluationContext::current_library`]); qualified
//!   references go through that library's include aliases.
//! - Definitions are evaluated once per context and cached; function bodies
//!   and referenced definitions run with the caller's query scopes hidden.
//! - Parameters take the supplied value, else their declared default, else
//!   `null` (CQL 1.5 §5.1 "Parameters").
//! - Value sets and code systems evaluate to tuples `{ name, id, version }`
//!   carrying the declared identifiers (URLs) that terminology providers use.
//! - `FHIRHelpers` conversions: FHIR data is expected to already be presented
//!   as CQL system values (see `octofhir_cql_model::fhir::json`), so calls to
//!   `FHIRHelpers.ToX(...)` on system values are the identity (or the
//!   equivalent system conversion). A loaded `FHIRHelpers` library is used
//!   for arguments that are still raw FHIR structures.

use crate::context::EvaluationContext;
use crate::engine::CqlEngine;
use crate::error::{EvalError, EvalResult};
use crate::operators::type_ops::value_is_type;
use octofhir_cql_elm::{
    CodeRef, CodeSystemRef, ConceptRef, ExpressionDef, ExpressionRef, FunctionRef, Library,
    ParameterRef, TypeSpecifier, ValueSetRef,
};
use octofhir_cql_types::{CqlCode, CqlConcept, CqlInterval, CqlList, CqlTuple, CqlValue};
use std::sync::Arc;

/// Name of the FHIRHelpers library.
pub const FHIR_HELPERS: &str = "FHIRHelpers";

/// A resolved terminology declaration (value set or code system).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminologyId {
    /// Declared name
    pub name: String,
    /// Declared identifier (canonical URL or OID)
    pub id: String,
    /// Declared version
    pub version: Option<String>,
}

impl TerminologyId {
    /// Tuple representation `{ name, id, version }`.
    pub fn to_value(&self) -> CqlValue {
        CqlValue::Tuple(CqlTuple::from_elements([
            ("name", CqlValue::string(&self.name)),
            ("id", CqlValue::string(&self.id)),
            (
                "version",
                self.version
                    .as_ref()
                    .map_or(CqlValue::Null, CqlValue::string),
            ),
        ]))
    }

    /// Read the `id` (and `version`) back from a value produced by
    /// [`to_value`](Self::to_value), or from a plain string identifier.
    pub fn from_value(value: &CqlValue) -> Option<Self> {
        match value {
            CqlValue::String(id) => Some(Self {
                name: id.clone(),
                id: id.clone(),
                version: None,
            }),
            CqlValue::Tuple(t) => {
                let id = t.get("id")?.as_string()?.to_string();
                Some(Self {
                    name: t
                        .get("name")
                        .and_then(CqlValue::as_string)
                        .unwrap_or(&id)
                        .to_string(),
                    id,
                    version: t
                        .get("version")
                        .and_then(CqlValue::as_string)
                        .map(str::to_string),
                })
            }
            _ => None,
        }
    }
}

impl CqlEngine {
    /// The library a reference qualified by `library_name` points to.
    fn referenced_library(
        &self,
        library_name: Option<&str>,
        ctx: &EvaluationContext,
    ) -> EvalResult<Arc<Library>> {
        match library_name {
            None => ctx
                .current_library()
                .ok_or_else(|| EvalError::internal("no library is being evaluated")),
            Some(alias) => ctx
                .resolve_included(alias)
                .ok_or_else(|| EvalError::UndefinedLibrary {
                    name: alias.to_string(),
                }),
        }
    }

    /// Evaluate the expression definition `name` of `library` (cached).
    pub(crate) fn evaluate_definition(
        &self,
        library: Arc<Library>,
        name: &str,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        let cache_key = format!(
            "def|{}|{}|{}",
            library.identifier.id,
            library.identifier.version.as_deref().unwrap_or(""),
            name
        );
        if let Some(cached) = ctx.get_cached(&cache_key) {
            return Ok(cached);
        }

        let expr = library
            .statements
            .as_ref()
            .and_then(|s| s.defs.iter().find(|d| d.name == name && !d.is_function()))
            .and_then(|d| d.expression.as_deref())
            .ok_or_else(|| EvalError::undefined_expression(name))?;

        ctx.push_library(library.clone());
        let result = ctx.with_isolated_scopes(|ctx| self.evaluate(expr, ctx));
        ctx.pop_library();

        let value = result?;
        ctx.cache_result(cache_key, value.clone());
        Ok(value)
    }

    pub(crate) fn eval_expression_ref(
        &self,
        r: &ExpressionRef,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        let library = self.referenced_library(r.library_name.as_deref(), ctx)?;
        let is_expression = library
            .statements
            .as_ref()
            .is_some_and(|s| s.defs.iter().any(|d| d.name == r.name && !d.is_function()));
        if is_expression {
            return self.evaluate_definition(library, &r.name, ctx);
        }

        // A library-qualified name the translator could not classify (the
        // included library was not available at translation time).
        if r.library_name.is_some() {
            if library
                .parameters
                .as_ref()
                .is_some_and(|p| p.defs.iter().any(|d| d.name == r.name))
            {
                return self.parameter_value(library, &r.name, ctx);
            }
            if let Some(vs) = find_value_set(&library, &r.name) {
                return Ok(vs.to_value());
            }
            if let Some(cs) = find_code_system(&library, &r.name) {
                return Ok(cs.to_value());
            }
            if library
                .codes
                .as_ref()
                .is_some_and(|c| c.defs.iter().any(|d| d.name == r.name))
            {
                return self.code_value(library, &r.name, ctx).map(CqlValue::Code);
            }
            if library
                .concepts
                .as_ref()
                .is_some_and(|c| c.defs.iter().any(|d| d.name == r.name))
            {
                return self.concept_value(library, &r.name, ctx);
            }
        }

        Err(EvalError::undefined_expression(qualified(
            r.library_name.as_deref(),
            &r.name,
        )))
    }

    pub(crate) fn eval_parameter_ref(
        &self,
        r: &ParameterRef,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        if let Some(value) = ctx.get_parameter_qualified(r.library_name.as_deref(), &r.name) {
            return Ok(value.clone());
        }
        let library = self.referenced_library(r.library_name.as_deref(), ctx)?;
        self.parameter_value(library, &r.name, ctx)
    }

    /// Supplied value, else declared default, else `null`.
    fn parameter_value(
        &self,
        library: Arc<Library>,
        name: &str,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        if let Some(value) = ctx.get_parameter(name) {
            return Ok(value.clone());
        }
        let def = library
            .parameters
            .as_ref()
            .and_then(|p| p.defs.iter().find(|d| d.name == name))
            .ok_or_else(|| EvalError::undefined_parameter(name))?;
        let Some(default) = def.default_expr.as_deref() else {
            return Ok(CqlValue::Null);
        };

        let cache_key = format!("param|{}|{}", library.identifier.id, name);
        if let Some(cached) = ctx.get_cached(&cache_key) {
            return Ok(cached);
        }
        ctx.push_library(library.clone());
        let result = ctx.with_isolated_scopes(|ctx| self.evaluate(default, ctx));
        ctx.pop_library();
        let value = result?;
        ctx.cache_result(cache_key, value.clone());
        Ok(value)
    }

    pub(crate) fn eval_function_ref(
        &self,
        r: &FunctionRef,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        let args: Vec<CqlValue> = match &r.operand {
            Some(operands) => operands
                .iter()
                .map(|op| self.evaluate(op, ctx))
                .collect::<EvalResult<Vec<_>>>()?,
            None => vec![],
        };

        if let Some(alias) = r.library_name.as_deref() {
            let path = ctx.include_path(alias).unwrap_or_else(|| alias.to_string());
            let library = ctx.resolve_included(alias);
            if path == FHIR_HELPERS
                && let Some(value) = fhir_helpers_call(&r.name, &args, library.is_some())
            {
                return Ok(value);
            }
            let library = library.ok_or_else(|| EvalError::UndefinedLibrary {
                name: alias.to_string(),
            })?;
            return match select_function(&library, &r.name, &args) {
                Some(def) => self.invoke_function(library.clone(), def, args, ctx),
                None => Err(EvalError::undefined_function(qualified(
                    Some(alias),
                    &r.name,
                ))),
            };
        }

        // Functions of the current library first (they may shadow system functions).
        if let Some(library) = ctx.current_library()
            && let Some(def) = select_function(&library, &r.name, &args)
        {
            return self.invoke_function(library.clone(), def, args, ctx);
        }

        // System functions.
        let arg_types: Vec<_> = args.iter().map(CqlValue::get_type).collect();
        if let Some(def) = self.registry.functions.get(&r.name, &arg_types)
            && let Some(implementation) = &def.implementation
        {
            return implementation(&args, ctx);
        }

        // Fluent (or unqualified) calls into included libraries.
        if let Some(library) = ctx.current_library()
            && let Some(includes) = &library.includes
        {
            for include in &includes.defs {
                if let Some(included) = ctx.resolve_included(&include.local_identifier)
                    && let Some(def) = select_function(&included, &r.name, &args)
                {
                    return self.invoke_function(included.clone(), def, args, ctx);
                }
            }
        }

        Err(EvalError::undefined_function(&r.name))
    }

    /// Invoke a user-defined function with already evaluated arguments.
    fn invoke_function(
        &self,
        library: Arc<Library>,
        def: &ExpressionDef,
        args: Vec<CqlValue>,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        if def.external == Some(true) {
            return Err(EvalError::unsupported_expression(format!(
                "external function {}",
                def.name
            )));
        }
        let body = def
            .expression
            .as_deref()
            .ok_or_else(|| EvalError::undefined_function(&def.name))?;
        let operands = def.operand.as_deref().unwrap_or_default();

        ctx.push_library(library.clone());
        let result = ctx.with_isolated_scopes(|ctx| {
            ctx.push_scope();
            for (operand, value) in operands.iter().zip(args) {
                ctx.set_alias(&operand.name, value);
            }
            let result = self.evaluate(body, ctx);
            ctx.pop_scope();
            result
        });
        ctx.pop_library();
        result
    }

    pub(crate) fn eval_valueset_ref(
        &self,
        r: &ValueSetRef,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        Ok(self.resolve_value_set(r, ctx)?.to_value())
    }

    pub(crate) fn eval_codesystem_ref(
        &self,
        r: &CodeSystemRef,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        Ok(self.resolve_code_system(r, ctx)?.to_value())
    }

    pub(crate) fn eval_code_ref(
        &self,
        r: &CodeRef,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        let library = self.referenced_library(r.library_name.as_deref(), ctx)?;
        self.code_value(library, &r.name, ctx).map(CqlValue::Code)
    }

    pub(crate) fn eval_concept_ref(
        &self,
        r: &ConceptRef,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        let library = self.referenced_library(r.library_name.as_deref(), ctx)?;
        self.concept_value(library, &r.name, ctx)
    }

    /// Resolve a value set reference to its declared id and version.
    pub fn resolve_value_set(
        &self,
        r: &ValueSetRef,
        ctx: &EvaluationContext,
    ) -> EvalResult<TerminologyId> {
        let library = self.referenced_library(r.library_name.as_deref(), ctx)?;
        find_value_set(&library, &r.name).ok_or_else(|| EvalError::ValueSetNotFound {
            name: qualified(r.library_name.as_deref(), &r.name),
        })
    }

    /// Resolve a code system reference to its declared id and version.
    pub fn resolve_code_system(
        &self,
        r: &CodeSystemRef,
        ctx: &EvaluationContext,
    ) -> EvalResult<TerminologyId> {
        let library = self.referenced_library(r.library_name.as_deref(), ctx)?;
        find_code_system(&library, &r.name).ok_or_else(|| EvalError::CodeSystemNotFound {
            name: qualified(r.library_name.as_deref(), &r.name),
        })
    }

    /// The `Code` declared as `name` in `library`.
    fn code_value(
        &self,
        library: Arc<Library>,
        name: &str,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlCode> {
        let def = library
            .codes
            .as_ref()
            .and_then(|c| c.defs.iter().find(|d| d.name == name))
            .ok_or_else(|| EvalError::undefined_expression(name))?;
        // The code system reference is relative to the declaring library.
        ctx.push_library(library.clone());
        let system = self.resolve_code_system(&def.code_system, ctx);
        ctx.pop_library();
        let system = system?;
        Ok(CqlCode {
            code: def.id.clone(),
            system: system.id,
            version: system.version,
            display: def.display.clone(),
        })
    }

    /// The `Concept` declared as `name` in `library`.
    fn concept_value(
        &self,
        library: Arc<Library>,
        name: &str,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        let def = library
            .concepts
            .as_ref()
            .and_then(|c| c.defs.iter().find(|d| d.name == name))
            .ok_or_else(|| EvalError::undefined_expression(name))?;
        ctx.push_library(library.clone());
        let codes: EvalResult<Vec<CqlCode>> = def
            .code
            .iter()
            .map(|code_ref| {
                let owner = self.referenced_library(code_ref.library_name.as_deref(), ctx)?;
                self.code_value(owner, &code_ref.name, ctx)
            })
            .collect();
        ctx.pop_library();
        Ok(CqlValue::Concept(CqlConcept {
            codes: codes?.into(),
            display: def.display.clone(),
        }))
    }
}

/// `library.name` for messages.
fn qualified(library: Option<&str>, name: &str) -> String {
    match library {
        Some(lib) => format!("{lib}.{name}"),
        None => name.to_string(),
    }
}

fn find_value_set(library: &Library, name: &str) -> Option<TerminologyId> {
    library
        .value_sets
        .as_ref()?
        .defs
        .iter()
        .find(|d| d.name == name)
        .map(|d| TerminologyId {
            name: d.name.clone(),
            id: d.id.clone(),
            version: d.version.clone(),
        })
}

fn find_code_system(library: &Library, name: &str) -> Option<TerminologyId> {
    library
        .code_systems
        .as_ref()?
        .defs
        .iter()
        .find(|d| d.name == name)
        .map(|d| TerminologyId {
            name: d.name.clone(),
            id: d.id.clone(),
            version: d.version.clone(),
        })
}

/// Pick the overload of `name` that accepts `args`: same arity, and when
/// several overloads share the arity, the first whose declared operand types
/// match the argument values.
fn select_function<'l>(
    library: &'l Library,
    name: &str,
    args: &[CqlValue],
) -> Option<&'l ExpressionDef> {
    let candidates: Vec<&ExpressionDef> = library
        .statements
        .as_ref()?
        .defs
        .iter()
        .filter(|d| d.is_function() && d.name == name && d.arity() == args.len())
        .collect();
    match candidates.as_slice() {
        [] => None,
        [only] => Some(only),
        _ => candidates
            .iter()
            .copied()
            .find(|d| {
                d.operand.iter().flatten().zip(args).all(|(operand, arg)| {
                    arg.is_null()
                        || operand
                            .operand_type_specifier
                            .as_ref()
                            .is_none_or(|ts| operand_accepts(ts, arg))
                })
            })
            .or(candidates.first().copied()),
    }
}

/// Whether a value can be passed for an operand declared with `ts`.
fn operand_accepts(ts: &TypeSpecifier, value: &CqlValue) -> bool {
    match ts {
        TypeSpecifier::Named(named) if matches!(named.namespace.as_deref(), Some("FHIR")) => {
            // FHIR-typed operands accept the raw structure or its system form.
            true
        }
        TypeSpecifier::Choice(choice) => choice.choice.iter().any(|t| operand_accepts(t, value)),
        _ => {
            value_is_type(value, ts)
                || matches!((ts, value), (TypeSpecifier::Named(n), CqlValue::Integer(_)) if n.name == "Decimal")
        }
    }
}

/// System-value implementation of the FHIRHelpers conversion functions.
///
/// Returns `None` when the call should go to a loaded FHIRHelpers library
/// instead (the argument is still a raw FHIR structure and the library is
/// available) or when the function is not a known conversion.
fn fhir_helpers_call(name: &str, args: &[CqlValue], library_loaded: bool) -> Option<CqlValue> {
    let [arg] = args else {
        return None;
    };
    let raw_fhir = matches!(arg, CqlValue::Tuple(_));
    if raw_fhir && library_loaded {
        return None;
    }
    let value = match (name, arg) {
        (_, CqlValue::Null) => CqlValue::Null,
        // FHIR primitive wrapper `{ value: ... }`
        (_, CqlValue::Tuple(t)) if t.get("value").is_some() && name != "ToQuantity" => {
            t.get("value").cloned().unwrap_or(CqlValue::Null)
        }
        ("ToConcept", CqlValue::Code(code)) => CqlValue::Concept(CqlConcept {
            codes: vec![code.clone()].into(),
            display: code.display.clone(),
        }),
        ("ToConcept", CqlValue::List(list)) => {
            let codes: Vec<CqlCode> = list
                .iter()
                .filter_map(|v| match v {
                    CqlValue::Code(c) => Some(c.clone()),
                    _ => None,
                })
                .collect();
            CqlValue::Concept(CqlConcept {
                codes: codes.into(),
                display: None,
            })
        }
        ("ToInterval", CqlValue::Interval(_)) => arg.clone(),
        ("ToInterval", point @ (CqlValue::DateTime(_) | CqlValue::Date(_))) => CqlValue::Interval(
            CqlInterval::closed(point.get_type(), point.clone(), point.clone()),
        ),
        (
            "ToCode" | "ToConcept" | "ToQuantity" | "ToRatio" | "ToDate" | "ToDateTime" | "ToTime"
            | "ToString" | "ToBoolean" | "ToInteger" | "ToDecimal" | "ToValue" | "ToInterval"
            | "ToCalendarUnit" | "ToReference",
            _,
        ) => arg.clone(),
        ("ToList", CqlValue::List(_)) => arg.clone(),
        ("ToList", other) => CqlValue::List(CqlList::from_elements(vec![other.clone()])),
        _ => return None,
    };
    Some(value)
}
