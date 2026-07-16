#![doc = include_str!("../README.md")]
#![warn(clippy::nursery, clippy::cargo, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod error;

// `BufReader` speeds up `from_reader` based deserializers (esp. JSON, ~100x).
// `Write` is only needed by the `write_all` based serializers.
#[cfg(any(feature = "json", feature = "xml", feature = "yaml", feature = "ron"))]
use std::io::BufReader;
#[cfg(any(feature = "json5", feature = "toml", feature = "xml", feature = "ron"))]
use std::io::Write;
use std::{
    ffi::OsStr,
    fmt::{self, Debug},
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
};

use error::Error;
#[cfg(feature = "json5")]
use error::Json5Error;
pub use error::Result;
#[cfg(feature = "xml")]
use error::XmlError;
use serde::{Serialize, de::DeserializeOwned};
#[cfg(feature = "toml")]
use {error::TomlError, toml_crate as toml};

/// Format of configuration file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigFormat {
    Json,
    Json5,
    Toml,
    Xml,
    Yaml,
    Ron,
}

impl ConfigFormat {
    /// Get the [`ConfigFormat`] from a file extension
    #[must_use]
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension.to_lowercase().as_str() {
            #[cfg(feature = "json")]
            "json" => Some(Self::Json),
            #[cfg(feature = "json5")]
            "json5" => Some(Self::Json5),
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

    /// Get the [`ConfigFormat`] from a path
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

#[allow(unused_macros)]
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
    #[allow(unused_variables)]
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
                .map(|x| serde_json::from_reader(BufReader::new(x)))
                .transpose()?),
            #[cfg(feature = "json5")]
            ConfigFormat::Json5 => Ok(not_found_to_none!(std::fs::read_to_string(path))?
                .map(|x| json_five::from_str(x.as_str()))
                .transpose()
                .map_err(Json5Error::DeserializationError)?),
            #[cfg(feature = "toml")]
            ConfigFormat::Toml => Ok(not_found_to_none!(std::fs::read_to_string(path))?
                .map(|x| toml::from_str(x.as_str()))
                .transpose()
                .map_err(TomlError::DeserializationError)?),
            #[cfg(feature = "xml")]
            ConfigFormat::Xml => Ok(not_found_to_none!(open_file(path))?
                // `quick_xml::de::from_reader` requires `R: BufRead`.
                .map(|x| quick_xml::de::from_reader(BufReader::new(x)))
                .transpose()
                .map_err(XmlError::DeserializationError)?),
            #[cfg(feature = "yaml")]
            ConfigFormat::Yaml => Ok(not_found_to_none!(open_file(path))?
                .map(|x| yaml_serde::from_reader(BufReader::new(x)))
                .transpose()?),
            #[cfg(feature = "ron")]
            ConfigFormat::Ron => Ok(not_found_to_none!(open_file(path))?
                .map(|x| ron_crate::de::from_reader(BufReader::new(x)))
                .transpose()
                .map_err(ron_crate::Error::from)?),
            #[allow(unreachable_patterns)]
            _ => Err(Error::UnsupportedFormat),
        }
    }
}

/// Trait for storing a struct into a configuration file.
/// This trait is automatically implemented when [`serde::Serialize`] is.
pub trait StoreConfigFile: Serialize {
    /// Store config file to path with specific format, do not use extension to
    /// determine. If the file already exists, the config file
    /// will be overwritten.
    ///
    /// Parent directories are created automatically if they don't exist.
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
    /// Parent directories are created automatically if they don't exist.
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

    /// Returns a [`StoreBuilder`] for storing `self` with full control over
    /// [`pretty`][StoreBuilder::pretty], [`overwrite`][StoreBuilder::overwrite]
    /// and [`format`][StoreBuilder::format]. Finish the chain with
    /// [`StoreBuilder::execute`].
    ///
    /// Defaults match [`StoreConfigFile::store`]: pretty on, overwrite on,
    /// format auto-detected from the path extension.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use config_file2::StoreConfigFile;
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct Config;
    ///
    /// Config
    ///     .store_opts("/tmp/config.json")
    ///     .pretty(false)
    ///     .overwrite(false)
    ///     .execute()
    ///     .unwrap();
    /// ```
    fn store_opts(&self, path: impl AsRef<Path>) -> StoreBuilder<'_, Self>
    where
        Self: Sized,
    {
        StoreBuilder::new(self, path)
    }
}

