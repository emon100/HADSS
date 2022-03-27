use std::fs;
use actix_web::{App, HttpResponse, HttpRequest, Responder, web, HttpServer, get, put};
use crate::ARGS;

fn transform_id_into_chunks(id: &str) -> Vec<&str> {
    id.as_bytes()
        .chunks(2)
        .map(|x| std::str::from_utf8(x).unwrap())
        .collect()
}

//TODO
#[get("/slice/{id}")]
async fn get_slice(req: HttpRequest) -> impl Responder {
    let id: String = req.match_info().get("id").unwrap().into();
    if id.len() != 64 && id.is_ascii() {
        return HttpResponse::NotAcceptable().body("ID should be 64 bytes long ascii.")
    }
    let chunks = transform_id_into_chunks(&id);

    let storage_directory_depth:usize = ARGS.storage_directory_depth;

    let directory: String = chunks
        .iter()
        .take(storage_directory_depth)
        .cloned()
        .intersperse("/")
        .collect();

    let filename: String = chunks
        .iter()
        .skip(storage_directory_depth)
        .cloned()
        .intersperse("")
        .collect();

    let path = format!("{}/{}/{}", ARGS.storage_location, directory, filename);
    println!("{}",path);

    if let Ok(result) = fs::read(path) {
        return HttpResponse::Ok().insert_header(("Content-Type","application/octet-stream")).body(result)
    } else {
        return HttpResponse::NotFound().body("No such result")
    }
}

#[put("/slice/{id}")]
async fn put_slice(req: HttpRequest, body: web::Bytes) -> impl Responder {
    let id: String = req.match_info().get("id").unwrap().into();
    println!("put: {}", id);
    if id.len() != 64 && id.is_ascii() {
        return HttpResponse::NotAcceptable().body("ID should be 64 bytes long ascii.")
    }

    let chunks = transform_id_into_chunks(&id);

    let storage_directory_depth:usize = ARGS.storage_directory_depth;

    let directory: String = chunks
        .iter()
        .take(storage_directory_depth)
        .cloned()
        .intersperse("/")
        .collect();

    let full_directory = format!("{}/{}", ARGS.storage_location, directory);
    if let Err(error) = fs::create_dir_all(&full_directory) {
        return HttpResponse::InternalServerError()
            .body(format!("Server can't create directory to storage file.\nDetail: {}", error));
    }

    let filename: String = chunks
        .iter()
        .skip(storage_directory_depth)
        .cloned()
        .intersperse("")
        .collect();

    let full_path = format!("{}/{}", full_directory, filename);
    println!("{}",full_path);
    if let Err(error) = fs::write(full_path, body) {
        return HttpResponse::InternalServerError()
            .body(format!("Server can't write file.\nDetail: {}", error));
    }

    return HttpResponse::Ok()
        .insert_header(("Content-Type","application/octet-stream"))
        .body("Write finished")
}

pub async fn init_httpserver() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(get_slice)
            .service(put_slice)
    })
        .bind((ARGS.addr.to_string(), ARGS.port))?
        .run()
        .await
}
