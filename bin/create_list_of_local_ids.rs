/// This is a one-time job to be run at the beginning of a global update,
/// which walks the local corpus and collects the ids available.
/// We only update *already available* ids.

use ar5iv_util::local::{
  create_list_of_ids, CORPUS_ROOT_PATH, UNCHECKED_IDS_FILEPATH,
};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
  eprintln!("-- gathering ids from local arXiv corpus directory");
  create_list_of_ids(CORPUS_ROOT_PATH, UNCHECKED_IDS_FILEPATH)?;
  eprintln!("-- Done!");
  Ok(())
}
