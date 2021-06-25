use std::{
    error,
    env,
    fs,
    path::PathBuf,
    path::Path,
    collections::HashMap,
    time::Duration,
};
//use reqwest::blocking::Client;
use reqwest::StatusCode;
use reqwest::Error;
use serde::Deserialize;

const DEEPL_KEY : &str  = "811746cf-4fe6-01a0-f728-4b0e6aff6373";
const DEEPL_ENDPOINT : &str = "https://api.deepl.com/v2/document";


#[derive(Deserialize)]
struct FileInfo {
    document_id: String,
    document_key:String,
}

#[derive(Deserialize,Debug)]
struct FileState {
    document_id: String,
    status:String,
    seconds_remaining:u32,
    //billed_characters:String,  //this field only existing for paid account.
}

fn main() -> Result<(),Box<dyn error::Error>> {

    //&str vector storing file names of which are failed to translate.
    let mut bad_translaion: Vec<String> = Vec::new();
    //for those successful transaltion.
    let mut good_translaion: Vec<String> = Vec::new();

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
        "en - {:?}\nzhcn - {:?}\nzhtw - {:?}\n",
        en_path, zhcn_path, zhtw_path
    );


    println!(
        "items modified in the last 24 hours in {:?}:\n\n",
        ja_path,
    );

    //reqwest part
    let client = reqwest::blocking::Client::new();

    for entry in fs::read_dir(ja_path)? {
        let entry = entry?;
        let path = entry.path();
        let filename = path.file_name().unwrap().to_str().unwrap();

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
            println!("checking on file {:?} ...", &en_path);
            //if exists
            if Path::new(en_path.to_str().unwrap()).exists() {
                println!("file {:?} had been translated by someone, skip.\n",filename);
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
                    StatusCode::OK => {
                        let info = resp.json::<FileInfo>()?;
                        match retrieve_file(filename, &client, &info.document_id, &info.document_key) {
                            Ok(v) =>  println!("file under translating, remaining time: {:?}",v.seconds_remaining) ,
                            Err(e) =>  {
                                println!("file {:?} translating failed: {:?}\n",filename,e);
                                bad_translaion.push(filename.to_string());
                                continue;
                            }
                        };
                    }
                    StatusCode::PAYLOAD_TOO_LARGE => {
                        println!("Request payload is too large!");
                    }
                    s => println!("Received response status: {:?}", s),
                };
            }
            en_path.pop();
        }
        println!();
    }

    println!("file(s) successfully translated:\n{:#?}\n",good_translaion);
    println!("file(s) failed to translate:\n{:#?}\n",bad_translaion);

    Ok(())
}

fn retrieve_file(filename: &str, client: &reqwest::blocking::Client, id: &str, key: &str) -> Result<FileState,reqwest::Error>  {
    println!("Retrieving translated file {:?} ...", filename);
    //TODO...
    let url = format! ("{}/{}",DEEPL_ENDPOINT , id);

    let params = [("auth_key",DEEPL_KEY),("document_key",key)];

    let resp = client.post(&url)
    .form(&params)
    .send()?;
    println!("{}",resp.status());

    let state = resp.json::<FileState>();
    match state {
        Ok(ref v) => {
            println!("state of the translation process = {:#?}", v.status);
            return state;
        }
        Err(e) => {
            println!("error:{:?}",e);
            panic!("{:?}",e);
        }
    }

}