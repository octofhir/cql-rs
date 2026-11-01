library CommonLogic version '1.0.0'

using FHIR version '4.0.1'

include FHIRHelpers version '4.0.1' called FHIRHelpers

codesystem "ConditionClinicalStatusCodes": 'http://terminology.hl7.org/CodeSystem/condition-clinical'

code "active": 'active' from "ConditionClinicalStatusCodes" display 'Active'

concept "Active Condition Status": { "active" } display 'Active'

/* Normalizes a dateTime or Period choice to an interval
   (same shape as QICoreCommon.toInterval). */
define fluent function "toInterval"(choice Choice<DateTime, Interval<DateTime>>):
  case
    when choice is DateTime then
      Interval[choice as DateTime, choice as DateTime]
    when choice is Interval<DateTime> then
      choice as Interval<DateTime>
    else null as Interval<DateTime>
  end

define function "Normalize Interval"(choice Choice<DateTime, Interval<DateTime>>):
  choice.toInterval()
