use actix_web::{get, HttpRequest, HttpResponse, put, Responder, web};
use openraft::EntryPayload;
use openraft::raft::ClientWriteRequest;

use crate::ExampleRequest;
use crate::app::ExampleApp;
use crate::store::fs_io::read_slice;

//TODO: implement consistent read
#[get("/slice/{id}")]
pub async fn get_slice(_app: web::Data<ExampleApp>, req: HttpRequest) -> impl Responder {
    let id: String = req.match_info().get("id").unwrap().into();
    if id.len() != 64 && id.is_ascii() {
        return HttpResponse::NotAcceptable().body("ID should be 64 bytes long ascii.");
    }
    match read_slice(&id) {
        Ok(result) => HttpResponse::Ok().insert_header(("Content-Type", "application/octet-stream")).body(result),
        Err(err) => HttpResponse::NotFound().body(format!("No such result.\nDetail: {}", err))
    }
}

#[put("/slice/{id}")]
pub async fn put_slice(app: web::Data<ExampleApp>, req: HttpRequest, body: web::Bytes) -> HttpResponse {
    let id: String = req.match_info().get("id").unwrap().into();
    println!("put: {}", id);
    if id.len() != 64 && id.is_ascii() {
        return HttpResponse::NotAcceptable().body("ID should be 64 bytes long ascii.");
    }

    let body = body.to_vec();

    let request = ClientWriteRequest::new(EntryPayload::Normal(ExampleRequest::Set { key: id, value: body }));
    let response = app.raft.client_write(request).await;
    HttpResponse::Ok().json(response)
    /*
Err(error) => HttpResponse::InternalServerError()
    .body(format!("Server can't write file.\nDetail: {}", error)),
Ok(..) => HttpResponse::Ok()
    .insert_header(("Content-Type","application/octet-stream"))
    .body("Write finished")
     */
}
