#![allow(clippy::multiple_crate_versions)] // TODO: Remove this when `directories` crate is updated to 3.0.2
use directories::ProjectDirs;
/// Use `tiny_update_notifier::run_notifier(pkg_version, pkg_name, pkg_repo_url)`
/// spawns a new thread to check for updates and notify user if there is a new version available.
///
/// ## Examples
///
/// ```rust,no_run
/// tiny_update_notifier::run_notifier(
///     env!("CARGO_PKG_VERSION"),
///     env!("CARGO_PKG_NAME"),
///     env!("CARGO_PKG_REPOSITORY"),
/// );
/// ```
use notify_rust::Notification;
use std::{
    fs,
    io::{self, Error, ErrorKind},
    time::Duration,
};

/// Spawns a thread to check for updates and notify user if there is a new version available.
/// 
/// This function returns immediately and does not block the current thread.
/// 
/// ## Examples
///
/// ```rust,no_run
/// tiny_update_notifier::run_notifier(
///     env!("CARGO_PKG_VERSION"),
///     env!("CARGO_PKG_NAME"),
///     env!("CARGO_PKG_REPOSITORY"),
/// );
/// ```
pub fn run_notifier(version: &'static str, name: &'static str, repo_url: &'static str) {
    std::thread::spawn(move || {
        Notifier::new(version, name, repo_url).run();
    });
}

/// Use `tiny_update_notifier::Notifier::new().run(pkg_version, pkg_name, pkg_repo_url)`
/// to check for updates and notify user if there is a new version available.
///
/// ## Examples
///
/// ```rust,no_run
/// std::thread::spawn(|| {
///     tiny_update_notifier::Notifier::new(
///         env!("CARGO_PKG_VERSION"),
///         env!("CARGO_PKG_NAME"),
///         env!("CARGO_PKG_REPOSITORY"),
///     )
///     .run();
/// });
/// ```
pub struct Notifier {
    version: &'static str,
    name: &'static str,
    repo_url: &'static str,
}

impl Notifier {
    /// Use `Notifier::new().run(pkg_version, pkg_name, pkg_repo_url)`
    /// to check for updates and notify user if there is a new version available.
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// std::thread::spawn(|| {
    ///     tiny_update_notifier::Notifier::new(
    ///         env!("CARGO_PKG_VERSION"),
    ///         env!("CARGO_PKG_NAME"),
    ///         env!("CARGO_PKG_REPOSITORY"),
    ///     )
    ///     .run();
    /// });
    /// ```

    #[must_use]
    pub const fn new(version: &'static str, name: &'static str, repo_url: &'static str) -> Self {
        Self {
            version,
            name,
            repo_url,
        }
    }

    pub fn run(&mut self) {
        match Self::should_check_update(self) {
            Err(e) => {
                Self::notification(self, &format!("Error: should_check_update() Failed: \n{e}"));
            }
            Ok(true) => Self::check_version(self),
            Ok(false) => (),
        };
    }

    fn check_version(&mut self) {
        let current_version = self.version;

        if let Ok(new_version) = Self::get_latest_version(self) {
            if new_version != current_version {
                Self::notification(
                    self,
                    &format!(
                        "A new release of {pkg_name} is available: \n\
        v{current_version} -> v{new_version}\n\
        {repo_url}/releases/tag/{new_version}",
                        pkg_name = self.name,
                        repo_url = self.repo_url
                    ),
                );
            }

            Self::write_last_checked(self).unwrap_or_else(|e| {
                Self::notification(self, &format!("Error: write_last_checked() failed: \n{e}"));
            });
        }
    }

    fn notification(&mut self, body: &str) {
        Notification::new()
            .summary(self.name)
            .body(body)
            .icon("/usr/share/icons/hicolor/256x256/apps/gnome-software.png")
            .timeout(5000)
            .show()
            .ok();
    }

    fn get_latest_version(&mut self) -> Result<String, Error> {
        let repo_url = self.repo_url;
        let data = repo_url.split('/').collect::<Vec<&str>>();
        if data.len() < 5 {
            return Err(Error::new(ErrorKind::InvalidInput, "Invalid repo url"));
        };
        let owner = data[3];
        let repo = data[4];

        let output = std::process::Command::new("curl")
            .arg("--silent")
            .arg(format!(
                "https://api.github.com/repos/{owner}/{repo}/releases/latest"
            ))
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                Ok(stdout
                    .split("\"tag_name\": \"")
                    .nth(1)
                    .unwrap()
                    .split('\"')
                    .next()
                    .unwrap()
                    .trim_start_matches('v')
                    .to_string())
            }
            Err(e) => {
                Self::notification(self, &format!("Error: get_latest_version() failed: \n{e}"));
                Err(e)
            }
        }
    }

    fn should_check_update(&mut self) -> io::Result<bool> {
        let binding = Self::get_cache_dir(self)?;
        let cache_dir = binding.cache_dir();
        if !cache_dir.exists() {
            fs::create_dir_all(cache_dir)?;
        }
        let path = cache_dir.join(format!("{}-last-update-check", self.name));
        if path.exists() {
            let metadata = fs::metadata(path)?;
            let last_modified_diff = metadata.modified()?.elapsed().unwrap_or_default();
            Ok(last_modified_diff > Duration::from_secs(60 * 60 * 24)) // 1 day
        } else {
            Ok(true)
        }
    }

    fn write_last_checked(&mut self) -> io::Result<()> {
        let path = Self::get_cache_dir(self)?
            .cache_dir()
            .join(format!("{}-last-update-check", self.name));
        fs::write(path, "")
    }

    fn get_cache_dir(&mut self) -> io::Result<ProjectDirs> {
        let project_dir = ProjectDirs::from("", "", self.name);
        project_dir
            .ok_or_else(|| io::Error::new(ErrorKind::Other, "Could not get project directory"))
    }
}
