#![doc = include_str!("../README.md")]
#![warn(clippy::nursery, clippy::cargo, clippy::pedantic)]
#[allow(clippy::module_name_repetitions)]
pub mod error;
use std::{
    ffi::OsStr,
    fmt::Debug,
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
};

use error::Error;
pub use error::Result;
use serde::{de::DeserializeOwned, Serialize};
#[cfg(feature = "toml")]
use {error::TomlError, toml_crate as toml};
#[cfg(feature = "xml")]
use {error::XmlError, std::io::BufReader};

/// Format of configuration file.
#[derive(Debug, Clone, Copy)]
pub enum ConfigFormat {
    Json,
    Toml,
    Xml,
    Yaml,
    Ron,
}

impl ConfigFormat {
    /// Get the [`ConfigType`] from a file extension
    #[must_use]
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
            #[cfg(feature = "ron")]
            "ron" => Some(Self::Ron),
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
    /// Load config from path with specific format, *do not use extension to
    /// determine*.
    ///
    /// # Returns
    ///
    /// - Returns `Ok(Some(config))` if the file exists.
    /// - Returns `Ok(None)` if the file does not exist.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::FileAccess`] if the file cannot be read.
    /// - Returns `Error::<Format>` if deserialization from file fails.
    fn load_with_specific_format(
        path: impl AsRef<Path>,
        config_type: ConfigFormat,
    ) -> Result<Option<Self>>
    where
        Self: Sized;

    /// Load config from path.
    ///
    /// # Returns
    ///
    /// - Returns `Ok(Some(config))` if the file exists.
    /// - Returns `Ok(None)` if the file does not exist.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::FileAccess`] if the file cannot be read.
    /// - Returns [`Error::UnsupportedFormat`] if the file extension is not
    ///   supported.
    /// - Returns `Error::<Format>` if deserialization from file fails.
    fn load(path: impl AsRef<Path>) -> Result<Option<Self>>
    where
        Self: Sized,
    {
        let path = path.as_ref();
        let config_type = ConfigFormat::from_path(path).ok_or(Error::UnsupportedFormat)?;
        Self::load_with_specific_format(path, config_type)
    }

    /// Load config from path, if not found, use default instead
    ///
    /// # Returns
    ///
    /// - Returns the config loaded from file if the file exists, or default
    ///   value if the file does not exist.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::FileAccess`] if the file cannot be read by Permission
    ///   denied or other failures.
    /// - Returns [`Error::UnsupportedFormat`] if the file extension is not
    ///   supported.
    /// - Returns `Error::<Format>` if deserialization from file fails.
    fn load_or_default(path: impl AsRef<Path>) -> Result<Self>
    where
        Self: Sized + Default,
    {
        Self::load(path).map(std::option::Option::unwrap_or_default)
    }
}

macro_rules! not_found_to_none {
    ($input:expr) => {
        match $input {
            Ok(config) => Ok(Some(config)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    };
}

impl<C: DeserializeOwned> LoadConfigFile for C {
    fn load_with_specific_format(
        path: impl AsRef<Path>,
        config_type: ConfigFormat,
    ) -> Result<Option<Self>>
    where
        Self: Sized,
    {
        let path = path.as_ref();

        match config_type {
            #[cfg(feature = "json")]
            ConfigFormat::Json => Ok(not_found_to_none!(open_file(path))?
                .map(|x| serde_json::from_reader(x))
                .transpose()?),
            #[cfg(feature = "toml")]
            ConfigFormat::Toml => Ok(not_found_to_none!(std::fs::read_to_string(path))?
                .map(|x| toml::from_str(x.as_str()))
                .transpose()
                .map_err(TomlError::DeserializationError)?),
            #[cfg(feature = "xml")]
            ConfigFormat::Xml => Ok(not_found_to_none!(open_file(path))?
                .map(|x| quick_xml::de::from_reader(BufReader::new(x)))
                .transpose()
                .map_err(XmlError::DeserializationError)?),
            #[cfg(feature = "yaml")]
            ConfigFormat::Yaml => Ok(not_found_to_none!(open_file(path))?
                .map(|x| serde_yml::from_reader(x))
                .transpose()?),
            #[cfg(feature = "ron")]
            ConfigFormat::Ron => Ok(not_found_to_none!(open_file(path))?
                .map(|x| ron_crate::de::from_reader(x))
                .transpose()
                .map_err(Into::<ron_crate::Error>::into)?),
            #[allow(unreachable_patterns)]
            _ => Err(Error::UnsupportedFormat),
        }
    }
}

/// Trait for storing a struct into a configuration file.
/// This trait is automatically implemented when [`serde::Serialize`] is.
pub trait StoreConfigFile {
    /// Store config file to path with specific format, do not use extension to
    /// determine. If the file already exists, the config file
    /// will be overwritten.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::FileAccess`] if the file cannot be written.
    /// - Returns [`Error::UnsupportedFormat`] if the file extension is not
    ///   supported.
    /// - Returns `Error::<Format>` if serialization to file fails.
    fn store_with_specific_format(
        &self,
        path: impl AsRef<Path>,
        config_type: ConfigFormat,
    ) -> Result<()>;

    /// Store config file to path. If the file already exists, the config file
    /// will be overwritten.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::UnsupportedFormat`] if the file extension is not
    ///   supported.
    /// - Returns `Error::<Format>` if serialization to file fails.
    fn store(&self, path: impl AsRef<Path>) -> Result<()>
    where
        Self: Sized,
    {
        let path = path.as_ref();
        let config_type = ConfigFormat::from_path(path).ok_or(Error::UnsupportedFormat)?;
        self.store_with_specific_format(path, config_type)
    }
    /// Store config file to path, if path exists, return error
    ///
    /// # Errors
    ///
    /// - Returns [`Error::FileExists`] if the file already exists.
    /// - Returns [`Error::UnsupportedFormat`] if the file extension is not
    ///   supported.
    /// - Returns `Error::<Format>` if serialization to file fails.
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
            ConfigFormat::Xml => Ok(std::fs::write(
                path,
                quick_xml::se::to_string(&self).map_err(XmlError::SerializationError)?,
            )?),
            #[cfg(feature = "yaml")]
            ConfigFormat::Yaml => {
                serde_yml::to_writer(open_write_file(path)?, &self).map_err(Error::Yaml)
            }
            #[cfg(feature = "ron")]
            ConfigFormat::Ron => {
                open_write_file(path)?.write_all(
                    ron_crate::ser::to_string_pretty(
                        &self,
                        ron_crate::ser::PrettyConfig::default(),
                    )?
                    .as_bytes(),
                )?;
                Ok(())
            }
            #[allow(unreachable_patterns)]
            _ => Err(Error::UnsupportedFormat),
        }
    }
}

