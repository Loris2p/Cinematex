use super::is_valid_email;

#[test]
fn accepts_valid_emails() {
    assert!(is_valid_email("admin@cinematex.fr"));
    assert!(is_valid_email("user.name+tag@sub.example.com"));
    assert!(is_valid_email("a@b.co"));
}

#[test]
fn rejects_invalid_emails() {
    assert!(!is_valid_email(""));
    assert!(!is_valid_email("plainaddress"));
    assert!(!is_valid_email("@example.com"));
    assert!(!is_valid_email("user@"));
    assert!(!is_valid_email("user@localhost"));
    assert!(!is_valid_email("user@.com"));
    assert!(!is_valid_email("user@example."));
    assert!(!is_valid_email("user@exa..mple.com"));
    assert!(!is_valid_email("user@a@b.com"));
    assert!(!is_valid_email("user name@example.com"));
}
