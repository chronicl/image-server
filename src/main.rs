#![allow(dead_code)]
use actix_service::Service;
use actix_web::{
    get,
    web::{Data, Path},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use futures::future::FutureExt;
use image_server::{split_last, whitelist, Image, ImageCache};
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, Write};
use std::path;

#[macro_use]
extern crate clap;
use std::sync::{Arc, Mutex};

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<path::Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

#[get("/images/{image}")]
async fn get_image(
    req: HttpRequest,
    Path(image): Path<String>,
    data: Data<Arc<Mutex<ImageCache>>>,
) -> impl Responder {
    let res = Image::new(&image)
        .filter_from_qs(req.query_string())
        .to_http_response(data);
    res
}

#[get("/images/{image}")]
async fn get_image_create_whitelist(
    req: HttpRequest,
    Path(image): Path<String>,
    data: Data<Arc<Mutex<ImageCache>>>,
) -> impl Responder {
    let uri = req.uri().to_string();
    let (_, file_name) = split_last(&uri, '/');
    let mut already_in_whitelist = false;
    for line in read_lines("whitelist").unwrap() {
        if line.unwrap() == file_name {
            already_in_whitelist = true;
            break;
        }
    }
    if !already_in_whitelist {
        let mut file = OpenOptions::new().append(true).open("whitelist").unwrap();
        file.write_all([file_name, "\n"].concat().as_bytes())
            .unwrap();
    }
    let res = Image::new(&image)
        .filter_from_qs(req.query_string())
        .to_http_response(data);
    res
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let data = Arc::new(Mutex::new(ImageCache::new()));

    let yaml = load_yaml!("cli.yml");
    let matches = clap::App::from_yaml(yaml).get_matches();
    if let ("whitelist", Some(subargs)) = matches.subcommand() {
        whitelist::Whitelist::new(
            subargs.value_of("directory").unwrap_or("."),
            subargs.value_of("preceding-pattern").unwrap_or("images/"),
        )
        .build();
        return Ok(());
    }

    std::env::set_current_dir(matches.value_of("directory").unwrap_or(".")).unwrap();
    println!(
        "Images being read from: {:?}.",
        std::env::current_dir().unwrap()
    );
    println!("Make sure that the source images are marked as <image-name>.source.<image-type>.\nFor example if you want to be able to request mountain.jpg, name the source file mountain.source.jpg.\n");

    let port = matches.value_of("port").unwrap_or("9000");
    let socket = ["127.0.0.1:", port].concat();
    println!("Listening on {}", socket);

    match matches.value_of("whitelist") {
        None => {
            if matches.is_present("track-for-whitelist") {
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("whitelist")
                    .unwrap();
                println!(
                    "Api calls being written to whitelist in {:?}",
                    std::env::current_dir().unwrap(),
                );
                HttpServer::new(move || {
                    App::new()
                        .data(data.clone())
                        .service(get_image_create_whitelist)
                })
                .bind(socket)?
                .run()
                .await
            } else {
                HttpServer::new(move || App::new().data(data.clone()).service(get_image))
                    .bind(socket)?
                    .run()
                    .await
            }
        }
        Some(whitelist) => {
            let whitelist: HashSet<String> = read_lines(whitelist)
                .unwrap()
                .filter_map(|line| {
                    if let Ok(line) = line {
                        Some(line)
                    } else {
                        None
                    }
                })
                .collect();

            let whitelist = Arc::new(Mutex::new(whitelist));
            HttpServer::new(move || {
                let whitelist = whitelist.clone();
                App::new()
                    .wrap_fn(move |req, srv| {
                        let whitelist = whitelist.clone();
                        let uri = req.uri().to_string();
                        let (_, file_name) = split_last(&uri, '/');
                        let file_name = file_name.to_owned();
                        srv.call(req).map(move |res| {
                            if whitelist.clone().lock().unwrap().contains(&file_name) {
                                return res;
                            }
                            Ok(res?.into_response(HttpResponse::NotFound().finish()))
                        })
                    })
                    .data(data.clone())
                    .service(get_image)
            })
            .bind(socket)?
            .run()
            .await
        }
    }
}
