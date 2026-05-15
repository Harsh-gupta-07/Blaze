// use blazefind_core::walker;
use blazefind_core::{db,walker, tantivy};
use std::{process, sync::Arc, thread};


fn main(){
    match db::initialize_db() {
        Ok(_)=>{},
        Err(err)=>{
            eprintln!("Failed to initialize database: {}", err);
            process::exit(1);
        }
    }

    let files = Arc::new(walker::scan_directory("/"));

    let db_files = Arc::clone(&files);
    let db_worker = thread::spawn(move || {
        let mut conn = db::get_connection()?;
        db::add_files(db_files.as_ref(), &mut conn)
    });

    let index_files = Arc::clone(&files);
    let index_worker = thread::spawn(move || tantivy::make_index(index_files.as_ref()));

    match db_worker.join() {
        Ok(Ok(_))=> {},
        Ok(Err(err))=>{
            eprintln!("Failed to Add Files: {}", err);
        }
        Err(_)=>{
            eprintln!("DB worker panicked");
        }
    }

    match index_worker.join() {
        Ok(Ok(_))=>{},
        Ok(Err(err))=>{
            eprintln!("Failed to Create tantivy index {}", err);
        },
        Err(_)=>{
            eprintln!("Index worker panicked");
        }
    }

    let conn = match db::get_connection() {
        Ok(conn) => conn,
        Err(err) => {
            eprintln!("Failed to connect to DB: {}", err);
            process::exit(1);
        }
    };

    let fetch = match db::get_files(&conn){
        Ok(fetch)=>{fetch},
        Err(err)=>{
            eprintln!("Error Fetching files List: {}", err);
            return;
        }
    };
    println!("{:#?}", fetch)
    
}
