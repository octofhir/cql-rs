//! FHIR test data builders
//!
//! Utilities for creating realistic FHIR resources for testing CQL evaluation
//! with clinical data.

use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use octofhir_cql_types::{CqlCode, CqlConcept, CqlDate, CqlDateTime, CqlQuantity, CqlTuple, CqlValue};
use rust_decimal::Decimal;

/// Builder for FHIR Patient resources
#[derive(Default)]
pub struct PatientBuilder {
    id: Option<String>,
    given: Vec<String>,
    family: Option<String>,
    birth_date: Option<CqlDate>,
    gender: Option<String>,
}

impl PatientBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn given(mut self, given: impl Into<String>) -> Self {
        self.given.push(given.into());
        self
    }

    pub fn family(mut self, family: impl Into<String>) -> Self {
        self.family = Some(family.into());
        self
    }

    pub fn birth_date(mut self, year: i32, month: u8, day: u8) -> Self {
        self.birth_date = Some(CqlDate::new(year, month, day));
        self
    }

    pub fn gender(mut self, gender: impl Into<String>) -> Self {
        self.gender = Some(gender.into());
        self
    }

    pub fn build(self) -> CqlValue {
        let mut fields = vec![];
        fields.push(("resourceType", CqlValue::string("Patient")));

        if let Some(id) = self.id {
            fields.push(("id", CqlValue::string(id)));
        }

        if !self.given.is_empty() || self.family.is_some() {
            let mut name_fields = vec![];
            if !self.given.is_empty() {
                name_fields.push((
                    "given",
                    CqlValue::list(self.given.into_iter().map(CqlValue::string)),
                ));
            }
            if let Some(family) = self.family {
                name_fields.push(("family", CqlValue::string(family)));
            }
            fields.push(("name", CqlValue::list(vec![CqlValue::Tuple(CqlTuple::from_elements(name_fields))])));
        }

        if let Some(birth_date) = self.birth_date {
            fields.push(("birthDate", CqlValue::Date(birth_date)));
        }

        if let Some(gender) = self.gender {
            fields.push(("gender", CqlValue::string(gender)));
        }

        CqlValue::Tuple(CqlTuple::from_elements(fields))
    }
}

/// Builder for FHIR Observation resources
#[derive(Default)]
pub struct ObservationBuilder {
    id: Option<String>,
    status: String,
    code: Option<CqlCode>,
    subject: Option<String>,
    value: Option<CqlValue>,
    effective: Option<CqlDateTime>,
}

