//! Measure whether wrapping the `File` in a `BufReader` speeds up the various
//! `from_reader` based deserializers.
//!
//! Run with:
//! `cargo run --release --example bench_bufreader --features json,yaml,ron,xml`

use std::{fs::File, hint::black_box, io::BufReader, time::Instant};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
struct Item {
    id: u64,
    name: String,
    value: f64,
}

fn make_data(n: usize) -> Vec<Item> {
    (0..n)
        .map(|i| Item {
            id: i as u64,
            name: format!("item_{i}"),
            value: i as f64 * 1.1,
        })
        .collect()
}

fn bench<R: PartialEq + std::fmt::Debug>(
    label: &str,
    iters: usize,
    unbuf: impl Fn() -> R,
    buf: impl Fn() -> R,
) {
    // sanity: both paths must parse identically
    assert_eq!(unbuf(), buf(), "results differ for {label}");

    let t = Instant::now();
    for _ in 0..iters {
        black_box(unbuf());
    }
    let unbuf_t = t.elapsed();

    let t = Instant::now();
    for _ in 0..iters {
        black_box(buf());
    }
    let buf_t = t.elapsed();

    println!(
        "{label:6} | unbuffered {unbuf_t:>9.2?} | buffered {buf_t:>9.2?} | speedup {:.2}x",
        unbuf_t.as_secs_f64() / buf_t.as_secs_f64()
    );
}

fn main() {
    let n = 50_000;
    let iters = 30;
    let dir = tempfile::tempdir().expect("tempdir");
    let data = make_data(n);
    println!("dataset: {n} items, iters: {iters}\n");

    #[cfg(feature = "json")]
    {
        let p = dir.path().join("big.json");
        serde_json::to_writer(File::create(&p).unwrap(), &data).unwrap();
        bench(
            "JSON",
            iters,
            || serde_json::from_reader::<_, Vec<Item>>(File::open(&p).unwrap()).unwrap(),
            || {
                serde_json::from_reader::<_, Vec<Item>>(BufReader::new(File::open(&p).unwrap()))
                    .unwrap()
            },
        );
    }

    #[cfg(feature = "yaml")]
    {
        let p = dir.path().join("big.yaml");
        yaml_serde::to_writer(File::create(&p).unwrap(), &data).unwrap();
        bench(
            "YAML",
            iters,
            || yaml_serde::from_reader::<_, Vec<Item>>(File::open(&p).unwrap()).unwrap(),
            || {
                yaml_serde::from_reader::<_, Vec<Item>>(BufReader::new(File::open(&p).unwrap()))
                    .unwrap()
            },
        );
    }

    #[cfg(feature = "ron")]
    {
        let p = dir.path().join("big.ron");
        std::fs::write(&p, ron_crate::to_string(&data).unwrap()).unwrap();
        bench(
            "RON",
            iters,
            || ron_crate::de::from_reader::<_, Vec<Item>>(File::open(&p).unwrap()).unwrap(),
            || {
                ron_crate::de::from_reader::<_, Vec<Item>>(BufReader::new(File::open(&p).unwrap()))
                    .unwrap()
            },
        );
    }
}
