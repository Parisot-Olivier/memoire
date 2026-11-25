//! Exécutable `memoire` : réunit arguments, environnement, source d'identités et flux, puis délègue
//! à `memoire_cli::executer` (CLI.md, section 6).

use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use memoire_app::RandomIds;
use memoire_cli::{executer, Environnement};

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let environnement = Environnement {
        base: std::env::var_os("MEMOIRE_BASE").map(PathBuf::from),
        repertoire: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };
    let code = executer(
        &arguments,
        &environnement,
        Box::new(RandomIds),
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    );
    ExitCode::from(u8::try_from(code).unwrap_or(1))
}
