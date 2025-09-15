use axum::{
    Router,
    extract::Path,
    http::{StatusCode, header},
    response::{Html, Response},
    routing::get,
};
use include_dir::{Dir, include_dir};
static STATIC_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/static");
pub fn create_routes(api_port: u16) -> Router {
    Router::new()
        .route("/", get(move || root(api_port)))
        .route("/static/*path", get(static_files))
}
async fn root(api_port: u16) -> Html<String> {
    let html = STATIC_DIR
        .get_file("html/index.html")
        .unwrap()
        .contents_utf8()
        .unwrap();
    let html_with_port = html.replace("{{API_PORT}}", &api_port.to_string());
    Html(html_with_port)
}
async fn static_files(Path(path): Path<String>) -> Result<Response, StatusCode> {
    let file = STATIC_DIR.get_file(&path).ok_or(StatusCode::NOT_FOUND)?;
    let content_type = match path.as_str() {
        p if p.ends_with(".css") => "text/css",
        p if p.ends_with(".js") => "application/javascript",
        p if p.ends_with(".html") => "text/html",
        p if p.ends_with(".png") => "image/png",
        p if p.ends_with(".jpg") | p.ends_with(".jpeg") => "image/jpeg",
        p if p.ends_with(".svg") => "image/svg+xml",
        p if p.ends_with(".json") => "application/json",
        _ => "application/octet-stream",
    };
    let content = file.contents().to_vec();
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .body(content.into())
        .unwrap())
}
