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
    File::options().write(true).open(destination_path)?
  } else {
    File::create(destination_path)?
  };
  let client = reqwest::blocking::Client::builder()
    .user_agent("ar5iv (https://ar5iv.labs.arxiv.org)")
    .build()?;

  for arxiv_id_batch in task_ids.chunks(4) {
    let ids_with_versions: Vec<_> = arxiv_id_batch
      .par_iter()
      .map(|id| (id, fish_out_version(&client, id)))
      .collect();
    for (id, version) in ids_with_versions {
      writeln!(dest_file, "{},{}", id, version)?;
    }
    thread::sleep(Duration::from_millis(200));
  }
  Ok(())
}

fn fish_out_version(client: &Client, arxiv_id: &str) -> usize {
  // try incrementing until we get a 404 for a version (also, we know v1 exists)
  let mut version_try = 2;
  let mut export_arxiv_url = format!("https://export.arxiv.org/abs/{}v{}", arxiv_id, version_try);
  while retry_check_url(client, &export_arxiv_url) {
    version_try += 1;
    export_arxiv_url = format!("https://export.arxiv.org/abs/{}v{}", arxiv_id, version_try);
  }
  version_try - 1
}

// We have a simple and efficient check:
// if the "/abs/IDvN" URL resolves with HTTP 200
// then version N exists. 404 (or other), we assume it doesn't.
fn retry_check_url(client: &Client, url: &str) -> bool {
  for _retries in 0..3 {
    if let Ok(resp) = client.head(url).send() {
      match resp.status().as_u16() {
        200 => return true,
        403 => panic!("This scraper has been forbidden from accessing export.arxiv.org, please contact an arXiv admin."),
        400 | 404 => return false,
        503 | 500 => thread::sleep(Duration::from_secs(10)),
        other => eprintln!("-- no handler for http code {}.", other)
      }
    }
  }
  false
}

// fn _oai_main() -> Result<(), Box<dyn std::error::Error>> {
//   let parser = Parser::default();
//   let client = reqwest::blocking::Client::new();
//   for arxiv_id in &[
//     "2203.11882",
//     "hep-ph/0702032",
//     "2203.11882",
//     "hep-ph/0702032",
//     "2203.11882",
//     "hep-ph/0702032",
//     "2203.11882",
//     "hep-ph/0702032",
//   ] {
//     let oai_arxivraw_url = format!("http://export.arxiv.org/oai2?verb=GetRecord&identifier=oai:arXiv.org:{}&metadataPrefix=arXivRaw",arxiv_id);
//     for _retries in 0..3 {
//       if let Ok(resp) = client.get(&oai_arxivraw_url).send() {
//         let status = resp.status();
//         if status != 200 {
//           if status == 503 {
//             thread::sleep(Duration::from_secs(10));
//           }
//         } else {
//           let payload = resp.text()?;
//           if !payload.is_empty() {
//             if let Ok(doc) = parser.parse_string(payload) {
//               if let Some(root) = doc.get_root_readonly() {
//                 let mut max_version = 1;
//                 for version_node in root
//                   .findnodes("//*[local-name()='version']", &doc)
//                   .unwrap_or_default()
//                 {
//                   if let Some(mut version_value) = version_node.get_attribute("version") {
//                     if version_value.starts_with('v') {
//                       version_value.remove(0);
//                     }
//                     let version_u8 = version_value.parse::<u8>().unwrap_or(1);
//                     if version_u8 > max_version {
//                       max_version = version_u8;
//                     }
//                   }
//                 }
//                 dbg!((arxiv_id, max_version));
//                 break; // no need to retry further.
//               }
//             }
//           }
//         }
//       }
//     }
//   }
// Ok(())
// }
