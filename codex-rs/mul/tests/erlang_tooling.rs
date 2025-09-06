use codex_mul::tooling::erlang::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["rebar3", "compile"]);
    assert_eq!(Adapter::test().unwrap(), vec!["rebar3", "eunit"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["rebar3", "dialyzer"]);
    assert_eq!(Adapter::run().unwrap(), vec!["erl"]);
}
