use codex_mul::tooling::perl::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["cpanm"]);
    assert_eq!(Adapter::test().unwrap(), vec!["prove"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["perlcritic"]);
    assert_eq!(Adapter::run().unwrap(), vec!["perl"]);
}
