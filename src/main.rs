#![allow(dead_code)]
use actix_web::{
    get, middleware,
    web::{Data, Path},
    App, HttpRequest, HttpServer, Responder,
};
use image_filter::{Image, ImageCache};

use std::sync::{Arc, Mutex};

#[get("/images/{image}")]
async fn get_image(
    req: HttpRequest,
    Path(image): Path<String>,
    data: Data<Arc<Mutex<ImageCache>>>,
) -> impl Responder {
    Image::new(&image)
        .filter_from_qs(req.query_string())
        .to_http_response(data)
}

#[test]
fn qs_test() {}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=trace");
    env_logger::init();
    let data = Arc::new(Mutex::new(ImageCache::new()));

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .data(data.clone())
            .service(get_image)
    })
    .bind("127.0.0.1:9001")?
    .run()
    .await
}
