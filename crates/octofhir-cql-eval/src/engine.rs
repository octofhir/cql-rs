//! CQL Evaluation Engine
//!
//! This module provides the main CqlEngine struct which evaluates ELM expressions
//! against an evaluation context.

use crate::context::EvaluationContext;
use crate::error::{EvalError, EvalResult};
use crate::registry::OperatorRegistry;
use bigdecimal::BigDecimal;
use octofhir_cql_elm::{Expression, Library, Literal};
use octofhir_cql_types::{CqlDate, CqlDateTime, CqlList, CqlTime, CqlValue};
use std::str::FromStr;
use std::sync::Arc;

/// The main CQL evaluation engine
///
/// The engine evaluates ELM expressions against a context, using registered
/// operator implementations for each expression type.
pub struct CqlEngine {
    /// Operator and function registry
    pub(crate) registry: OperatorRegistry,
}

impl Default for CqlEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl CqlEngine {
    /// Create a new engine with standard operators
    pub fn new() -> Self {
        Self {
            registry: OperatorRegistry::with_standard_operators(),
        }
    }

    /// Create an engine with a custom registry
    pub fn with_registry(registry: OperatorRegistry) -> Self {
        Self { registry }
    }

    /// Get a mutable reference to the registry
    pub fn registry_mut(&mut self) -> &mut OperatorRegistry {
        &mut self.registry
    }

