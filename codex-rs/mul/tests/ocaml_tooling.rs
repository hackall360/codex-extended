use codex_mul::tooling::ocaml::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["dune", "build"]);
    assert_eq!(Adapter::test().unwrap(), vec!["dune", "test"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["ocamlformat"]);
    assert_eq!(Adapter::run().unwrap(), vec!["dune", "exec"]);
}
