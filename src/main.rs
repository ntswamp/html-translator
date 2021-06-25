use std::{
    error,
    env,
    fs,
    fs::File,
    io::prelude::*,
    path::PathBuf,
    path::Path,
    time::Duration,
    thread,
};

//use reqwest::blocking::Client;
use reqwest::StatusCode;
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
    seconds_remaining:Option<u32>,
    //billed_characters:String,  //this field only existing for paid account.
}

enum Language {
    EN(String),
    ZHCN(String),
    ZHTW(String),
}

fn main() -> Result<(),Box<dyn error::Error>> {

    //TODO: verify wether the program can create/delete files on user's computer or not.

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

    'outer: for entry in fs::read_dir(ja_path)? {
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
                    .text("target_lang","ZH")
                    .text("auth_key",DEEPL_KEY)
                    .file("file",&path)?;
                
                let resp = client.post(DEEPL_ENDPOINT)
                    .multipart(form)
                    .send()?;
                //response received
                match resp.status() {
                    //in case of http request succeeded
                    StatusCode::OK => {

                        let info = resp.json::<FileInfo>()?;

                        loop {

                            match know_file_state(filename, &client, &info.document_id, &info.document_key) {
                                Ok(v) =>  {
                                    match v.as_str() {
                                        "error" => {
                                            println!("[ERROR]file {:?}'s translation was failed due to an unknown error.\n",filename);
                                            bad_translaion.push(filename.to_string());
                                            continue 'outer;
                                        }
                                        "done" => {
                                            let translated = download_file(filename, &client, &info.document_id, &info.document_key);
                                            match translated {
                                                Ok(v) => {
                                                    create_file(v);
                                                },
                                                Err(e) => {
                                                    println!("[ERROR]file was translated, but failed to download:{}",e);
                                                }
                                            }
                                            good_translaion.push(filename.to_string());
                                            break;
                                        }
                                        //still under translation
                                        _ => {
                                            thread::sleep( Duration::from_secs(3) );
                                        }
                                    }
                                },
                                Err(e) =>  {
                                    println!("[ERROR]file {:?}'s translation was failed: {:?}\n",filename,e);
                                    bad_translaion.push(filename.to_string());
                                    continue 'outer;
                                }
                            }

                        }
                    },
                    s => {
                        println!("[ERROR]failed to translate file {:?} : {:?}\n",filename,s);
                        bad_translaion.push(filename.to_string());
                        continue 'outer;
                    }
                }

            }
            en_path.pop();
        }
        println!();
    }

    println!("file(s) successfully translated:\n{:#?}\n",good_translaion);
    println!("file(s) failed to translate:\n{:#?}\n",bad_translaion);

    Ok(())
}


/**
 *TODO: comment about return value.
 */
fn know_file_state(filename: &str, client: &reqwest::blocking::Client, id: &str, key: &str) -> Result<String,reqwest::Error>  {

    println!("\nacquiring translation state for file {:?} ...\n", filename);

    let url = format! ("{}/{}",DEEPL_ENDPOINT , id);

    let params = [("auth_key",DEEPL_KEY),("document_key",key)];

    let resp = client.post(&url)
    .form(&params)
    .send();

    match resp {
        //HTTP request was successful
        Ok(v) =>{ 
            //deserialize JSON.
            let state = v.json::<FileState>();
            match state {
                Ok(ref v) => {
                    println!("translation state got: {:#?}", v.status);
                    if v.status.as_str() == "translating" || v.status.as_str() == "queued" {                        
                        println!("remaining seconds = {:?}",v.seconds_remaining.unwrap());
                    }
                    //ugle clone. may fix in future.
                    return Ok(v.status.clone());
                }
                Err(e) => {
                    println!("[ERROR]failed to deserialize JSON.");
                    return Err(e);
                }
            }
        },
        //propagate HTTP error
        Err(e) => return Err(e)
    }

}


use bytes::Bytes;
fn download_file(filename: &str, client: &reqwest::blocking::Client, id: &str, key: &str) -> Result<Bytes,reqwest::Error>{
    println!("Retrieving translated file {:?} ...", filename);

    let url = format! ("{}/{}/result",DEEPL_ENDPOINT , id);

    let params = [("auth_key",DEEPL_KEY),("document_key",key)];

    let resp = client.post(&url)
    .form(&params)
    .send();

    match resp {
        Ok(v) => {
            let content = v.bytes();
            match content {
                Ok(v)=> Ok(v),
                Err(e) => {
                    println!("[ERROR]file was failed to convert to bytes");
                    return Err(e);
                }
            }
        },
        Err(e) => {
            return Err(e);
        }
    }
}


fn create_file(content:Bytes){
    let mut file = File::create("test-translated.html").unwrap();
    file.write_all(&content).unwrap();
}