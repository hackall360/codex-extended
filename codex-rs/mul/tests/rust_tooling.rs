use codex_mul::tooling::rust::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["cargo", "build"]);
    assert_eq!(Adapter::test().unwrap(), vec!["cargo", "test"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["cargo", "clippy"]);
    assert_eq!(Adapter::run().unwrap(), vec!["cargo", "run"]);
}
