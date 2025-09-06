use codex_mul::tooling::lua::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["luarocks", "install"]);
    assert_eq!(Adapter::test().unwrap(), vec!["busted"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["luacheck"]);
    assert_eq!(Adapter::run().unwrap(), vec!["lua"]);
}
