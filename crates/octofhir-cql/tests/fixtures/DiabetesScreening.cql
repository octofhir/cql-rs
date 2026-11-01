library DiabetesScreening version '1.0.0'

using FHIR version '4.0.1'

include FHIRHelpers version '4.0.1' called FHIRHelpers
include CommonLogic version '1.0.0' called Common

codesystem "LOINC": 'http://loinc.org'
codesystem "SNOMEDCT": 'http://snomed.info/sct'

valueset "Diabetes": 'http://example.org/fhir/ValueSet/diabetes'
valueset "Office Visit": 'http://example.org/fhir/ValueSet/office-visit'

code "HbA1c Laboratory Test": '4548-4' from "LOINC" display 'Hemoglobin A1c/Hemoglobin.total in Blood'
code "Type 2 diabetes": '44054006' from "SNOMEDCT" display 'Diabetes mellitus type 2'

concept "HbA1c": { "HbA1c Laboratory Test" } display 'HbA1c'

parameter "Measurement Period" Interval<DateTime>
  default Interval[@2024-01-01T00:00:00.0Z, @2025-01-01T00:00:00.0Z)

parameter "HbA1c Threshold" Decimal default 9.0

context Patient

define "Qualifying Encounters":
  [Encounter: "Office Visit"] E
    where E.status = 'finished'
      and E.period during "Measurement Period"

define "Diabetes Diagnosis":
  [Condition: "Diabetes"] C
    where C.clinicalStatus ~ Common."Active Condition Status"

define "Initial Population":
  AgeInYearsAt(start of "Measurement Period") between 18 and 75
    and exists "Qualifying Encounters"

define "Denominator":
  "Initial Population" and exists "Diabetes Diagnosis"

define "HbA1c Tests":
  [Observation: "HbA1c Laboratory Test"] O
    where O.status in { 'final', 'amended', 'corrected' }
      and Common."Normalize Interval"(O.effective) during "Measurement Period"

define "Most Recent HbA1c":
  Last("HbA1c Tests" O sort by start of Common."Normalize Interval"(effective))

define "Most Recent HbA1c Value":
  "Most Recent HbA1c".value as Quantity

define "Numerator":
  "Most Recent HbA1c Value" > Quantity { value: "HbA1c Threshold", unit: '%' }

define "Encounters With Diabetes":
  "Qualifying Encounters" E
    with "Diabetes Diagnosis" D
      such that Common."Normalize Interval"(D.onset) starts before end of E.period

define "Encounters Without HbA1c":
  "Qualifying Encounters" E
    without "HbA1c Tests" O
      such that Common."Normalize Interval"(O.effective) during E.period

define function "Is Poorly Controlled"(value Quantity):
  value > Quantity { value: "HbA1c Threshold", unit: '%' }

define "Poorly Controlled":
  "Is Poorly Controlled"("Most Recent HbA1c Value")

define "Has Type 2 Code":
  exists ([Condition: "Type 2 diabetes"])

define "HbA1c Concept Matches":
  "Most Recent HbA1c".code ~ "HbA1c"

define "HbA1c Test Periods":
  "HbA1c Tests" O
    return O.effective.toInterval()

define "Active Diabetes Codes":
  "Diabetes Diagnosis" C
    let status: C.clinicalStatus
    where status ~ Common."Active Condition Status"
    return C.code
