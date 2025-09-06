use codex_mul::tooling::swift::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["swift", "build"]);
    assert_eq!(Adapter::test().unwrap(), vec!["swift", "test"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["swiftlint"]);
    assert_eq!(Adapter::run().unwrap(), vec!["swift", "run"]);
}