impl ObservationBuilder {
    pub fn new() -> Self {
        Self {
            status: "final".to_string(),
            ..Default::default()
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.status = status.into();
        self
    }

    pub fn code(mut self, system: impl Into<String>, code: impl Into<String>, display: Option<String>) -> Self {
        self.code = Some(CqlCode {
            system: system.into(),
            version: None,
            code: code.into(),
            display,
        });
        self
    }

    pub fn subject(mut self, reference: impl Into<String>) -> Self {
        self.subject = Some(reference.into());
        self
    }

    pub fn value_quantity(mut self, value: &str, unit: impl Into<String>) -> Self {
        let decimal = value.parse::<Decimal>().expect("Invalid decimal");
        self.value = Some(CqlValue::Quantity(CqlQuantity {
            value: decimal,
            unit: unit.into(),
        }));
        self
    }

    pub fn value_integer(mut self, value: i64) -> Self {
        self.value = Some(CqlValue::integer(value));
        self
    }

    pub fn value_string(mut self, value: impl Into<String>) -> Self {
        self.value = Some(CqlValue::string(value));
        self
    }

    pub fn effective_datetime(mut self, dt: CqlDateTime) -> Self {
        self.effective = Some(dt);
        self
    }

    pub fn build(self) -> CqlValue {
        let mut fields = vec![];
        fields.push(("resourceType", CqlValue::string("Observation")));

        if let Some(id) = self.id {
            fields.push(("id", CqlValue::string(id)));
        }

        fields.push(("status", CqlValue::string(self.status)));

        if let Some(code) = self.code {
            fields.push(("code", CqlValue::Code(code)));
        }

        if let Some(subject) = self.subject {
            fields.push((
                "subject",
                CqlValue::Tuple(CqlTuple::from_elements([("reference", CqlValue::string(subject))])),
            ));
        }

        if let Some(value) = self.value {
            fields.push(("value", value));
        }

        if let Some(effective) = self.effective {
            fields.push(("effectiveDateTime", CqlValue::DateTime(effective)));
        }

        CqlValue::Tuple(CqlTuple::from_elements(fields))
    }
}

/// Builder for FHIR Condition resources
#[derive(Default)]
pub struct ConditionBuilder {
    id: Option<String>,
    clinical_status: String,
    code: Option<CqlCode>,
    subject: Option<String>,
    onset: Option<CqlDateTime>,
}

impl ConditionBuilder {
    pub fn new() -> Self {
        Self {
            clinical_status: "active".to_string(),
            ..Default::default()
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn clinical_status(mut self, status: impl Into<String>) -> Self {
        self.clinical_status = status.into();
        self
    }

    pub fn code(mut self, system: impl Into<String>, code: impl Into<String>, display: Option<String>) -> Self {
        self.code = Some(CqlCode {
            system: system.into(),
            version: None,
            code: code.into(),
            display,
        });
        self
    }

    pub fn subject(mut self, reference: impl Into<String>) -> Self {
        self.subject = Some(reference.into());
        self
    }

    pub fn onset_datetime(mut self, dt: CqlDateTime) -> Self {
        self.onset = Some(dt);
        self
    }

    pub fn build(self) -> CqlValue {
        let mut fields = vec![];
        fields.push(("resourceType", CqlValue::string("Condition")));

        if let Some(id) = self.id {
            fields.push(("id", CqlValue::string(id)));
        }

        fields.push(("clinicalStatus", CqlValue::string(self.clinical_status)));

        if let Some(code) = self.code {
            fields.push(("code", CqlValue::Code(code)));
        }

        if let Some(subject) = self.subject {
            fields.push((
                "subject",
                CqlValue::Tuple(CqlTuple::from_elements([("reference", CqlValue::string(subject))])),
            ));
        }

        if let Some(onset) = self.onset {
            fields.push(("onsetDateTime", CqlValue::DateTime(onset)));
        }

        CqlValue::Tuple(CqlTuple::from_elements(fields))
    }
}

/// Builder for FHIR Medication resources
#[derive(Default)]
pub struct MedicationBuilder {
    id: Option<String>,
    code: Option<CqlCode>,
}

impl MedicationBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn code(mut self, system: impl Into<String>, code: impl Into<String>, display: Option<String>) -> Self {
        self.code = Some(CqlCode {
            system: system.into(),
            version: None,
            code: code.into(),
            display,
        });
        self
    }

    pub fn build(self) -> CqlValue {
        let mut fields = vec![];
        fields.push(("resourceType", CqlValue::string("Medication")));

        if let Some(id) = self.id {
            fields.push(("id", CqlValue::string(id)));
        }

        if let Some(code) = self.code {
            fields.push(("code", CqlValue::Code(code)));
        }

        CqlValue::Tuple(CqlTuple::from_elements(fields))
    }
}

/// Builder for FHIR Encounter resources
#[derive(Default)]
pub struct EncounterBuilder {
    id: Option<String>,
    status: String,
    class_code: Option<CqlCode>,
    subject: Option<String>,
    period_start: Option<CqlDateTime>,
    period_end: Option<CqlDateTime>,
}

impl EncounterBuilder {
    pub fn new() -> Self {
        Self {
            status: "finished".to_string(),
            ..Default::default()
        }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.status = status.into();
        self
    }

    pub fn class(mut self, system: impl Into<String>, code: impl Into<String>, display: Option<String>) -> Self {
        self.class_code = Some(CqlCode {
            system: system.into(),
            version: None,
            code: code.into(),
            display,
        });
        self
    }

    pub fn subject(mut self, reference: impl Into<String>) -> Self {
        self.subject = Some(reference.into());
        self
    }

    pub fn period(mut self, start: CqlDateTime, end: CqlDateTime) -> Self {
        self.period_start = Some(start);
        self.period_end = Some(end);
        self
    }

