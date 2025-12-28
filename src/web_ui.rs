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
async fn root(api_port: u16) -> Result<Html<String>, StatusCode> {
    let html = STATIC_DIR
        .get_file("html/index.html")
        .and_then(|f| f.contents_utf8())
        .ok_or_else(|| {
            tracing::error!("index.html not found in embedded static files");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    let html_with_port = html.replace("{{API_PORT}}", &api_port.to_string());
    Ok(Html(html_with_port))
}
async fn static_files(Path(path): Path<String>) -> Result<Response, StatusCode> {
    if path.contains("..") {
        tracing::warn!("Blocked path traversal attempt: {}", path);
        return Err(StatusCode::FORBIDDEN);
    }
    let sanitized_path = path.trim_start_matches('/');
    let file = STATIC_DIR.get_file(sanitized_path).ok_or(StatusCode::NOT_FOUND)?;
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
    Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .body(content.into())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
