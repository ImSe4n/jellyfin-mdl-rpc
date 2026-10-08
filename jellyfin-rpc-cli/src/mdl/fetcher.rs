//! Works out how to run the MyDramaList fetch: through a Python interpreter (a
//! source checkout) or the bundled `mdl_fetch` executable (the Windows installer).

use std::path::{Path, PathBuf};
use std::process::Command;

/// Name of the bundled fetcher, which the installer puts next to `jellyfin-rpc`.
pub const BUNDLED_FETCHER: &str = if cfg!(windows) {
    "mdl_fetch.exe"
} else {
    "mdl_fetch"
};
/// Stats file name used when `mdl.stats_file` is left out; it sits beside the config.
pub const DEFAULT_STATS_FILE: &str = "mdl_stats.json";

#[derive(Debug, Clone, PartialEq)]
pub enum Fetcher {
    /// `python script <username> <stats_file>`
    Python { python: String, script: String },
    /// `fetcher <username> <stats_file>`
    Exe(PathBuf),
}

impl Fetcher {
    pub fn command(&self) -> Command {
        match self {
            Fetcher::Python { python, script } => {
                let mut command = Command::new(python);
                command.arg(script);
                command
            }
            Fetcher::Exe(path) => Command::new(path),
        }
    }

    /// The program that gets launched, for log messages.
    pub fn program(&self) -> String {
        match self {
            Fetcher::Python { python, .. } => python.clone(),
            Fetcher::Exe(path) => path.display().to_string(),
        }
    }
}

/// `python` + `script` win when both are set, then an explicit `fetcher`, then the
/// bundled fetcher next to the running executable.
pub fn resolve(
    python: Option<String>,
    script: Option<String>,
    fetcher: Option<String>,
    exe_dir: Option<&Path>,
) -> Result<Fetcher, String> {
    match (python, script, fetcher) {
        (Some(python), Some(script), _) => Ok(Fetcher::Python { python, script }),
        (Some(_), None, _) | (None, Some(_), _) => {
            Err("set both mdl.python and mdl.script, or neither".to_string())
        }
        (None, None, Some(fetcher)) => Ok(Fetcher::Exe(PathBuf::from(fetcher))),
        (None, None, None) => exe_dir
            .map(|dir| Fetcher::Exe(dir.join(BUNDLED_FETCHER)))
            .ok_or_else(|| "could not find the folder jellyfin-rpc runs from".to_string()),
    }
}

/// `mdl.stats_file`, or `mdl_stats.json` in the same folder as the config file.
pub fn stats_file(configured: Option<String>, config_path: &str) -> String {
    configured.unwrap_or_else(|| {
        Path::new(config_path)
            .with_file_name(DEFAULT_STATS_FILE)
            .display()
            .to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn some(text: &str) -> Option<String> {
        Some(text.to_string())
    }

    #[test]
    fn python_and_script_are_used_when_both_set() {
        let fetcher = resolve(some("py"), some("mdl_fetch.py"), some("other"), None);
        assert_eq!(
            fetcher,
            Ok(Fetcher::Python {
                python: "py".to_string(),
                script: "mdl_fetch.py".to_string()
            })
        );
    }

    #[test]
    fn only_one_of_python_and_script_is_an_error() {
        assert!(resolve(some("py"), None, None, Some(Path::new("dir"))).is_err());
        assert!(resolve(None, some("mdl_fetch.py"), None, Some(Path::new("dir"))).is_err());
    }

    #[test]
    fn explicit_fetcher_beats_the_bundled_one() {
        let fetcher = resolve(None, None, some("custom_fetch"), Some(Path::new("dir")));
        assert_eq!(fetcher, Ok(Fetcher::Exe(PathBuf::from("custom_fetch"))));
    }

    #[test]
    fn bundled_fetcher_sits_next_to_the_executable() {
        let fetcher = resolve(None, None, None, Some(Path::new("install_dir")));
        assert_eq!(
            fetcher,
            Ok(Fetcher::Exe(Path::new("install_dir").join(BUNDLED_FETCHER)))
        );
    }

    #[test]
    fn no_fetcher_and_no_executable_dir_is_an_error() {
        assert!(resolve(None, None, None, None).is_err());
    }

    #[test]
    fn stats_file_defaults_beside_the_config() {
        let expected = Path::new("conf_dir").join(DEFAULT_STATS_FILE);
        let config_path = Path::new("conf_dir").join("main.json");
        assert_eq!(
            stats_file(None, config_path.to_str().unwrap()),
            expected.display().to_string()
        );
    }

    #[test]
    fn configured_stats_file_is_kept() {
        assert_eq!(stats_file(some("elsewhere.json"), "conf/main.json"), "elsewhere.json");
    }
}
