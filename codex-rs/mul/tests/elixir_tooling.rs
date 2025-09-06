use codex_mul::tooling::elixir::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["mix", "compile"]);
    assert_eq!(Adapter::test().unwrap(), vec!["mix", "test"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["mix", "format"]);
    assert_eq!(Adapter::run().unwrap(), vec!["mix", "run"]);
}
