use serde_json::{Deserializer, Value};
use std::fs::File;
use std::io::prelude::*;
use std::error::Error;
use std::io::BufReader;
use std::path::Path;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn Error>> {
  // JSON obtained from:
  // https://www.kaggle.com/datasets/1b6883fb66c5e7f67c697c2547022cc04c9ee98c3742f9a4d6c671b4f4eda591?resource=download&select=arxiv-metadata-oai-snapshot.json
  let snapshot_path = Path::new("arxiv-metadata-oai-snapshot.json");
  let snapshot_file = File::open(snapshot_path)?;
  let reader = BufReader::new(snapshot_file);
  let stream = Deserializer::from_reader(reader).into_iter::<Value>();

  let mut gather_file = File::create("multi_version_ids.json").unwrap();
  let mut gather_stats = HashMap::new();
  let mut total_gathered = 0;

  for value_result in stream {
    let value = value_result.unwrap();
    let version_count = value.get("versions").unwrap().as_array().unwrap().len();
    let stat_entry = gather_stats.entry(version_count).or_insert(0);
    *stat_entry +=1;

    if version_count > 1 {
      total_gathered+=1;
      let value_str = value.get("id").unwrap().as_str().unwrap();
      writeln!(gather_file, "{}", value_str)?;
    }
  }
  let mut stats_file = File::create("version_stats.json").unwrap();
  writeln!(stats_file, "{}", serde_json::to_string(&gather_stats).unwrap())?;

  eprintln!("-- gathered {} aritcle ids with version 2 or up.", total_gathered);

  Ok(())
}