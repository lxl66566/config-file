#[cfg(feature = "toml")]
use toml_crate as toml;

pub type Result<T> = std::result::Result<T, Error>;

/// This type represents all possible errors that can occur when loading or
/// storing data from a configuration file.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// There was an error while reading the configuration file
    #[error("couldn't read or write config file")]
    FileAccess(#[from] std::io::Error),

    #[error("file already exists")]
    FileExists,

    /// There was an error while parsing the JSON data
    #[cfg(feature = "json")]
    #[error("couldn't parse JSON file")]
    Json(#[from] serde_json::Error),

    /// There was an error while parsing the TOML data
    #[cfg(feature = "toml")]
    #[error("couldn't parse TOML file")]
    Toml(#[from] TomlError),

    /// There was an error while parsing the XML data
    #[cfg(feature = "xml")]
    #[error("couldn't parse XML file")]
    Xml(#[from] XmlError),

    /// There was an error while parsing the YAML data
    #[cfg(feature = "yaml")]
    #[error("couldn't parse YAML file")]
    Yaml(#[from] serde_yml::Error),

    /// There was an error while parsing the Ron data
    #[cfg(feature = "ron")]
    #[error("couldn't parse Ron file")]
    Ron(#[from] ron_crate::Error),

    /// We don't know how to parse this format according to the file extension
    #[error("don't know how to parse file")]
    UnsupportedFormat,
}

/// Merge two TOML errors into one
#[cfg(feature = "toml")]
#[derive(Debug, thiserror::Error)]
pub enum TomlError {
    /// TOML deserialization error
    #[error("Toml deserialization error: {0}")]
    DeserializationError(#[from] toml::de::Error),

    /// TOML serialization error
    #[error("Toml serialization error: {0}")]
    SerializationError(#[from] toml::ser::Error),
}

/// Merge two XML errors into one
#[cfg(feature = "toml")]
#[derive(Debug, thiserror::Error)]
pub enum XmlError {
    /// XML deserialization error
    #[error("Xml deserialization error: {0}")]
    DeserializationError(#[from] quick_xml::DeError),

    /// XML serialization error
    #[error("Xml serialization error: {0}")]
    SerializationError(#[from] quick_xml::SeError),
}
