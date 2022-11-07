use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::path::Path;

use lazy_static::lazy_static;
use regex::Regex;

use jwalk::WalkDir;

lazy_static! {
  static ref LETTER_DIGIT_REGEX: Regex = Regex::new("(^\\D+)(\\d.+)$").unwrap();
}

pub fn create_list_of_ids(root_path: &str, unchecked_filepath: &str) -> Result<(), Box<dyn Error>> {
  // only do this once, i.e. if the file exists - skip.
  let unchecked_path = Path::new(unchecked_filepath);
  if unchecked_path.exists() {
    return Ok(());
  }

  let mut unchecked_file = File::create(unchecked_filepath)?;

  for entry in WalkDir::new(root_path)
    .follow_links(true)
    .sort(true)
    .max_depth(2)
    .min_depth(2)
    .into_iter()
    .flatten()
  {
    let id = entry.file_name().to_str().unwrap();
    if let Some(cap) = LETTER_DIGIT_REGEX.captures(id) {
      writeln!(
        unchecked_file,
        "{}/{}",
        cap.get(1).unwrap().as_str(),
        cap.get(2).unwrap().as_str()
      )?;
    } else {
      writeln!(unchecked_file, "{}", id)?;
    }
  }
  Ok(())
}

pub fn filter_list_to_check(
  unchecked_filepath: &str,
  checked_filepath: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
  // create a HashSet of the ids already checked
  let checked_path = Path::new(checked_filepath);
  let checked_set = if !checked_path.exists() {
    HashSet::new()
  } else {
    let checked_file = File::options().read(true).open(checked_filepath)?;
    let reader = BufReader::new(checked_file);
    let mut set = HashSet::new();
    for line in reader.lines().flatten() {
      set.insert(line);
    }
    set
  };
  // load the uncecked ids and avoid checking them twice.
  let unchecked_file = File::options().read(true).open(unchecked_filepath)?;
  let reader = BufReader::new(unchecked_file);

  let list_to_check = reader
    .lines()
    .map(|line| line.unwrap_or_default())
    .filter(|line| !line.is_empty() && !checked_set.contains(line))
    .collect();
  Ok(list_to_check)
}
