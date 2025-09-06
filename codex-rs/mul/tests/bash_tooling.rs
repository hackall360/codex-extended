use codex_mul::tooling::bash::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["bash"]);
    assert_eq!(Adapter::test().unwrap(), vec!["bats"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["shellcheck"]);
    assert_eq!(Adapter::run().unwrap(), vec!["bash"]);
}
