//! Complexity analyzer for skill selection
//!
//! Analyzes user messages to determine complexity and guide progressive
//! disclosure of skills and tools.

use std::collections::HashSet;

/// Complexity analyzer for determining task complexity
///
/// Analyzes user messages to estimate complexity on a scale from 0.0 to 1.0,
/// where higher values indicate more complex tasks requiring more powerful skills.
#[derive(Debug, Clone, Default)]
pub struct ComplexityAnalyzer {
    /// Keywords that increase complexity score
    complex_keywords: HashSet<String>,
    /// Keywords that decrease complexity score
    simple_keywords: HashSet<String>,
}

impl ComplexityAnalyzer {
    /// Create a new complexity analyzer with default keywords
    pub fn new() -> Self {
        let mut analyzer = Self::default();
        analyzer.load_default_keywords();
        analyzer
    }

    /// Load default complexity keywords
    fn load_default_keywords(&mut self) {
        // Complex task indicators
        for kw in &[
            "architecture",
            "design",
            "implement",
            "refactor",
            "optimize",
            "analyze",
            "integrate",
            "debug",
            "troubleshoot",
            "configure",
            "deploy",
            "migration",
            "security",
            "authentication",
            "authorization",
            "database",
            "api",
            "webhook",
            "microservice",
            "distributed",
            "concurrent",
            "async",
            "performance",
            "scalability",
            "testing",
            "monitoring",
            "telemetry",
            "pipeline",
            "workflow",
            "automation",
            "orchestration",
            "container",
            "kubernetes",
            "docker",
            "infrastructure",
            "provisioning",
        ] {
            self.complex_keywords.insert(kw.to_string());
        }

        // Simple task indicators
        for kw in &[
            "hello", "hi", "help", "list", "show", "what", "how", "explain", "describe", "tell",
            "version", "status", "check", "simple", "basic", "quick", "easy",
        ] {
            self.simple_keywords.insert(kw.to_string());
        }
    }

    /// Add a custom complex keyword
    pub fn add_complex_keyword(&mut self, keyword: String) {
        self.complex_keywords.insert(keyword.to_lowercase());
    }

    /// Add a custom simple keyword
    pub fn add_simple_keyword(&mut self, keyword: String) {
        self.simple_keywords.insert(keyword.to_lowercase());
    }

    /// Analyze a message and return complexity score (0.0 to 1.0)
    ///
    /// Factors that increase complexity:
    /// - Presence of complex keywords (+0.15 each)
    /// - Code snippets (detected by backticks or indentation)
    /// - Multiple questions (detected by '?' count)
    /// - Message length
    ///
    /// Factors that decrease complexity:
    /// - Presence of simple keywords (-0.1 each)
    pub fn analyze(&self, message: &str) -> f32 {
        let mut score = 0.3; // Base complexity
        let lower = message.to_lowercase();

        // Count keyword matches
        for keyword in &self.complex_keywords {
            if lower.contains(keyword) {
                score += 0.15;
            }
        }

        for keyword in &self.simple_keywords {
            if lower.contains(keyword) {
                score -= 0.1;
            }
        }

        // Check for code snippets
        if message.contains("```") || message.contains("`") {
            score += 0.2;
        }

        // Check for multiple questions
        let question_count = lower.matches('?').count();
        if question_count > 1 {
            score += 0.1 * (question_count - 1) as f32;
        }

        // Length factor (very long messages are more complex)
        if message.len() > 500 {
            score += 0.1;
        }

        // Clamp between 0.0 and 1.0
        score.clamp(0.0, 1.0)
    }

    /// Get complexity level from score
    ///
    /// Returns 1-5 where:
    /// - 1: Very simple (0.0 - 0.2)
    /// - 2: Simple (0.2 - 0.4)
    /// - 3: Moderate (0.4 - 0.6)
    /// - 4: Complex (0.6 - 0.8)
    /// - 5: Very complex (0.8 - 1.0)
    pub fn complexity_level(&self, score: f32) -> u8 {
        match score {
            s if s <= 0.2 => 1,
            s if s <= 0.4 => 2,
            s if s <= 0.6 => 3,
            s if s <= 0.8 => 4,
            _ => 5,
        }
    }

