use memory_graph::redaction;

#[test]
fn test_redacts_aws_access_key() {
    let input = "Use key AKIAIOSFODNN7EXAMPLE for access";
    let result = redaction::redact(input);
    assert!(!result.clean_content.contains("AKIAIOSFODNN7EXAMPLE"));
    assert!(result.clean_content.contains("[REDACTED]"));
    assert!(!result.redactions.is_empty());
}

#[test]
fn test_redacts_generic_api_key_pattern() {
    let input = "api_key = sk-1234567890abcdef1234567890abcdef";
    let result = redaction::redact(input);
    assert!(!result.clean_content.contains("sk-1234567890abcdef1234567890abcdef"));
    assert!(!result.redactions.is_empty());
}

#[test]
fn test_redacts_bearer_token() {
    let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
    let result = redaction::redact(input);
    assert!(!result.clean_content.contains("eyJhbGci"));
    assert!(!result.redactions.is_empty());
}

#[test]
fn test_redacts_connection_string() {
    let input = "DATABASE_URL=postgresql://user:password123@host:5432/db";
    let result = redaction::redact(input);
    assert!(!result.clean_content.contains("password123"));
    assert!(!result.redactions.is_empty());
}

#[test]
fn test_redacts_private_key() {
    let input = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA\n-----END RSA PRIVATE KEY-----";
    let result = redaction::redact(input);
    assert!(!result.clean_content.contains("MIIEpAIBAAKCAQEA"));
    assert!(!result.redactions.is_empty());
}

#[test]
fn test_redacts_email() {
    let input = "Contact jake.grogan@example.com for details";
    let result = redaction::redact(input);
    assert!(!result.clean_content.contains("jake.grogan@example.com"));
    assert!(!result.redactions.is_empty());
}

#[test]
fn test_clean_content_passes_through() {
    let input = "Rust is great for building memory systems";
    let result = redaction::redact(input);
    assert_eq!(result.clean_content, input);
    assert!(result.redactions.is_empty());
}

#[test]
fn test_multiple_redactions() {
    let input = "key=AKIAIOSFODNN7EXAMPLE and email=test@example.com";
    let result = redaction::redact(input);
    assert!(!result.clean_content.contains("AKIAIOSFODNN7EXAMPLE"));
    assert!(!result.clean_content.contains("test@example.com"));
    assert!(result.redactions.len() >= 2);
}

#[test]
fn test_has_secrets_check() {
    assert!(redaction::has_secrets("my key is AKIAIOSFODNN7EXAMPLE"));
    assert!(!redaction::has_secrets("just normal text here"));
}
