use codex_mul::tooling::javascript::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["npm", "install"]);
    assert_eq!(Adapter::test().unwrap(), vec!["npm", "test"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["npm", "run", "lint"]);
    assert_eq!(Adapter::run().unwrap(), vec!["node"]);
}
