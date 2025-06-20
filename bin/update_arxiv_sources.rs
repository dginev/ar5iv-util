#![feature(iter_array_chunks)]
use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
// use std::thread;
use std::time::{Instant, Duration};
use std::path::Path;

use regex::Regex;
use reqwest::blocking::Client;
use rayon::prelude::*;

use ar5iv_util::local::{CORPUS_ROOT_PATH, IDS_TO_UPDATE_FILEPATH};//UNCHECKED_IDS_FILEPATH
use ar5iv_util::local::repackage_arxiv_download;

const NUM_THREADS : usize = 4;
const RESUME_LOG_FILEPATH : &str = "already_updated.log";

fn main() -> Result<(), Box<dyn Error>> {
  let start_time = Instant::now();
  let retry_indexes: [i32;3] = [0,1,2];
  let mut args = env::args();
  let _ = args.next();
  let ids_to_update_path = args
    .next()
    .unwrap_or_else(|| String::from(IDS_TO_UPDATE_FILEPATH));
  // let unchecked_ids_path = args
  //   .next()
  //   .unwrap_or_else(|| String::from(UNCHECKED_IDS_FILEPATH));

  // load the Set of ids to update.
  let all_ids_to_update = build_set(&ids_to_update_path);
  // load the Set of local ids we have available
  // let all_local_ids = build_set(&unchecked_ids_path);
  // load the Set of already covered ids
  let already_updated = build_set(RESUME_LOG_FILEPATH);
  // save newly updated files to allow easy resume.
  let mut resume_file = if Path::new(RESUME_LOG_FILEPATH).exists() {
    File::options().append(true).open(RESUME_LOG_FILEPATH)?
  } else {
    File::create(RESUME_LOG_FILEPATH)?
  };
  // cover the intersection
  // let mut ids_to_update = all_ids_to_update.into_iter().filter(|e| all_local_ids.contains(e) && !already_updated.contains(e));
  // recovery for 2308, also download fresh entries:
  let mut ids_to_update = all_ids_to_update.into_iter()
    .filter(|e| !already_updated.contains(e));

  let slash_regex = Regex::new("^([^/]+)/([^/]+)$").unwrap();

  // reuse a group of download Clients
  let clients : Vec<Client> = (0..NUM_THREADS).map(|_| reqwest::blocking::Client::builder()
    .user_agent("ar5iv (https://ar5iv.labs.arxiv.org)")
    .timeout(Duration::from_secs(120))
    .build().unwrap()).collect();
  let mut updated = 0;
  while let Some(batch_id) = ids_to_update.next() {
    let mut batch = vec![(batch_id, &clients[0])];
    for this_client in clients.iter().take(NUM_THREADS).skip(1) {
      if let Some(nid) = ids_to_update.next() {
        batch.push((nid, this_client));
      }
    }
    let _downloaded_ok: Vec<bool> = batch.par_iter().map(|(id, client)| {
      // the URL we download from
      let url_owned = format!("https://export.arxiv.org/e-print/{id}");
      let url = &url_owned;
      'retries: for _retry in &retry_indexes {
        if let Ok(payload) = client.get(url).send() {
          match payload.status().as_u16() {
            200 => match payload.bytes() {
              Ok(bytes) => {
                // only execute if we get some bytes
                if !bytes.is_empty() {
                  let (to_dir,base_name) = if let Some(cap) = slash_regex.captures(id) {
                    let base = cap.get(1).unwrap().as_str();
                    let id = cap.get(2).unwrap().as_str();
                    let mmyy = &id[..4];
                    (format!("{CORPUS_ROOT_PATH}/{mmyy}/{base}{id}"),
                    format!("{base}{id}"))
                  } else {
                    let mmyy = &id[..4];
                    (format!("{CORPUS_ROOT_PATH}/{mmyy}/{id}"),
                    id.to_owned())
                  };
                  repackage_arxiv_download(&mut bytes.to_vec(), to_dir, base_name);
                  break 'retries;
                } else {
                  eprintln!("Code 200 but no bytes returned; article id {id}.");
                }
              },
              Err(e) => {
                eprintln!("Code 200 but bytes returned had error {e:?}; article id {id}.");
              }
            },
            403 => {eprintln!("code 403 for article id {id}, skip."); break 'retries;},
            other => eprintln!("code {other} for article id {id}."),
          }
        }
      }
      true
    }).collect();
    updated += batch.len();
    if updated % 100 == 0 {
      eprintln!("-- updated {} articles in {} sec...", updated, (Instant::now()-start_time).as_secs());
      dbg!(&batch);
    }
    // Update: if we get a 403, it is almost always per author's request
    // so skip them as done.
    // // save in resume after the full batch finishes to avoid data races.
    // // *except* if issues were encountered, in which case we may want to revisit later...
    // // see for example the author-requested 403 here: https://export.arxiv.org/e-print/math/0607467
    // if downloaded_ok.into_iter().all(|a| a) {
    for (id, _) in batch {
      writeln!(resume_file, "{id}")?;
    }
    // }
    // courtesy sleep for reducing the load on arXiv's infra.
    // thread::sleep(Duration::from_secs(1));
  }
  eprintln!("-- Done: updated {} articles in {} sec.", updated, (Instant::now()-start_time).as_secs());
  Ok(())
}

fn build_set(path: &str) -> HashSet<String> {
  if let Ok(file) = File::open(path) {
    let reader = BufReader::new(file);
    let mut set = HashSet::new();
    for line in reader.lines().map_while(Result::ok) {
      set.insert(line);
    }
    set
  } else {
    HashSet::new()
  }
}
