// backend/src/rules — Country-Specific Rules Engine.
//
// IDV acceptance policy varies by issuing country: confidence floors,
// permitted document types, mandatory fields, and (for NQM-aligned
// deployments such as India) whether a post-quantum signature is mandatory.
//
// Rulepacks are keyed by ISO 3166-1 alpha-3 with alpha-2 aliases, with a
// DEFAULT fallback for unlisted countries. Defaults ship embedded in the
// binary; operators may replace them at startup by pointing
// AJNA_COUNTRY_RULES_PATH at a JSON document with the same shape — rules
// are configuration, not code.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const EMBEDDED_RULES: &str = include_str!("country_rules.json");
const DEFAULT_PACK_NAME: &str = "DEFAULT";

/// Acceptance thresholds for one country (or the DEFAULT fallback).
#[derive(Debug, Clone, Deserialize)]
pub struct CountryRulePack {
    #[serde(default)]
    pub aliases: Vec<String>,
    pub min_confidence: f32,
    pub allowed_document_types: Vec<String>,
    pub required_fields: Vec<String>,
    /// NQM alignment: when true, only post-quantum algorithms (ml-dsa-*)
    /// are accepted for results issued by this country.
    pub pqc_required: bool,
}

#[derive(Debug, Deserialize)]
struct RulesDocument {
    default: CountryRulePack,
    countries: HashMap<String, CountryRulePack>,
}

/// Outcome of applying a rulepack to one verification. Serialized into the
/// verify response and the audit trail.
#[derive(Debug, Clone, Serialize)]
pub struct RuleOutcome {
    /// Which rulepack was applied (alpha-3 code or "DEFAULT").
    pub rule_pack: String,
    pub passed: bool,
    pub violations: Vec<String>,
}

#[derive(Debug)]
pub struct RulesEngine {
    default: CountryRulePack,
    /// Alpha-3 codes and their alias spellings, all uppercase.
    packs: HashMap<String, (String, CountryRulePack)>,
}

impl RulesEngine {
    /// Engine with the embedded default rulepacks.
    pub fn from_embedded() -> Self {
        Self::from_json(EMBEDDED_RULES).expect("embedded country_rules.json must be valid")
    }

