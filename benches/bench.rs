use actix_web::web;
use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::{criterion_group, criterion_main};
use image_filter::{Image, ImageCache};
use std::sync::{Arc, Mutex};

const image: &str = "grin.jpeg";

fn get_image(data: web::Data<Arc<Mutex<ImageCache>>>) {
  Image::new(image)
    .filter_from_qs("width=500")
    .to_http_response(data);
}

fn bench_get_image(c: &mut Criterion) {
  let data = web::Data::new(Arc::new(Mutex::new(ImageCache::new())));

  c.bench_with_input(BenchmarkId::new("image_cache", "ram"), &data, |b, d| {
    b.iter(|| get_image(d.clone()));
  });
}

criterion_group!(benches, bench_get_image);
criterion_main!(benches);
