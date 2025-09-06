use codex_mul::tooling::scala::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["sbt", "compile"]);
    assert_eq!(Adapter::test().unwrap(), vec!["sbt", "test"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["scalafmt"]);
    assert_eq!(Adapter::run().unwrap(), vec!["sbt", "run"]);
}
