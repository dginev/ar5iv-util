use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::thread;
use std::time::Duration;

use rayon::prelude::*;
use reqwest::blocking::Client;

pub fn check_ids_http(
  task_ids: Vec<String>,
  destination_filepath: &str,
) -> Result<(), Box<dyn Error>> {
  let destination_path = Path::new(destination_filepath);
  let mut dest_file = if destination_path.exists() {
    File::options()
      .append(true)
      .open(destination_path)?
  } else {
    File::create(destination_path)?
  };
  let client = reqwest::blocking::Client::builder()
    .user_agent("ar5iv (https://ar5iv.labs.arxiv.org)")
    .build()?;

  for arxiv_id_batch in task_ids.chunks(4) {
    let ids_with_versions: Vec<_> = arxiv_id_batch
      .par_iter()
      .map(|id| (id, fish_out_article_version(&client, id)))
      .collect();
    for (id, version) in ids_with_versions {
      writeln!(dest_file, "{id},{version}")?;
    }
    thread::sleep(Duration::from_secs(1));
  }
  Ok(())
}

fn fish_out_article_version(client: &Client, arxiv_id: &str) -> usize {
  // try incrementing until we get a 404 for a version (also, we know v1 exists)
  let mut version_try = 2;
  let mut export_arxiv_url = format!("https://export.arxiv.org/abs/{arxiv_id}v{version_try}");
  while retry_check_url(client, &export_arxiv_url) {
    version_try += 1;
    export_arxiv_url = format!("https://export.arxiv.org/abs/{arxiv_id}v{version_try}");
    thread::sleep(Duration::from_secs(1));
  }
  version_try - 1
}

// We have a simple and efficient check:
// if the "/abs/IDvN" URL resolves with HTTP 200
// then version N exists. 404 (or other), we assume it doesn't.
fn retry_check_url(client: &Client, url: &str) -> bool {
  for _retries in 0..3 {
    match client.head(url).send() {
      Ok(resp) => match resp.status().as_u16() {
        200 => return true,
        403 => panic!("This scraper has been forbidden from accessing export.arxiv.org, please contact an arXiv admin."),
        400 | 404 => return false,
        503 | 500 => thread::sleep(Duration::from_secs(10)),
        other => eprintln!("-- no handler for http code {other}.")
      },
      Err(e) => {
        if e.is_connect() {
          panic!("Failed to connect, aborting. Reason: {e:?}");
        }
      }
    }
  }
  false
}
