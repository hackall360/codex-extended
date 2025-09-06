use codex_mul::tooling::haskell::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["stack", "build"]);
    assert_eq!(Adapter::test().unwrap(), vec!["stack", "test"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["hlint"]);
    assert_eq!(Adapter::run().unwrap(), vec!["stack", "run"]);
}