    /// Analyze and return both score and level
    pub fn analyze_with_level(&self, message: &str) -> (f32, u8) {
        let score = self.analyze(message);
        let level = self.complexity_level(score);
        (score, level)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_new() {
        let analyzer = ComplexityAnalyzer::new();
        assert!(!analyzer.complex_keywords.is_empty());
        assert!(!analyzer.simple_keywords.is_empty());
    }

    #[test]
    fn test_default_complex_keywords() {
        let analyzer = ComplexityAnalyzer::new();
        assert!(analyzer.complex_keywords.contains("architecture"));
        assert!(analyzer.complex_keywords.contains("implement"));
    }

    #[test]
    fn test_default_simple_keywords() {
        let analyzer = ComplexityAnalyzer::new();
        assert!(analyzer.simple_keywords.contains("hello"));
        assert!(analyzer.simple_keywords.contains("help"));
    }

    #[test]
    fn test_add_custom_complex_keyword() {
        let mut analyzer = ComplexityAnalyzer::new();
        analyzer.add_complex_keyword("custom_complex".to_string());
        assert!(analyzer.complex_keywords.contains("custom_complex"));
    }

    #[test]
    fn test_add_custom_simple_keyword() {
        let mut analyzer = ComplexityAnalyzer::new();
        analyzer.add_simple_keyword("custom_simple".to_string());
        assert!(analyzer.simple_keywords.contains("custom_simple"));
    }

    #[test]
    fn test_analyze_simple_message() {
        let analyzer = ComplexityAnalyzer::new();
        let score = analyzer.analyze("hello help me please");
        assert!(score < 0.3);
    }

    #[test]
    fn test_analyze_complex_message() {
        let analyzer = ComplexityAnalyzer::new();
        let score = analyzer.analyze("help me design and implement a secure authentication system");
        assert!(score > 0.4);
    }

    #[test]
    fn test_analyze_with_code_snippet() {
        let analyzer = ComplexityAnalyzer::new();
        let without_code = analyzer.analyze("show me a function");
        let with_code = analyzer.analyze("show me a ```fn example()```");
        assert!(with_code > without_code);
    }

    #[test]
    fn test_analyze_multiple_questions() {
        let analyzer = ComplexityAnalyzer::new();
        let single = analyzer.analyze("what is this?");
        let multiple = analyzer.analyze("what is this? how does it work? why use it?");
        assert!(multiple > single);
    }

    #[test]
    fn test_analyze_long_message() {
        let analyzer = ComplexityAnalyzer::new();
        let short = analyzer.analyze("help me implement");
        let long = "help me implement ".repeat(100);
        let long_score = analyzer.analyze(&long);
        assert!(long_score > short);
    }

    #[test]
    fn test_complexity_level_1() {
        let analyzer = ComplexityAnalyzer::new();
        assert_eq!(analyzer.complexity_level(0.1), 1);
        assert_eq!(analyzer.complexity_level(0.0), 1);
    }

    #[test]
    fn test_complexity_level_2() {
        let analyzer = ComplexityAnalyzer::new();
        assert_eq!(analyzer.complexity_level(0.3), 2);
    }

    #[test]
    fn test_complexity_level_3() {
        let analyzer = ComplexityAnalyzer::new();
        assert_eq!(analyzer.complexity_level(0.5), 3);
    }

    #[test]
    fn test_complexity_level_4() {
        let analyzer = ComplexityAnalyzer::new();
        assert_eq!(analyzer.complexity_level(0.7), 4);
    }

    #[test]
    fn test_complexity_level_5() {
        let analyzer = ComplexityAnalyzer::new();
        assert_eq!(analyzer.complexity_level(0.9), 5);
        assert_eq!(analyzer.complexity_level(1.0), 5);
    }

    #[test]
    fn test_analyze_with_level() {
        let analyzer = ComplexityAnalyzer::new();
        let (score, level) = analyzer.analyze_with_level("implement secure auth");
        assert!(score > 0.0);
        assert!(level >= 1 && level <= 5);
    }

    #[test]
    fn test_score_clamping() {
        let analyzer = ComplexityAnalyzer::new();
        // Very complex message
        let very_complex = "implement design architecture refactor optimize analyze integrate debug troubleshoot configure deploy migration security authentication authorization database api webhook microservice distributed concurrent async performance scalability testing monitoring telemetry pipeline workflow automation orchestration container kubernetes docker infrastructure provisioning";
        let score = analyzer.analyze(very_complex);
        assert!(score <= 1.0);
    }

    #[test]
    fn test_case_insensitive() {
        let analyzer = ComplexityAnalyzer::new();
        let lower = analyzer.analyze("help me IMPLEMENT this");
        let upper = analyzer.analyze("HELP ME implement THIS");
        // Scores should be similar (not exact due to base complexity)
        assert!((lower - upper).abs() < 0.01);
    }
}
