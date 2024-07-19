# config-file

Extreamly easy to load and store your configuration file!

## Features

- `toml` (enabled by default)
- `json`
- `xml`
- `yaml`

## Examples

```rust, no_run
use config_file2::{LoadConfigFile, StoreConfigFile};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Config {
    host: String,
}

// store
Config { host: "example.com".into() }.store("/tmp/myconfig.toml").unwrap();

// load
let config = Config::load("/tmp/myconfig.toml").unwrap();
assert_eq!(config.host.as_str(), "example.com");
```

## more functions

```rust, ignore
fn load_with_specific_format(path: impl AsRef<Path>, config_type: ConfigFormat) -> Result<Self>;
fn load_or_default(path: impl AsRef<Path>) -> Result<Self>;
fn store_with_specific_format(self, path: impl AsRef<Path>, config_type: ConfigFormat) -> Result<()>;
fn store_without_overwrite(self, path: impl AsRef<Path>) -> Result<()>;
```
