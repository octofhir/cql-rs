//! AST to ELM Converter
//!
//! This module provides comprehensive conversion from CQL AST to ELM representation.
//! It handles all expression types, library structures, and preserves type information.

use octofhir_cql_ast::{
    self as ast, AccessModifier as AstAccessModifier, BinaryOp, DateTimeComponent, Expression as AstExpression,
    IntervalOp, Library as AstLibrary, Literal, Query as AstQuery, Retrieve as AstRetrieve,
    SortDirection as AstSortDirection, Statement, TemporalPrecision, UnaryOp,
};
// Note: Spanned has `.inner` field (not `.inner`)

use crate::model::{
    AccessModifier, AggregateClause, AggregateExpression, AliasRef, AliasedQuerySource,
    AsExpression, BinaryExpression, BoundaryExpression, CalculateAgeAtExpression,
    CalculateAgeExpression, CanConvertExpression, CaseExpression, CaseItem, CodeDef,
    CodeDefs, CodeLiteralExpression, CodeRef, CodeSystemDef, CodeSystemDefs, CodeSystemRef,
    CombineExpression, ConceptDef, ConceptDefs, ConceptRef, ContextDef, ContextDefs,
    ConvertExpression, CurrentExpression, DateExpression, DateTimeComponentFromExpression,
    DateTimeExpression, DateTimePrecision, DifferenceBetweenExpression, DurationBetweenExpression,
    Element, ExpandExpression, Expression, ExpressionDef, ExpressionRef, FilterExpression,
    FirstLastExpression, ForEachExpression, FunctionDef, FunctionRef, IdentifierRef, IfExpression,
    IncludeDef, IncludeDefs, IndexOfExpression, InstanceElementExpression, InstanceExpression,
    IntervalExpression, IsExpression, IterationExpression, LastPositionOfExpression, LetClause,
    Library, ListExpression, ListTypeSpecifier, Literal as ElmLiteral, MessageExpression,
    MinMaxValueExpression, NamedTypeSpecifier, NaryExpression, NowExpression, NullLiteral,
    OperandDef, OperandRef, ParameterDef, ParameterDefs, ParameterRef, PositionOfExpression,
    Property, QuantityExpression, Query, QueryLetRef, RatioExpression, RelationshipClause,
    RepeatExpression, Retrieve, ReturnClause, RoundExpression, SameAsExpression,
    SameOrAfterExpression, SameOrBeforeExpression, SliceExpression, SortByItem, SortClause,
    SortDirection, SortExpression, SplitExpression, SplitOnMatchesExpression, Statements,
    SubstringExpression, TernaryExpression, TimeExpression, TimeOfDayExpression, TodayExpression,
    TotalExpression, TupleElementDefinition, TupleElementExpression, TupleExpression,
    TupleTypeSpecifier, TypeSpecifier, UnaryExpression, UsingDef, UsingDefs, ValueSetDef,
    ValueSetDefs, ValueSetRef, VersionedIdentifier, WithClause, WithoutClause,
};

/// AST to ELM Converter
///
/// Converts a CQL AST (from parsing) to an ELM representation (for serialization/execution).
#[derive(Debug, Default)]
pub struct AstToElmConverter {
    /// Current context (e.g., "Patient")
    current_context: Option<String>,
    /// Library name for references
    library_name: Option<String>,
}

impl AstToElmConverter {
    /// Create a new converter
    pub fn new() -> Self {
        Self::default()
    }

    /// Convert a complete AST library to ELM
    pub fn convert_library(&mut self, ast_lib: &AstLibrary) -> Library {
        // Extract library identifier
        let identifier = if let Some(def) = &ast_lib.definition {
            self.library_name = Some(def.name.name.name.clone());
            VersionedIdentifier {
                id: def.name.name.name.clone(),
                system: def.name.qualifier.clone(),
                version: def.version.as_ref().map(|v| v.version.clone()),
            }
        } else {
            VersionedIdentifier {
                id: "Anonymous".to_string(),
                system: None,
                version: None,
            }
        };

        let mut library = Library::new(&identifier.id, identifier.version.as_deref());

        // Convert usings
        if !ast_lib.usings.is_empty() {
            library.usings = Some(UsingDefs {
                defs: ast_lib
                    .usings
                    .iter()
                    .map(|u| self.convert_using(&u.inner))
                    .collect(),
            });
        }

        // Convert includes
        if !ast_lib.includes.is_empty() {
            library.includes = Some(IncludeDefs {
                defs: ast_lib
                    .includes
                    .iter()
                    .map(|i| self.convert_include(&i.inner))
                    .collect(),
            });
        }

        // Convert parameters
        if !ast_lib.parameters.is_empty() {
            library.parameters = Some(ParameterDefs {
                defs: ast_lib
                    .parameters
                    .iter()
                    .map(|p| self.convert_parameter(&p.inner))
                    .collect(),
            });
        }

        // Convert codesystems
        if !ast_lib.codesystems.is_empty() {
            library.code_systems = Some(CodeSystemDefs {
                defs: ast_lib
                    .codesystems
                    .iter()
                    .map(|cs| self.convert_codesystem(&cs.inner))
                    .collect(),
            });
        }

        // Convert valuesets
        if !ast_lib.valuesets.is_empty() {
            library.value_sets = Some(ValueSetDefs {
                defs: ast_lib
                    .valuesets
                    .iter()
                    .map(|vs| self.convert_valueset(&vs.inner))
                    .collect(),
            });
        }

        // Convert codes
        if !ast_lib.codes.is_empty() {
            library.codes = Some(CodeDefs {
                defs: ast_lib
                    .codes
                    .iter()
                    .map(|c| self.convert_code(&c.inner))
                    .collect(),
            });
        }

        // Convert concepts
        if !ast_lib.concepts.is_empty() {
            library.concepts = Some(ConceptDefs {
                defs: ast_lib
                    .concepts
                    .iter()
                    .map(|c| self.convert_concept(&c.inner))
                    .collect(),
            });
        }

        // Convert contexts
        if !ast_lib.contexts.is_empty() {
            library.contexts = Some(ContextDefs {
                defs: ast_lib
                    .contexts
                    .iter()
                    .map(|c| {
                        self.current_context = Some(c.inner.context.name.clone());
                        ContextDef {
                            name: c.inner.context.name.clone(),
                        }
                    })
                    .collect(),
            });
        }

        // Convert statements (expression and function definitions)
        if !ast_lib.statements.is_empty() {
            library.statements = Some(Statements {
                defs: ast_lib
                    .statements
                    .iter()
                    .filter_map(|s| self.convert_statement(&s.inner))
                    .collect(),
            });
        }

        library
    }

    /// Convert using definition
    fn convert_using(&self, using: &ast::UsingDefinition) -> UsingDef {
        UsingDef {
            local_identifier: using.model.name.clone(),
            uri: format!("urn:hl7-org:elm-modelinfo:{}", using.model.name),
            version: using.version.as_ref().map(|v| v.version.clone()),
            annotation: None,
        }
    }

    /// Convert include definition
    fn convert_include(&self, include: &ast::IncludeDefinition) -> IncludeDef {
        IncludeDef {
            local_identifier: include
                .alias
                .as_ref()
                .map(|a| a.name.clone())
                .unwrap_or_else(|| include.library.name.name.clone()),
            path: include.library.name.name.clone(),
            version: include.version.as_ref().map(|v| v.version.clone()),
            annotation: None,
        }
    }

    /// Convert parameter definition
    fn convert_parameter(&self, param: &ast::ParameterDefinition) -> ParameterDef {
        ParameterDef {
            name: param.name.name.clone(),
            access_level: Some(self.convert_access_modifier(param.access)),
            parameter_type_specifier: param
                .type_specifier
                .as_ref()
                .map(|ts| self.convert_type_specifier(&ts.inner)),
            default_expr: param
                .default
                .as_ref()
                .map(|e| Box::new(self.convert_expression(&e.inner))),
            annotation: None,
        }
    }

    /// Convert codesystem definition
    fn convert_codesystem(&self, cs: &ast::CodesystemDefinition) -> CodeSystemDef {
        CodeSystemDef {
            name: cs.name.name.clone(),
            id: cs.uri.clone(),
            version: cs.version.as_ref().map(|v| v.version.clone()),
            access_level: Some(self.convert_access_modifier(cs.access)),
            annotation: None,
        }
    }

