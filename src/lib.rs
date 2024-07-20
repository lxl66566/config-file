#![doc = include_str!("../README.md")]
#![warn(clippy::nursery, clippy::cargo)]

pub mod error;
#[cfg(feature = "xml")]
use std::io::BufReader;
use std::{
    ffi::OsStr,
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
};

pub use error::Result;
use error::{Error, TomlError};
use serde::{de::DeserializeOwned, Serialize};
#[cfg(feature = "toml")]
use toml_crate as toml;

/// Format of configuration file.
#[derive(Debug, Clone, Copy)]
pub enum ConfigFormat {
    Json,
    Toml,
    Xml,
    Yaml,
}

impl ConfigFormat {
    /// Get the [`ConfigType`] from a file extension
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension.to_lowercase().as_str() {
            #[cfg(feature = "json")]
            "json" => Some(Self::Json),
            #[cfg(feature = "toml")]
            "toml" => Some(Self::Toml),
            #[cfg(feature = "xml")]
            "xml" => Some(Self::Xml),
            #[cfg(feature = "yaml")]
            "yaml" | "yml" => Some(Self::Yaml),
            _ => None,
        }
    }

    /// Get the [`ConfigType`] from a path
    pub fn from_path(path: &Path) -> Option<Self> {
        Self::from_extension(path.extension().and_then(OsStr::to_str)?)
    }
}

/// Trait for loading a struct from a configuration file.
/// This trait is automatically implemented when [`serde::Deserialize`] is.
pub trait LoadConfigFile {
    /// Load config from path with specific format, do not use extension to
    /// determine.
    fn load_with_specific_format(path: impl AsRef<Path>, config_type: ConfigFormat) -> Result<Self>
    where
        Self: Sized;
    /// Load config from path
    fn load(path: impl AsRef<Path>) -> Result<Self>
    where
        Self: Sized,
    {
        let path = path.as_ref();
        let config_type = ConfigFormat::from_path(path).ok_or(Error::UnsupportedFormat)?;
        Self::load_with_specific_format(path, config_type)
    }
    /// Load config from path, if not found, use default instead
    fn load_or_default(path: impl AsRef<Path>) -> Result<Self>
    where
        Self: Sized + Default,
    {
        match Self::load(path) {
            Err(error::Error::FileAccess(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                Ok(Self::default())
            }
            other => other,
        }
    }
}

impl<C: DeserializeOwned> LoadConfigFile for C {
    fn load_with_specific_format(path: impl AsRef<Path>, config_type: ConfigFormat) -> Result<Self>
    where
        Self: Sized,
    {
        let path = path.as_ref();
        match config_type {
            #[cfg(feature = "json")]
            ConfigFormat::Json => serde_json::from_reader(open_file(path)?).map_err(Error::Json),
            #[cfg(feature = "toml")]
            ConfigFormat::Toml => Ok(toml::from_str(
                std::fs::read_to_string(path)
                    .map_err(Error::FileAccess)?
                    .as_str(),
            )
            .map_err(TomlError::DeserializationError)?),
            #[cfg(feature = "xml")]
            ConfigFormat::Xml => Ok(quick_xml::de::from_reader(BufReader::new(open_file(
                path,
            )?))?),
            #[cfg(feature = "yaml")]
            ConfigFormat::Yaml => serde_yml::from_reader(open_file(path)?).map_err(Error::Yaml),
            #[allow(unreachable_patterns)]
            _ => Err(Error::UnsupportedFormat),
        }
    }
}

/// Trait for storing a struct into a configuration file.
/// This trait is automatically implemented when [`serde::Serialize`] is.
pub trait StoreConfigFile {
    /// Store config file to path with specific format, do not use extension to
    /// determine.
    fn store_with_specific_format(
        &self,
        path: impl AsRef<Path>,
        config_type: ConfigFormat,
    ) -> Result<()>;
    /// Store config file to path
    fn store(&self, path: impl AsRef<Path>) -> Result<()>
    where
        Self: Sized,
    {
        let path = path.as_ref();
        let config_type = ConfigFormat::from_path(path).ok_or(Error::UnsupportedFormat)?;
        self.store_with_specific_format(path, config_type)
    }
    /// Store config file to path, if path exists, return error
    fn store_without_overwrite(&self, path: impl AsRef<Path>) -> Result<()>
    where
        Self: Sized,
    {
        if path.as_ref().exists() {
            return Err(Error::FileExists);
        }
        self.store(path)
    }
}

impl<C: Serialize> StoreConfigFile for C {
    fn store_with_specific_format(
        &self,
        path: impl AsRef<Path>,
        config_type: ConfigFormat,
    ) -> Result<()> {
        let path = path.as_ref();
        match config_type {
            #[cfg(feature = "json")]
            ConfigFormat::Json => {
                serde_json::to_writer_pretty(open_write_file(path)?, &self).map_err(Error::Json)
            }
            #[cfg(feature = "toml")]
            ConfigFormat::Toml => {
                open_write_file(path)?.write_all(
                    toml::to_string_pretty(&self)
                        .map_err(TomlError::SerializationError)?
                        .as_bytes(),
                )?;
                Ok(())
            }
            #[cfg(feature = "xml")]
            ConfigFormat::Xml => Ok(std::fs::write(path, quick_xml::se::to_string(&self)?)?),
            #[cfg(feature = "yaml")]
            ConfigFormat::Yaml => {
                serde_yml::to_writer(open_write_file(path)?, &self).map_err(Error::Yaml)
            }
            #[allow(unreachable_patterns)]
            _ => Err(Error::UnsupportedFormat),
        }
    }
}

/// Open a file in read-only mode
#[allow(unused)]
fn open_file(path: &Path) -> Result<File> {
    File::open(path).map_err(Error::FileAccess)
}

/// Open a file in write mode
#[allow(unused)]
fn open_write_file(path: &Path) -> Result<File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .map_err(Error::FileAccess)
}

