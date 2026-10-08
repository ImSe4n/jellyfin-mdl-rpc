use crate::VERSION;
use colored::Colorize;
use log::warn;

const LATEST_RELEASE: &str = "https://github.com/ImSe4n/jellyfin-mdl-rpc/releases/latest";
const TAG_PREFIX: &str = "https://github.com/ImSe4n/jellyfin-mdl-rpc/releases/tag/";

pub fn checker() {
    let current = VERSION.unwrap_or("0.0.0").to_string();
    let latest = get_latest_github().unwrap_or(current.clone());
    if latest != current {
        warn!(
            "{} (Current: v{}, Latest: v{})",
            "You are not running the latest version of Jellyfin-MDL-RPC"
                .red()
                .bold(),
            current,
            latest,
        );
        warn!("{}", "A newer version can be found at".red().bold());
        warn!("{}", LATEST_RELEASE.green().bold());
        warn!(
            "{}",
            "This can be safely ignored if you are running a prerelease version".bold()
        );
    }
}

/// `None` when there is no release yet: GitHub then redirects to the releases page
/// instead of a tag.
fn get_latest_github() -> Option<String> {
    let response = reqwest::blocking::get(LATEST_RELEASE).ok()?;
    latest_tag(response.url().as_str())
}

fn latest_tag(redirected_url: &str) -> Option<String> {
    redirected_url
        .strip_prefix(TAG_PREFIX)
        .map(|tag| tag.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_is_read_from_the_release_redirect() {
        let url = format!("{TAG_PREFIX}1.4.0");
        assert_eq!(latest_tag(&url), Some("1.4.0".to_string()));
    }

    #[test]
    fn no_releases_yet_means_no_update_warning() {
        let url = "https://github.com/ImSe4n/jellyfin-mdl-rpc/releases";
        assert_eq!(latest_tag(url), None);
    }
}
