use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

const APP_DIRECTORY_NAME: &str = "starnote";
const LEGACY_APP_DIRECTORY_NAME: &str = "castle";
const DATABASE_FILE_NAME: &str = "starnote.db";
const LEGACY_DATABASE_FILE_NAME: &str = "castle.db";

pub struct AppPaths {
    pub data_dir: PathBuf,
    pub database_url: String,
}

impl AppPaths {
    pub fn discover() -> Result<Self> {
        #[cfg(debug_assertions)]
        if let Ok(database_url) = env::var("DATABASE_URL")
            && !database_url.trim().is_empty()
        {
            return Self::from_database_url(database_url);
        }

        let data_dir = native_data_dir()?;
        let database_url = database_url_for_path(&resolve_database_path(&data_dir));

        Ok(Self {
            data_dir,
            database_url,
        })
    }

    #[cfg(debug_assertions)]
    fn from_database_url(database_url: String) -> Result<Self> {
        let database_path = database_path_from_url(&database_url)?;
        let data_dir = database_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        Ok(Self {
            data_dir,
            database_url,
        })
    }

    pub fn database_path(&self) -> Result<PathBuf> {
        database_path_from_url(&self.database_url)
    }
}

fn native_data_dir() -> Result<PathBuf> {
    native_data_dir_from(|name| env::var_os(name))
}

fn native_data_dir_from(get_env: impl Fn(&str) -> Option<OsString>) -> Result<PathBuf> {
    let base_dir = if let Some(data_home) = get_env("XDG_DATA_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
    {
        data_home
    } else {
        let home = get_env("HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .context("HOME is not available; StarNote cannot determine its data directory")?;
        home.join(".local").join("share")
    };

    let target_dir = base_dir.join(APP_DIRECTORY_NAME);
    if !target_dir.exists() {
        let legacy_dir = base_dir.join(LEGACY_APP_DIRECTORY_NAME);
        if legacy_dir.exists() {
            return Ok(legacy_dir);
        }
    }

    Ok(target_dir)
}

fn resolve_database_path(data_dir: &Path) -> PathBuf {
    let target_db = data_dir.join(DATABASE_FILE_NAME);
    if !target_db.exists() {
        let legacy_db = data_dir.join(LEGACY_DATABASE_FILE_NAME);
        if legacy_db.exists() {
            return legacy_db;
        }
    }
    target_db
}

fn database_url_for_path(path: &Path) -> String {
    format!("sqlite:{}", path.to_string_lossy())
}

fn database_path_from_url(database_url: &str) -> Result<PathBuf> {
    let path = database_url
        .strip_prefix("sqlite:")
        .context("DATABASE_URL must use the sqlite: scheme")?;

    if path.is_empty() || path == ":memory:" {
        bail!("DATABASE_URL must point to a SQLite database file");
    }

    Ok(PathBuf::from(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_override_keeps_all_app_data_beside_the_database() -> Result<()> {
        let paths = AppPaths::from_database_url("sqlite:custom/location/starnote.db".to_string())?;

        assert_eq!(paths.data_dir, PathBuf::from("custom/location"));
        assert_eq!(
            paths.database_path()?,
            PathBuf::from("custom/location/starnote.db")
        );

        Ok(())
    }

    #[test]
    fn database_paths_preserve_url_characters_in_directory_names() -> Result<()> {
        let paths = AppPaths::from_database_url("sqlite:custom/#notes/starnote.db".to_string())?;

        assert_eq!(paths.data_dir, PathBuf::from("custom/#notes"));
        assert_eq!(
            paths.database_path()?,
            PathBuf::from("custom/#notes/starnote.db")
        );

        Ok(())
    }

    #[test]
    fn linux_data_prefers_xdg_data_home() -> Result<()> {
        let path = native_data_dir_from(|name| {
            (name == "XDG_DATA_HOME").then(|| OsString::from("/home/ada/data"))
        })?;

        assert_eq!(path, PathBuf::from("/home/ada/data/starnote"));
        Ok(())
    }

    #[test]
    fn linux_data_falls_back_to_the_home_directory() -> Result<()> {
        let path =
            native_data_dir_from(|name| (name == "HOME").then(|| OsString::from("/home/ada")))?;

        assert_eq!(path, PathBuf::from("/home/ada/.local/share/starnote"));
        Ok(())
    }
}
