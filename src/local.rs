use std::collections::HashSet;
use std::error::Error;
use std::fs::{self,File};
use std::io::{prelude::*, BufReader};
use std::path::Path;

use lazy_static::lazy_static;
use regex::Regex;
use Archive::*;
use jwalk::WalkDir;

pub const UNCHECKED_IDS_FILEPATH: &str = "unchecked_ids.txt";
pub const IDS_TO_UPDATE_FILEPATH: &str = "ids_to_update.txt";
pub const CHECKED_IDS_FILEPATH: &str = "checked_ids.csv";
pub const CORPUS_ROOT_PATH: &str = "/data/arxmliv";

const BUFFER_SIZE: usize = 10_240;

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
      writeln!(unchecked_file, "{id}")?;
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
    for line in reader
      .lines()
      .map_while(Result::ok)
      .map(|l| l.split(',').next().unwrap().to_owned())
    {
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
    .filter(|unchecked_line| !unchecked_line.is_empty() && !checked_set.contains(unchecked_line))
    .collect();
  Ok(list_to_check)
}

pub fn repackage_arxiv_download(memory: &mut [u8], to_dir: String, base_name: String) {
  let default_tex_target = base_name.to_string() + ".tex";
  fs::create_dir_all(&to_dir).unwrap_or_else(|reason| {
    println!(
      "Failed to mkdir -p {:?} because: {:?}",
      to_dir.clone(),
      reason.kind()
    );
  });
  // We'll write out a ZIP file for each entry
  let mut archive_writer_new = Writer::new()
    .unwrap()
    //.add_filter(ArchiveFilter::Lzip)
    // .set_compression(ArchiveFilter::None)
    .set_format(ArchiveFormat::Zip);
  let to_path = format!("{to_dir}/{base_name}.zip");
  archive_writer_new
    .open_filename(&to_path)
    .unwrap();

  // Careful here, some of arXiv's .gz files are really plain-text TeX files (surprise!!!)
  let mut raw_read_needed = false;
  match Reader::new()
    .unwrap()
    .support_filter_all()
    .support_format_all()
    .open_memory(memory)
  {
    Err(_) => raw_read_needed = true,
    Ok(archive_reader) => {
      let mut file_count = 0;
      while let Ok(e) = archive_reader.next_header() {
        file_count += 1;
        match archive_writer_new.write_header(e) {
          Ok(_) => {}, // TODO: If we need to print an error message, we can do so later.
          Err(e2) => println!("Header write failed: {e2:?}"),
        };
        while let Ok(chunk) = archive_reader.read_data(BUFFER_SIZE) {
          archive_writer_new.write_data(chunk).unwrap();
        }
      }
      if file_count == 0 {
        // Special case (bug? in libarchive crate), single file in .gz
        raw_read_needed = true;
      }
    },
  }

  if raw_read_needed {
    let raw_reader_new = Reader::new()
      .unwrap()
      .support_filter_all()
      .support_format_raw()
      .open_memory(memory);
    match raw_reader_new {
      Ok(raw_reader) => match raw_reader.next_header() {
        Ok(_) => {
          single_file_transfer(&default_tex_target, &raw_reader, &mut archive_writer_new);
        },
        Err(_) => println!("No content in archive: {to_dir:?}"),
      },
      Err(_) => println!("Unrecognizeable archive: {to_dir:?}"),
    }
  }
}


/// Transfer the data contained within `Reader` to a `Writer`, assuming it was a single file
pub fn single_file_transfer(tex_target: &str, reader: &Reader, writer: &mut Writer) {
  // In a "raw" read, we don't know the data size in advance. So we bite the
  // bullet and read the usually tiny tex file in memory,
  // obtaining a size estimate
  let mut raw_data = Vec::new();
  while let Ok(chunk) = reader.read_data(BUFFER_SIZE) {
    raw_data.extend(chunk.into_iter());
  }
  let mut ok_header = false;
  match writer.write_header_new(tex_target, raw_data.len() as i64) {
    Ok(_) => {
      ok_header = true;
    },
    Err(e) => {
      println!("Couldn't write header: {e:?}");
    },
  }
  if ok_header {
    match writer.write_data(raw_data) {
      Ok(_) => {},
      Err(e) => println!("Failed to write data to {tex_target:?} because {e:?}"),
    };
  }
}
