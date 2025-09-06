use codex_mul::tooling::matlab::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["matlab"]);
    assert_eq!(Adapter::test().unwrap(), vec!["matlab"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["matlab"]);
    assert_eq!(Adapter::run().unwrap(), vec!["matlab"]);
}