#[cfg(test)]
mod test {
    use std::env::temp_dir;

    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Serialize, Deserialize, PartialEq, Default, Eq)]
    struct TestConfig {
        host: String,
        port: u64,
        tags: Vec<String>,
        inner: TestConfigInner,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Default, Eq)]
    struct TestConfigInner {
        answer: u8,
    }

    impl TestConfig {
        #[allow(unused)]
        fn example() -> Self {
            Self {
                host: "example.com".to_string(),
                port: 443,
                tags: vec!["example".to_string(), "test".to_string()],
                inner: TestConfigInner { answer: 42 },
            }
        }
    }

    fn test_read_with_extension(extension: &str) {
        let config = TestConfig::load(format!("testdata/config.{extension}"));
        assert_eq!(config.unwrap(), TestConfig::example());
    }

    fn test_write_with_extension(extension: &str) {
        let mut temp = temp_dir().join("config");
        temp.set_extension(extension);
        TestConfig::example().store(dbg!(&temp)).unwrap();
        assert!(temp.is_file());
        dbg!(std::fs::read_to_string(&temp).unwrap());
        assert_eq!(TestConfig::example(), TestConfig::load(&temp).unwrap());
        std::fs::remove_file(temp).unwrap();
    }

    #[test]
    fn test_unknown() {
        let config = TestConfig::load("/tmp/foobar");
        assert!(matches!(config, Err(Error::UnsupportedFormat)));
    }

    #[test]
    #[cfg(feature = "toml")]
    fn test_file_not_found() {
        let config = TestConfig::load("/tmp/foobar.toml");
        assert!(matches!(config, Err(Error::FileAccess(_))));
    }

    #[test]
    #[cfg(feature = "json")]
    fn test_json() {
        test_read_with_extension("json");
        test_write_with_extension("json");
    }

    #[test]
    #[cfg(feature = "toml")]
    fn test_toml() {
        test_read_with_extension("toml");
        test_write_with_extension("toml");
    }

    #[test]
    #[cfg(feature = "xml")]
    fn test_xml() {
        test_read_with_extension("xml");
        test_write_with_extension("xml");
    }

    #[test]
    #[cfg(feature = "yaml")]
    fn test_yaml() {
        test_read_with_extension("yml");
        test_write_with_extension("yaml");
    }

    #[test]
    #[cfg(feature = "toml")]
    fn test_store_without_overwrite() {
        let temp = temp_dir().join("test_store_without_overwrite.toml");
        std::fs::File::create(&temp).unwrap();
        assert!(TestConfig::example()
            .store_without_overwrite(dbg!(&temp))
            .is_err());
        std::fs::remove_file(temp).unwrap();
    }

    #[test]
    #[cfg(all(feature = "toml", feature = "yaml"))]
    fn test_store_load_with_specific_format() {
        let temp = temp_dir().join("test_store_load_with_specific_format.toml");
        std::fs::File::create(&temp).unwrap();
        TestConfig::example()
            .store_with_specific_format(dbg!(&temp), ConfigFormat::Yaml)
            .unwrap();
        assert!(TestConfig::load(&temp).is_err());
        assert!(TestConfig::load_with_specific_format(&temp, ConfigFormat::Yaml).is_ok());
        std::fs::remove_file(temp).unwrap();
    }

    #[test]
    #[cfg(feature = "toml")]
    fn test_load_or_default() {
        let temp = temp_dir().join("test_load_or_default.toml");
        assert_eq!(
            TestConfig::load_or_default(&temp).unwrap(),
            TestConfig::default()
        );
    }
}
