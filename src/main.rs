use std::process::ExitCode;

use cargo_bsize::{CargoBsize, cargo_bsize_options};

fn main() -> ExitCode {
    let options = cargo_bsize_options().run();
    CargoBsize::new(std::io::stdout(), options).run()
}
