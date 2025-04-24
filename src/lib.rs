//!
//! git-blame-pr
//!

use std::collections::HashMap;

//-----------------------------------------------------------------------------
// git command wrapper
//-----------------------------------------------------------------------------
#[derive(Debug, Default)]
pub struct GitLog {
    pub commit: String,
    pub title_line: String,
}

fn git_log(commit_id: &str) -> Result<GitLog, String> {
    let output = std::process::Command::new("git")
        .args(["log", "-1", "--oneline"])
        .arg(commit_id)
        .output()
        .unwrap();

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("failed to execute `git log {commit_id}`: {err}"));
    }

    let raw_log = String::from_utf8_lossy(&output.stdout).to_string();
    if raw_log.is_empty() {
        return Err(format!("missing log: `git log {commit_id}`"));
    }

    match raw_log.split_once(' ') {
        Some((commit, title_line)) => Ok(GitLog {
            commit: commit.to_string(),
            title_line: title_line.to_string(),
        }),

        // --allow-empty-message
        None => Ok(GitLog {
            commit: raw_log.to_string(),
            title_line: String::new(),
        }),
    }
}

fn git_blame(path: &std::path::Path) -> Result<Vec<git_blame_parser::Blame>, String> {
    let output = std::process::Command::new("git")
        .args(["blame", "--first-parent", "--line-porcelain"])
        .arg(path)
        .output()
        .unwrap();

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "failed to execute `git blame {}`: {err}",
            path.display()
        ));
    }

    let raw_blame = String::from_utf8_lossy(&output.stdout);
    match git_blame_parser::parse(&raw_blame) {
        Ok(blames) => Ok(blames),
        Err(e) => Err(format!("{e}")),
    }
}

//-----------------------------------------------------------------------------
// git-blame-pr
//-----------------------------------------------------------------------------
pub struct Config {
    pub filepath: std::path::PathBuf,
}

impl Config {
    pub fn build(args: &[String]) -> Result<Config, String> {
        if args.len() < 2 {
            return Err(String::from("missing args: <filepath>"));
        }

        let filepath = std::path::PathBuf::from(args[1].clone());
        if !filepath.is_file() {
            return Err(String::from("invalid file path"));
        }

        Ok(Config { filepath })
    }
}

pub fn lookup_pr(commit: &str) -> Option<String> {
    let log = match git_log(commit) {
        Ok(log) => log,
        Err(_) => return None,
    };

    let re = regex::Regex::new(r"(?i)Merge\s+(?:pull\s+request|pr)\s+#?(\d+)\s").unwrap();

    if let Some(captures) = re.captures(&log.title_line) {
        if let Some(pr) = captures.get(1) {
            return Some(format!("PR#{}", pr.as_str()));
        }
    }

    None
}

pub fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let blames = git_blame(&config.filepath)?;
    let line_digits = if blames.is_empty() {
        1
    } else {
        blames.len().ilog10() as usize + 1
    };

    let mut cached = HashMap::new();
    for blame in blames.iter() {
        let idx =
            cached
                .entry(blame.short_commit())
                .or_insert_with(|| match lookup_pr(&blame.commit) {
                    Some(pr) => pr,
                    None => blame.short_commit(),
                });

        println!(
            "{:<8} {:0line_digits$}│ {}",
            idx, blame.final_line_no, blame.content
        );
    }

    Ok(())
}
