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
   - `json5`
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

Another way to store a struct into a configuration file, where the struct owns
its path via [`Storable`]:

```rust, no_run
use config_file2::Storable;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct TestStorable {
    path: PathBuf,
}

impl Storable for TestStorable {
    fn path(&self) -> impl AsRef<Path> {
        &self.path
    }
}

TestStorable { path: PathBuf::from("/tmp/myconfig.toml") }.save().unwrap();
```

## Storing with options (builder)

When you need control over pretty-printing, overwriting, or the format, use a
builder. For an explicit path use [`StoreConfigFile::store_opts`]:

```rust, no_run
use config_file2::StoreConfigFile;
use serde::Serialize;

#[derive(Serialize)]
struct Config;

// pretty off, no overwrite, format still auto-detected from the extension.
Config
    .store_opts("/tmp/myconfig.json")
    .pretty(false)
    .overwrite(false)
    .execute()
    .unwrap();
```

For a type implementing [`Storable`], use [`Storable::save_opts`] (bound to the
intrinsic path):

```rust, ignore
use config_file2::Storable;

config.save_opts().pretty(false).execute().unwrap();
```

## more

```rust, ignore
fn load_with_specific_format(path: impl AsRef<Path>, config_type: ConfigFormat) -> Result<Self>;
fn load_or_default(path: impl AsRef<Path>) -> Result<Self>;
fn store(&self, path: impl AsRef<Path>) -> Result<()>;
fn store_with_specific_format(&self, path: impl AsRef<Path>, config_type: ConfigFormat) -> Result<()>;
fn store_opts(&self, path: impl AsRef<Path>) -> StoreBuilder<'_, Self>;
// Storable:
fn save(&self) -> Result<()>;
fn save_opts(&self) -> StoreBuilder<'_, Self>;
```
