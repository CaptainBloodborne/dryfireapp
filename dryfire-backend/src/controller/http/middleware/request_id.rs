//! Request ID middleware. Generates a UUID per request, puts it into
//! the request extensions (so error handlers can include it in error
//! bodies), and copies it onto the response header `x-request-id`.

use axum::{
    extract::Request,
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

pub static REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

#[derive(Clone, Debug)]
pub struct RequestId(pub String);

pub async fn set_request_id(mut req: Request, next: Next) -> Response {
    let rid = req
        .headers()
        .get(&REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    req.extensions_mut().insert(RequestId(rid.clone()));

    let mut resp = next.run(req).await;
    if let Ok(v) = HeaderValue::from_str(&rid) {
        resp.headers_mut().insert(&REQUEST_ID_HEADER, v);
    }
    resp
}
