use actix_web::{get, HttpRequest, HttpResponse, put, Responder, web};
use actix_web::http::header;
use openraft::EntryPayload;
use openraft::error::ClientWriteError;

use crate::app::StorageApp;
use crate::StorageNodeRequest;
use crate::store::fs_io::read_slice;

//TODO: implement consistent read
#[get("/slice/{id}")]
pub async fn get_slice(_app: web::Data<StorageApp>, req: HttpRequest) -> impl Responder {
    let id: String = req.match_info().get("id").unwrap().into();
    if id.len() < 64 + 1 + 1 && id.is_ascii() {
        return HttpResponse::NotAcceptable().body("ID should be 64 bytes long ascii and '.' and object name.");
    }
    match read_slice(&id) {
        Ok(result) => HttpResponse::Ok().insert_header(("Content-Type", "application/octet-stream")).body(result),
        Err(err) => HttpResponse::NotFound().body(format!("No such result.\nDetail: {}", err))
    }
}

#[put("/slice/{id}")]
pub async fn put_slice(app: web::Data<StorageApp>, req: HttpRequest, body: web::Bytes) -> HttpResponse {
    let id: String = req.match_info().get("id").unwrap().into();
    println!("put: {}", id);
    if id.len() < 64 + 1 + 1 && id.is_ascii() {
        return HttpResponse::NotAcceptable().body("ID should be 64 bytes long ascii and '.' and object name.");
    }

    let request = StorageNodeRequest::new(EntryPayload::Normal(StorageNodeRequest::StoreData { id: id.clone(), value: body.to_vec() }));
    let response = app.raft.client_write(request).await;
    match &response {
        Err(e) => {
            match e {
                ClientWriteError::ForwardToLeader(nid) => {
                    if let Some(leader) = nid.clone().leader_node {
                        HttpResponse::TemporaryRedirect()
                            .insert_header((header::LOCATION, format!("http://{}/slice/{}", leader.addr, id)))
                            .json(&response)
                    } else {
                        HttpResponse::InternalServerError()
                            .json(&response)
                    }
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
