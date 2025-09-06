use codex_mul::tooling::php::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["composer", "install"]);
    assert_eq!(Adapter::test().unwrap(), vec!["phpunit"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["phpcs"]);
    assert_eq!(Adapter::run().unwrap(), vec!["php"]);
}