impl<C: Serialize> StoreConfigFile for C {
    fn store_with_specific_format(
        &self,
        path: impl AsRef<Path>,
        config_type: ConfigFormat,
    ) -> Result<()> {
        store_impl(self, path.as_ref(), config_type, true, true)
    }
}

/// A struct that knows the path it stores itself at.
///
/// Implement [`Storable::path`] to give your type an intrinsic config path,
/// then use [`Storable::save`] / [`Storable::save_opts`] to write it without
/// passing a path every call.
///
/// `Storable` intentionally uses method names (`save`, `save_opts`) that differ
/// from [`StoreConfigFile`] (`store`, `store_opts`), so importing both traits
/// never causes method-name ambiguity on types that implement both.
pub trait Storable: Serialize + Sized {
    /// The path this struct stores itself at.
    fn path(&self) -> impl AsRef<Path>;

    /// Save this struct to its [`path`][Storable::path] with default options
    /// (pretty on, overwrite on, format auto-detected from the extension).
    ///
    /// Parent directories are created automatically if they don't exist.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::UnsupportedFormat`] if the file extension is not
    ///   supported.
    /// - Returns `Error::<Format>` if serialization to file fails.
    fn save(&self) -> Result<()> {
        StoreConfigFile::store(self, self.path())
    }

    /// Returns a [`StoreBuilder`] bound to this struct's
    /// [`path`][Storable::path], for full control over
    /// [`pretty`][StoreBuilder::pretty], [`overwrite`][StoreBuilder::overwrite]
    /// and [`format`][StoreBuilder::format]. Finish with
    /// [`StoreBuilder::execute`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use config_file2::Storable;
    /// use serde::Serialize;
    /// use std::path::{Path, PathBuf};
    ///
    /// #[derive(Serialize)]
    /// struct Config;
    ///
    /// impl Storable for Config {
    ///     fn path(&self) -> impl AsRef<Path> {
    ///         PathBuf::from("/tmp/config.json")
    ///     }
    /// }
    ///
    /// Config.save_opts().pretty(false).execute().unwrap();
    /// ```
    fn save_opts(&self) -> StoreBuilder<'_, Self> {
        StoreBuilder::new(self, self.path())
    }
}

/// Open a file in read-only mode
#[allow(unused)]
fn open_file(path: &Path) -> std::io::Result<File> {
    File::open(path)
}

/// Open a file for writing, truncating it if it already exists. Parent
/// directories are created automatically.
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

/// Open a file for writing, atomically failing with [`Error::FileExists`] if it
/// already exists (no TOCTOU window). Parent directories are created
/// automatically.
#[allow(unused)]
fn open_create_new_file(path: &Path) -> Result<File> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                Error::FileExists
            } else {
                Error::FileAccess(e)
            }
        })
}

