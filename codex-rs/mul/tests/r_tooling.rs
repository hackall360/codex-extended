use codex_mul::tooling::r::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["R", "CMD", "INSTALL"]);
    assert_eq!(
        Adapter::test().unwrap(),
        vec!["R", "-e", "testthat::test_dir('.')"]
    );
    assert_eq!(Adapter::lint().unwrap(), vec!["lintr"]);
    assert_eq!(Adapter::run().unwrap(), vec!["Rscript"]);
}
