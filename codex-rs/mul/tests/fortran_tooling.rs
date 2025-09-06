use codex_mul::tooling::fortran::Adapter;

#[test]
fn commands() {
    assert_eq!(Adapter::build().unwrap(), vec!["gfortran"]);
    assert_eq!(Adapter::test().unwrap(), vec!["fpm", "test"]);
    assert_eq!(Adapter::lint().unwrap(), vec!["flint"]);
    assert_eq!(Adapter::run().unwrap(), vec!["./a.out"]);
}
