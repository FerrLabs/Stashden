use std::path::PathBuf;

use cargo_packager_updater::semver::Version;
use cargo_packager_updater::url::Url;
use cargo_packager_updater::{Config, Update, UpdaterBuilder};

use crate::failure::Failure;

const FEED: &str = "https://github.com/FerrLabs/Stashden/releases/latest/download/latest.json";
const PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDFERTRERThDMTJFQUMxMDgKUldRSXdlb1NqTjdrSFFCWmJJOUg2c3lBVEtpSGFKWk5UdFNlcWxtNzhiMU1GK1BiOTJVazJkOVkK";

pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

pub struct Offered {
    update: Update,
}

impl Offered {
    pub fn version(&self) -> &str {
        &self.update.version
    }

    pub fn notes(&self) -> Option<&str> {
        self.update
            .body
            .as_deref()
            .filter(|notes| !notes.trim().is_empty())
    }
}

fn config() -> Config {
    Config {
        endpoints: vec![Url::parse(FEED).expect("the release feed is a valid url")],
        pubkey: PUBLIC_KEY.to_owned(),
        windows: None,
    }
}

pub fn check() -> Result<Option<Offered>, Failure> {
    let current = Version::parse(CURRENT).expect("the crate version is semver");
    let updater = UpdaterBuilder::new(current, config())
        .build()
        .map_err(|error| error.to_string())?;
    let update = updater.check().map_err(|error| error.to_string())?;
    Ok(update.map(|update| Offered { update }))
}

pub fn install(offered: &Offered) -> Result<PathBuf, Failure> {
    offered
        .update
        .download_and_install()
        .map_err(|error| Failure::from(error.to_string()))?;
    Ok(offered.update.extract_path.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_running_version_and_the_feed_are_well_formed() {
        assert!(Version::parse(CURRENT).is_ok());
        let config = config();
        assert_eq!(config.endpoints[0].as_str(), FEED);
        assert!(
            UpdaterBuilder::new(Version::new(0, 0, 0), config)
                .build()
                .is_ok()
        );
    }
}
