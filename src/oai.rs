use std::error::Error;
use std::result::Result;
use std::thread;
use std::time::Duration;
use libxml::parser::Parser;

pub fn fetch_article_list_since(yyyymmdd: &str) -> Result<Vec<String>, Box<dyn Error>> {
  assert!(
    yyyymmdd.split("-").count() == 3,
    "expecting a date string in the format YYYY-MM-DD"
  );
  let oai_arxiv_url = format!(
    "http://export.arxiv.org/oai2?verb=ListIdentifiers&metadataPrefix=oai_dc&from={}",
    yyyymmdd
  );
  fetch_article_list_by_url(oai_arxiv_url)
}

pub fn fetch_article_list_by_url(url: String) -> Result<Vec<String>, Box<dyn Error>> {
  let client = reqwest::blocking::Client::builder()
    .user_agent("ar5iv (https://ar5iv.labs.arxiv.org)")
    .build()?;
  let mut ids = Vec::new();

  for _retries in 0..3 {
    if let Ok(resp) = client.get(&url).send() {
      let status = resp.status();
      if status != 200 {
        if status == 503 {
          thread::sleep(Duration::from_secs(10));
        }
      } else {
        let payload = resp.text()?;
        if !payload.is_empty() {
          let parser = Parser::default();
          if let Ok(doc) = parser.parse_string(payload) {
            if let Some(root) = doc.get_root_readonly() {
              for version_node in root
                .findnodes("//*[local-name()='identifier']", &doc)
                .unwrap_or_default()
              {
                let oai_id = version_node.get_content();
                if !oai_id.is_empty() {
                  ids.push(oai_id[14..].to_string());
                }
              }

              // Check for a resumption token, in which case we recurse into the next set:
              // <resumptionToken cursor=\"0\" completeListSize=\"34188\">6380191|10001</resumptionToken>
              let resumption_nodes = root.findnodes("//*[local-name()='resumptionToken']", &doc).unwrap_or_default();
              if let Some(resumption_node) = resumption_nodes.first() {
                let resume_token = resumption_node.get_content();
                if !resume_token.is_empty() {
                  ids.extend(fetch_article_list_resume(&resume_token)?);
                }
              }
            }
          }
        }
        break;
      }
    }
  }
  Ok(ids)
}

pub fn fetch_article_list_resume(token: &str) -> Result<Vec<String>, Box<dyn Error>> {
  let oai_arxiv_resume_url = format!(
    "http://export.arxiv.org/oai2?verb=ListIdentifiers&resumptionToken={}",
    token
  );
  fetch_article_list_by_url(oai_arxiv_resume_url)
}
