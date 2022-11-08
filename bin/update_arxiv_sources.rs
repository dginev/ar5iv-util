#![feature(iter_array_chunks)]
#![feature(array_zip)]
use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::thread;
use std::time::{Instant, Duration};
use std::path::Path;

use regex::Regex;
use reqwest::blocking::Client;
use rayon::prelude::*;

use ar5iv_util::local::{CORPUS_ROOT_PATH, IDS_TO_UPDATE_FILEPATH, UNCHECKED_IDS_FILEPATH};
use ar5iv_util::local::repackage_arxiv_download;

const NUM_THREADS : usize = 4;
const RESUME_LOG_FILEPATH : &str = "already_updated.log";

fn main() -> Result<(), Box<dyn Error>> {
  let start_time = Instant::now();
  let mut args = env::args();
  let _ = args.next();
  let ids_to_update_path = args
    .next()
    .unwrap_or_else(|| String::from(IDS_TO_UPDATE_FILEPATH));
  let unchecked_ids_path = args
    .next()
    .unwrap_or_else(|| String::from(UNCHECKED_IDS_FILEPATH));

  // load the Set of ids to update.
  let all_ids_to_update = build_set(&ids_to_update_path);
  // load the Set of local ids we have available
  let all_local_ids = build_set(&unchecked_ids_path);
  // load the Set of already covered ids
  let already_updated = build_set(RESUME_LOG_FILEPATH);
  // save newly updated files to allow easy resume.
  let mut resume_file = if Path::new(RESUME_LOG_FILEPATH).exists() {
    File::options().write(true).append(true).open(RESUME_LOG_FILEPATH)?
  } else {
    File::create(RESUME_LOG_FILEPATH)?
  };
  // cover the intersection
  let mut ids_to_update = all_ids_to_update.into_iter().filter(|e| all_local_ids.contains(e) && !already_updated.contains(e));

  let slash_regex = Regex::new("^([^/]+)/([^/]+)$").unwrap();

  // reuse a group of download Clients
  let clients : Vec<Client> = (0..NUM_THREADS).map(|_| reqwest::blocking::Client::builder()
    .user_agent("ar5iv (https://ar5iv.labs.arxiv.org)")
    .timeout(Duration::from_secs(60))
    .build().unwrap()).collect();
  let mut updated = 0;
  while let Some(batch_id) = ids_to_update.next() {
    let mut batch = vec![(batch_id, &clients[0])];
    for this_client in clients.iter().take(NUM_THREADS).skip(1) {
      if let Some(nid) = ids_to_update.next() {
        batch.push((nid, this_client));
      }
    }
    let _downloaded_results: Vec<_> = batch.par_iter().map(|(id, client)| {
      // the URL we download from
      let url = format!("https://export.arxiv.org/e-print/{}", id);
      let mut update_ok = false;
      for _retry in 0..3 {
        if let Ok(payload) = client.get(&url).send() {
          if payload.status() == 200 {
            if let Ok(bytes) = payload.bytes() {
              // only execute if we get some bytes
              if !bytes.is_empty() {
                let (to_dir,base_name) = if let Some(cap) = slash_regex.captures(id) {
                  let base = cap.get(1).unwrap().as_str();
                  let id = cap.get(2).unwrap().as_str();
                  let mmyy = &id[..4];
                  (format!("{}/{}/{}{}", CORPUS_ROOT_PATH, mmyy, base, id),
                  format!("{}{}",base, id))
                } else {
                  let mmyy = &id[..4];
                  (format!("{}/{}/{}", CORPUS_ROOT_PATH, mmyy, id),
                  id.to_owned())
                };
                repackage_arxiv_download(&mut bytes.to_vec(), to_dir, base_name);
                update_ok = true;
              }
            }
            break;
          }
        }
      }
      if !update_ok {
        panic!("Failed to update {}; debug request: {:}", url, client.get(&url).send().unwrap().status());
      }
    }).collect();
    updated += batch.len();
    if updated % 100 == 0 {
      eprintln!("-- updated {} articles in {} sec...", updated, (Instant::now()-start_time).as_secs());
      dbg!(&batch);
    }
    // save in resume after the full batch finishes to avoid data races.
    for (id, _) in batch {
      writeln!(resume_file, "{}", id)?;
    }
    // courtesy sleep for reducing the load on arXiv's infra.
    thread::sleep(Duration::from_secs(1));
  }
  eprintln!("-- Done: updated {} articles in {} sec.", updated, (Instant::now()-start_time).as_secs());
  Ok(())
}

fn build_set(path: &str) -> HashSet<String> {
  if let Ok(file) = File::open(path) {
    let reader = BufReader::new(file);
    let mut set = HashSet::new();
    for line in reader.lines().flatten() {
      set.insert(line);
    }
    set
  } else {
    HashSet::new()
  }
}
