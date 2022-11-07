use ar5iv_util::local::{create_list_of_ids, filter_list_to_check};
use ar5iv_util::remote::check_ids_http;
use std::error::Error;

const UNCHECKED_IDS_FILEPATH: &str = "unchecked_ids.csv";
const CHECKED_IDS_FILEPATH: &str = "checked_ids.csv";
const CORPUS_ROOT_PATH: &str = "/data/arxmliv";
// Importantly, we want to make this script reentrant,:&'static str
// so that any interruptions lead to resuming from the previous point
// as quickly as possible.
fn main() -> Result<(), Box<dyn Error>> {
  create_list_of_ids(CORPUS_ROOT_PATH, UNCHECKED_IDS_FILEPATH)?;
  let task_ids = filter_list_to_check(UNCHECKED_IDS_FILEPATH, CHECKED_IDS_FILEPATH)?;
  check_ids_http(task_ids, CHECKED_IDS_FILEPATH)
}
