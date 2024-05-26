# config-file

[![API Docs](https://docs.rs/config-file/badge.svg)](https://docs.rs/config-file)
[![Downloads](https://img.shields.io/crates/d/config-file.svg)](https://crates.io/crates/config-file)

## Read and write configuration file automatically

config-file reads and writes your configuration files and parse them automatically using their extension.

## Features

- toml is enabled by default
- json is optional
- xml is optional
- yaml is optional

## Examples

```rust
use config_file::FromConfigFile;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Config {
    host: String,
}

// read
let config = Config::from_config_file("/etc/myconfig.toml").unwrap();

// write
Config { host: "example.com" }.to_config_file("/tmp/myconfig.toml").unwrap();
```
