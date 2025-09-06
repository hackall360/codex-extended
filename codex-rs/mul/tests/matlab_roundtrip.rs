use codex_mul::{MulProgram, MulStatement, MulType, langs::matlab::Adapter};

#[test]
fn roundtrip() {
    let program = MulProgram {
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
    };
    let source = Adapter::to_source(&program).unwrap();
    let parsed = Adapter::from_source(&source).unwrap();
    assert_eq!(program, parsed);
}
