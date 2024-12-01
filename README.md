# config-file2

Extremely easy to load and store your configuration file!

## Usage

1. Add dependency:
   ```bash
   cargo add config-file2
   ```
2. Enable which format you want to use in features.
   - `all`
   - `toml` (enabled by default)
   - `json`
   - `xml`
   - `yaml`
   - [`ron`](https://github.com/ron-rs/ron)

Here's an example of how to use it with json and yaml format:

```toml
[dependencies]
config-file2 = { version = "0.4", features = ["json", "yaml"] }
```

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
let config = Config::load("/tmp/myconfig.toml").unwrap().unwrap();
assert_eq!(config.host.as_str(), "example.com");
```

Another way to store a struct into a configuration file:

```rust, no_run
use config_file2::Storable;
use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct TestStorable {
    path: PathBuf,
}

impl Storable for TestStorable {
    fn path(&self) -> &Path {
        &self.path
    }
}

TestStorable { path: PathBuf::from("/tmp/myconfig.toml") }.store().unwrap();
```

## more

```rust, ignore
fn load_with_specific_format(path: impl AsRef<Path>, config_type: ConfigFormat) -> Result<Self>;
fn load_or_default(path: impl AsRef<Path>) -> Result<Self>;
fn store_with_specific_format(self, path: impl AsRef<Path>, config_type: ConfigFormat) -> Result<()>;
fn store_without_overwrite(self, path: impl AsRef<Path>) -> Result<()>;
```
