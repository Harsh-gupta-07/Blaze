// use blazefind_core::walker;
use blazefind_core::{db,walker};



fn main(){
    let mut conn = match db::get_connection(){
        Ok(conn)=> conn,
        Err(err)=>{
            eprintln!("Failed to connect to DB: {}", err);
            panic!();
        }
    };

    match db::initialize_db() {
        Ok(_)=>{},
        Err(err)=>{
            eprintln!("Failed to initialize database: {}", err)
        }
    }

    let files = walker::scan_directory("/");
    match db::add_files(files, &mut conn) {
        Ok(_)=> {},
        Err(err)=>{
            eprintln!("Failed to Add Files: {}", err)
        }
    };

    let fetch = match db::get_files(&mut conn){
        Ok(fetch)=>{fetch},
        Err(err)=>{
            eprintln!("Error Fetching files List: {}", err);
            return;
        }
    };
    println!("{:#?}", fetch)
    
}