    /// Convert valueset definition
    fn convert_valueset(&self, vs: &ast::ValuesetDefinition) -> ValueSetDef {
        ValueSetDef {
            name: vs.name.name.clone(),
            id: vs.uri.clone(),
            version: vs.version.as_ref().map(|v| v.version.clone()),
            access_level: Some(self.convert_access_modifier(vs.access)),
            code_system: if vs.codesystems.is_empty() {
                None
            } else {
                Some(
                    vs.codesystems
                        .iter()
                        .map(|cs| CodeSystemRef {
                            element: Element::default(),
                            library_name: cs.name.qualifier.clone(),
                            name: cs.name.name.name.clone(),
                        })
                        .collect(),
                )
            },
            annotation: None,
        }
    }

    /// Convert code definition
    fn convert_code(&self, code: &ast::CodeDefinition) -> CodeDef {
        CodeDef {
            name: code.name.name.clone(),
            id: code.code.clone(),
            display: code.display.clone(),
            access_level: Some(self.convert_access_modifier(code.access)),
            code_system: CodeSystemRef {
                element: Element::default(),
                library_name: code.codesystem.qualifier.clone(),
                name: code.codesystem.name.name.clone(),
            },
            annotation: None,
        }
    }

    /// Convert concept definition
    fn convert_concept(&self, concept: &ast::ConceptDefinition) -> ConceptDef {
        ConceptDef {
            name: concept.name.name.clone(),
            display: concept.display.clone(),
            access_level: Some(self.convert_access_modifier(concept.access)),
            code: concept
                .codes
                .iter()
                .map(|c| CodeRef {
                    element: Element::default(),
                    library_name: c.qualifier.clone(),
                    name: c.name.name.clone(),
                })
                .collect(),
            annotation: None,
        }
    }

    /// Convert statement (expression or function definition)
    fn convert_statement(&self, stmt: &Statement) -> Option<ExpressionDef> {
        match stmt {
            Statement::ExpressionDef(def) => Some(ExpressionDef {
                name: def.name.name.clone(),
                context: self.current_context.clone(),
                access_level: Some(self.convert_access_modifier(def.access)),
                expression: Some(Box::new(self.convert_expression(&def.expression.inner))),
                result_type_specifier: None, // Type info added during type checking
                annotation: None,
            }),
            Statement::FunctionDef(def) => {
                // Function definitions are also stored as ExpressionDefs in ELM
                // with operands
                Some(ExpressionDef {
                    name: def.name.name.clone(),
                    context: self.current_context.clone(),
                    access_level: Some(self.convert_access_modifier(def.access)),
                    expression: def
                        .body
                        .as_ref()
                        .map(|e| Box::new(self.convert_expression(&e.inner))),
                    result_type_specifier: def
                        .return_type
                        .as_ref()
                        .map(|ts| self.convert_type_specifier(&ts.inner)),
                    annotation: None,
                })
            }
        }
    }

    /// Convert access modifier
    fn convert_access_modifier(&self, access: AstAccessModifier) -> AccessModifier {
        match access {
            AstAccessModifier::Public => AccessModifier::Public,
            AstAccessModifier::Private => AccessModifier::Private,
        }
    }

    /// Convert type specifier from AST to ELM
    pub fn convert_type_specifier(&self, ts: &ast::TypeSpecifier) -> TypeSpecifier {
        match ts {
            ast::TypeSpecifier::Named(named) => TypeSpecifier::Named(NamedTypeSpecifier {
                namespace: named.namespace.clone(),
                name: named.name.clone(),
            }),
            ast::TypeSpecifier::List(list) => TypeSpecifier::List(ListTypeSpecifier {
                element_type: Box::new(self.convert_type_specifier(&list.element_type)),
            }),
            ast::TypeSpecifier::Interval(interval) => {
                TypeSpecifier::Interval(crate::model::IntervalTypeSpecifier {
                    point_type: Box::new(self.convert_type_specifier(&interval.point_type)),
                })
            }
            ast::TypeSpecifier::Tuple(tuple) => TypeSpecifier::Tuple(TupleTypeSpecifier {
                element: tuple
                    .elements
                    .iter()
                    .map(|e| TupleElementDefinition {
                        name: e.name.name.clone(),
                        element_type: e
                            .element_type
                            .as_ref()
                            .map(|t| Box::new(self.convert_type_specifier(t))),
                    })
                    .collect(),
            }),
            ast::TypeSpecifier::Choice(choice) => {
                TypeSpecifier::Choice(crate::model::ChoiceTypeSpecifier {
                    choice: choice.types.iter().map(|t| self.convert_type_specifier(t)).collect(),
                })
            }
        }
    }

