#[cfg(test)]
mod test {
  use image_filter::split_last;
  #[test]
  fn split_last_test() {
    assert!(("grin.source", "jpeg") == split_last("grin.source.jpeg", '.'));
    assert!(("wow", "") == split_last("woww", 'w'));
  }
}