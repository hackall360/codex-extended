use codex_mul::tooling::groovy::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["gradle", "build"]);
    assert_eq!(Adapter::test().unwrap(), vec!["gradle", "test"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["codenarc"]);
    assert_eq!(Adapter::run().unwrap(), vec!["groovy"]);
}