    /// Engine from an operator-supplied JSON document.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let document: RulesDocument = serde_json::from_str(json)?;
        let mut packs = HashMap::new();
        for (code, pack) in document.countries {
            let canonical = code.to_ascii_uppercase();
            for alias in &pack.aliases {
                packs.insert(
                    alias.to_ascii_uppercase(),
                    (canonical.clone(), pack.clone()),
                );
            }
            packs.insert(canonical.clone(), (canonical, pack));
        }
        Ok(Self { default: document.default, packs })
    }

    /// Engine honoring `AJNA_COUNTRY_RULES_PATH` when set, embedded otherwise.
    pub fn from_env() -> Self {
        match std::env::var("AJNA_COUNTRY_RULES_PATH") {
            Ok(path) => match std::fs::read_to_string(&path) {
                Ok(json) => match Self::from_json(&json) {
                    Ok(engine) => {
                        tracing::info!(path, "loaded country rules override");
                        engine
                    }
                    Err(e) => {
                        tracing::error!(path, error = %e, "invalid country rules override; using embedded defaults");
                        Self::from_embedded()
                    }
                },
                Err(e) => {
                    tracing::error!(path, error = %e, "cannot read country rules override; using embedded defaults");
                    Self::from_embedded()
                }
            },
            Err(_) => Self::from_embedded(),
        }
    }

    /// Resolve the rulepack for an ISO country code (alpha-2 or alpha-3,
    /// any case). Unknown codes get the DEFAULT pack.
    pub fn resolve(&self, country_code: &str) -> (&str, &CountryRulePack) {
        match self.packs.get(&country_code.to_ascii_uppercase()) {
            Some((canonical, pack)) => (canonical.as_str(), pack),
            None => (DEFAULT_PACK_NAME, &self.default),
        }
    }

    /// Apply the country's rulepack to one verified scan.
    pub fn evaluate(
        &self,
        country_code: &str,
        document_type: &str,
        confidence: f32,
        field_keys: &[&str],
        algorithm: &str,
    ) -> RuleOutcome {
        let (rule_pack, pack) = self.resolve(country_code);
        let mut violations = Vec::new();

        if confidence < pack.min_confidence {
            violations.push(format!(
                "confidence {confidence:.2} below {rule_pack} minimum {:.2}",
                pack.min_confidence
            ));
        }
        if !pack
            .allowed_document_types
            .iter()
            .any(|allowed| allowed == document_type)
        {
            violations.push(format!(
                "document type {document_type:?} not accepted for {rule_pack}"
            ));
        }
        for required in &pack.required_fields {
            if !field_keys.contains(&required.as_str()) {
                violations.push(format!("required field {required:?} missing"));
            }
        }
        if pack.pqc_required && !algorithm.starts_with("ml-dsa") {
            violations.push(format!(
                "{rule_pack} requires a post-quantum signature (NQM); got {algorithm:?}"
            ));
        }

        RuleOutcome {
            rule_pack: rule_pack.to_owned(),
            passed: violations.is_empty(),
            violations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> RulesEngine {
        RulesEngine::from_embedded()
    }

    #[test]
    fn unknown_country_falls_back_to_default_pack() {
        let outcome = engine().evaluate("ZZZ", "passport", 0.75, &["document_number"], "ed25519");
        assert_eq!(outcome.rule_pack, "DEFAULT");
        assert!(outcome.passed, "violations: {:?}", outcome.violations);
    }

    #[test]
    fn alpha2_alias_resolves_to_alpha3_pack() {
        let engine = engine();
        let (code, pack) = engine.resolve("in");
        assert_eq!(code, "IND");
        assert!(pack.pqc_required);
    }

    #[test]
    fn india_requires_pqc_signature_per_nqm() {
        let outcome = engine().evaluate(
            "IND",
            "passport",
            0.95,
            &["document_number", "date_of_birth"],
            "ed25519",
        );
        assert!(!outcome.passed);
        assert!(outcome.violations[0].contains("post-quantum"), "{:?}", outcome.violations);

        let outcome = engine().evaluate(
            "IND",
            "passport",
            0.95,
            &["document_number", "date_of_birth"],
            "ml-dsa-65",
        );
        assert!(outcome.passed, "violations: {:?}", outcome.violations);
    }

    #[test]
    fn confidence_threshold_is_country_specific() {
        // 0.82 clears the DEFAULT floor (0.7) but not Germany's (0.85).
        let default_outcome =
            engine().evaluate("ZZZ", "passport", 0.82, &["document_number"], "ed25519");
        assert!(default_outcome.passed);

        let german_outcome = engine().evaluate(
            "DEU",
            "passport",
            0.82,
            &["document_number", "date_of_birth"],
            "ed25519",
        );
        assert!(!german_outcome.passed);
        assert!(german_outcome.violations[0].contains("confidence"));
    }

    #[test]
    fn disallowed_document_type_is_a_violation() {
        let outcome = engine().evaluate(
            "SGP",
            "driving_license",
            0.95,
            &["document_number", "date_of_birth"],
            "ed25519",
        );
        assert!(!outcome.passed);
        assert!(outcome.violations[0].contains("not accepted"));
    }

    #[test]
    fn missing_required_field_is_a_violation() {
        let outcome = engine().evaluate("USA", "passport", 0.95, &["document_number"], "ed25519");
        assert!(!outcome.passed);
        assert!(outcome.violations[0].contains("date_of_birth"));
    }

    #[test]
    fn operator_override_document_replaces_embedded_rules() {
        let json = r#"{
            "default": {
                "min_confidence": 0.5,
                "allowed_document_types": ["passport"],
                "required_fields": [],
                "pqc_required": false
            },
            "countries": {}
        }"#;
        let engine = RulesEngine::from_json(json).expect("override must parse");
        let outcome = engine.evaluate("IND", "passport", 0.6, &[], "ed25519");
        assert_eq!(outcome.rule_pack, "DEFAULT");
        assert!(outcome.passed);
    }
}