    /// Convert expression from AST to ELM
    pub fn convert_expression(&self, expr: &AstExpression) -> Expression {
        match expr {
            // === Literals ===
            AstExpression::Literal(lit) => self.convert_literal(lit),

            // === Identifiers and References ===
            AstExpression::IdentifierRef(id_ref) => Expression::IdentifierRef(IdentifierRef {
                element: Element::default(),
                library_name: None,
                name: id_ref.name.name.clone(),
            }),
            AstExpression::QualifiedIdentifierRef(qid_ref) => {
                Expression::ExpressionRef(ExpressionRef {
                    element: Element::default(),
                    library_name: qid_ref.name.qualifier.clone(),
                    name: qid_ref.name.name.name.clone(),
                })
            }
            AstExpression::Property(prop) => Expression::Property(Property {
                element: Element::default(),
                source: Some(Box::new(self.convert_expression(&prop.source.inner))),
                path: prop.property.name.clone(),
                scope: None,
            }),

            // === Operators ===
            AstExpression::BinaryOp(bin_op) => self.convert_binary_op(bin_op),
            AstExpression::UnaryOp(un_op) => self.convert_unary_op(un_op),
            AstExpression::IntervalOp(int_op) => self.convert_interval_op(int_op),

            // === Type Operations ===
            AstExpression::As(as_expr) => Expression::As(AsExpression {
                element: Element::default(),
                operand: Box::new(self.convert_expression(&as_expr.operand.inner)),
                as_type_specifier: Some(self.convert_type_specifier(&as_expr.as_type.inner)),
                as_type: None,
                strict: Some(as_expr.strict),
            }),
            AstExpression::Is(is_expr) => Expression::Is(IsExpression {
                element: Element::default(),
                operand: Box::new(self.convert_expression(&is_expr.operand.inner)),
                is_type_specifier: Some(self.convert_type_specifier(&is_expr.is_type.inner)),
                is_type: None,
            }),
            AstExpression::Convert(conv_expr) => Expression::Convert(ConvertExpression {
                element: Element::default(),
                operand: Box::new(self.convert_expression(&conv_expr.operand.inner)),
                to_type_specifier: Some(self.convert_type_specifier(&conv_expr.to_type.inner)),
                to_type: None,
            }),
            AstExpression::Cast(cast_expr) => Expression::As(AsExpression {
                element: Element::default(),
                operand: Box::new(self.convert_expression(&cast_expr.operand.inner)),
                as_type_specifier: Some(self.convert_type_specifier(&cast_expr.as_type.inner)),
                as_type: None,
                strict: Some(true),
            }),

            // === Conditionals ===
            AstExpression::If(if_expr) => Expression::If(IfExpression {
                element: Element::default(),
                condition: Box::new(self.convert_expression(&if_expr.condition.inner)),
                then: Box::new(self.convert_expression(&if_expr.then_expr.inner)),
                else_clause: Box::new(self.convert_expression(&if_expr.else_expr.inner)),
            }),
            AstExpression::Case(case_expr) => Expression::Case(CaseExpression {
                element: Element::default(),
                comparand: case_expr
                    .comparand
                    .as_ref()
                    .map(|c| Box::new(self.convert_expression(&c.inner))),
                case_item: case_expr
                    .items
                    .iter()
                    .map(|item| CaseItem {
                        when: Box::new(self.convert_expression(&item.when.inner)),
                        then: Box::new(self.convert_expression(&item.then.inner)),
                    })
                    .collect(),
                else_clause: case_expr
                    .else_expr
                    .as_ref()
                    .map(|e| Box::new(self.convert_expression(&e.inner))),
            }),
            AstExpression::Coalesce(coal_expr) => Expression::Coalesce(NaryExpression {
                element: Element::default(),
                operand: coal_expr
                    .operands
                    .iter()
                    .map(|e| Box::new(self.convert_expression(&e.inner)))
                    .collect(),
            }),

            // === Nulls ===
            AstExpression::IsNull(is_null) => Expression::IsNull(UnaryExpression {
                element: Element::default(),
                operand: Box::new(self.convert_expression(&is_null.operand.inner)),
            }),
            AstExpression::IsFalse(is_false) => Expression::IsFalse(UnaryExpression {
                element: Element::default(),
                operand: Box::new(self.convert_expression(&is_false.operand.inner)),
            }),
            AstExpression::IsTrue(is_true) => Expression::IsTrue(UnaryExpression {
                element: Element::default(),
                operand: Box::new(self.convert_expression(&is_true.operand.inner)),
            }),

            // === Collections ===
            AstExpression::List(list_expr) => Expression::List(ListExpression {
                element: Element::default(),
                type_specifier: list_expr
                    .element_type
                    .as_ref()
                    .map(|ts| self.convert_type_specifier(&ts.inner)),
                elements: if list_expr.elements.is_empty() {
                    None
                } else {
                    Some(
                        list_expr
                            .elements
                            .iter()
                            .map(|e| Box::new(self.convert_expression(&e.inner)))
                            .collect(),
                    )
                },
            }),
            AstExpression::Tuple(tuple_expr) => Expression::Tuple(TupleExpression {
                element: Element::default(),
                elements: if tuple_expr.elements.is_empty() {
                    None
                } else {
                    Some(
                        tuple_expr
                            .elements
                            .iter()
                            .map(|e| TupleElementExpression {
                                name: e.name.name.clone(),
                                value: Box::new(self.convert_expression(&e.value.inner)),
                            })
                            .collect(),
                    )
                },
            }),
            AstExpression::Instance(inst_expr) => Expression::Instance(InstanceExpression {
                element: Element::default(),
                class_type: self.type_specifier_to_string(&inst_expr.class_type.inner),
                elements: if inst_expr.elements.is_empty() {
                    None
                } else {
                    Some(
                        inst_expr
                            .elements
                            .iter()
                            .map(|e| InstanceElementExpression {
                                name: e.name.name.clone(),
                                value: Box::new(self.convert_expression(&e.value.inner)),
                            })
                            .collect(),
                    )
                },
            }),
            AstExpression::Indexer(indexer) => Expression::Indexer(BinaryExpression {
                element: Element::default(),
                operand: vec![
                    Box::new(self.convert_expression(&indexer.source.inner)),
                    Box::new(self.convert_expression(&indexer.index.inner)),
                ],
            }),

            // === Intervals ===
            AstExpression::Interval(int_expr) => Expression::Interval(IntervalExpression {
                element: Element::default(),
                low: int_expr
                    .low
                    .as_ref()
                    .map(|e| Box::new(self.convert_expression(&e.inner))),
                low_closed_expression: None,
                high: int_expr
                    .high
                    .as_ref()
                    .map(|e| Box::new(self.convert_expression(&e.inner))),
                high_closed_expression: None,
                low_closed: Some(int_expr.low_closed),
                high_closed: Some(int_expr.high_closed),
            }),
            AstExpression::Start(start) => Expression::Start(UnaryExpression {
                element: Element::default(),
                operand: Box::new(self.convert_expression(&start.operand.inner)),
            }),
            AstExpression::End(end) => Expression::End(UnaryExpression {
                element: Element::default(),
                operand: Box::new(self.convert_expression(&end.operand.inner)),
            }),
            AstExpression::PointFrom(pf) => Expression::PointFrom(UnaryExpression {
                element: Element::default(),
                operand: Box::new(self.convert_expression(&pf.operand.inner)),
            }),
            AstExpression::Width(width) => Expression::Width(UnaryExpression {
                element: Element::default(),
                operand: Box::new(self.convert_expression(&width.operand.inner)),
            }),
            AstExpression::Size(size) => Expression::Size(UnaryExpression {
                element: Element::default(),
                operand: Box::new(self.convert_expression(&size.operand.inner)),
            }),

            // === Queries ===
            AstExpression::Query(query) => self.convert_query(query),
            AstExpression::Retrieve(retrieve) => self.convert_retrieve(retrieve),

            // === Function Calls ===
            AstExpression::FunctionRef(fn_ref) => {
                // Check if this is a built-in function that should be converted to an operator
                if fn_ref.library.is_none() {
                    if let Some(expr) = self.try_convert_builtin_function(&fn_ref.name.name, &fn_ref.arguments) {
                        return expr;
                    }
                }
                // Otherwise, treat as a regular function reference
                Expression::FunctionRef(FunctionRef {
                    element: Element::default(),
                    library_name: fn_ref.library.as_ref().map(|l| l.name.clone()),
                    name: fn_ref.name.name.clone(),
                    operand: if fn_ref.arguments.is_empty() {
                        None
                    } else {
                        Some(
                            fn_ref
                                .arguments
                                .iter()
                                .map(|a| Box::new(self.convert_expression(&a.inner)))
                                .collect(),
                        )
                    },
                    signature: None,
                })
            }
            AstExpression::ExternalFunctionRef(ext_fn) => Expression::FunctionRef(FunctionRef {
                element: Element::default(),
                library_name: None,
                name: ext_fn.name.name.clone(),
                operand: if ext_fn.arguments.is_empty() {
                    None
                } else {
                    Some(
                        ext_fn
                            .arguments
                            .iter()
                            .map(|a| Box::new(self.convert_expression(&a.inner)))
                            .collect(),
                    )
                },
                signature: None,
            }),

            // === Aggregate Expressions ===
            AstExpression::Aggregate(agg) => Expression::Aggregate(AggregateExpression {
                element: Element::default(),
                source: Some(Box::new(self.convert_expression(&agg.source.inner))),
                iteration: Some(Box::new(self.convert_expression(&agg.expression.inner))),
                starting: agg
                    .starting
                    .as_ref()
                    .map(|e| Box::new(self.convert_expression(&e.inner))),
                path: None,
            }),

            // === Date/Time ===
            AstExpression::Now => Expression::Now(NowExpression {
                element: Element::default(),
            }),
            AstExpression::Today => Expression::Today(TodayExpression {
                element: Element::default(),
            }),
            AstExpression::TimeOfDay => Expression::TimeOfDay(TimeOfDayExpression {
                element: Element::default(),
            }),
            AstExpression::Date(date) => Expression::Date(DateExpression {
                element: Element::default(),
                year: Box::new(self.convert_expression(&date.year.inner)),
                month: date
                    .month
                    .as_ref()
                    .map(|m| Box::new(self.convert_expression(&m.inner))),
                day: date
                    .day
                    .as_ref()
                    .map(|d| Box::new(self.convert_expression(&d.inner))),
            }),
            AstExpression::DateTime(dt) => Expression::DateTime(DateTimeExpression {
                element: Element::default(),
                year: Box::new(self.convert_expression(&dt.year.inner)),
                month: dt
                    .month
                    .as_ref()
                    .map(|m| Box::new(self.convert_expression(&m.inner))),
                day: dt
                    .day
                    .as_ref()
                    .map(|d| Box::new(self.convert_expression(&d.inner))),
                hour: dt
                    .hour
                    .as_ref()
                    .map(|h| Box::new(self.convert_expression(&h.inner))),
                minute: dt
                    .minute
                    .as_ref()
                    .map(|m| Box::new(self.convert_expression(&m.inner))),
                second: dt
                    .second
                    .as_ref()
                    .map(|s| Box::new(self.convert_expression(&s.inner))),
                millisecond: dt
                    .millisecond
                    .as_ref()
                    .map(|ms| Box::new(self.convert_expression(&ms.inner))),
                timezone_offset: dt
                    .timezone_offset
                    .as_ref()
                    .map(|tz| Box::new(self.convert_expression(&tz.inner))),
            }),
            AstExpression::Time(time) => Expression::Time(TimeExpression {
                element: Element::default(),
                hour: Box::new(self.convert_expression(&time.hour.inner)),
                minute: time
                    .minute
                    .as_ref()
                    .map(|m| Box::new(self.convert_expression(&m.inner))),
                second: time
                    .second
                    .as_ref()
                    .map(|s| Box::new(self.convert_expression(&s.inner))),
                millisecond: time
                    .millisecond
                    .as_ref()
                    .map(|ms| Box::new(self.convert_expression(&ms.inner))),
            }),
            AstExpression::DurationBetween(dur) => {
                Expression::DurationBetween(DurationBetweenExpression {
                    element: Element::default(),
                    operand: vec![
                        Box::new(self.convert_expression(&dur.low.inner)),
                        Box::new(self.convert_expression(&dur.high.inner)),
                    ],
                    precision: self.convert_temporal_precision(dur.precision),
                })
            }
            AstExpression::DifferenceBetween(diff) => {
                Expression::DifferenceBetween(DifferenceBetweenExpression {
                    element: Element::default(),
                    operand: vec![
                        Box::new(self.convert_expression(&diff.low.inner)),
                        Box::new(self.convert_expression(&diff.high.inner)),
                    ],
                    precision: self.convert_temporal_precision(diff.precision),
                })
            }
            AstExpression::DateTimeComponent(dtc) => {
                Expression::DateTimeComponentFrom(DateTimeComponentFromExpression {
                    element: Element::default(),
                    operand: Box::new(self.convert_expression(&dtc.source.inner)),
                    precision: self.convert_datetime_component(dtc.component),
                })
            }

            // === String Operations ===
            AstExpression::Concatenate(concat) => Expression::Concatenate(NaryExpression {
                element: Element::default(),
                operand: concat
                    .operands
                    .iter()
                    .map(|e| Box::new(self.convert_expression(&e.inner)))
                    .collect(),
            }),
            AstExpression::Combine(combine) => Expression::Combine(CombineExpression {
                element: Element::default(),
                source: Box::new(self.convert_expression(&combine.source.inner)),
                separator: combine
                    .separator
                    .as_ref()
                    .map(|s| Box::new(self.convert_expression(&s.inner))),
            }),
            AstExpression::Split(split) => Expression::Split(SplitExpression {
                element: Element::default(),
                string_to_split: Box::new(self.convert_expression(&split.source.inner)),
                separator: Some(Box::new(self.convert_expression(&split.separator.inner))),
            }),
            AstExpression::Matches(matches) => Expression::Matches(BinaryExpression {
                element: Element::default(),
                operand: vec![
                    Box::new(self.convert_expression(&matches.source.inner)),
                    Box::new(self.convert_expression(&matches.pattern.inner)),
                ],
            }),
            AstExpression::ReplaceMatches(replace) => {
                Expression::ReplaceMatches(TernaryExpression {
                    element: Element::default(),
                    operand: vec![
                        Box::new(self.convert_expression(&replace.source.inner)),
                        Box::new(self.convert_expression(&replace.pattern.inner)),
                        Box::new(self.convert_expression(&replace.replacement.inner)),
                    ],
                })
            }

            // === List Operations ===
            AstExpression::First(first) => Expression::First(FirstLastExpression {
                element: Element::default(),
                source: Box::new(self.convert_expression(&first.source.inner)),
                order_by: None,
            }),
            AstExpression::Last(last) => Expression::Last(FirstLastExpression {
                element: Element::default(),
                source: Box::new(self.convert_expression(&last.source.inner)),
                order_by: None,
            }),
            AstExpression::Single(single) => Expression::SingletonFrom(UnaryExpression {
                element: Element::default(),
                operand: Box::new(self.convert_expression(&single.source.inner)),
            }),
            AstExpression::Slice(slice) => Expression::Slice(SliceExpression {
                element: Element::default(),
                source: Box::new(self.convert_expression(&slice.source.inner)),
                start_index: Box::new(self.convert_expression(&slice.start_index.inner)),
                end_index: slice
                    .end_index
                    .as_ref()
                    .map(|e| Box::new(self.convert_expression(&e.inner))),
            }),
            AstExpression::IndexOf(idx_of) => Expression::IndexOf(IndexOfExpression {
                element: Element::default(),
                source: Box::new(self.convert_expression(&idx_of.source.inner)),
                element_to_find: Box::new(self.convert_expression(&idx_of.element.inner)),
            }),

            // === Membership and Comparison ===
            AstExpression::Between(between) => {
                // Between is equivalent to: operand >= low and operand <= high
                Expression::And(BinaryExpression {
                    element: Element::default(),
                    operand: vec![
                        Box::new(Expression::GreaterOrEqual(BinaryExpression {
                            element: Element::default(),
                            operand: vec![
                                Box::new(self.convert_expression(&between.operand.inner)),
                                Box::new(self.convert_expression(&between.low.inner)),
                            ],
                        })),
                        Box::new(Expression::LessOrEqual(BinaryExpression {
                            element: Element::default(),
                            operand: vec![
                                Box::new(self.convert_expression(&between.operand.inner)),
                                Box::new(self.convert_expression(&between.high.inner)),
                            ],
                        })),
                    ],
                })
            }

            // === Message ===
            AstExpression::Message(msg) => Expression::Message(MessageExpression {
                element: Element::default(),
                source: Box::new(self.convert_expression(&msg.source.inner)),
                condition: Box::new(self.convert_expression(&msg.condition.inner)),
                code: Box::new(self.convert_expression(&msg.code.inner)),
                severity: Box::new(self.convert_expression(&msg.severity.inner)),
                message: Box::new(self.convert_expression(&msg.message.inner)),
            }),

            // === Timing Expressions ===
            AstExpression::SameAs(same_as) => Expression::SameAs(SameAsExpression {
                element: Element::default(),
                operand: vec![
                    Box::new(self.convert_expression(&same_as.left.inner)),
                    Box::new(self.convert_expression(&same_as.right.inner)),
                ],
                precision: same_as.precision.map(|p| self.convert_temporal_precision(p)),
            }),
            AstExpression::SameOrBefore(sob) => Expression::SameOrBefore(SameOrBeforeExpression {
                element: Element::default(),
                operand: vec![
                    Box::new(self.convert_expression(&sob.left.inner)),
                    Box::new(self.convert_expression(&sob.right.inner)),
                ],
                precision: sob.precision.map(|p| self.convert_temporal_precision(p)),
            }),
            AstExpression::SameOrAfter(soa) => Expression::SameOrAfter(SameOrAfterExpression {
                element: Element::default(),
                operand: vec![
                    Box::new(self.convert_expression(&soa.left.inner)),
                    Box::new(self.convert_expression(&soa.right.inner)),
                ],
                precision: soa.precision.map(|p| self.convert_temporal_precision(p)),
            }),

            // === Total ===
            AstExpression::Total(_total) => Expression::Total(TotalExpression {
                element: Element::default(),
                scope: None,
            }),

            // === Iteration ===
            AstExpression::Iteration => Expression::Current(CurrentExpression {
                element: Element::default(),
                scope: None,
            }),
            AstExpression::Index => Expression::Iteration(IterationExpression {
                element: Element::default(),
                scope: None,
            }),
            AstExpression::TotalRef => Expression::Total(TotalExpression {
                element: Element::default(),
                scope: None,
            }),

            // === Error Recovery ===
            AstExpression::Error => Expression::Null(NullLiteral {
                element: Element::default(),
            }),
        }
    }

