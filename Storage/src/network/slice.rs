use actix_web::{get, HttpRequest, HttpResponse, put, Responder, web};
use actix_web::http::header;
use openraft::EntryPayload;
use openraft::error::ClientWriteError;
use openraft::raft::ClientWriteRequest;

use crate::app::StorageNode;
use crate::{StorageNodeId, StoreFileRequest};
use crate::store::fs_io::read_slice;

//TODO: implement consistent read
#[get("/slice/{id}")]
pub async fn get_slice(_app: web::Data<StorageNode>, req: HttpRequest) -> impl Responder {
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
pub async fn put_slice(app: web::Data<StorageNode>, req: HttpRequest, body: web::Bytes) -> HttpResponse {
    let id: String = req.match_info().get("id").unwrap().into();
    println!("put: {}", id);
    if id.len() != 64 && id.is_ascii() {
        return HttpResponse::NotAcceptable().body("ID should be 64 bytes long ascii.");
    }

    let request = ClientWriteRequest::new(EntryPayload::Normal(StoreFileRequest::Set { id: id.clone(), value: body.to_vec() }));
    let response = app.raft.client_write(request).await;
    match &response {
        Err(e) => {
            match e {
                ClientWriteError::ForwardToLeader(nid) => {
                    let addr = nid.clone().leader_node.unwrap().addr;
                    HttpResponse::TemporaryRedirect()
                        .insert_header((header::LOCATION,format!("http://{}/slice/{}", addr, id)))
                        .json(&response)
                }
                _ => {
                    HttpResponse::InternalServerError()
                        .json(&response)
                }
            }
        }
        Ok(..) => HttpResponse::Ok()
            .json(&response)
    }
}
