use codex_mul::tooling::java::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["mvn", "package"]);
    assert_eq!(Adapter::test().unwrap(), vec!["mvn", "test"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["mvn", "checkstyle:check"]);
    assert_eq!(Adapter::run().unwrap(), vec!["java"]);
}