    /// Convert literal
    fn convert_literal(&self, lit: &Literal) -> Expression {
        match lit {
            Literal::Null => Expression::Null(NullLiteral {
                element: Element::default(),
            }),
            Literal::Boolean(b) => Expression::Literal(ElmLiteral {
                element: Element::default(),
                value_type: "{urn:hl7-org:elm-types:r1}Boolean".to_string(),
                value: Some(b.to_string()),
            }),
            Literal::Integer(i) => Expression::Literal(ElmLiteral {
                element: Element::default(),
                value_type: "{urn:hl7-org:elm-types:r1}Integer".to_string(),
                value: Some(i.to_string()),
            }),
            Literal::Long(l) => Expression::Literal(ElmLiteral {
                element: Element::default(),
                value_type: "{urn:hl7-org:elm-types:r1}Long".to_string(),
                value: Some(l.to_string()),
            }),
            Literal::Decimal(d) => Expression::Literal(ElmLiteral {
                element: Element::default(),
                value_type: "{urn:hl7-org:elm-types:r1}Decimal".to_string(),
                value: Some(d.to_string()),
            }),
            Literal::String(s) => Expression::Literal(ElmLiteral {
                element: Element::default(),
                value_type: "{urn:hl7-org:elm-types:r1}String".to_string(),
                value: Some(s.clone()),
            }),
            Literal::Date(d) => {
                let value = format!(
                    "@{:04}{}{}",
                    d.year,
                    d.month.map(|m| format!("-{:02}", m)).unwrap_or_default(),
                    d.day.map(|d| format!("-{:02}", d)).unwrap_or_default()
                );
                Expression::Literal(ElmLiteral {
                    element: Element::default(),
                    value_type: "{urn:hl7-org:elm-types:r1}Date".to_string(),
                    value: Some(value),
                })
            }
            Literal::DateTime(dt) => {
                let mut value = format!(
                    "@{:04}{}{}",
                    dt.date.year,
                    dt.date
                        .month
                        .map(|m| format!("-{:02}", m))
                        .unwrap_or_default(),
                    dt.date.day.map(|d| format!("-{:02}", d)).unwrap_or_default()
                );
                if let Some(h) = dt.hour {
                    value.push_str(&format!("T{:02}", h));
                    if let Some(m) = dt.minute {
                        value.push_str(&format!(":{:02}", m));
                        if let Some(s) = dt.second {
                            value.push_str(&format!(":{:02}", s));
                            if let Some(ms) = dt.millisecond {
                                value.push_str(&format!(".{:03}", ms));
                            }
                        }
                    }
                }
                if let Some(offset) = dt.timezone_offset {
                    if offset >= 0 {
                        value.push_str(&format!("+{:02}:{:02}", offset / 60, offset % 60));
                    } else {
                        let abs_offset = offset.abs();
                        value.push_str(&format!("-{:02}:{:02}", abs_offset / 60, abs_offset % 60));
                    }
                }
                Expression::Literal(ElmLiteral {
                    element: Element::default(),
                    value_type: "{urn:hl7-org:elm-types:r1}DateTime".to_string(),
                    value: Some(value),
                })
            }
            Literal::Time(t) => {
                let mut value = format!("@T{:02}", t.hour);
                if let Some(m) = t.minute {
                    value.push_str(&format!(":{:02}", m));
                    if let Some(s) = t.second {
                        value.push_str(&format!(":{:02}", s));
                        if let Some(ms) = t.millisecond {
                            value.push_str(&format!(".{:03}", ms));
                        }
                    }
                }
                Expression::Literal(ElmLiteral {
                    element: Element::default(),
                    value_type: "{urn:hl7-org:elm-types:r1}Time".to_string(),
                    value: Some(value),
                })
            }
            Literal::Quantity(q) => Expression::Quantity(QuantityExpression {
                element: Element::default(),
                value: Some(q.value),
                unit: q.unit.clone(),
            }),
            Literal::Ratio(r) => Expression::Ratio(RatioExpression {
                element: Element::default(),
                numerator: Box::new(QuantityExpression {
                    element: Element::default(),
                    value: Some(r.numerator.value),
                    unit: r.numerator.unit.clone(),
                }),
                denominator: Box::new(QuantityExpression {
                    element: Element::default(),
                    value: Some(r.denominator.value),
                    unit: r.denominator.unit.clone(),
                }),
            }),
        }
    }

