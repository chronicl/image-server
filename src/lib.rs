#![allow(dead_code)]
#![feature(pattern)]
use actix_web::web;
use actix_web::HttpResponse;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::process::Command;
use std::sync::{Arc, Mutex};
pub mod whitelist;

pub fn split_last(string: &str, delimiter: char) -> (&str, &str) {
  for (i, c) in string.chars().rev().enumerate() {
    if c == delimiter {
      return (
        &string[..(string.len() - i - 1)],
        &string[(string.len() - i)..],
      );
    }
  }
  (string, "")
}

fn split_back(string: &str, delimiter: char, mut count: u8) -> (&str, &str) {
  for (i, c) in string.chars().rev().enumerate() {
    if c == delimiter {
      count -= 1;
      if count == 0 {
        return (
          &string[..(string.len() - i - 1)],
          &string[(string.len() - i)..],
        );
      }
    }
  }
  (string, "")
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct Filter {
  // width has higher priority than height (only aspect ratio keeping resizing possible)
  width: Option<u32>,
  height: Option<u32>,
  quality: Option<u8>,
  webp: Option<()>,
}

impl Filter {
  pub fn from_qs(qs: &str) -> Result<Filter, serde_qs::Error> {
    serde_qs::from_str::<Filter>(qs)
  }
  fn get_command(self, input: &str, output: &str) -> String {
    if self.webp.is_some() {
      let resize: String;
      if let Some(width) = self.width {
        resize = format!("-resize {} 0", width);
      } else if let Some(height) = self.height {
        resize = format!("-resize 0 {}", height);
      } else {
        resize = "".into();
      }

      format!(
        "cwebp {} {} {} -o {}",
        self
          .quality
          .map(|q| format!("-q {}", q))
          .unwrap_or_else(|| "".into()),
        resize,
        input,
        output
      )
    } else {
      let mut filter_options: String;
      if let Some(width) = self.width {
        filter_options = format!("-resize {}", width);
      } else if let Some(height) = self.height {
        filter_options = format!("-resize x{}", height);
      } else {
        filter_options = "".into();
      }

      if let Some(quality) = self.quality {
        filter_options = format!("{} -q {}", filter_options, quality)
      } else if filter_options == "" {
        return format!("cp {} {}", input, output);
      }

      format!("convert {} {} {}", filter_options, input, output)
    }
  }
}

#[derive(Debug)]
pub struct Image<'a> {
  name: &'a str,
  image_type: &'a str,
  filter: Option<Filter>,
  file_name: String,
}

impl<'a> Image<'a> {
  pub fn new(file_name: &'a str) -> Self {
    let (name, image_type) = split_last(file_name, '.');

    Image {
      name,
      image_type,
      filter: None,
      file_name: file_name.into(),
    }
  }

  pub fn filter(&mut self, filter: Option<Filter>) -> &mut Self {
    self.filter = filter.map(|mut f| {
      if self.image_type == "webp" {
        f.webp = Some(());
        f
      } else {
        f
      }
    });
    if let Some(filter) = self.filter {
      self.file_name = {
        let mut filter_options: String;
        if let Some(width) = filter.width {
          filter_options = format!(".w{}", width);
        } else if let Some(height) = filter.height {
          filter_options = format!(".h{}", height);
        } else {
          filter_options = "".into();
        }

        if let Some(quality) = filter.quality {
          filter_options = format!(
            "{}q{}",
            if filter_options == "" {
              ".".into()
            } else {
              filter_options
            },
            quality
          )
        }
        format!("{}{}.{}", self.name, filter_options, self.image_type)
      }
    }

    self
  }

  pub fn filter_from_qs(&mut self, qs: &str) -> &mut Self {
    if qs == "" {
      return self;
    };
    self.filter(Filter::from_qs(qs).ok())
  }

  fn to_mime_str(&self) -> String {
    format!("image/{}", self.image_type)
  }

  pub fn to_http_response(
    &mut self,
    image_cache: web::Data<Arc<Mutex<ImageCache>>>,
  ) -> HttpResponse {
    match image_cache.lock().unwrap().get_image_data(self) {
      Some(file_data) => HttpResponse::Ok()
        .content_type(self.to_mime_str())
        .body(file_data),
      None => HttpResponse::NotFound().finish(),
    }
  }

  fn get_command(&self, source_file: &str) -> String {
    match self.filter {
      None => format!("cp {} {}", source_file, self.file_name),
      Some(filter) => filter.get_command(&source_file, &self.file_name),
    }
  }
}

#[derive(Debug, Clone)]
pub struct ImageCache {
  // file_name -> file_data
  images: HashMap<String, web::Bytes>,
  // example: image_name -> image_name.source.jpg
  sources: HashMap<String, String>,
}

impl ImageCache {
  pub fn new() -> ImageCache {
    ImageCache {
      images: HashMap::new(),
      sources: HashMap::new(),
    }
  }

  fn update_sources(&mut self) -> std::io::Result<()> {
    println!("updating sources");
    let mut sources = HashMap::<String, String>::new();
    for file in fs::read_dir(".")? {
      match file?.file_name().to_str() {
        None => {}
        Some(file_name) => {
          if file_name.contains(".source") {
            let (name, _image_type) = split_back(file_name, '.', 2);
            sources.insert(name.into(), file_name.into());
          }
        }
      }
    }
    self.sources = sources;
    Ok(())
  }

  fn get_image_data(&mut self, image: &mut Image) -> Option<web::Bytes> {
    // checking cache for image
    match self.images.get(&image.file_name) {
      Some(file_data) => Some(file_data.clone()),
      // if image not cached yet
      None => {
        // finding source file for creating filtered image
        let mut source_file = self.sources.get(image.name);
        if source_file.is_none() {
          self.update_sources().expect("failed to update sources");
          source_file = self.sources.get(image.name);
        }
        if let Some(source_file) = source_file {
          if Command::new("sh")
            .arg("-c")
            .arg(image.get_command(&source_file))
            .output()
            .is_err()
          {
            println!("errored");
            return None;
          }

          let file_data = fs::read(&image.file_name).unwrap();
          self
            .sources
            .insert(image.file_name.clone(), image.file_name.clone());

          let bytes = web::Bytes::from(file_data);
          self.images.insert(image.file_name.clone(), bytes.clone());
          return Some(bytes);
        }
        // if no source file found
        None
      }
    }
  }
}

#[test]
fn image_cache_test() {
  let image_cache = ImageCache::new();
  let file_names: Vec<&String> = image_cache.images.keys().collect();
  println!("{:?}", file_names);
}

#[test]
fn update_sources_test() {
  let mut image_cache = ImageCache::new();
  image_cache.update_sources().unwrap();
  println!("{:?}", image_cache.sources);
}

#[test]
fn filter_test() {
  println!("{:?}", Filter::from_qs("width=500").unwrap());
}

#[test]
fn command_test() {
  if Command::new("sh")
    .arg("-c")
    .arg(Image::new("grin.jpeg").get_command("grin.source.jpeg"))
    .output()
    .is_err()
  {};
}
