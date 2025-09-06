use codex_mul::tooling::kotlin::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["gradle", "build"]);
    assert_eq!(Adapter::test().unwrap(), vec!["gradle", "test"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["ktlint"]);
    assert_eq!(Adapter::run().unwrap(), vec!["kotlin"]);
}
