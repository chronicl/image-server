use glob::glob;
use std::fs;
use std::io::{Error, ErrorKind, Write};
use std::str::pattern::{Pattern, Searcher};

fn split_last(string: &str, delimiter: char) -> (&str, &str) {
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

pub struct Whitelist {
  dir_to_parse: String,
  preceded_by: Vec<String>,
  succeeded_by: Vec<char>,
}

impl Whitelist {
  pub fn new(dir_to_parse: &str, preceded_by: &str) -> Self {
    Whitelist {
      dir_to_parse: dir_to_parse.to_owned(),
      preceded_by: vec![preceded_by.to_owned()],
      succeeded_by: vec![' ', '/', '>', '"'],
    }
  }

  pub fn build(&self) {
    fs::File::create("whitelist").unwrap();
    for file_name in self.find_files_to_parse() {
      self.parse_file(&file_name).unwrap();
    }
  }

  fn find_files_to_parse(&self) -> Vec<String> {
    let mut file_paths = Vec::<String>::new();
    for entry in glob(&[&self.dir_to_parse, "/**/*"].join("")).expect("Failed to read glob pattern")
    {
      match entry
        .ok()
        .and_then(|e| e.to_str().map(|path| path.to_owned()))
        .and_then(|path| {
          let (_, ending) = split_last(&path, '.');
          if ["html", "js", "css"].contains(&ending) {
            return Some(path);
          } else {
            return None;
          }
        }) {
        Some(path) => {
          file_paths.push(path);
        }
        None => {}
      }
    }
    file_paths
  }

  fn parse_file(&self, input_file: &str) -> std::io::Result<()> {
    let content = fs::read_to_string(input_file)?;
    for pattern in self.preceded_by.clone() {
      let mut searcher = pattern.into_searcher(&content);
      let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("whitelist")
        .unwrap();
      while let Some((_, a)) = searcher.next_match() {
        match (&content[a..]).find(&self.succeeded_by[..]) {
          Some(b) => file.write_all([&content[a..(a + b)], "\n"].join("").as_bytes())?,
          None => return Err(Error::new(ErrorKind::Other, "Ending char not found")),
        }
      }
    }

    Ok(())
  }
}
