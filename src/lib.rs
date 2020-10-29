#![allow(dead_code)]
#![feature(str_split_once)]
use actix_web::{HttpResponse};
use std::fs;
use std::sync::{Arc, Mutex};
use actix_web::web;
use std::process::Command;
use std::collections::HashMap;

pub fn split_last(string: &str, delimiter: char) -> (&str, &str) {
  for (i, c) in string.chars().rev().enumerate() {
    if c == delimiter {
      return (&string[..(string.len() - i - 1)], &string[(string.len() - i)..])
    }
  }
  (string, "")
}

pub struct Image<'a> {
  name: &'a str,
  image_type: &'a str,
  resize: Option<&'a str>,
  file_name: String
}



impl<'a> Image<'a> {
  pub fn new(file_name: &'a str) -> Self {
    let (name, image_type) = split_last(file_name, '.');

    Image{name, image_type, resize: None, file_name: String::from(file_name)}
  }

  pub fn resize(&mut self, size: Option<&'a str>) -> &mut Self {
      self.resize = size;
      if let Some(s) = size {
        self.file_name = format!("{}.{}.{}", self.name, s, self.image_type);
      }
      
      self
  }

  fn to_mime_str(&self) -> String {
      format!("image/{}", self.image_type)
  }

  pub fn to_http_response(&mut self, image_cache: web::Data<Arc<Mutex<ImageCache>>>) -> HttpResponse {
    match self.prepare_http_response(image_cache.clone()) {
      Some(file_data) => HttpResponse::Ok().content_type(self.to_mime_str()).body(file_data),
      None => HttpResponse::NotFound().finish()
    }
  }

  /// prepares the http response by checking if the image is already cached else creating the image
  /// then caching it. the return value (value) is equal to "the image wasn't cached yet".
  fn prepare_http_response(&mut self, image_cache: web::Data<Arc<Mutex<ImageCache>>>) -> Option<Vec<u8>> {
    let mut cache = image_cache.lock().unwrap();
    // checking cache for image
    match cache.0.get(&self.file_name) {
      Some(file_data) => Some(file_data.clone()),   
      // if image not cached yet
      None => {
        // finding source file for creating filtered image
        for file in cache.0.keys() {
          if file.starts_with(&format!("{}.source", &self.name)) {
            self.make_file_from_source(file.clone());
            let file_data = fs::read(&self.file_name).unwrap();
            cache.0.insert(self.file_name.clone(), file_data.clone());
            return Some(file_data)
          }
        }
        // if no source file found
        None
      }
    }
  }

  fn make_file_from_source(&mut self, source_file: String) {
    match self.resize {
      Some(size) => {
        let (width, height) = split_last(size, 'x');
        if self.file_name == "webp" {
          Command::new("sh").arg("-c").arg(format!("webp -q 80 {} -o {} -resize {} {}", &source_file, self.file_name, width, height)).output().expect("failed command");
        }
        else {
          Command::new("sh").arg("-c").arg(format!("convert -resize {}x{} {} {}", if width == "0" { "10000" } else { width }, if height == "0" { "10000" } else { height }, &source_file, self.file_name)).output().expect("failed command");
        }
      }
      None => 
        if self.file_name == "webp" {
          Command::new("sh").arg("-c").arg(format!("webp -q 80 {} -o {}", &source_file, self.file_name)).output().expect("failed command");
        }
        else {
          self.file_name = source_file;
        }
    }
  }
}

#[derive(Debug,Clone)]
pub struct ImageCache(HashMap<String, Vec<u8>>);

impl ImageCache {
  pub fn new() -> std::io::Result<ImageCache> {
    let mut image_cache: HashMap<String, Vec<u8>> = HashMap::new();

    for file in fs::read_dir(".")? {
      match file?.file_name().to_str() {
        None => {},
        Some(file_name) => 
          if ["webp", "jpg", "jpeg", "png", "gif", "jfif"].contains(&file_name.split('.').next_back().unwrap()) {
            image_cache.insert(file_name.into(), fs::read(file_name)?);
          }
      }
    }
    Ok(ImageCache(image_cache))
  }
}


#[test]
fn image_cache_test() {
  let image_cache = ImageCache::new().unwrap();
  let file_names: Vec<&String> = image_cache.0.keys().collect();
  println!("{:?}", file_names);
}
 