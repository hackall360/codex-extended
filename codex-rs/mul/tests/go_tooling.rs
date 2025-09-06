use codex_mul::tooling::go::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["go", "build"]);
    assert_eq!(Adapter::test().unwrap(), vec!["go", "test"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["golangci-lint", "run"]);
    assert_eq!(Adapter::run().unwrap(), vec!["go", "run"]);
}