    /// Convert binary operation
    fn convert_binary_op(&self, bin_op: &ast::BinaryOpExpr) -> Expression {
        let left = Box::new(self.convert_expression(&bin_op.left.inner));
        let right = Box::new(self.convert_expression(&bin_op.right.inner));
        let operand = vec![left, right];

        match bin_op.op {
            BinaryOp::Add => Expression::Add(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            BinaryOp::Subtract => Expression::Subtract(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            BinaryOp::Multiply => Expression::Multiply(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            BinaryOp::Divide => Expression::Divide(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            BinaryOp::TruncatedDivide => Expression::TruncatedDivide(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            BinaryOp::Modulo => Expression::Modulo(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            BinaryOp::Power => Expression::Power(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            BinaryOp::And => Expression::And(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            BinaryOp::Or => Expression::Or(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            BinaryOp::Xor => Expression::Xor(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            BinaryOp::Implies => Expression::Implies(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            BinaryOp::Equal => Expression::Equal(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            BinaryOp::NotEqual => Expression::NotEqual(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            BinaryOp::Equivalent => Expression::Equivalent(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            BinaryOp::NotEquivalent => {
                // NotEquivalent is: not (a ~ b)
                Expression::Not(UnaryExpression {
                    element: Element::default(),
                    operand: Box::new(Expression::Equivalent(BinaryExpression {
                        element: Element::default(),
                        operand,
                    })),
                })
            }
            BinaryOp::Less => Expression::Less(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            BinaryOp::LessOrEqual => Expression::LessOrEqual(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            BinaryOp::Greater => Expression::Greater(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            BinaryOp::GreaterOrEqual => Expression::GreaterOrEqual(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            BinaryOp::In => Expression::In(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            BinaryOp::Contains => Expression::Contains(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            BinaryOp::Union => Expression::Union(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            BinaryOp::Concatenate => Expression::Concatenate(NaryExpression {
                element: Element::default(),
                operand,
            }),
            BinaryOp::Is | BinaryOp::As => {
                // These should be handled separately via As/Is expressions
                Expression::Null(NullLiteral {
                    element: Element::default(),
                })
            }
        }
    }

    /// Convert unary operation
    fn convert_unary_op(&self, un_op: &ast::UnaryOpExpr) -> Expression {
        let operand = Box::new(self.convert_expression(&un_op.operand.inner));

        match un_op.op {
            UnaryOp::Not => Expression::Not(UnaryExpression {
                element: Element::default(),
                operand,
            }),
            UnaryOp::Plus => {
                // Unary plus is a no-op
                *operand
            }
            UnaryOp::Negate => Expression::Negate(UnaryExpression {
                element: Element::default(),
                operand,
            }),
            UnaryOp::Exists => Expression::Exists(UnaryExpression {
                element: Element::default(),
                operand,
            }),
            UnaryOp::Distinct => Expression::Distinct(UnaryExpression {
                element: Element::default(),
                operand,
            }),
            UnaryOp::Flatten => Expression::Flatten(UnaryExpression {
                element: Element::default(),
                operand,
            }),
            UnaryOp::Collapse => Expression::Collapse(UnaryExpression {
                element: Element::default(),
                operand,
            }),
            UnaryOp::SingletonFrom => Expression::SingletonFrom(UnaryExpression {
                element: Element::default(),
                operand,
            }),
        }
    }

    /// Convert interval operation
    fn convert_interval_op(&self, int_op: &ast::IntervalOpExpr) -> Expression {
        let left = Box::new(self.convert_expression(&int_op.left.inner));
        let right = Box::new(self.convert_expression(&int_op.right.inner));
        let operand = vec![left, right];

        match int_op.op {
            IntervalOp::ProperlyIncludes => Expression::ProperIncludes(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            IntervalOp::ProperlyIncludedIn => Expression::ProperIncludedIn(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            IntervalOp::Includes => Expression::Includes(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            IntervalOp::IncludedIn => Expression::IncludedIn(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            IntervalOp::Before => Expression::Before(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            IntervalOp::After => Expression::After(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            IntervalOp::Meets => Expression::Meets(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            IntervalOp::MeetsBefore => Expression::MeetsBefore(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            IntervalOp::MeetsAfter => Expression::MeetsAfter(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            IntervalOp::Overlaps => Expression::Overlaps(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            IntervalOp::OverlapsBefore => Expression::OverlapsBefore(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            IntervalOp::OverlapsAfter => Expression::OverlapsAfter(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            IntervalOp::Starts => Expression::Starts(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            IntervalOp::Ends => Expression::Ends(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            IntervalOp::During => Expression::IncludedIn(BinaryExpression {
                element: Element::default(),
                operand,
            }),
            IntervalOp::SameAs => Expression::SameAs(SameAsExpression {
                element: Element::default(),
                operand,
                precision: int_op.precision.map(|p| self.convert_temporal_precision(p)),
            }),
            IntervalOp::SameOrBefore => Expression::SameOrBefore(SameOrBeforeExpression {
                element: Element::default(),
                operand,
                precision: int_op.precision.map(|p| self.convert_temporal_precision(p)),
            }),
            IntervalOp::SameOrAfter => Expression::SameOrAfter(SameOrAfterExpression {
                element: Element::default(),
                operand,
                precision: int_op.precision.map(|p| self.convert_temporal_precision(p)),
            }),
        }
    }

    /// Convert query
    fn convert_query(&self, query: &AstQuery) -> Expression {
        Expression::Query(Query {
            element: Element::default(),
            source: query
                .sources
                .iter()
                .map(|s| AliasedQuerySource {
                    expression: Box::new(self.convert_expression(&s.expression.inner)),
                    alias: s.alias.name.clone(),
                })
                .collect(),
            let_clause: if query.lets.is_empty() {
                None
            } else {
                Some(
                    query
                        .lets
                        .iter()
                        .map(|l| LetClause {
                            identifier: l.identifier.name.clone(),
                            expression: Box::new(self.convert_expression(&l.expression.inner)),
                        })
                        .collect(),
                )
            },
            relationship: if query.relationships.is_empty() {
                None
            } else {
                Some(
                    query
                        .relationships
                        .iter()
                        .map(|r| self.convert_relationship(r))
                        .collect(),
                )
            },
            where_clause: query
                .where_clause
                .as_ref()
                .map(|w| Box::new(self.convert_expression(&w.inner))),
            return_clause: query.return_clause.as_ref().map(|r| ReturnClause {
                expression: Box::new(self.convert_expression(&r.expression.inner)),
                distinct: if r.distinct { Some(true) } else { None },
            }),
            aggregate: query.aggregate_clause.as_ref().map(|a| AggregateClause {
                identifier: a.identifier.name.clone(),
                expression: Box::new(self.convert_expression(&a.expression.inner)),
                starting: a
                    .starting
                    .as_ref()
                    .map(|s| Box::new(self.convert_expression(&s.inner))),
                distinct: if a.distinct { Some(true) } else { None },
            }),
            sort: query.sort_clause.as_ref().map(|s| SortClause {
                by: s
                    .items
                    .iter()
                    .map(|item| SortByItem {
                        direction: self.convert_sort_direction(item.direction),
                        path: item.expression.as_ref().and_then(|e| {
                            // Try to extract path from expression if it's a property
                            if let AstExpression::Property(prop) = &e.inner {
                                Some(prop.property.name.clone())
                            } else {
                                None
                            }
                        }),
                    })
                    .collect(),
            }),
        })
    }

    /// Convert relationship clause
    fn convert_relationship(&self, rel: &ast::RelationshipClause) -> RelationshipClause {
        match rel.kind {
            ast::RelationshipKind::With => RelationshipClause::With(WithClause {
                expression: Box::new(self.convert_expression(&rel.source.expression.inner)),
                alias: rel.source.alias.name.clone(),
                such_that: Box::new(
                    rel.such_that
                        .as_ref()
                        .map(|s| self.convert_expression(&s.inner))
                        .unwrap_or(Expression::Literal(ElmLiteral {
                            element: Element::default(),
                            value_type: "{urn:hl7-org:elm-types:r1}Boolean".to_string(),
                            value: Some("true".to_string()),
                        })),
                ),
            }),
            ast::RelationshipKind::Without => RelationshipClause::Without(WithoutClause {
                expression: Box::new(self.convert_expression(&rel.source.expression.inner)),
                alias: rel.source.alias.name.clone(),
                such_that: Box::new(
                    rel.such_that
                        .as_ref()
                        .map(|s| self.convert_expression(&s.inner))
                        .unwrap_or(Expression::Literal(ElmLiteral {
                            element: Element::default(),
                            value_type: "{urn:hl7-org:elm-types:r1}Boolean".to_string(),
                            value: Some("true".to_string()),
                        })),
                ),
            }),
        }
    }

    /// Convert retrieve
    fn convert_retrieve(&self, retrieve: &AstRetrieve) -> Expression {
        Expression::Retrieve(Retrieve {
            element: Element::default(),
            data_type: self.type_specifier_to_string(&retrieve.data_type.inner),
            template_id: retrieve.template_id.clone(),
            id_expression: None,
            code_property: retrieve.code_path.clone(),
            codes: retrieve
                .codes
                .as_ref()
                .map(|c| Box::new(self.convert_expression(&c.inner))),
            date_property: retrieve.date_path.clone(),
            date_range: retrieve
                .date_range
                .as_ref()
                .map(|d| Box::new(self.convert_expression(&d.inner))),
            context: retrieve.context.as_ref().map(|c| c.name.clone()),
            include: None,
        })
    }

    /// Convert sort direction
    fn convert_sort_direction(&self, dir: AstSortDirection) -> SortDirection {
        match dir {
            AstSortDirection::Ascending => SortDirection::Ascending,
            AstSortDirection::Asc => SortDirection::Asc,
            AstSortDirection::Descending => SortDirection::Descending,
            AstSortDirection::Desc => SortDirection::Desc,
        }
    }

    /// Convert temporal precision
    fn convert_temporal_precision(&self, prec: TemporalPrecision) -> DateTimePrecision {
        match prec {
            TemporalPrecision::Year => DateTimePrecision::Year,
            TemporalPrecision::Month => DateTimePrecision::Month,
            TemporalPrecision::Week => DateTimePrecision::Week,
            TemporalPrecision::Day => DateTimePrecision::Day,
            TemporalPrecision::Hour => DateTimePrecision::Hour,
            TemporalPrecision::Minute => DateTimePrecision::Minute,
            TemporalPrecision::Second => DateTimePrecision::Second,
            TemporalPrecision::Millisecond => DateTimePrecision::Millisecond,
        }
    }

    /// Convert datetime component
    fn convert_datetime_component(&self, component: DateTimeComponent) -> DateTimePrecision {
        match component {
            DateTimeComponent::Year => DateTimePrecision::Year,
            DateTimeComponent::Month => DateTimePrecision::Month,
            DateTimeComponent::Day => DateTimePrecision::Day,
            DateTimeComponent::Hour => DateTimePrecision::Hour,
            DateTimeComponent::Minute => DateTimePrecision::Minute,
            DateTimeComponent::Second => DateTimePrecision::Second,
            DateTimeComponent::Millisecond => DateTimePrecision::Millisecond,
            DateTimeComponent::TimezoneOffset => DateTimePrecision::Hour, // Approximation
            DateTimeComponent::Date => DateTimePrecision::Day,
            DateTimeComponent::Time => DateTimePrecision::Millisecond,
        }
    }

    /// Convert type specifier to string representation
    fn type_specifier_to_string(&self, ts: &ast::TypeSpecifier) -> String {
        match ts {
            ast::TypeSpecifier::Named(named) => {
                if let Some(ns) = &named.namespace {
                    format!("{{{}}}{}", ns, named.name)
                } else {
                    named.name.clone()
                }
            }
            ast::TypeSpecifier::List(list) => {
                format!("List<{}>", self.type_specifier_to_string(&list.element_type))
            }
            ast::TypeSpecifier::Interval(interval) => {
                format!(
                    "Interval<{}>",
                    self.type_specifier_to_string(&interval.point_type)
                )
            }
            ast::TypeSpecifier::Tuple(_) => "Tuple".to_string(),
            ast::TypeSpecifier::Choice(choice) => {
                let types: Vec<String> = choice
                    .types
                    .iter()
                    .map(|t| self.type_specifier_to_string(t))
                    .collect();
                format!("Choice<{}>", types.join(", "))
            }
        }
    }

    /// Try to convert a function call to a built-in operator expression
    fn try_convert_builtin_function(
        &self,
        name: &str,
        args: &[ast::Spanned<AstExpression>],
    ) -> Option<Expression> {
        // Helper to convert arguments
        let convert_args = |args: &[ast::Spanned<AstExpression>]| -> Vec<Box<Expression>> {
            args.iter()
                .map(|a| Box::new(self.convert_expression(&a.inner)))
                .collect()
        };

        // Helper to create unary expression
        let unary = |args: &[ast::Spanned<AstExpression>]| -> Option<UnaryExpression> {
            if args.len() == 1 {
                Some(UnaryExpression {
                    element: Element::default(),
                    operand: Box::new(self.convert_expression(&args[0].inner)),
                })
            } else {
                None
            }
        };

        // Helper to create binary expression
        let binary = |args: &[ast::Spanned<AstExpression>]| -> Option<BinaryExpression> {
            if args.len() == 2 {
                Some(BinaryExpression {
                    element: Element::default(),
                    operand: vec![
                        Box::new(self.convert_expression(&args[0].inner)),
                        Box::new(self.convert_expression(&args[1].inner)),
                    ],
                })
            } else {
                None
            }
        };

        // Helper to create nary expression
        let nary = |args: &[ast::Spanned<AstExpression>]| -> NaryExpression {
            NaryExpression {
                element: Element::default(),
                operand: convert_args(args),
            }
        };

        match name {
            // === String Operators ===
            "Combine" => {
                if args.len() >= 1 {
                    Some(Expression::Combine(CombineExpression {
                        element: Element::default(),
                        source: Box::new(self.convert_expression(&args[0].inner)),
                        separator: args.get(1).map(|a| Box::new(self.convert_expression(&a.inner))),
                    }))
                } else {
                    None
                }
            }
            "Concatenate" => Some(Expression::Concatenate(nary(args))),
            "EndsWith" => binary(args).map(Expression::EndsWith),
            "StartsWith" => binary(args).map(Expression::StartsWith),
            "Upper" => unary(args).map(Expression::Upper),
            "Lower" => unary(args).map(Expression::Lower),
            "Length" => unary(args).map(Expression::Length),
            "PositionOf" => {
                if args.len() == 2 {
                    Some(Expression::PositionOf(PositionOfExpression {
                        element: Element::default(),
                        pattern: Box::new(self.convert_expression(&args[0].inner)),
                        string: Box::new(self.convert_expression(&args[1].inner)),
                    }))
                } else {
                    None
                }
            }
            "LastPositionOf" => {
                if args.len() == 2 {
                    Some(Expression::LastPositionOf(LastPositionOfExpression {
                        element: Element::default(),
                        pattern: Box::new(self.convert_expression(&args[0].inner)),
                        string: Box::new(self.convert_expression(&args[1].inner)),
                    }))
                } else {
                    None
                }
            }
            "Substring" => {
                if args.len() >= 2 {
                    Some(Expression::Substring(SubstringExpression {
                        element: Element::default(),
                        string_to_sub: Box::new(self.convert_expression(&args[0].inner)),
                        start_index: Box::new(self.convert_expression(&args[1].inner)),
                        length: args.get(2).map(|a| Box::new(self.convert_expression(&a.inner))),
                    }))
                } else {
                    None
                }
            }
            "Matches" => binary(args).map(Expression::Matches),
            "ReplaceMatches" => {
                if args.len() == 3 {
                    Some(Expression::ReplaceMatches(TernaryExpression {
                        element: Element::default(),
                        operand: convert_args(args),
                    }))
                } else {
                    None
                }
            }
            "Split" => {
                if args.len() == 2 {
                    Some(Expression::Split(SplitExpression {
                        element: Element::default(),
                        string_to_split: Box::new(self.convert_expression(&args[0].inner)),
                        separator: Some(Box::new(self.convert_expression(&args[1].inner))),
                    }))
                } else {
                    None
                }
            }
            "SplitOnMatches" => {
                if args.len() == 2 {
                    Some(Expression::SplitOnMatches(SplitOnMatchesExpression {
                        element: Element::default(),
                        string_to_split: Box::new(self.convert_expression(&args[0].inner)),
                        separator_pattern: Box::new(self.convert_expression(&args[1].inner)),
                    }))
                } else {
                    None
                }
            }
            "Indexer" => binary(args).map(Expression::Indexer),

            // === Arithmetic Operators ===
            "Abs" => unary(args).map(Expression::Abs),
            "Ceiling" => unary(args).map(Expression::Ceiling),
            "Floor" => unary(args).map(Expression::Floor),
            "Truncate" => unary(args).map(Expression::Truncate),
            "Round" => {
                if args.len() >= 1 {
                    Some(Expression::Round(RoundExpression {
                        element: Element::default(),
                        operand: Box::new(self.convert_expression(&args[0].inner)),
                        precision: args.get(1).map(|a| Box::new(self.convert_expression(&a.inner))),
                    }))
                } else {
                    None
                }
            }
            "Ln" => unary(args).map(Expression::Ln),
            "Exp" => unary(args).map(Expression::Exp),
            "Log" => binary(args).map(Expression::Log),
            "Power" => binary(args).map(Expression::Power),
            "Successor" => unary(args).map(Expression::Successor),
            "Predecessor" => unary(args).map(Expression::Predecessor),
            "MinValue" => {
                // MinValue takes a type name as string
                if args.len() == 1 {
                    Some(Expression::MinValue(MinMaxValueExpression {
                        element: Element::default(),
                        value_type: self.extract_type_from_arg(&args[0]),
                    }))
                } else {
                    None
                }
            }
            "MaxValue" => {
                if args.len() == 1 {
                    Some(Expression::MaxValue(MinMaxValueExpression {
                        element: Element::default(),
                        value_type: self.extract_type_from_arg(&args[0]),
                    }))
                } else {
                    None
                }
            }
            "Precision" => unary(args).map(Expression::Precision),
            "LowBoundary" => {
                if args.len() >= 1 {
                    Some(Expression::LowBoundary(BoundaryExpression {
                        element: Element::default(),
                        operand: Box::new(self.convert_expression(&args[0].inner)),
                        precision: args.get(1).map(|a| Box::new(self.convert_expression(&a.inner))),
                    }))
                } else {
                    None
                }
            }
            "HighBoundary" => {
                if args.len() >= 1 {
                    Some(Expression::HighBoundary(BoundaryExpression {
                        element: Element::default(),
                        operand: Box::new(self.convert_expression(&args[0].inner)),
                        precision: args.get(1).map(|a| Box::new(self.convert_expression(&a.inner))),
                    }))
                } else {
                    None
                }
            }

            // === Aggregate Operators ===
            "Sum" | "Avg" | "Min" | "Max" | "Count" | "Median" | "Mode" |
            "StdDev" | "Variance" | "PopulationStdDev" | "PopulationVariance" |
            "AllTrue" | "AnyTrue" | "Product" | "GeometricMean" => {
                if args.len() >= 1 {
                    let agg_expr = AggregateExpression {
                        element: Element::default(),
                        source: Some(Box::new(self.convert_expression(&args[0].inner))),
                        iteration: None,
                        starting: None,
                        path: None,
                    };
                    Some(match name {
                        "Sum" => Expression::Sum(agg_expr),
                        "Avg" => Expression::Avg(agg_expr),
                        "Min" => Expression::Min(agg_expr),
                        "Max" => Expression::Max(agg_expr),
                        "Count" => Expression::Count(agg_expr),
                        "Median" => Expression::Median(agg_expr),
                        "Mode" => Expression::Mode(agg_expr),
                        "StdDev" => Expression::StdDev(agg_expr),
                        "Variance" => Expression::Variance(agg_expr),
                        "PopulationStdDev" => Expression::PopulationStdDev(agg_expr),
                        "PopulationVariance" => Expression::PopulationVariance(agg_expr),
                        "AllTrue" => Expression::AllTrue(agg_expr),
                        "AnyTrue" => Expression::AnyTrue(agg_expr),
                        "Product" => Expression::Product(agg_expr),
                        "GeometricMean" => Expression::GeometricMean(agg_expr),
                        _ => unreachable!(),
                    })
                } else {
                    None
                }
            }

            // === List Operators ===
            "First" => {
                if args.len() >= 1 {
                    Some(Expression::First(FirstLastExpression {
                        element: Element::default(),
                        source: Box::new(self.convert_expression(&args[0].inner)),
                        order_by: None,
                    }))
                } else {
                    None
                }
            }
            "Last" => {
                if args.len() >= 1 {
                    Some(Expression::Last(FirstLastExpression {
                        element: Element::default(),
                        source: Box::new(self.convert_expression(&args[0].inner)),
                        order_by: None,
                    }))
                } else {
                    None
                }
            }
            "Distinct" => unary(args).map(Expression::Distinct),
            "Flatten" => unary(args).map(Expression::Flatten),
            "SingletonFrom" => unary(args).map(Expression::SingletonFrom),
            "Exists" => unary(args).map(Expression::Exists),
            "IndexOf" => {
                if args.len() == 2 {
                    Some(Expression::IndexOf(IndexOfExpression {
                        element: Element::default(),
                        source: Box::new(self.convert_expression(&args[0].inner)),
                        element_to_find: Box::new(self.convert_expression(&args[1].inner)),
                    }))
                } else {
                    None
                }
            }
            "Slice" => {
                if args.len() >= 2 {
                    Some(Expression::Slice(SliceExpression {
                        element: Element::default(),
                        source: Box::new(self.convert_expression(&args[0].inner)),
                        start_index: Box::new(self.convert_expression(&args[1].inner)),
                        end_index: args.get(2).map(|a| Box::new(self.convert_expression(&a.inner))),
                    }))
                } else {
                    None
                }
            }
            "Skip" | "Take" | "Tail" => {
                // These are typically handled as special list operations
                None
            }

            // === Type Conversion Operators ===
            "ToBoolean" => unary(args).map(Expression::ToBoolean),
            "ToInteger" => unary(args).map(Expression::ToInteger),
            "ToLong" => unary(args).map(Expression::ToLong),
            "ToDecimal" => unary(args).map(Expression::ToDecimal),
            "ToString" => unary(args).map(Expression::ToString),
            "ToDate" => unary(args).map(Expression::ToDate),
            "ToDateTime" => unary(args).map(Expression::ToDateTime),
            "ToTime" => unary(args).map(Expression::ToTime),
            "ToQuantity" => unary(args).map(Expression::ToQuantity),
            "ToRatio" => unary(args).map(Expression::ToRatio),
            "ToConcept" => unary(args).map(Expression::ToConcept),
            "ToChars" => unary(args).map(Expression::ToChars),
            "ToList" => unary(args).map(Expression::ToList),
            "ConvertsToBoolean" => unary(args).map(Expression::ConvertsToBoolean),
            "ConvertsToInteger" => unary(args).map(Expression::ConvertsToInteger),
            "ConvertsToLong" => unary(args).map(Expression::ConvertsToLong),
            "ConvertsToDecimal" => unary(args).map(Expression::ConvertsToDecimal),
            "ConvertsToString" => unary(args).map(Expression::ConvertsToString),
            "ConvertsToDate" => unary(args).map(Expression::ConvertsToDate),
            "ConvertsToDateTime" => unary(args).map(Expression::ConvertsToDateTime),
            "ConvertsToTime" => unary(args).map(Expression::ConvertsToTime),
            "ConvertsToQuantity" => unary(args).map(Expression::ConvertsToQuantity),
            "ConvertsToRatio" => unary(args).map(Expression::ConvertsToRatio),

            // === Nullological Operators ===
            "IsNull" => unary(args).map(Expression::IsNull),
            "IsTrue" => unary(args).map(Expression::IsTrue),
            "IsFalse" => unary(args).map(Expression::IsFalse),
            "Coalesce" => Some(Expression::Coalesce(nary(args))),

            // === Interval Operators ===
            "Start" => unary(args).map(Expression::Start),
            "End" => unary(args).map(Expression::End),
            "PointFrom" => unary(args).map(Expression::PointFrom),
            "Width" => unary(args).map(Expression::Width),
            "Size" => unary(args).map(Expression::Size),
            "Contains" => binary(args).map(Expression::Contains),
            "In" => binary(args).map(Expression::In),
            "Includes" => binary(args).map(Expression::Includes),
            "IncludedIn" => binary(args).map(Expression::IncludedIn),
            "ProperContains" => binary(args).map(Expression::ProperContains),
            "ProperIn" => binary(args).map(Expression::ProperIn),
            "ProperIncludes" => binary(args).map(Expression::ProperIncludes),
            "ProperIncludedIn" => binary(args).map(Expression::ProperIncludedIn),
            "Before" => binary(args).map(Expression::Before),
            "After" => binary(args).map(Expression::After),
            "Meets" => binary(args).map(Expression::Meets),
            "MeetsBefore" => binary(args).map(Expression::MeetsBefore),
            "MeetsAfter" => binary(args).map(Expression::MeetsAfter),
            "Overlaps" => binary(args).map(Expression::Overlaps),
            "OverlapsBefore" => binary(args).map(Expression::OverlapsBefore),
            "OverlapsAfter" => binary(args).map(Expression::OverlapsAfter),
            "Starts" => binary(args).map(Expression::Starts),
            "Ends" => binary(args).map(Expression::Ends),
            "Collapse" => unary(args).map(Expression::Collapse),
            "Expand" => {
                if args.len() >= 1 {
                    Some(Expression::Expand(ExpandExpression {
                        element: Element::default(),
                        operand: Box::new(self.convert_expression(&args[0].inner)),
                        per: args.get(1).map(|a| Box::new(self.convert_expression(&a.inner))),
                    }))
                } else {
                    None
                }
            }
            "Union" => binary(args).map(Expression::Union),
            "Intersect" => binary(args).map(Expression::Intersect),
            "Except" => binary(args).map(Expression::Except),

            // === Date/Time Functions ===
            "Today" => {
                if args.is_empty() {
                    Some(Expression::Today(TodayExpression {
                        element: Element::default(),
                    }))
                } else {
                    None
                }
            }
            "Now" => {
                if args.is_empty() {
                    Some(Expression::Now(NowExpression {
                        element: Element::default(),
                    }))
                } else {
                    None
                }
            }
            "TimeOfDay" => {
                if args.is_empty() {
                    Some(Expression::TimeOfDay(TimeOfDayExpression {
                        element: Element::default(),
                    }))
                } else {
                    None
                }
            }
            "Date" => {
                if args.len() >= 1 {
                    Some(Expression::Date(DateExpression {
                        element: Element::default(),
                        year: Box::new(self.convert_expression(&args[0].inner)),
                        month: args.get(1).map(|a| Box::new(self.convert_expression(&a.inner))),
                        day: args.get(2).map(|a| Box::new(self.convert_expression(&a.inner))),
                    }))
                } else {
                    None
                }
            }
            "DateTime" => {
                if args.len() >= 1 {
                    Some(Expression::DateTime(DateTimeExpression {
                        element: Element::default(),
                        year: Box::new(self.convert_expression(&args[0].inner)),
                        month: args.get(1).map(|a| Box::new(self.convert_expression(&a.inner))),
                        day: args.get(2).map(|a| Box::new(self.convert_expression(&a.inner))),
                        hour: args.get(3).map(|a| Box::new(self.convert_expression(&a.inner))),
                        minute: args.get(4).map(|a| Box::new(self.convert_expression(&a.inner))),
                        second: args.get(5).map(|a| Box::new(self.convert_expression(&a.inner))),
                        millisecond: args.get(6).map(|a| Box::new(self.convert_expression(&a.inner))),
                        timezone_offset: args.get(7).map(|a| Box::new(self.convert_expression(&a.inner))),
                    }))
                } else {
                    None
                }
            }
            "Time" => {
                if args.len() >= 1 {
                    Some(Expression::Time(TimeExpression {
                        element: Element::default(),
                        hour: Box::new(self.convert_expression(&args[0].inner)),
                        minute: args.get(1).map(|a| Box::new(self.convert_expression(&a.inner))),
                        second: args.get(2).map(|a| Box::new(self.convert_expression(&a.inner))),
                        millisecond: args.get(3).map(|a| Box::new(self.convert_expression(&a.inner))),
                    }))
                } else {
                    None
                }
            }

            // === Comparison Operators ===
            "Equal" => binary(args).map(Expression::Equal),
            "Equivalent" => binary(args).map(Expression::Equivalent),
            "NotEqual" => binary(args).map(Expression::NotEqual),
            "Less" => binary(args).map(Expression::Less),
            "Greater" => binary(args).map(Expression::Greater),
            "LessOrEqual" => binary(args).map(Expression::LessOrEqual),
            "GreaterOrEqual" => binary(args).map(Expression::GreaterOrEqual),

            // === Logical Operators ===
            "And" => binary(args).map(Expression::And),
            "Or" => binary(args).map(Expression::Or),
            "Xor" => binary(args).map(Expression::Xor),
            "Not" => unary(args).map(Expression::Not),
            "Implies" => binary(args).map(Expression::Implies),

            // Not a built-in function
            _ => None,
        }
    }

    /// Extract type name from an argument (for MinValue/MaxValue)
    fn extract_type_from_arg(&self, arg: &ast::Spanned<AstExpression>) -> String {
        match &arg.inner {
            AstExpression::Literal(Literal::String(s)) => s.clone(),
            AstExpression::IdentifierRef(id) => id.name.name.clone(),
            _ => "Any".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_converter() {
        let converter = AstToElmConverter::new();
        assert!(converter.current_context.is_none());
        assert!(converter.library_name.is_none());
    }

    #[test]
    fn test_convert_empty_library() {
        let mut converter = AstToElmConverter::new();
        let ast_lib = AstLibrary::new();
        let elm_lib = converter.convert_library(&ast_lib);

        assert_eq!(elm_lib.identifier.id, "Anonymous");
        assert!(elm_lib.usings.is_none());
        assert!(elm_lib.includes.is_none());
    }
}
