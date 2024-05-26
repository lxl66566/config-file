#![deny(missing_docs)]
#![warn(rust_2018_idioms)]
#![doc(html_root_url = "https://docs.rs/config-file/0.2.3/")]

//! # Read and parse configuration file automatically
//!
//! config-file reads your configuration files and parse them automatically
//! using their extension.
//!
//! # Features
//!
//! - toml is enabled by default
//! - json is optional
//! - xml is optional
//! - yaml is optional
//!
//! # Examples
//!
//! ```rust,no_run
//! use config_file::FromConfigFile;
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize)]
//! struct Config {
//!     host: String,
//! }
//!
//! // read
//! let config = Config::from_config_file("/etc/myconfig.toml").unwrap();
//!
//! // write
//! Config { host: "example.com" }.to_config_file("/tmp/myconfig.toml").unwrap();
//! ```

use std::{
    ffi::OsStr,
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
};

use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;
#[cfg(feature = "toml")]
use toml_crate as toml;

/// Trait for loading a struct from a configuration file.
/// This trait is automatically implemented when [`serde::Deserialize`] is.
pub trait FromConfigFile {
    /// Load ourselves from the configuration file located at @path
    fn from_config_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigFileError>
    where
        Self: Sized;
}

impl<C: DeserializeOwned> FromConfigFile for C {
    fn from_config_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigFileError>
    where
        Self: Sized,
    {
        let path = path.as_ref();
        let extension = path
            .extension()
            .and_then(OsStr::to_str)
            .map(|extension| extension.to_lowercase());
        match extension.as_deref() {
            #[cfg(feature = "json")]
            Some("json") => {
                serde_json::from_reader(open_file(path)?).map_err(ConfigFileError::Json)
            }
            #[cfg(feature = "toml")]
            Some("toml") => Ok(toml::from_str(
                std::fs::read_to_string(path)
                    .map_err(ConfigFileError::FileAccess)?
                    .as_str(),
            )
            .map_err(TomlError::DeserializationError)?),
            #[cfg(feature = "xml")]
            Some("xml") => {
                serde_xml_rs::from_reader(open_file(path)?).map_err(ConfigFileError::Xml)
            }
            #[cfg(feature = "yaml")]
            Some("yaml") | Some("yml") => {
                serde_yaml::from_reader(open_file(path)?).map_err(ConfigFileError::Yaml)
            }
            _ => Err(ConfigFileError::UnsupportedFormat),
        }
    }
}

/// Trait for storing a struct into a configuration file.
/// This trait is automatically implemented when [`serde::Serialize`] is.
pub trait IntoConfigFile {
    /// Load ourselves from the configuration file located at @path
    fn to_config_file(self, path: impl AsRef<Path>) -> Result<(), ConfigFileError>
    where
        Self: Sized;
}

impl<C: Serialize> IntoConfigFile for C {
    fn to_config_file(self, path: impl AsRef<Path>) -> Result<(), ConfigFileError>
    where
        Self: Sized,
    {
        let path = path.as_ref();
        let extension = path
            .extension()
            .and_then(OsStr::to_str)
            .map(|extension| extension.to_lowercase());
        match extension.as_deref() {
            #[cfg(feature = "json")]
            Some("json") => serde_json::to_writer_pretty(open_write_file(path)?, &self)
                .map_err(ConfigFileError::Json),
            #[cfg(feature = "toml")]
            Some("toml") => {
                open_write_file(path)?.write_all(
                    toml::to_string_pretty(&self)
                        .map_err(TomlError::SerializationError)?
                        .as_bytes(),
                )?;
                Ok(())
            }
            #[cfg(feature = "xml")]
            Some("xml") => {
                serde_xml_rs::to_writer(open_write_file(path)?, &self).map_err(ConfigFileError::Xml)
            }
            #[cfg(feature = "yaml")]
            Some("yaml") | Some("yml") => {
                serde_yaml::to_writer(open_write_file(path)?, &self).map_err(ConfigFileError::Yaml)
            }
            _ => Err(ConfigFileError::UnsupportedFormat),
        }
    }
}

/// Open a file in read-only mode
#[allow(unused)]
fn open_file(path: &Path) -> Result<File, ConfigFileError> {
    File::open(path).map_err(ConfigFileError::FileAccess)
}

/// Open a file in write mode
#[allow(unused)]
fn open_write_file(path: &Path) -> Result<File, ConfigFileError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .map_err(ConfigFileError::FileAccess)
}

/// This type represents all possible errors that can occur when loading or
/// storing data from a configuration file.
#[derive(Error, Debug)]
pub enum ConfigFileError {
    #[error("couldn't read config file")]
    /// There was an error while reading the configuration file
    FileAccess(#[from] std::io::Error),
    #[cfg(feature = "json")]
    #[error("couldn't parse JSON file")]
    /// There was an error while parsing the JSON data
    Json(#[from] serde_json::Error),
    #[cfg(feature = "toml")]
    #[error("couldn't parse TOML file")]
    /// There was an error while parsing the TOML data
    Toml(#[from] TomlError),
    #[cfg(feature = "xml")]
    #[error("couldn't parse XML file")]
    /// There was an error while parsing the XML data
    Xml(#[from] serde_xml_rs::Error),
    #[cfg(feature = "yaml")]
    #[error("couldn't parse YAML file")]
    /// There was an error while parsing the YAML data
    Yaml(#[from] serde_yaml::Error),
    #[error("don't know how to parse file")]
    /// We don't know how to parse this format according to the file extension
    UnsupportedFormat,
}

/// Merge two TOML errors into one
#[cfg(feature = "toml")]
#[derive(Debug, Error)]
pub enum TomlError {
    /// TOML deserialization error
    #[error("Toml deserialization error: {0}")]
    DeserializationError(#[from] toml::de::Error),
    /// TOML serialization error
    #[error("Toml serialization error: {0}")]
    SerializationError(#[from] toml::ser::Error),
}

#[cfg(test)]
mod test {
    use std::env::temp_dir;

    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestConfig {
        host: String,
        port: u64,
        tags: Vec<String>,
        inner: TestConfigInner,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
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
        let config = TestConfig::from_config_file(format!("testdata/config.{extension}"));
        assert_eq!(config.unwrap(), TestConfig::example());
    }

    fn test_write_with_extension(extension: &str) {
        let mut temp = temp_dir().join("config");
        temp.set_extension(extension);
        TestConfig::example().to_config_file(dbg!(&temp)).unwrap();
        assert!(temp.is_file());
        dbg!(std::fs::read_to_string(&temp).unwrap());
        assert_eq!(
            TestConfig::example(),
            TestConfig::from_config_file(&temp).unwrap()
        );
        std::fs::remove_file(temp).unwrap();
    }

    #[test]
    fn test_unknown() {
        let config = TestConfig::from_config_file("/tmp/foobar");
        assert!(matches!(config, Err(ConfigFileError::UnsupportedFormat)));
    }

    #[test]
    #[cfg(feature = "toml")]
    fn test_file_not_found() {
        let config = TestConfig::from_config_file("/tmp/foobar.toml");
        assert!(matches!(config, Err(ConfigFileError::FileAccess(_))));
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
}
