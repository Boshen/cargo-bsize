use std::process::ExitCode;

use cargo_bsize::cargo_bsize_options;

fn main() -> ExitCode {
    let _options = cargo_bsize_options().run();
    ExitCode::SUCCESS
}
