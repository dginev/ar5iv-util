/// This script is meant to be ran periodically, ideally once every day with an arXiv update
///
use std::result::Result;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::process::Command;
use std::str;
use std::path::Path;

use ar5iv_util::oai::fetch_article_list_since;

pub fn main() -> Result<(), Box<dyn Error>> {
  // Step 0. Now the current date
  let today_exec = Command::new("date")
    .arg("-I")
    .output()
    .expect("made to run on a Unix box with 'date -I' available.");
  let mut today_stdout = today_exec.stdout;
  today_stdout.pop();
  let today =  str::from_utf8(&today_stdout)?;
  // Step 1. Obtain the list of all modified articles since last update, via OAI
  // last update is stored in `last_oai_update.txt`
  let last_oai_update_file = File::open("last_oai_update.txt")?;
  let reader = BufReader::new(last_oai_update_file);
  let last_date = reader.lines().last()
    .expect("The last line of last_oai_update.txt must contain a date.")
    .expect("The last line of last_oai_update.txt must contain a date.");
  let mut article_list = fetch_article_list_since(&last_date)?;
  eprintln!("oai listed {} entries to update.", article_list.len());
  article_list.sort();
  // 1.1 save in log/ for today.
  let oai_today_log_path_str = format!("./log/oai_ids_upto_{}.log", today);
  let oai_today_log_path = Path::new(&oai_today_log_path_str);
  let mut oai_log_file = File::create(oai_today_log_path)?;
  for article_id in article_list.into_iter() {
    writeln!(oai_log_file, "{}", article_id)?;
  }

  // Step 2. Fetch the sources of all articles that need update.

  // Step 3. For all successfully fetched articles, update CorTeX tasks to "TODO"

  // Step 4. Wrap up. If everything looks nominal, mark today's date as a successful update.

  Ok(())
}


/* --------------------------
  Side-note: This script assumes that an active CorTeX [1] dispatcher  is
  running in the background, and that a sufficient number of `tex_to_html` workers are active and ready to receive conversion jobs.

  [1] https://github.com/dginev/CorTeX/
  [2] https://github.com/dginev/LaTeXML-Plugin-Cortex
*/