    /// Evaluate a library and return the value of every public expression
    /// definition, in declaration order (function definitions are skipped).
    ///
    /// Libraries the main library includes must be registered on the context
    /// with [`EvaluationContext::add_library`].
    pub fn evaluate_library(
        &self,
        library: &Library,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<indexmap::IndexMap<String, CqlValue>> {
        let library = Self::shared_library(library, ctx);
        let mut results = indexmap::IndexMap::new();

        if let Some(statements) = &library.statements {
            for def in &statements.defs {
                if def.is_function()
                    || def.expression.is_none()
                    || def.access_level == Some(octofhir_cql_elm::AccessModifier::Private)
                {
                    continue;
                }
                let value = self.evaluate_definition(library.clone(), &def.name, ctx)?;
                results.insert(def.name.clone(), value);
            }
        }

        Ok(results)
    }

    /// Evaluate a single expression definition by name
    pub fn evaluate_expression(
        &self,
        library: &Library,
        name: &str,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        let library = Self::shared_library(library, ctx);
        self.evaluate_definition(library, name, ctx)
    }

    /// The shared handle for `library`: the context's main library when it has
    /// the same identifier (id and version), otherwise a new handle, which
    /// also becomes the main library if the context has none.
    fn shared_library(library: &Library, ctx: &mut EvaluationContext) -> Arc<Library> {
        if let Some(main) = ctx.main_library_arc()
            && main.identifier.id == library.identifier.id
            && main.identifier.version == library.identifier.version
        {
            return main;
        }
        let shared = Arc::new(library.clone());
        if ctx.main_library().is_none() {
            ctx.set_main_library(shared.clone());
        }
        shared
    }

    /// Main expression evaluation dispatcher
    ///
    /// This dispatches to the appropriate evaluation method based on expression type.
    pub fn evaluate(&self, expr: &Expression, ctx: &mut EvaluationContext) -> EvalResult<CqlValue> {
        // Check recursion limit
        if !ctx.enter_recursion() {
            return Err(EvalError::RecursionLimit);
        }

        let result = match expr {
            // === Literals ===
            Expression::Null(_) => Ok(CqlValue::Null),
            Expression::Literal(lit) => self.eval_literal(lit),

            // === References ===
            Expression::ExpressionRef(r) => self.eval_expression_ref(r, ctx),
            Expression::FunctionRef(r) => self.eval_function_ref(r, ctx),
            Expression::ParameterRef(r) => self.eval_parameter_ref(r, ctx),
            Expression::OperandRef(r) => self.eval_operand_ref(r, ctx),
            Expression::AliasRef(r) => self.eval_alias_ref(r, ctx),
            Expression::QueryLetRef(r) => self.eval_query_let_ref(r, ctx),
            Expression::IdentifierRef(r) => self.eval_identifier_ref(r, ctx),
            Expression::Property(p) => self.eval_property(p, ctx),
            Expression::ValueSetRef(r) => self.eval_valueset_ref(r, ctx),
            Expression::CodeSystemRef(r) => self.eval_codesystem_ref(r, ctx),
            Expression::CodeRef(r) => self.eval_code_ref(r, ctx),
            Expression::ConceptRef(r) => self.eval_concept_ref(r, ctx),

            // === Arithmetic ===
            Expression::Add(e) => self.eval_add(e, ctx),
            Expression::Subtract(e) => self.eval_subtract(e, ctx),
            Expression::Multiply(e) => self.eval_multiply(e, ctx),
            Expression::Divide(e) => self.eval_divide(e, ctx),
            Expression::TruncatedDivide(e) => self.eval_truncated_divide(e, ctx),
            Expression::Modulo(e) => self.eval_modulo(e, ctx),
            Expression::Power(e) => self.eval_power(e, ctx),
            Expression::Negate(e) => self.eval_negate(e, ctx),
            Expression::Abs(e) => self.eval_abs(e, ctx),
            Expression::Ceiling(e) => self.eval_ceiling(e, ctx),
            Expression::Floor(e) => self.eval_floor(e, ctx),
            Expression::Truncate(e) => self.eval_truncate(e, ctx),
            Expression::Round(e) => self.eval_round(e, ctx),
            Expression::Ln(e) => self.eval_ln(e, ctx),
            Expression::Exp(e) => self.eval_exp(e, ctx),
            Expression::Log(e) => self.eval_log(e, ctx),
            Expression::Successor(e) => self.eval_successor(e, ctx),
            Expression::Predecessor(e) => self.eval_predecessor(e, ctx),
            Expression::MinValue(e) => self.eval_min_value(e),
            Expression::MaxValue(e) => self.eval_max_value(e),
            Expression::Precision(e) => self.eval_precision(e, ctx),
            Expression::LowBoundary(e) => self.eval_low_boundary(e, ctx),
            Expression::HighBoundary(e) => self.eval_high_boundary(e, ctx),

            // === Comparison ===
            Expression::Equal(e) => self.eval_equal(e, ctx),
            Expression::Equivalent(e) => self.eval_equivalent(e, ctx),
            Expression::NotEqual(e) => self.eval_not_equal(e, ctx),
            Expression::Less(e) => self.eval_less(e, ctx),
            Expression::Greater(e) => self.eval_greater(e, ctx),
            Expression::LessOrEqual(e) => self.eval_less_or_equal(e, ctx),
            Expression::GreaterOrEqual(e) => self.eval_greater_or_equal(e, ctx),

            // === Logical ===
            Expression::And(e) => self.eval_and(e, ctx),
            Expression::Or(e) => self.eval_or(e, ctx),
            Expression::Xor(e) => self.eval_xor(e, ctx),
            Expression::Implies(e) => self.eval_implies(e, ctx),
            Expression::Not(e) => self.eval_not(e, ctx),

            // === Nullological ===
            Expression::IsNull(e) => self.eval_is_null(e, ctx),
            Expression::IsTrue(e) => self.eval_is_true(e, ctx),
            Expression::IsFalse(e) => self.eval_is_false(e, ctx),
            Expression::Coalesce(e) => self.eval_coalesce(e, ctx),
            Expression::If(e) => self.eval_if(e, ctx),
            Expression::Case(e) => self.eval_case(e, ctx),

            // === String ===
            Expression::Concatenate(e) => self.eval_concatenate(e, ctx),
            Expression::Combine(e) => self.eval_combine(e, ctx),
            Expression::Split(e) => self.eval_split(e, ctx),
            Expression::SplitOnMatches(e) => self.eval_split_on_matches(e, ctx),
            Expression::Length(e) => self.eval_string_length(e, ctx),
            Expression::Upper(e) => self.eval_upper(e, ctx),
            Expression::Lower(e) => self.eval_lower(e, ctx),
            Expression::Indexer(e) => self.eval_indexer(e, ctx),
            Expression::PositionOf(e) => self.eval_position_of(e, ctx),
            Expression::LastPositionOf(e) => self.eval_last_position_of(e, ctx),
            Expression::Substring(e) => self.eval_substring(e, ctx),
            Expression::StartsWith(e) => self.eval_starts_with(e, ctx),
            Expression::EndsWith(e) => self.eval_ends_with(e, ctx),
            Expression::Matches(e) => self.eval_matches(e, ctx),
            Expression::ReplaceMatches(e) => self.eval_replace_matches(e, ctx),

            // === DateTime ===
            Expression::Now(_) => Ok(CqlValue::DateTime(ctx.now())),
            Expression::Today(_) => Ok(CqlValue::Date(ctx.today())),
            Expression::TimeOfDay(_) => Ok(CqlValue::Time(ctx.time_of_day())),
            Expression::Date(e) => self.eval_date_constructor(e, ctx),
            Expression::DateTime(e) => self.eval_datetime_constructor(e, ctx),
            Expression::Time(e) => self.eval_time_constructor(e, ctx),
            Expression::DateFrom(e) => self.eval_date_from(e, ctx),
            Expression::TimeFrom(e) => self.eval_time_from(e, ctx),
            Expression::TimezoneFrom(e) => self.eval_timezone_from(e, ctx),
            Expression::TimezoneOffsetFrom(e) => self.eval_timezone_offset_from(e, ctx),
            Expression::DateTimeComponentFrom(e) => self.eval_datetime_component_from(e, ctx),
            Expression::DurationBetween(e) => self.eval_duration_between(e, ctx),
            Expression::DifferenceBetween(e) => self.eval_difference_between(e, ctx),
            Expression::SameAs(e) => self.eval_same_as(e, ctx),
            Expression::SameOrBefore(e) => self.eval_same_or_before(e, ctx),
            Expression::SameOrAfter(e) => self.eval_same_or_after(e, ctx),

            // === Interval ===
            Expression::Interval(e) => self.eval_interval_constructor(e, ctx),
            Expression::Start(e) => self.eval_start(e, ctx),
            Expression::End(e) => self.eval_end(e, ctx),
            Expression::PointFrom(e) => self.eval_point_from(e, ctx),
            Expression::Width(e) => self.eval_width(e, ctx),
            Expression::Size(e) => self.eval_size(e, ctx),
            Expression::Contains(e) => self.eval_contains(e, ctx),
            Expression::In(e) => self.eval_in(e, ctx),
            Expression::Includes(e) => self.eval_includes(e, ctx),
            Expression::IncludedIn(e) => self.eval_included_in(e, ctx),
            Expression::ProperContains(e) => self.eval_proper_contains(e, ctx),
            Expression::ProperIn(e) => self.eval_proper_in(e, ctx),
            Expression::ProperIncludes(e) => self.eval_proper_includes(e, ctx),
            Expression::ProperIncludedIn(e) => self.eval_proper_included_in(e, ctx),
            Expression::Before(e) => self.eval_before(e, ctx),
            Expression::After(e) => self.eval_after(e, ctx),
            Expression::Meets(e) => self.eval_meets(e, ctx),
            Expression::MeetsBefore(e) => self.eval_meets_before(e, ctx),
            Expression::MeetsAfter(e) => self.eval_meets_after(e, ctx),
            Expression::Overlaps(e) => self.eval_overlaps(e, ctx),
            Expression::OverlapsBefore(e) => self.eval_overlaps_before(e, ctx),
            Expression::OverlapsAfter(e) => self.eval_overlaps_after(e, ctx),
            Expression::Starts(e) => self.eval_starts(e, ctx),
            Expression::Ends(e) => self.eval_ends(e, ctx),
            Expression::Collapse(e) => self.eval_collapse(e, ctx),
            Expression::Expand(e) => self.eval_expand(e, ctx),
            Expression::Union(e) => self.eval_union(e, ctx),
            Expression::Intersect(e) => self.eval_intersect(e, ctx),
            Expression::Except(e) => self.eval_except(e, ctx),

            // === List ===
            Expression::List(e) => self.eval_list_constructor(e, ctx),
            Expression::Exists(e) => self.eval_exists(e, ctx),
            Expression::Times(e) => self.eval_times(e, ctx),
            Expression::Filter(e) => self.eval_filter(e, ctx),
            Expression::First(e) => self.eval_first(e, ctx),
            Expression::Last(e) => self.eval_last(e, ctx),
            Expression::Slice(e) => self.eval_slice(e, ctx),
            Expression::IndexOf(e) => self.eval_index_of(e, ctx),
            Expression::Flatten(e) => self.eval_flatten(e, ctx),
            Expression::Sort(e) => self.eval_sort(e, ctx),
            Expression::ForEach(e) => self.eval_for_each(e, ctx),
            Expression::Repeat(e) => self.eval_repeat(e, ctx),
            Expression::Distinct(e) => self.eval_distinct(e, ctx),
            Expression::Current(e) => self.eval_current(e, ctx),
            Expression::Iteration(e) => self.eval_iteration(e, ctx),
            Expression::Total(e) => self.eval_total(e, ctx),
            Expression::SingletonFrom(e) => self.eval_singleton_from(e, ctx),

            // === Aggregate ===
            Expression::Aggregate(e) => self.eval_aggregate(e, ctx),
            Expression::Count(e) => self.eval_count(e, ctx),
            Expression::Sum(e) => self.eval_sum(e, ctx),
            Expression::Product(e) => self.eval_product(e, ctx),
            Expression::Min(e) => self.eval_min(e, ctx),
            Expression::Max(e) => self.eval_max(e, ctx),
            Expression::Avg(e) => self.eval_avg(e, ctx),
            Expression::GeometricMean(e) => self.eval_geometric_mean(e, ctx),
            Expression::Median(e) => self.eval_median(e, ctx),
            Expression::Mode(e) => self.eval_mode(e, ctx),
            Expression::Variance(e) => self.eval_variance(e, ctx),
            Expression::StdDev(e) => self.eval_stddev(e, ctx),
            Expression::PopulationVariance(e) => self.eval_population_variance(e, ctx),
            Expression::PopulationStdDev(e) => self.eval_population_stddev(e, ctx),
            Expression::AllTrue(e) => self.eval_all_true(e, ctx),
            Expression::AnyTrue(e) => self.eval_any_true(e, ctx),

            // === Type Operations ===
            Expression::As(e) => self.eval_as(e, ctx),
            Expression::Convert(e) => self.eval_convert(e, ctx),
            Expression::Is(e) => self.eval_is(e, ctx),
            Expression::CanConvert(e) => self.eval_can_convert(e, ctx),
            Expression::ToBoolean(e) => self.eval_to_boolean(e, ctx),
            Expression::ToChars(e) => self.eval_to_chars(e, ctx),
            Expression::ToConcept(e) => self.eval_to_concept(e, ctx),
            Expression::ToDate(e) => self.eval_to_date(e, ctx),
            Expression::ToDateTime(e) => self.eval_to_datetime(e, ctx),
            Expression::ToDecimal(e) => self.eval_to_decimal(e, ctx),
            Expression::ToInteger(e) => self.eval_to_integer(e, ctx),
            Expression::ToLong(e) => self.eval_to_long(e, ctx),
            Expression::ToList(e) => self.eval_to_list(e, ctx),
            Expression::ToQuantity(e) => self.eval_to_quantity(e, ctx),
            Expression::ToRatio(e) => self.eval_to_ratio(e, ctx),
            Expression::ToString(e) => self.eval_to_string(e, ctx),
            Expression::ToTime(e) => self.eval_to_time(e, ctx),
            Expression::ConvertsToBoolean(e) => self.eval_converts_to_boolean(e, ctx),
            Expression::ConvertsToDate(e) => self.eval_converts_to_date(e, ctx),
            Expression::ConvertsToDateTime(e) => self.eval_converts_to_datetime(e, ctx),
            Expression::ConvertsToDecimal(e) => self.eval_converts_to_decimal(e, ctx),
            Expression::ConvertsToInteger(e) => self.eval_converts_to_integer(e, ctx),
            Expression::ConvertsToLong(e) => self.eval_converts_to_long(e, ctx),
            Expression::ConvertsToQuantity(e) => self.eval_converts_to_quantity(e, ctx),
            Expression::ConvertsToRatio(e) => self.eval_converts_to_ratio(e, ctx),
            Expression::ConvertsToString(e) => self.eval_converts_to_string(e, ctx),
            Expression::ConvertsToTime(e) => self.eval_converts_to_time(e, ctx),

            // === Clinical ===
            Expression::Code(e) => self.eval_code_literal(e, ctx),
            Expression::Concept(e) => self.eval_concept_literal(e, ctx),
            Expression::Quantity(e) => self.eval_quantity(e),
            Expression::Ratio(e) => self.eval_ratio(e, ctx),
            Expression::InCodeSystem(e) => self.eval_in_code_system(e, ctx),
            Expression::InValueSet(e) => self.eval_in_value_set(e, ctx),
            Expression::CalculateAge(e) => self.eval_calculate_age(e, ctx),
            Expression::CalculateAgeAt(e) => self.eval_calculate_age_at(e, ctx),

            // === Query ===
            Expression::Query(q) => self.eval_query(q, ctx),
            Expression::Retrieve(r) => self.eval_retrieve(r, ctx),

            // === Tuple/Instance ===
            Expression::Tuple(e) => self.eval_tuple(e, ctx),
            Expression::Instance(e) => self.eval_instance(e, ctx),

            // === Message ===
            Expression::Message(e) => self.eval_message(e, ctx),
        };

        ctx.exit_recursion();
        result
    }

    // =========================================================================
    // Literal evaluation
    // =========================================================================

    fn eval_literal(&self, lit: &Literal) -> EvalResult<CqlValue> {
        let value_str = match &lit.value {
            Some(v) => v.as_str(),
            None => return Ok(CqlValue::Null),
        };

        // Parse based on value type
        let type_name = &lit.value_type;

        // Handle qualified type names like "{urn:hl7-org:elm-types:r1}Boolean"
        let simple_type = type_name.rsplit('}').next().unwrap_or(type_name);

        match simple_type {
            "Boolean" => {
                let b = value_str
                    .parse::<bool>()
                    .map_err(|_| EvalError::conversion_error(value_str, "Boolean"))?;
                Ok(CqlValue::Boolean(b))
            }
            "Integer" => {
                let i = value_str
                    .parse::<i32>()
                    .map_err(|_| EvalError::conversion_error(value_str, "Integer"))?;
                Ok(CqlValue::Integer(i))
            }
            "Long" => {
                let l = value_str
                    .parse::<i64>()
                    .map_err(|_| EvalError::conversion_error(value_str, "Long"))?;
                Ok(CqlValue::Long(l))
            }
            "Decimal" => {
                let d = BigDecimal::from_str(value_str)
                    .map_err(|_| EvalError::conversion_error(value_str, "Decimal"))?;
                Ok(CqlValue::Decimal(d))
            }
            "String" => Ok(CqlValue::String(value_str.to_string())),
            "Date" => {
                let date = CqlDate::parse(value_str)
                    .ok_or_else(|| EvalError::conversion_error(value_str, "Date"))?;
                Ok(CqlValue::Date(date))
            }
            "DateTime" => {
                let datetime = CqlDateTime::parse(value_str)
                    .ok_or_else(|| EvalError::conversion_error(value_str, "DateTime"))?;
                Ok(CqlValue::DateTime(datetime))
            }
            "Time" => {
                let time = CqlTime::parse(value_str)
                    .ok_or_else(|| EvalError::conversion_error(value_str, "Time"))?;
                Ok(CqlValue::Time(time))
            }
            _ => Err(EvalError::unsupported_expression(format!(
                "Literal type: {}",
                type_name
            ))),
        }
    }

    // =========================================================================
    // Local references (operands, aliases, lets); library-level references are
    // evaluated in `references.rs`
    // =========================================================================

    fn eval_operand_ref(
        &self,
        r: &octofhir_cql_elm::OperandRef,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        ctx.get_alias(&r.name)
            .or_else(|| ctx.get_let(&r.name))
            .or_else(|| ctx.get_special(&r.name))
            .cloned()
            .ok_or_else(|| EvalError::undefined_alias(&r.name))
    }

    fn eval_alias_ref(
        &self,
        r: &octofhir_cql_elm::AliasRef,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        ctx.get_alias(&r.name)
            .cloned()
            .ok_or_else(|| EvalError::undefined_alias(&r.name))
    }

    fn eval_query_let_ref(
        &self,
        r: &octofhir_cql_elm::QueryLetRef,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        ctx.get_let(&r.name)
            .cloned()
            .ok_or_else(|| EvalError::UndefinedLetVariable {
                name: r.name.clone(),
            })
    }

    fn eval_identifier_ref(
        &self,
        r: &octofhir_cql_elm::IdentifierRef,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        // Try alias first, then let, then parameter
        if let Some(v) = ctx.get_alias(&r.name) {
            return Ok(v.clone());
        }
        if let Some(v) = ctx.get_let(&r.name) {
            return Ok(v.clone());
        }
        if let Some(v) = ctx.get_parameter(&r.name) {
            return Ok(v.clone());
        }
        Err(EvalError::undefined_alias(&r.name))
    }

    fn eval_property(
        &self,
        p: &octofhir_cql_elm::Property,
        ctx: &mut EvaluationContext,
    ) -> EvalResult<CqlValue> {
        // Get the source value
        let source = if let Some(src) = &p.source {
            self.evaluate(src, ctx)?
        } else if let Some(scope) = &p.scope {
            ctx.get_alias(scope)
                .cloned()
                .ok_or_else(|| EvalError::undefined_alias(scope))?
        } else {
            return Err(EvalError::internal("Property without source or scope"));
        };

        // Navigate the path
        self.get_property_value(&source, &p.path, ctx)
    }

    pub(crate) fn get_property_value(
        &self,
        value: &CqlValue,
        path: &str,
        ctx: &EvaluationContext,
    ) -> EvalResult<CqlValue> {
        if let Some(property) = structured_property(value, path) {
            return Ok(property);
        }
        match value {
            CqlValue::Null => Ok(CqlValue::Null),
            // Absent elements (e.g. FHIR elements omitted from the JSON) are null.
            CqlValue::Tuple(t) => Ok(t.get(path).cloned().unwrap_or(CqlValue::Null)),
            CqlValue::List(l) => {
                // For lists, project the property across all elements
                let projected: Vec<CqlValue> = l
                    .iter()
                    .map(|elem| self.get_property_value(elem, path, ctx))
                    .collect::<EvalResult<Vec<_>>>()?;
                Ok(CqlValue::List(CqlList::from_elements(projected)))
            }
            _ => {
                // Use data provider if available
                if let Some(provider) = ctx.data_provider() {
                    provider
                        .get_property(value, path)
                        .ok_or_else(|| EvalError::invalid_property(path, value.get_type().name()))
                } else {
                    Err(EvalError::invalid_property(path, value.get_type().name()))
                }
            }
        }
    }
}

/// Properties of the structured system types (CQL 1.5 Appendix B), plus the
/// FHIR element names of the FHIR types they represent (`Period.start/end`,
/// `CodeableConcept.coding/text`, `Coding.*`, `Quantity.code/system`).
fn structured_property(value: &CqlValue, path: &str) -> Option<CqlValue> {
    let boxed = |v: &Option<Box<CqlValue>>| v.as_deref().cloned().unwrap_or(CqlValue::Null);
    let text = |v: &Option<String>| v.as_ref().map_or(CqlValue::Null, CqlValue::string);
    Some(match (value, path) {
        (CqlValue::Interval(i), "low" | "start") => boxed(&i.low),
        (CqlValue::Interval(i), "high" | "end") => boxed(&i.high),
        (CqlValue::Interval(i), "lowClosed") => CqlValue::Boolean(i.low_closed),
        (CqlValue::Interval(i), "highClosed") => CqlValue::Boolean(i.high_closed),
        (CqlValue::Quantity(q), "value") => CqlValue::Decimal(q.value.clone()),
        (CqlValue::Quantity(q), "unit" | "code") => text(&q.unit),
        (CqlValue::Quantity(_), "system") => CqlValue::string("http://unitsofmeasure.org"),
        (CqlValue::Ratio(r), "numerator") => CqlValue::Quantity(r.numerator.clone()),
        (CqlValue::Ratio(r), "denominator") => CqlValue::Quantity(r.denominator.clone()),
        (CqlValue::Code(c), "code") => CqlValue::string(&c.code),
        (CqlValue::Code(c), "system") => CqlValue::string(&c.system),
        (CqlValue::Code(c), "version") => text(&c.version),
        (CqlValue::Code(c), "display") => text(&c.display),
        (CqlValue::Concept(c), "codes" | "coding") => CqlValue::List(CqlList::from_elements(
            c.codes
                .iter()
                .cloned()
                .map(CqlValue::Code)
                .collect::<Vec<_>>(),
        )),
        (CqlValue::Concept(c), "display" | "text") => text(&c.display),
        _ => return None,
    })
}
