use codex_mul::tooling::csharp::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["dotnet", "build"]);
    assert_eq!(Adapter::test().unwrap(), vec!["dotnet", "test"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["dotnet", "format"]);
    assert_eq!(Adapter::run().unwrap(), vec!["dotnet", "run"]);
}
