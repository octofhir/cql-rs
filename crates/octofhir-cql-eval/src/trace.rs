//! Debug trace of an evaluation.
//!
//! The events are those of the language-neutral debug trace of the FHIR Mapping Language IG
//! (`debug.md`). The evaluation units of CQL are its definitions and function calls: each has an
//! `enter` and an `exit` with the value it produced, and `Message()` adds a `step`. A definition
//! already evaluated is not entered again. Without a recorder on the context nothing is recorded.

use octofhir_cql_types::CqlValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TracePhase {
    Enter,
    Step,
    Exit,
}

impl TracePhase {
    pub fn code(self) -> &'static str {
        match self {
            Self::Enter => "enter",
            Self::Step => "step",
            Self::Exit => "exit",
        }
    }
}

/// A named value in scope at an event.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceVariable {
    pub name: String,
    /// `parameter` for the operand of a function, `expression` for what a unit returned.
    pub context_type: &'static str,
    pub value: CqlValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TraceEvent {
    pub phase: TracePhase,
    pub depth: usize,
    /// The library the unit is in.
    pub library: String,
    /// The definition or function.
    pub name: String,
    /// Where it is in the library, as `line:column-line:column`, 1-based.
    pub locator: Option<String>,
    pub variables: Vec<TraceVariable>,
    pub message: Option<String>,
}

/// Collects the events of one evaluation. Past `max_events` evaluation goes on and the trace is
/// truncated.
#[derive(Debug, Clone)]
pub struct TraceRecorder {
    max_events: usize,
    depth: usize,
    truncated: bool,
    events: Vec<TraceEvent>,
}

impl TraceRecorder {
    pub fn new(max_events: usize) -> Self {
        Self {
            max_events,
            depth: 0,
            truncated: false,
            events: Vec::new(),
        }
    }

    fn record(&mut self, event: TraceEvent) {
        if self.events.len() >= self.max_events {
            self.truncated = true;
        } else {
            self.events.push(event);
        }
    }

    pub(crate) fn enter(
        &mut self,
        library: &str,
        name: &str,
        locator: Option<&str>,
        variables: Vec<TraceVariable>,
    ) {
        let depth = self.depth;
        self.depth += 1;
        self.record(TraceEvent {
            phase: TracePhase::Enter,
            depth,
            library: library.to_owned(),
            name: name.to_owned(),
            locator: locator.map(str::to_owned),
            variables,
            message: None,
        });
    }

    /// A failing unit has no `exit`, as the format says of a stopped trace.
    pub(crate) fn exit(
        &mut self,
        library: &str,
        name: &str,
        locator: Option<&str>,
        value: Option<&CqlValue>,
    ) {
        self.depth = self.depth.saturating_sub(1);
        let Some(value) = value else {
            return;
        };
        self.record(TraceEvent {
            phase: TracePhase::Exit,
            depth: self.depth,
            library: library.to_owned(),
            name: name.to_owned(),
            locator: locator.map(str::to_owned),
            variables: vec![TraceVariable {
                name: "result".to_owned(),
                context_type: "expression",
                value: value.clone(),
            }],
            message: None,
        });
    }

    pub(crate) fn message(&mut self, library: &str, text: String) {
        self.record(TraceEvent {
            phase: TracePhase::Step,
            depth: self.depth,
            library: library.to_owned(),
            name: String::new(),
            locator: None,
            variables: Vec::new(),
            message: Some(text),
        });
    }

    pub fn truncated(&self) -> bool {
        self.truncated
    }

    pub fn events(&self) -> &[TraceEvent] {
        &self.events
    }

    pub fn into_events(self) -> Vec<TraceEvent> {
        self.events
    }
}
