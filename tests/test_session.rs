use memory_graph::session::Session;

#[test]
fn test_session_starts_not_incognito() {
    let session = Session::new("session-001");
    assert!(!session.is_incognito());
}

#[test]
fn test_set_incognito() {
    let mut session = Session::new("session-001");
    session.set_incognito(true);
    assert!(session.is_incognito());
}

#[test]
fn test_disable_incognito() {
    let mut session = Session::new("session-001");
    session.set_incognito(true);
    assert!(session.is_incognito());
    session.set_incognito(false);
    assert!(!session.is_incognito());
}

#[test]
fn test_session_id() {
    let session = Session::new("test-session-42");
    assert_eq!(session.id(), "test-session-42");
}
