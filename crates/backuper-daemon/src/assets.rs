use axum::{
    body::Body,
    extract::Request,
    http::{Response, StatusCode, header},
    response::IntoResponse,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../../webui/.output/public"]
struct Assets;

pub async fn serve(req: Request<Body>) -> impl IntoResponse {
    let path = req.uri().path().trim_start_matches('/');

    if let Some(file) = Assets::get(path) {
        let mime = file.metadata.mimetype();
        return Response::builder()
            .header(header::CONTENT_TYPE, mime)
            .body(Body::from(file.data))
            .unwrap();
    }

    match Assets::get("index.html") {
        Some(file) => Response::builder()
            .header(header::CONTENT_TYPE, "text/html")
            .body(Body::from(file.data))
            .unwrap(),
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap(),
    }
}
