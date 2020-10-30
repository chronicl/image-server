#![allow(dead_code)]
#![feature(str_split_once)]
use actix_web::{HttpResponse};
use std::fs;
use std::sync::{Arc, Mutex};
use actix_web::web;
use std::process::Command;
use std::collections::{HashMap};

pub fn split_last(string: &str, delimiter: char) -> (&str, &str) {
  for (i, c) in string.chars().rev().enumerate() {
    if c == delimiter {
      return (&string[..(string.len() - i - 1)], &string[(string.len() - i)..])
    }
  }
  (string, "")
}

pub fn split_back(string: &str, delimiter: char, mut count: u8) -> (&str, &str) {
  for (i, c) in string.chars().rev().enumerate() {
    if c == delimiter {
      count -= 1;
      if count == 0 {
        return (&string[..(string.len() - i - 1)], &string[(string.len() - i)..])
      }
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

    match image_cache.lock().unwrap().prepare_http_response(self) {
      Some(file_data) => HttpResponse::Ok().content_type(self.to_mime_str()).body(file_data),
      None => HttpResponse::NotFound().finish()
    }
  }
}

#[derive(Debug,Clone)]
pub struct ImageCache(HashMap<String, String>);

impl ImageCache {
  pub fn new() -> ImageCache {
    ImageCache( HashMap::new())
  }

  fn update_sources(&mut self) -> std::io::Result<()> {    
    println!("updating sources");
    let mut sources = HashMap::<String, String>::new();
    for file in fs::read_dir(".")? {
      match file?.file_name().to_str() {
        None => {},
        Some(file_name) =>
          if file_name.contains(".source") {
            let (name, _image_type) = split_back(file_name, '.', 2);
            sources.insert(name.into(), file_name.into());
          }
      }
    }
    self.0 = sources;
    Ok(())
  }

  fn prepare_http_response(&mut self, image: &mut Image) -> Option<Vec<u8>> {
    // checking cache for image
    match self.0.get(&image.file_name) {
      Some(file_name) => Some(fs::read(file_name).unwrap()),   
      // if image not cached yet
      None => {
        // finding source file for creating filtered image
        let mut source_file = self.0.get(image.name);
        if source_file.is_none() {
            self.update_sources().expect("failed to update sources");
            source_file = self.0.get(image.name);
        }
        if let Some(source_file) = source_file {
          self.make_file_from_source(source_file.clone(), image);

          let file_data = fs::read(&image.file_name).unwrap();
          self.0.insert(image.file_name.clone(), image.file_name.clone());

          return Some(file_data)
        }
        // if no source file found
        None
      }
    }
  }

  fn make_file_from_source(&self, source_file: String, image: &mut Image) {
    match image.resize {
      Some(size) => {
        let (width, height) = split_last(size, 'x');
        if  image.image_type == "webp" {
          Command::new("sh").arg("-c").arg(format!("webp -q 80 {} -o {} -resize {} {}", &source_file, image.file_name, width, height)).output().expect("failed command");
        }
        else {
          Command::new("sh").arg("-c").arg(format!("convert -resize {}x{} {} {}", if width == "0" { "10000" } else { width }, if height == "0" { "10000" } else { height }, &source_file, image.file_name)).output().expect("failed command");
        }
      }
      None => 
        if image.file_name == "webp" {
          Command::new("sh").arg("-c").arg(format!("webp -q 80 {} -o {}", &source_file, image.file_name)).output().expect("failed command");
        }
        else {
          image.file_name = source_file;
        }
    }
  }
}


#[test]
fn image_cache_test() {
  let image_cache = ImageCache::new();
  let file_names: Vec<&String> = image_cache.0.keys().collect();
  println!("{:?}", file_names);
}

#[test]
fn update_sources_test() {
  let mut image_cache = ImageCache::new();
  image_cache.update_sources().unwrap();
  println!("{:?}", image_cache.0);
}