/// Serialize `value` to `path` in the given `config_type`.
///
/// `pretty` selects pretty-printing where the format supports it
/// (json/toml/ron); other formats ignore it. When `overwrite` is `true`, an
/// existing file is truncated; when `false`, the file is created atomically via
/// [`open_create_new_file`] and [`Error::FileExists`] is returned if it already
/// exists.
#[allow(unused)]
fn store_impl<S: Serialize>(
    value: &S,
    path: &Path,
    config_type: ConfigFormat,
    pretty: bool,
    overwrite: bool,
) -> Result<()> {
    let open: fn(&Path) -> Result<File> = if overwrite {
        open_write_file
    } else {
        open_create_new_file
    };
    match config_type {
        #[cfg(feature = "json")]
        ConfigFormat::Json => {
            let file = open(path)?;
            if pretty {
                serde_json::to_writer_pretty(file, value)
            } else {
                serde_json::to_writer(file, value)
            }
            .map_err(Error::Json)
        }
        #[cfg(feature = "json5")]
        ConfigFormat::Json5 => {
            open(path)?.write_all(
                json_five::to_string(value)
                    .map_err(Json5Error::SerializationError)?
                    .as_bytes(),
            )?;
            Ok(())
        }
        #[cfg(feature = "toml")]
        ConfigFormat::Toml => {
            let serialized = if pretty {
                toml::to_string_pretty(value)
            } else {
                toml::to_string(value)
            };
            open(path)?.write_all(
                serialized
                    .map_err(TomlError::SerializationError)?
                    .as_bytes(),
            )?;
            Ok(())
        }
        #[cfg(feature = "xml")]
        ConfigFormat::Xml => {
            open(path)?.write_all(
                quick_xml::se::to_string(value)
                    .map_err(XmlError::SerializationError)?
                    .as_bytes(),
            )?;
            Ok(())
        }
        #[cfg(feature = "yaml")]
        ConfigFormat::Yaml => yaml_serde::to_writer(open(path)?, value).map_err(Error::Yaml),
        #[cfg(feature = "ron")]
        ConfigFormat::Ron => {
            let serialized = if pretty {
                ron_crate::ser::to_string_pretty(value, ron_crate::ser::PrettyConfig::default())
            } else {
                ron_crate::ser::to_string(value)
            }?;
            open(path)?.write_all(serialized.as_bytes())?;
            Ok(())
        }
        #[allow(unreachable_patterns)]
        _ => Err(Error::UnsupportedFormat),
    }
}

/// Builder for storing a value with fine-grained options.
///
/// Created via [`StoreConfigFile::store_opts`]. Configure with
/// [`pretty`][StoreBuilder::pretty], [`overwrite`][StoreBuilder::overwrite] and
/// [`format`][StoreBuilder::format], then call
/// [`execute`][StoreBuilder::execute] to write the file.
///
/// Defaults: pretty on, overwrite on, format auto-detected from the path
/// extension (i.e. the same behavior as [`StoreConfigFile::store`]).
#[must_use]
pub struct StoreBuilder<'a, T: Sized + Serialize> {
    value: &'a T,
    path: PathBuf,
    pretty: bool,
    overwrite: bool,
    config_format: Option<ConfigFormat>,
}

impl<T: Sized + Serialize> fmt::Debug for StoreBuilder<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StoreBuilder")
            .field("path", &self.path)
            .field("pretty", &self.pretty)
            .field("overwrite", &self.overwrite)
            .field("config_format", &self.config_format)
            .finish()
    }
}

impl<'a, T: Sized + Serialize> StoreBuilder<'a, T> {
    fn new(value: &'a T, path: impl AsRef<Path>) -> Self {
        Self {
            value,
            path: path.as_ref().to_path_buf(),
            pretty: true,
            overwrite: true,
            config_format: None,
        }
    }

    /// Toggle pretty-printing. Default: `true`. Formats without a pretty
    /// variant (xml/yaml/json5) ignore this.
    pub const fn pretty(mut self, pretty: bool) -> Self {
        self.pretty = pretty;
        self
    }