    pub fn build(self) -> CqlValue {
        let mut fields = vec![];
        fields.push(("resourceType", CqlValue::string("Encounter")));

        if let Some(id) = self.id {
            fields.push(("id", CqlValue::string(id)));
        }

        fields.push(("status", CqlValue::string(self.status)));

        if let Some(class) = self.class_code {
            fields.push(("class", CqlValue::Code(class)));
        }

        if let Some(subject) = self.subject {
            fields.push((
                "subject",
                CqlValue::Tuple(CqlTuple::from_elements([("reference", CqlValue::string(subject))])),
            ));
        }

        if self.period_start.is_some() || self.period_end.is_some() {
            let mut period_fields = vec![];
            if let Some(start) = self.period_start {
                period_fields.push(("start", CqlValue::DateTime(start)));
            }
            if let Some(end) = self.period_end {
                period_fields.push(("end", CqlValue::DateTime(end)));
            }
            fields.push(("period", CqlValue::Tuple(CqlTuple::from_elements(period_fields))));
        }

        CqlValue::Tuple(CqlTuple::from_elements(fields))
    }
}

/// Helper to create a LOINC code
pub fn loinc(code: &str, display: Option<&str>) -> CqlCode {
    CqlCode {
        system: "http://loinc.org".to_string(),
        version: None,
        code: code.to_string(),
        display: display.map(|s| s.to_string()),
    }
}

/// Helper to create a SNOMED CT code
pub fn snomed(code: &str, display: Option<&str>) -> CqlCode {
    CqlCode {
        system: "http://snomed.info/sct".to_string(),
        version: None,
        code: code.to_string(),
        display: display.map(|s| s.to_string()),
    }
}

/// Helper to create an ICD-10 code
pub fn icd10(code: &str, display: Option<&str>) -> CqlCode {
    CqlCode {
        system: "http://hl7.org/fhir/sid/icd-10-cm".to_string(),
        version: None,
        code: code.to_string(),
        display: display.map(|s| s.to_string()),
    }
}

/// Helper to create an RxNorm code
pub fn rxnorm(code: &str, display: Option<&str>) -> CqlCode {
    CqlCode {
        system: "http://www.nlm.nih.gov/research/umls/rxnorm".to_string(),
        version: None,
        code: code.to_string(),
        display: display.map(|s| s.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patient_builder() {
        let patient = PatientBuilder::new()
            .id("p1")
            .given("John")
            .family("Doe")
            .birth_date(1980, 5, 15)
            .gender("male")
            .build();

        match patient {
            CqlValue::Tuple(tuple) => {
                assert_eq!(tuple.get("id"), Some(&CqlValue::string("p1")));
                assert_eq!(tuple.get("gender"), Some(&CqlValue::string("male")));
            }
            _ => panic!("Expected tuple"),
        }
    }

    #[test]
    fn test_observation_builder() {
        let obs = ObservationBuilder::new()
            .id("obs1")
            .code("http://loinc.org", "8480-6", Some("Systolic BP".to_string()))
            .value_quantity("120", "mmHg")
            .build();

        match obs {
            CqlValue::Tuple(tuple) => {
                assert_eq!(tuple.get("id"), Some(&CqlValue::string("obs1")));
                assert_eq!(
                    tuple.get("resourceType"),
                    Some(&CqlValue::string("Observation"))
                );
            }
            _ => panic!("Expected tuple"),
        }
    }

    #[test]
    fn test_condition_builder() {
        let condition = ConditionBuilder::new()
            .id("c1")
            .code("http://snomed.info/sct", "44054006", Some("Diabetes".to_string()))
            .build();

        match condition {
            CqlValue::Tuple(tuple) => {
                assert_eq!(tuple.get("id"), Some(&CqlValue::string("c1")));
            }
            _ => panic!("Expected tuple"),
        }
    }

    #[test]
    fn test_loinc_helper() {
        let code = loinc("8480-6", Some("Systolic BP"));
        assert_eq!(code.code, "8480-6");
        assert_eq!(code.system, "http://loinc.org");
    }

    #[test]
    fn test_snomed_helper() {
        let code = snomed("44054006", Some("Diabetes"));
        assert_eq!(code.code, "44054006");
        assert_eq!(code.system, "http://snomed.info/sct");
    }
}
