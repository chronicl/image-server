use image_filter::{Image, ImageCache};
use actix_web::web;
use std::sync::{Arc, Mutex};
use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::{criterion_group, criterion_main};


fn get_image(size: &str, image: &str, data: web::Data<Arc<Mutex<ImageCache>>>) {
  Image::new(&image).resize(None).to_http_response(data);
}


fn bench_get_image(c: &mut Criterion) {
  let data = web::Data::new(Arc::new(Mutex::new(ImageCache::new())));

  c.bench_with_input(BenchmarkId::new("input_example", 1), &data, |b, d| {
      b.iter(|| get_image("400x300", "grin.jpeg", d.clone()));
  });
}

criterion_group!(benches, bench_get_image);
criterion_main!(benches);