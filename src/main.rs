/**
 * 
 * PLACE THIS FILE UNDER  `.../static/webview/information/ja` DIRECTORY
 * BEFORE RUNNING.
 * 
 **/

use std::{
    error,
    env,
    fs::File,
    io,
    io::prelude::*,
    io::Error,
    path::PathBuf,
    path::Path,
    time::Duration,
    thread,
};


//use reqwest::blocking::Client;
use bytes::Bytes;
use reqwest::StatusCode;
use serde::Deserialize;
use walkdir::WalkDir;

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

/*
enum Language {
    EN(String),
    ZHCN(String),
    ZHTW(String),
}
*/

fn main() -> Result<(),Box<dyn error::Error>> {

    //&str vector storing file names of which are failed to translate.
    let mut bad_translaion: Vec<String> = Vec::new();
    //for those successful translation.
    let mut good_translaion: Vec<String> = Vec::new();
    //for those files might be translated already.
    let mut skipped_translaion: Vec<String> = Vec::new();

    //languages translate to
    let language = vec!["en", "zh"];


    //supposed to be .../information/ja
    let exe_folder = get_exe_folder()?;
    //check if the program is located in the correct directory.
    if &exe_folder.file_name().unwrap().to_str().unwrap() != &"ja" {
        eprintln!("[ERROR] place this file under  `.../static/webview/information/ja` folder.\ncurrent path: {:?}.\n",&exe_folder);
        std::process::exit(1);
    }
    println!("researching on folder {:?}",&exe_folder);

    //.../information
    let parent_path = exe_folder.parent().unwrap();

    //let mut en_path = PathBuf::from(parent_path);
    //en_path.push("en");

    /*
    //create english folder
    let en = fs::create_dir("../en");
    match en {
        Ok(path) => path,
        Err(e) => {
            if e.kind() == ErrorKind::AlreadyExists{
            } else {
            return Err(Box::from(e));
            }
        }
    }
    */


    //reqwest blocking client
    let client = reqwest::blocking::Client::new();

    

    //walk through current "ja" folder
    for entry in WalkDir::new(exe_folder.as_path())
    .follow_links(true)
    .into_iter()
    .filter_map(|e| e.ok()) {
        let filename = entry.file_name().to_str().unwrap();
        let sec = entry.metadata()?.modified()?;
        if filename.ends_with(".html") && sec.elapsed()?.as_secs() < 86400 {
            println!("untranslated file found: {:?}...", entry.path());
             
                'lang: for lang in &language {
                    //get html file path on corresponding language folder
                    let entry_path_stripped = entry.path().strip_prefix(&exe_folder.as_path()).unwrap();
                    let lang_file_path = &parent_path.join(lang).join(entry_path_stripped);
                    if Path::new(lang_file_path.to_str().unwrap()).exists() {
                        println!("[SKIP] file {:?} may have been translated by someone, skip.\n",filename);
                        skipped_translaion.push(format!("{} - {}",filename.to_string(),&lang));
                        continue;
                    }
                    //make the translation
                    println!("translating {:?} to {:?}...", filename,&lang);
                    //POST request
                    let form = reqwest::blocking::multipart::Form::new()
                    .text("source_lang","JA")
                    .text("target_lang",*lang)
                    .text("auth_key",DEEPL_KEY)
                    .file("file",entry.path())?;
                    
                    let resp = client.post(DEEPL_ENDPOINT)
                    .multipart(form)
                    .send()?;
                    
                    //receiving response
                    match resp.status() {
                        //in case of http request succeeded
                        StatusCode::OK => {
                        
                            let info = resp.json::<FileInfo>()?;
                        
                            loop {
                            
                                match know_file_state(filename, &client, &info.document_id, &info.document_key) {
                                    Ok(v) =>  {
                                        match v.status.as_str() {
                                            "error" => {
                                                println!("[ERROR] file {:?}'s translation was failed due to an unknown error.\n",filename);
                                                bad_translaion.push(format!("{} - {}",filename.to_string(),&lang));
                                                continue 'lang;
                                            }
                                            "done" => {
                                                println!("\ntranslation completed. retrieving file...");
                                                let translated = download_file(filename, &client, &info.document_id, &info.document_key);
                                                match translated {
                                                    Ok(v) => {
                                                        println!("file retrieved. copy to local folder...");
                                                        match create_file(lang_file_path.to_str().unwrap(),v,&lang){
                                                            Ok(_) => {
                                                                println!("done.\n");
                                                                good_translaion.push(format!("{} - {}",filename.to_string(),&lang));
                                                                break;
                                                            },
                                                            Err(e) => {
                                                                println!("[ERROR] file was retrived, but failed to create local copy : {:?}",e);
                                                                bad_translaion.push(format!("{} - {}",filename.to_string(),&lang));
                                                                continue 'lang;
                                                            }
                                                        }
                                                        
                                                        
                                                    },
                                                    Err(e) => {
                                                        println!("[ERROR] file was translated, but failed to download:{}",e);
                                                        bad_translaion.push(format!("{} - {}",filename.to_string(),&lang));
                                                        continue 'lang;
                                                    }
                                                }
                                            }
                                            //still under translation
                                            _ => {
                                                println!("state: {:#?}.", v.status);
                                                if v.status.as_str() == "translating" || v.status.as_str() == "queued" {                        
                                                    println!("{:?} seconds remaining.",v.seconds_remaining.unwrap());
                                                }
                                                thread::sleep( Duration::from_secs(3) );
                                            }
                                        }
                                    },
                                    Err(e) =>  {
                                        println!("[ERROR] failed to translate file {:?} in {:?} : {:?}\n",filename,&lang,e);
                                        bad_translaion.push(format!("{} - {}",filename.to_string(),&lang));
                                        continue 'lang;
                                    }
                                }
                                
                            }// inner loop finished.
                        },
                        err => {
                            println!("[ERROR] http error: {:?}\n",err);
                            bad_translaion.push(format!("{} - {}",filename.to_string(),&lang));
                            continue 'lang;
                        }
                    }
                
                }//'lang loop
        
            
        }
    }//outer loop


    println!("\nsuccessful translation:\n{:#?}\n",good_translaion);
    println!("failed translation:\n{:#?}\n",bad_translaion);
    println!("skipped translation:\n{:#?}\n",skipped_translaion);

    Ok(())
}


    

