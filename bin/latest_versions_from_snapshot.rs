use serde_json::{Deserializer, Value};
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;
use chrono::{DateTime, TimeZone, FixedOffset};
use once_cell::sync::Lazy;

// When was the last date a full update was ran?
pub static LAST_UPDATE : Lazy<DateTime<FixedOffset>> = Lazy::new(||
  FixedOffset::east_opt(0).unwrap().with_ymd_and_hms(2022, 11, 1, 0, 0, 0).unwrap()
);

fn main() -> Result<(), Box<dyn Error>> {
  // JSON obtained from:
  // https://www.kaggle.com/datasets/1b6883fb66c5e7f67c697c2547022cc04c9ee98c3742f9a4d6c671b4f4eda591?resource=download&select=arxiv-metadata-oai-snapshot.json
  let snapshot_path = Path::new("arxiv-metadata-oai-snapshot.json");
  let snapshot_file = File::open(snapshot_path)?;
  let reader = BufReader::new(snapshot_file);
  let stream = Deserializer::from_reader(reader).into_iter::<Value>();
  // gather a simple list, one id per line
  let mut gather_file = File::create("multi_version_ids.txt")?;
  let mut total_gathered = 0;

  for value_result in stream {
    let value = value_result?;
    let versions = value.get("versions").unwrap().as_array().unwrap();
    for val in versions {
      let created : DateTime<FixedOffset> = DateTime::parse_from_rfc2822(
      val.get("created").unwrap().as_str().unwrap())?;
      if *LAST_UPDATE < created {
        let v = val.get("version").unwrap().as_str().unwrap();
        if v != "v1" { // we have v1 from the S3 bucket downloads
          total_gathered += 1;
          let value_str = value.get("id").unwrap().as_str().unwrap();
          writeln!(gather_file, "{value_str}")?;
          break;
        }
      }
    }
  }

  eprintln!(
    "-- gathered {total_gathered} aritcle ids with version 2 or up.");

  Ok(())
}
