use axum::{body::Body, extract::Request, middleware::Next, response::Response};
use tracing::debug;

pub async fn req_stamp(req: Request<Body>) -> Request<Body> {
    // debug!("{:<12} - {:?} {:?}", "MIDDLEWARE", req.uri(), req.method());

    req
}
