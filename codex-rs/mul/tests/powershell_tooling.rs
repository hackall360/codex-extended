use codex_mul::tooling::powershell::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["pwsh"]);
    assert_eq!(Adapter::test().unwrap(), vec!["pester"]);
    assert_eq!(
        Adapter::lint().unwrap(),
        vec!["pwsh", "-Command", "Invoke-ScriptAnalyzer"]
    );
    assert_eq!(Adapter::run().unwrap(), vec!["pwsh"]);
}