    /// Toggle overwriting an existing file. Default: `true`. When `false`,
    /// [`execute`][StoreBuilder::execute] returns [`Error::FileExists`] if the
    /// file already exists.
    pub const fn overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }

    /// Use a specific [`ConfigFormat`], overriding extension-based detection.
    pub const fn format(mut self, config_format: ConfigFormat) -> Self {
        self.config_format = Some(config_format);
        self
    }

    /// Store the value with the configured options.
    ///
    /// # Errors
    ///
    /// - Returns [`Error::UnsupportedFormat`] if the format can't be
    ///   determined.
    /// - Returns [`Error::FileAccess`] if the file can't be written.
    /// - Returns [`Error::FileExists`] if `overwrite` is `false` and the file
    ///   exists.
    /// - Returns `Error::<Format>` on serialization failure.
    pub fn execute(&self) -> Result<()> {
        let config_type = self
            .config_format
            .or_else(|| ConfigFormat::from_path(&self.path))
            .ok_or(Error::UnsupportedFormat)?;
        store_impl(
            self.value,
            &self.path,
            config_type,
            self.pretty,
            self.overwrite,
        )
    }
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
        TestConfig::example().store(&temp).unwrap();
        assert!(temp.is_file());
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
    #[cfg(feature = "json5")]
    fn test_json5() {
        test_read_with_extension("json5");
        test_write_with_extension("json5");
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
    #[cfg(all(feature = "toml", feature = "yaml"))]
    fn test_store_load_with_specific_format() {
        let tempdir = TempDir::new().unwrap();
        let temp = tempdir
            .path()
            .join("test_store_load_with_specific_format.toml");
        std::fs::File::create(&temp).unwrap();
        TestConfig::example()
            .store_with_specific_format(&temp, ConfigFormat::Yaml)
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

    #[test]
    #[cfg(feature = "toml")]
    fn test_store_builder_round_trip() {
        let tempdir = TempDir::new().unwrap();
        for pretty in [true, false] {
            let temp = tempdir
                .path()
                .join(format!("test_store_builder_{pretty}.toml"));
            TestConfig::example()
                .store_opts(&temp)
                .pretty(pretty)
                .execute()
                .unwrap();
            assert_eq!(
                TestConfig::example(),
                TestConfig::load(&temp).unwrap().unwrap()
            );
        }
    }

    #[test]
    #[cfg(feature = "toml")]
    fn test_store_builder_pretty_compact_differ() {
        let tempdir = TempDir::new().unwrap();
        let pretty = tempdir.path().join("pretty.toml");
        let compact = tempdir.path().join("compact.toml");
        TestConfig::example()
            .store_opts(&pretty)
            .pretty(true)
            .execute()
            .unwrap();
        TestConfig::example()
            .store_opts(&compact)
            .pretty(false)
            .execute()
            .unwrap();
        // pretty TOML is multi-line and therefore larger than compact TOML.
        assert!(
            std::fs::metadata(&pretty).unwrap().len() > std::fs::metadata(&compact).unwrap().len()
        );
    }

    #[test]
    #[cfg(feature = "toml")]
    fn test_store_builder_overwrite_false() {
        let tempdir = TempDir::new().unwrap();
        let temp = tempdir.path().join("test_store_builder_overwrite.toml");
        std::fs::File::create(&temp).unwrap();
        assert!(
            TestConfig::example()
                .store_opts(&temp)
                .overwrite(false)
                .execute()
                .is_err()
        );
    }

    #[test]
    #[cfg(feature = "toml")]
    fn test_store_builder_format_override() {
        let tempdir = TempDir::new().unwrap();
        // an extension we don't recognize, but force toml via the builder.
        let temp = tempdir.path().join("test_store_builder_override.dat");
        TestConfig::example()
            .store_opts(&temp)
            .format(ConfigFormat::Toml)
            .execute()
            .unwrap();
        assert!(TestConfig::load(&temp).is_err());
        assert!(
            TestConfig::load_with_specific_format(&temp, ConfigFormat::Toml)
                .unwrap()
                .is_some()
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
    fn test_save() {
        let tempdir = TempDir::new().unwrap();
        let temp = tempdir.path().join("test_save.toml");
        TestStorable { path: temp.clone() }.save().unwrap();
        assert!(temp.is_file());
    }

    #[test]
    fn test_save_opts() {
        let tempdir = TempDir::new().unwrap();
        let temp = tempdir.path().join("test_save_opts.toml");
        TestStorable { path: temp.clone() }
            .save_opts()
            .pretty(false)
            .execute()
            .unwrap();
        assert!(temp.is_file());
    }

    #[test]
    fn test_both_traits_no_ambiguity() {
        // Regression: importing both StoreConfigFile and Storable must not make
        // `.store(path)` / `.save()` ambiguous.
        use super::StoreConfigFile;

        let tempdir = TempDir::new().unwrap();
        let temp = tempdir.path().join("test_no_ambiguity.toml");
        let config = TestStorable { path: temp.clone() };
        config.store(&temp).unwrap(); // StoreConfigFile
        config.save().unwrap(); // Storable
        assert!(temp.is_file());
    }
}
