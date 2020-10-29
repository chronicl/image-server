#![allow(dead_code)]
use actix_web::{middleware, get, web::{Data, Path}, App, HttpRequest, HttpServer, Responder};
use serde::{Deserialize};
use serde_qs;
use image_filter::{Image, ImageCache};
use std::sync::{Arc, Mutex};

#[derive(Deserialize,Debug)]
struct Params<'a>{
    size: Option<&'a str>,
}


#[get("/images/{image}")]
async fn get_image(req: HttpRequest, Path(image): Path<String>, data: Data<Arc<Mutex<ImageCache>>>) -> impl Responder {
    let params: Params = serde_qs::from_str(req.query_string()).unwrap();
    
    Image::new(&image).resize(params.size).to_http_response(data)
}

#[test]
fn qs_test() {
    
}

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