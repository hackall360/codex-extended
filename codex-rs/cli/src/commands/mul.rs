use clap::Parser;
use codex_common::CliConfigOverrides;
use codex_mul::{
    MulProgram,
    adapter::{JsonAdapter, MulAdapter},
    error::Result as MulResult,
};
use std::io::{self, Read};

/// Convert MUL source code between languages.
#[derive(Debug, Parser)]
pub struct MulCli {
    #[clap(skip)]
    pub config_overrides: CliConfigOverrides,

    /// Language to parse the input as.
    #[arg(long = "from", value_name = "LANG", required = true)]
    pub from: String,

    /// Language to emit the output as.
    #[arg(long = "to", value_name = "LANG", required = true)]
    pub to: String,
}

pub fn run(cli: MulCli) -> ! {
    let from_fn = get_from_fn(&cli.from).unwrap_or_else(|| {
        eprintln!("unknown --from language: {}", cli.from);
        std::process::exit(1);
    });
    let to_fn = get_to_fn(&cli.to).unwrap_or_else(|| {
        eprintln!("unknown --to language: {}", cli.to);
        std::process::exit(1);
    });

    let mut input_source = String::new();
    if let Err(err) = io::stdin().read_to_string(&mut input_source) {
        eprintln!("failed to read stdin: {err}");
        std::process::exit(1);
    }

    let program = match from_fn(&input_source) {
        Ok(program) => program,
        Err(err) => {
            eprintln!("failed to parse input: {err}");
            std::process::exit(1);
        }
    };

    let output_source = match to_fn(&program) {
        Ok(src) => src,
        Err(err) => {
            eprintln!("failed to serialize output: {err}");
            std::process::exit(1);
        }
    };

    println!("{output_source}");
    std::process::exit(0);
}

type FromFn = fn(&str) -> MulResult<MulProgram>;
type ToFn = fn(&MulProgram) -> MulResult<String>;

mod lang {
    pub use codex_mul::langs::{
        ada, bash, c, clojure, cpp, csharp, dart, elixir, erlang, fortran, fsharp, go, groovy,
        haskell, java, javascript, julia, kotlin, lua, matlab, objectivec, ocaml, perl, php,
        powershell, python, r, ruby, rust, scala, sql, swift, typescript,
    };
}

macro_rules! gen_match {
    ($lang:expr, $method:ident) => {
        match $lang {
            "json" => Some(JsonAdapter::$method),
            "ada" => Some(lang::ada::Adapter::$method),
            "bash" => Some(lang::bash::Adapter::$method),
            "c" => Some(lang::c::Adapter::$method),
            "clojure" => Some(lang::clojure::Adapter::$method),
            "cpp" => Some(lang::cpp::Adapter::$method),
            "csharp" => Some(lang::csharp::Adapter::$method),
            "dart" => Some(lang::dart::Adapter::$method),
            "elixir" => Some(lang::elixir::Adapter::$method),
            "erlang" => Some(lang::erlang::Adapter::$method),
            "fortran" => Some(lang::fortran::Adapter::$method),
            "fsharp" => Some(lang::fsharp::Adapter::$method),
            "go" => Some(lang::go::Adapter::$method),
            "groovy" => Some(lang::groovy::Adapter::$method),
            "haskell" => Some(lang::haskell::Adapter::$method),
            "java" => Some(lang::java::Adapter::$method),
            "javascript" => Some(lang::javascript::Adapter::$method),
            "julia" => Some(lang::julia::Adapter::$method),
            "kotlin" => Some(lang::kotlin::Adapter::$method),
            "lua" => Some(lang::lua::Adapter::$method),
            "matlab" => Some(lang::matlab::Adapter::$method),
            "objectivec" => Some(lang::objectivec::Adapter::$method),
            "ocaml" => Some(lang::ocaml::Adapter::$method),
            "perl" => Some(lang::perl::Adapter::$method),
            "php" => Some(lang::php::Adapter::$method),
            "powershell" => Some(lang::powershell::Adapter::$method),
            "python" => Some(lang::python::Adapter::$method),
            "r" => Some(lang::r::Adapter::$method),
            "ruby" => Some(lang::ruby::Adapter::$method),
            "rust" => Some(lang::rust::Adapter::$method),
            "scala" => Some(lang::scala::Adapter::$method),
            "sql" => Some(lang::sql::Adapter::$method),
            "swift" => Some(lang::swift::Adapter::$method),
            "typescript" => Some(lang::typescript::Adapter::$method),
            _ => None,
        }
    };
}

fn get_from_fn(lang: &str) -> Option<FromFn> {
    gen_match!(lang, from_source)
}

fn get_to_fn(lang: &str) -> Option<ToFn> {
    gen_match!(lang, to_source)
}
