use std::{
    error,
    env,
    fs,
    path::PathBuf,
    path::Path
};
//use reqwest::blocking::Client;
use reqwest::StatusCode;

const DEEPL_KEY : &str  = "811746cf-4fe6-01a0-f728-4b0e6aff6373";
const DEEPL_ENDPOINT : &str = "https://api.deepl.com/v2/document";

fn main() -> Result<(),Box<dyn error::Error>> {
    let ja_path = env::current_dir()?;
    let parent_path = ja_path.parent().unwrap();
    //en
    let mut en_path = PathBuf::from(parent_path);
    en_path.push("en");
    
    //zhcn
    let mut zhcn_path = PathBuf::from(parent_path);
    zhcn_path.push("zhcn");

    //zhtw
    let mut zhtw_path = PathBuf::from(parent_path);
    zhtw_path.push("zhtw");
    
    println!(
        "en - {:?} zhcn - {:?} zhtw - {:?}",
        en_path, zhcn_path, zhtw_path
    );


    println!(
        "Entries modified in the last 24 hours in {:?}:",
        ja_path,
    );

    //reqwest part
    let client = reqwest::blocking::Client::new();

    for entry in fs::read_dir(ja_path)? {
        let entry = entry?;
        let path = entry.path();
        let filename = &path.file_name().unwrap();

        let metadata = fs::metadata(&path)?;
        let last_modified = metadata.modified()?.elapsed()?.as_secs();

        if last_modified < 24 * 3600 && metadata.is_file() {
            println!(
                "Last modified: {:?} seconds, is read only: {:?}, size: {:?} bytes, filename: {:?}",
                last_modified,
                metadata.permissions().readonly(),
                metadata.len(),
                path.file_name().ok_or("No filename")?
            );

            //check if file exists in en_path
            en_path.push(filename);
            //  en_path = "/home/nts/rust/en/Cargo.lock"
            println!("checking if the file `{:?}` has been already translated...", &en_path);
            //if exists
            if Path::new(en_path.to_str().unwrap()).exists() {
                continue;
            } else {
                //do the translate
                //English
                println!("starting translate {:?}", &path);
                let form = reqwest::blocking::multipart::Form::new()
                    .text("source_lang","JA")
                    .text("target_lang","EN-US")
                    .text("auth_key",DEEPL_KEY)
                    .file("file",&path)?;
                
                let resp = client.post(DEEPL_ENDPOINT)
                    .multipart(form)
                    .send()?;
                //response received
                match resp.status() {
                    //in case of success
                    StatusCode::OK => retrieve_file(&filename.to_str().unwrap()),
                    StatusCode::PAYLOAD_TOO_LARGE => {
                        println!("Request payload is too large!");
                    }
                    s => println!("Received response status: {:?}", s),
                };
            }
            en_path.pop();
        }

    }

    Ok(())
}

fn retrieve_file(filename: &str){
    println!("Retrieving file `{}`...", filename);
    //TODO...

}