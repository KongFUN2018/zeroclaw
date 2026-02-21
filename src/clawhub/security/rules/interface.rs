use super::{CheckError, SecurityRule};
use crate::clawhub::sif::{ParamType, SkillSIF};
use crate::clawhub::security::report::{Finding, Severity};

pub struct InterfaceBoundaryRule;

impl SecurityRule for InterfaceBoundaryRule {
    fn name(&self) -> &str {
        "interface-boundary"
    }

    fn check(&self, skill: &SkillSIF) -> Result<Vec<Finding>, CheckError> {
        let mut findings = Vec::new();

        let interface = match &skill.interface {
            Some(i) => i,
            None => return Ok(findings),
        };

        // Check input parameters
        for (idx, input) in interface.inputs.iter().enumerate() {
            let location = format!("interface.inputs[{}]", idx);

            // String inputs should have max_length
            if input.param_type == ParamType::String && input.max_length.is_none() {
                findings.push(Finding {
                    severity: Severity::Medium,
                    rule: "interface-boundary".into(),
                    message: format!(
                        "String parameter '{}' missing max_length constraint",
                        input.name
                    ),
                    location: location.clone(),
                    suggestion: "Add max_length to prevent DoS via large inputs".into(),
                });
            }

            // Array inputs should have item_schema
            if input.param_type == ParamType::Array && input.item_schema.is_none() {
                findings.push(Finding {
                    severity: Severity::Medium,
                    rule: "interface-boundary".into(),
                    message: format!(
                        "Array parameter '{}' missing item_schema definition",
                        input.name
                    ),
                    location: location.clone(),
                    suggestion: "Define item_schema to validate array elements".into(),
                });
            }

            // Any type is unsafe
            if input.param_type == ParamType::Any {
                findings.push(Finding {
                    severity: Severity::High,
                    rule: "interface-boundary".into(),
                    message: format!(
                        "Parameter '{}' uses Any type which bypasses type checking",
                        input.name
                    ),
                    location: location.clone(),
                    suggestion: "Use specific type instead of Any".into(),
                });
            }
        }

        // Check for missing or empty output definitions
        if interface.outputs.is_empty() {
            findings.push(Finding {
                severity: Severity::Low,
                rule: "interface-boundary".into(),
                message: "Interface has no output definitions".into(),
                location: "interface.outputs".into(),
                suggestion: "Define expected outputs for better documentation and validation".into(),
            });
        }

        Ok(findings)
    }
}
