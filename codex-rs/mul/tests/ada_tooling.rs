use codex_mul::tooling::ada::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["gnatmake"]);
    assert_eq!(Adapter::test().unwrap(), vec!["gnattest"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["gnatpp"]);
    assert_eq!(Adapter::run().unwrap(), vec!["gnat"]);
}
