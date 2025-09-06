use codex_mul::tooling::python::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["pip", "install"]);
    assert_eq!(Adapter::test().unwrap(), vec!["pytest"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["flake8"]);
    assert_eq!(Adapter::run().unwrap(), vec!["python"]);
}
