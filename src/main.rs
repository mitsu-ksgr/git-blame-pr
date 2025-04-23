//!
//! git-blame-pr
//!

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let config = git_blame_pr::Config::build(&args).unwrap_or_else(|err| {
        eprintln!("Error: {err}");
        std::process::exit(1);
    });

    if let Err(err) = git_blame_pr::run(config) {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