/// A more easy way to store a struct into a configuration file.
///
/// Just impl `Storable::path(&self) -> &Path;` to your struct, and then you can
/// use `store_with_specific_format`, `store`, `store_without_overwrite`
/// directly by calling the method on your struct.
pub trait Storable: Serialize + Sized {
    /// impl by struct.
    fn path(&self) -> impl AsRef<Path>;

    /// Store config file to path with specific format, do not use extension to
    /// determine. If the file already exists, the config file
    /// will be overwritten.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::FileAccess`] if the file cannot be written.
    /// - Returns [`Error::UnsupportedFormat`] if the file extension is not
    ///   supported.
    /// - Returns `Error::<Format>` if serialization to file fails.
    fn store_with_specific_format(&self, config_type: ConfigFormat) -> Result<()> {
        StoreConfigFile::store_with_specific_format(self, self.path().as_ref(), config_type)
    }

    /// Store config file to path. If the file already exists, the config file
    /// will be overwritten.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::UnsupportedFormat`] if the file extension is not
    ///   supported.
    /// - Returns `Error::<Format>` if serialization to file fails.
    fn store(&self) -> Result<()> {
        StoreConfigFile::store(self, self.path())
    }
    /// Store config file to path, if path exists, return error
    ///
    /// # Errors
    ///
    /// - Returns [`Error::FileExists`] if the file already exists.
    /// - Returns [`Error::UnsupportedFormat`] if the file extension is not
    ///   supported.
    /// - Returns `Error::<Format>` if serialization to file fails.
    fn store_without_overwrite(&self) -> Result<()> {
        StoreConfigFile::store_without_overwrite(self, self.path())
    }
}

/// Open a file in read-only mode
#[allow(unused)]
fn open_file(path: &Path) -> std::io::Result<File> {
    File::open(path)
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

    use serde::Deserialize;
    use tempfile::TempDir;

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
        assert_eq!(config.unwrap().unwrap(), TestConfig::example());
    }

    fn test_write_with_extension(extension: &str) {
        let tempdir = TempDir::new().unwrap();
        let mut temp = tempdir.path().join("config");
        temp.set_extension(extension);
        TestConfig::example().store(dbg!(&temp)).unwrap();
        assert!(temp.is_file());
        dbg!(std::fs::read_to_string(&temp).unwrap());
        assert_eq!(
            TestConfig::example(),
            TestConfig::load(&temp).unwrap().unwrap()
        );
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
        assert!(config.unwrap().is_none());
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
    #[cfg(feature = "ron")]
    fn test_ron() {
        test_read_with_extension("ron");
        test_write_with_extension("ron");
    }

    #[test]
    #[cfg(feature = "toml")]
    fn test_store_without_overwrite() {
        let tempdir = TempDir::new().unwrap();
        let temp = tempdir.path().join("test_store_without_overwrite.toml");
        std::fs::File::create(&temp).unwrap();
        assert!(TestConfig::example()
            .store_without_overwrite(dbg!(&temp))
            .is_err());
    }

    #[test]
    #[cfg(all(feature = "toml", feature = "yaml"))]
    fn test_store_load_with_specific_format() {
        let tempdir = TempDir::new().unwrap();
        let temp = tempdir
            .path()
            .join("test_store_load_with_specific_format.toml");
        std::fs::File::create(&temp).unwrap();
        TestConfig::example()
            .store_with_specific_format(dbg!(&temp), ConfigFormat::Yaml)
            .unwrap();
        assert!(TestConfig::load(&temp).is_err());
        assert!(TestConfig::load_with_specific_format(&temp, ConfigFormat::Yaml).is_ok());
    }

    #[test]
    #[cfg(feature = "toml")]
    fn test_load_or_default() {
        let tempdir = TempDir::new().unwrap();
        let temp = tempdir.path().join("test_load_or_default.toml");
        assert_eq!(
            TestConfig::load_or_default(&temp).expect("load_or_default failed"),
            TestConfig::default()
        );
    }
}

#[cfg(test)]
mod storable {
    use std::path::{Path, PathBuf};

    use serde::Serialize;
    use tempfile::TempDir;

    use super::Storable;

    #[derive(Serialize)]
    struct TestStorable {
        path: PathBuf,
    }

    impl Storable for TestStorable {
        fn path(&self) -> impl AsRef<Path> {
            &self.path
        }
    }

    #[test]
    fn test_store() {
        let tempdir = TempDir::new().unwrap();
        let temp = tempdir.path().join("test_store.toml");
        TestStorable { path: temp.clone() }.store().unwrap();
        assert!(temp.is_file());
    }
}
