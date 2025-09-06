use codex_mul::tooling::objectivec::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["clang"]);
    assert_eq!(Adapter::test().unwrap(), vec!["xctest"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["clang-tidy"]);
    assert_eq!(Adapter::run().unwrap(), vec!["./a.out"]);
}
