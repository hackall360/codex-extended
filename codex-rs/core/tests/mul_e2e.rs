#![cfg(feature = "mul")]

use codex_mul::{
    MulProgram, MulStatement, MulType,
    langs::{go, java, javascript, python, rust},
    tooling,
};
use insta::assert_snapshot;

fn sample_program() -> MulProgram {
    MulProgram {
        statements: vec![
            MulStatement::Let {
                name: "a".into(),
                value: MulType::Number(2),
            },
            MulStatement::Let {
                name: "b".into(),
                value: MulType::Number(3),
            },
            MulStatement::Mul {
                name: "c".into(),
                left: MulType::Variable("a".into()),
                right: MulType::Variable("b".into()),
            },
        ],
    }
}

#[test]
fn translation_tooling_roundtrip() {
    let program = sample_program();

    // Python
    let src = python::Adapter::to_source(&program).unwrap();
    assert_snapshot!("python", src);
    assert_eq!(program, python::Adapter::from_source(&src).unwrap());
    assert_eq!(
        tooling::python::Adapter::build().unwrap(),
        vec!["pip", "install"]
    );
    assert_eq!(tooling::python::Adapter::test().unwrap(), vec!["pytest"]);
    assert_eq!(tooling::python::Adapter::lint().unwrap(), vec!["flake8"]);
    assert_eq!(tooling::python::Adapter::run().unwrap(), vec!["python"]);

    // JavaScript
    let src = javascript::Adapter::to_source(&program).unwrap();
    assert_snapshot!("javascript", src);
    assert_eq!(program, javascript::Adapter::from_source(&src).unwrap());
    assert_eq!(
        tooling::javascript::Adapter::build().unwrap(),
        vec!["npm", "install"]
    );
    assert_eq!(
        tooling::javascript::Adapter::test().unwrap(),
        vec!["npm", "test"]
    );
    assert_eq!(
        tooling::javascript::Adapter::lint().unwrap(),
        vec!["npm", "run", "lint"]
    );
    assert_eq!(tooling::javascript::Adapter::run().unwrap(), vec!["node"]);

    // Rust
    let src = rust::Adapter::to_source(&program).unwrap();
    assert_snapshot!("rust", src);
    assert_eq!(program, rust::Adapter::from_source(&src).unwrap());
    assert_eq!(
        tooling::rust::Adapter::build().unwrap(),
        vec!["cargo", "build"]
    );
    assert_eq!(
        tooling::rust::Adapter::test().unwrap(),
        vec!["cargo", "test"]
    );
    assert_eq!(
        tooling::rust::Adapter::lint().unwrap(),
        vec!["cargo", "clippy"]
    );
    assert_eq!(tooling::rust::Adapter::run().unwrap(), vec!["cargo", "run"]);

    // Java
    let src = java::Adapter::to_source(&program).unwrap();
    assert_snapshot!("java", src);
    assert_eq!(program, java::Adapter::from_source(&src).unwrap());
    assert_eq!(
        tooling::java::Adapter::build().unwrap(),
        vec!["mvn", "package"]
    );
    assert_eq!(tooling::java::Adapter::test().unwrap(), vec!["mvn", "test"]);
    assert_eq!(
        tooling::java::Adapter::lint().unwrap(),
        vec!["mvn", "checkstyle:check"]
    );
    assert_eq!(tooling::java::Adapter::run().unwrap(), vec!["java"]);

    // Go
    let src = go::Adapter::to_source(&program).unwrap();
    assert_snapshot!("go", src);
    assert_eq!(program, go::Adapter::from_source(&src).unwrap());
    assert_eq!(tooling::go::Adapter::build().unwrap(), vec!["go", "build"]);
    assert_eq!(tooling::go::Adapter::test().unwrap(), vec!["go", "test"]);
    assert_eq!(
        tooling::go::Adapter::lint().unwrap(),
        vec!["golangci-lint", "run"]
    );
    assert_eq!(tooling::go::Adapter::run().unwrap(), vec!["go", "run"]);
}
