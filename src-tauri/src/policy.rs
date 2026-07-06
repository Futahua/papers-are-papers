use crate::models::PolicyDecision;
use uuid::Uuid;

pub fn classify(kind: &str, target: &str, payload: &str) -> PolicyDecision {
    let combined = format!("{kind} {target} {payload}").to_lowercase();
    let blocked_terms = [
        "password",
        "passcode",
        "2fa",
        "two-factor",
        "credit card",
        "payment card",
        "seed phrase",
        "private key",
        "permission dialog",
        "administrator prompt",
        "uac",
    ];
    let consequential_terms = [
        "send",
        "submit",
        "publish",
        "purchase",
        "buy",
        "delete",
        "remove",
        "overwrite",
        "replace",
        "install",
        "uninstall",
        "account",
        "security",
        "system setting",
        "registry",
        "move",
        "rename",
        "upload",
        "commit",
        "push",
    ];
    let safe_kinds = [
        "read", "inspect", "capture", "list", "search", "navigate", "launch", "draft",
    ];

    let (decision, risk, reason, reversible) = if blocked_terms
        .iter()
        .any(|term| combined.contains(term))
    {
        (
            "block",
            "blocked",
            "Secrets, authentication, payments, and permission dialogs remain human-only.",
            false,
        )
    } else if consequential_terms
        .iter()
        .any(|term| combined.contains(term))
    {
        (
            "preview",
            "high",
            "This could affect important data, accounts, money, software, or another person.",
            false,
        )
    } else if safe_kinds
        .iter()
        .any(|safe| kind.eq_ignore_ascii_case(safe))
    {
        (
            "allow",
            "low",
            "This is a read-only or easily reversible preparatory action.",
            true,
        )
    } else {
        (
                "preview",
                "medium",
                "Papers could not prove that this action is harmless, so uncertainty requires a preview.",
                false,
            )
    };

    PolicyDecision {
        action_id: Uuid::new_v4().to_string(),
        decision: decision.to_string(),
        risk: risk.to_string(),
        reason: reason.to_string(),
        reversible,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_read_only_work() {
        assert_eq!(classify("read", "notes", "").decision, "allow");
    }

    #[test]
    fn previews_external_effects() {
        assert_eq!(
            classify("click", "Send email", "send message").decision,
            "preview"
        );
    }

    #[test]
    fn blocks_credentials() {
        assert_eq!(
            classify("type", "password field", "secret").decision,
            "block"
        );
    }
}
