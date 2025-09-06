use codex_mul::tooling::sql::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["psql"]);
    assert_eq!(Adapter::test().unwrap(), vec!["psql"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["sqlfluff"]);
    assert_eq!(Adapter::run().unwrap(), vec!["psql"]);
}
