use codex_mul::tooling::clojure::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["lein"]);
    assert_eq!(Adapter::test().unwrap(), vec!["lein"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["clj-kondo"]);
    assert_eq!(Adapter::run().unwrap(), vec!["clojure"]);
}
