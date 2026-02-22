use super::{CheckError, SecurityRule};
use crate::clawhub::sif::{CodeLogic, Logic, SkillSIF};
use crate::clawhub::security::report::{Finding, Severity};

pub struct CodeExecutionRiskRule;

impl SecurityRule for CodeExecutionRiskRule {
    fn name(&self) -> &str {
        "code-execution-risk"
    }

    fn check(&self, skill: &SkillSIF) -> Result<Vec<Finding>, CheckError> {
        let mut findings = Vec::new();

        // Check code logic type
        if let Some(Logic::Code(code)) = &skill.logic {
            self.check_code(code, "logic.code", &mut findings);
        }

        // Check hybrid pre/post-processing code
        if let Some(Logic::Hybrid(hybrid)) = &skill.logic {
            if let Some(pre) = &hybrid.pre_process {
                self.check_code(pre, "logic.hybrid.pre_process", &mut findings);
            }
            if let Some(post) = &hybrid.post_process {
                self.check_code(post, "logic.hybrid.post_process", &mut findings);
            }
        }

        Ok(findings)
    }
}

impl CodeExecutionRiskRule {
    fn check_code(&self, code: &CodeLogic, location: &str, findings: &mut Vec<Finding>) {
        let source = match &code.source {
            Some(s) => s,
            None => return,
        };

        let lower = source.to_lowercase();

        // Get dangerous patterns based on runtime
        let dangerous_patterns: &[(&str, &str)] = match code.runtime.to_lowercase().as_str() {
            "python" => &[
                ("eval(", "eval() can execute arbitrary code"),
                ("exec(", "exec() can execute arbitrary code"),
                ("compile(", "compile() can create executable code"),
                ("__import__", "__import__ can import arbitrary modules"),
                ("os.system", "os.system executes shell commands"),
                ("subprocess.", "subprocess module executes shell commands"),
                ("os.popen", "os.popen executes shell commands"),
                ("commands.", "commands module executes shell commands"),
                ("pty.spawn", "pty.spawn can spawn shells"),
            ],
            "javascript" | "js" | "node" => &[
                ("eval(", "eval() can execute arbitrary code"),
                ("Function(", "Function constructor can execute arbitrary code"),
                ("setTimeout(", "setTimeout can execute code strings"),
                ("setInterval(", "setInterval can execute code strings"),
                ("require(", "require can load arbitrary modules"),
                ("import(", "dynamic import can load arbitrary modules"),
                ("child_process", "child_process spawns processes"),
                ("process.exec", "process.exec executes shell commands"),
                ("process.spawn", "process.spawn spawns processes"),
            ],
            "ruby" => &[
                ("eval(", "eval() can execute arbitrary code"),
                ("instance_eval", "instance_eval can execute arbitrary code"),
                ("class_eval", "class_eval can execute arbitrary code"),
                ("system(", "system() executes shell commands"),
                ("exec(", "exec() replaces process with command"),
                ("`", "backtick execution executes shell commands"),
                ("%x(", "%x() executes shell commands"),
                ("Open3", "Open3 spawns processes"),
            ],
            "bash" | "sh" | "shell" => &[
                ("eval", "eval executes arbitrary code"),
                ("exec", "exec replaces process with command"),
            ],
            _ => &[],
        };

        for (pattern, desc) in dangerous_patterns {
            if lower.contains(pattern) {
                let severity = if pattern.contains("eval") || pattern.contains("exec") {
                    Severity::Critical
                } else {
                    Severity::High
                };

                findings.push(Finding {
                    severity,
                    rule: "code-execution-risk".into(),
                    message: format!("Dangerous code execution in {}: {} — {}", code.runtime, pattern, desc),
                    location: location.into(),
                    suggestion: "Remove or sandbox dangerous code execution".into(),
                });
            }
        }

        // Check for file operations without proper permissions
        let file_patterns: &[(&str, &str)] = &[
            ("open(", "file open operation"),
            ("file(", "file operation"),
            ("read(", "file read operation"),
            ("write(", "file write operation"),
        ];

        for (pattern, desc) in file_patterns {
            if lower.contains(pattern) {
                findings.push(Finding {
                    severity: Severity::Medium,
                    rule: "code-execution-risk".into(),
                    message: format!("File operation in code: {} — {}", pattern, desc),
                    location: location.into(),
                    suggestion: "Ensure permissions allow file operations".into(),
                });
            }
        }
    }
}
