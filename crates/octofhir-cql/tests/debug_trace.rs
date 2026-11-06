//! The debug trace of an evaluation: definitions, function calls and `Message()`.

use octofhir_cql::CompiledLibrary;
use octofhir_cql::eval::{EvaluationContext, TraceEvent, TracePhase, TraceRecorder};
use octofhir_cql::types::CqlValue;

const SOURCE: &str = r#"library Trace version '1.0'

define function Twice(value Integer):
  value * 2

define "Base": 21
define "Answer": Twice("Base")
define "Again": "Answer" + "Base"
define "Said": Message(1, true, 'code', 'Warning', 'careful')
"#;

fn run(max_events: usize) -> (Vec<TraceEvent>, bool) {
    let library = CompiledLibrary::compile(SOURCE).unwrap();
    let mut ctx = EvaluationContext::new();
    library.register(&mut ctx);
    ctx.record_trace(TraceRecorder::new(max_events));
    library.evaluate(&mut ctx).unwrap();
    let trace = ctx.take_trace().unwrap();
    let truncated = trace.truncated();
    (trace.into_events(), truncated)
}

#[test]
fn definitions_and_functions_enter_and_exit() {
    let (events, truncated) = run(100);
    assert!(!truncated);
    let names: Vec<_> = events
        .iter()
        .map(|event| (event.phase, event.depth, event.name.as_str()))
        .collect();
    assert_eq!(
        names[..8],
        [
            (TracePhase::Enter, 0, "Base"),
            (TracePhase::Exit, 0, "Base"),
            (TracePhase::Enter, 0, "Answer"),
            (TracePhase::Enter, 1, "Twice"),
            (TracePhase::Exit, 1, "Twice"),
            (TracePhase::Exit, 0, "Answer"),
            (TracePhase::Enter, 0, "Again"),
            (TracePhase::Exit, 0, "Again"),
        ]
    );
}

#[test]
fn a_function_shows_its_operands_and_a_unit_its_result() {
    let (events, _) = run(100);
    let call = events.iter().find(|event| event.name == "Twice").unwrap();
    assert_eq!(call.variables[0].name, "value");
    assert_eq!(call.variables[0].context_type, "parameter");
    assert_eq!(call.variables[0].value, CqlValue::Integer(21));
    let exit = events
        .iter()
        .find(|event| event.name == "Answer" && event.phase == TracePhase::Exit)
        .unwrap();
    assert_eq!(exit.variables[0].value, CqlValue::Integer(42));
}

#[test]
fn units_know_where_they_are() {
    let (events, _) = run(100);
    let locator = |name: &str| {
        events
            .iter()
            .find(|event| event.name == name)
            .and_then(|event| event.locator.clone())
            .unwrap()
    };
    assert!(locator("Twice").starts_with("3:1-"), "{}", locator("Twice"));
    assert!(locator("Base").starts_with("6:1-6:"), "{}", locator("Base"));
    assert_eq!(events[0].library, "Trace");
}

#[test]
fn message_adds_a_step() {
    let (events, _) = run(100);
    let step = events
        .iter()
        .find(|event| event.phase == TracePhase::Step)
        .unwrap();
    assert_eq!(step.message.as_deref(), Some("Warning code: careful"));
}

#[test]
fn the_limit_cuts_the_events_and_not_the_evaluation() {
    let (events, truncated) = run(2);
    assert_eq!(events.len(), 2);
    assert!(truncated);
}
