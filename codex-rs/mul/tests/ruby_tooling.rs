use codex_mul::tooling::ruby::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["bundle", "install"]);
    assert_eq!(Adapter::test().unwrap(), vec!["rspec"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["rubocop"]);
    assert_eq!(Adapter::run().unwrap(), vec!["ruby"]);
}
