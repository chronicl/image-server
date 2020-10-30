use actix_web::web::Bytes;
use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::{criterion_group, criterion_main};
use std::collections::HashMap;
use std::fs;

const name: &str = "grin";
const file_name: &str = "grin.jpeg";

fn bytes_fs(map: HashMap<String, String>) {
    match map.get(name) {
        None => {}
        Some(file) => {
            let done = fs::read(file).unwrap();
        }
    }
}

fn bytes1(c: &mut Criterion) {
    let mut map: HashMap<String, String> = HashMap::new();
    map.insert(name.into(), file_name.into());

    c.bench_with_input(BenchmarkId::new("bytes", "fs"), &map, |b, s| {
        b.iter(|| bytes_fs(s.clone()));
    });
}

fn bytes_bytes(map: HashMap<String, Bytes>) {
    match map.get(name) {
        None => {}
        Some(file) => {
            let done = file;
        }
    }
}

fn bytes2(c: &mut Criterion) {
    let mut map: HashMap<String, Bytes> = HashMap::new();
    map.insert(name.into(), Bytes::from(fs::read(file_name).unwrap()));

    c.bench_with_input(BenchmarkId::new("bytes", "bytes"), &map, |b, s| {
        b.iter(|| bytes_bytes(s.clone()));
    });
}

criterion_group!(benches, bytes1, bytes2);
criterion_main!(benches);
