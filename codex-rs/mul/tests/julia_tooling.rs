use codex_mul::tooling::julia::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["julia"]);
    assert_eq!(Adapter::test().unwrap(), vec!["julia"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["julia"]);
    assert_eq!(Adapter::run().unwrap(), vec!["julia"]);
}