/*
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
                                            println!("[ERROR] file {:?}'s translation was failed due to an unknown error.\n",filename);
                                            bad_translaion.push(filename.to_string());
                                            continue 'outer;
                                        }
                                        "done" => {
                                            let translated = download_file(filename, &client, &info.document_id, &info.document_key);
                                            match translated {
                                                Ok(v) => {
                                                    //create_file(v);
                                                },
                                                Err(e) => {
                                                    println!("[ERROR] file was translated, but failed to download:{}",e);
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
                                    println!("[ERROR] file {:?}'s translation was failed: {:?}\n",filename,e);
                                    bad_translaion.push(filename.to_string());
                                    continue 'outer;
                                }
                            }

                        }
                    },
                    s => {
                        println!("[ERROR] failed to translate file {:?} : {:?}\n",filename,s);
                        bad_translaion.push(filename.to_string());
                        continue 'outer;
                    }
                }

            }
            en_path.pop();
        }
        println!();
    }
    */

//helper
fn get_exe_folder() -> io::Result<PathBuf> {
    let mut exe_folder = env::current_exe()?;
    exe_folder.pop();
    Ok(exe_folder)
}


/**
 *TODO: comment about return value.
 */
fn know_file_state(_filename: &str, client: &reqwest::blocking::Client, id: &str, key: &str) -> Result<FileState,reqwest::Error>  {

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
                Ok(v) => return Ok(v),
                Err(e) => return Err(e)
            }
        },
        //propagate HTTP error
        Err(e) => return Err(e)
    }
}


fn download_file(_filename: &str, client: &reqwest::blocking::Client, id: &str, key: &str) -> Result<Bytes,reqwest::Error>{
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
                Err(e) => Err(e),
            }
        },
        Err(e) => {
            return Err(e);
        }
    }
}

fn create_file(filename: &str, content:Bytes, _language:&str) -> Result<(),Error>{
    println!("DEBBBBBBBBBUG: {:?}",filename);
    let file = File::create(filename);
    match file {
        Ok(mut v) => {
            match v.write_all(&content) {
                Ok(_) => return Ok(()),
                Err(e) => return Err(e),
            }
        },
        Err(e) => return Err(e),
    }
}