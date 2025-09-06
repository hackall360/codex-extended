use codex_mul::tooling::c::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["gcc"]);
    assert_eq!(Adapter::test().unwrap(), vec!["ctest"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["clang-tidy"]);
    assert_eq!(Adapter::run().unwrap(), vec!["./a.out"]);
}
