use codex_mul::tooling::dart::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["dart", "pub", "get"]);
    assert_eq!(Adapter::test().unwrap(), vec!["dart", "test"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["dart", "analyze"]);
    assert_eq!(Adapter::run().unwrap(), vec!["dart", "run"]);
}
