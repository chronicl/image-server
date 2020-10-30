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
    let is_new_file = self.prepare_http_response(image_cache.clone());

    if let Ok(file) = fs::read(&self.file_name) {
      if is_new_file {image_cache.lock().unwrap().0.push(String::from(&self.file_name));}
      HttpResponse::Ok().content_type(self.to_mime_str()).body(file)
    } else {
      HttpResponse::NotFound().finish()
    }
  }

  /// prepares the http response by checking if the image is already cached else creating the image
  /// then caching it. the return value (value) is equal to "the image wasn't cached yet".
  fn prepare_http_response(&mut self, image_cache: web::Data<Arc<Mutex<ImageCache>>>) -> bool {
    let cache = image_cache.lock().unwrap();
    if cache.0.contains(&self.file_name) {
      return false
    } 
    else {
      let mut source_file: Option<String> = None;
      for file in cache.0.iter() {
        if file.starts_with(&format!("{}.source", &self.name)) {
          source_file = Some(file.clone());
          break
        }
      }
      if let Some(source_file) = source_file {
        match self.resize {
          Some(size) => {
            let (width, height) = split_last(size, 'x');
            if self.image_type == "webp" {
              Command::new("sh").arg("-c").arg(format!("webp -q 80 {} -o {} -resize {} {}", &source_file, self.file_name, width, height)).output().expect("failed command");
            }
            else {
              Command::new("sh").arg("-c").arg(format!("convert -resize {}x{} {} {}", if width == "0" { "10000" } else { width }, if height == "0" { "10000" } else { height }, &source_file, self.file_name)).output().expect("failed command");
            }
            return true
          }
          None => {
            if self.image_type == "webp" {
              Command::new("sh").arg("-c").arg(format!("webp -q 80 {} -o {}", &source_file, self.file_name)).output().expect("failed command");
            }
            else {
              self.file_name = source_file;
            }
            return true

          }
        }
      } else {
        return true
      }



    }
  }
}

#[derive(Debug)]
pub struct ImageCache(Vec<String>);

impl ImageCache {
  pub fn new() -> std::io::Result<ImageCache> {
    let image_cache_vec = &mut Vec::<String>::new();
    for file in fs::read_dir(".")? {
      file?.file_name().to_str().and_then(|file_name| Some(image_cache_vec.push(file_name.into())));
    }
    Ok(ImageCache(image_cache_vec.clone()))
  }
}


#[test]
fn image_cache_test() {
  let image_cache = ImageCache::new().unwrap();
  println!("{:?}", image_cache.0);
}
 