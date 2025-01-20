use aide::{
    axum::{
        routing::{get_with, post_with},
        ApiRouter, IntoApiResponse,
    },
    transform::TransformOperation,
};
use axum::{
    extract::{DefaultBodyLimit, Multipart, Path},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::{headers::Range, TypedHeader};
use axum_range::{KnownSize, Ranged};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::{errors::AppError, extractors::Json, state::AppState};

pub fn file_service_routes(state: AppState) -> ApiRouter {
    let file_upload_route = ApiRouter::new()
        .api_route("/upload", post_with(upload_file, upload_file_docs))
        // FIXME: want a sensible max file upload size! (2 MB is the default https://docs.rs/axum/latest/axum/extract/struct.Multipart.html)
        .layer(DefaultBodyLimit::disable());
    let file_download_route = ApiRouter::new().api_route(
        "/download:filename",
        get_with(download_file, download_file_docs),
    );
    let merged_routes = ApiRouter::new()
        // accounts
        .merge(file_upload_route)
        .merge(file_download_route)
        .with_state(state);
    merged_routes
}

/// New File details.
#[derive(Serialize, JsonSchema)]
struct FileWrapper {
    /// The ID of the new file.
    id: Uuid,
}

/// Unfortunately, Aide requires a struct in the form below to properly display
/// parameters in the OpenAPI json for path variables https://github.com/tamasfe/aide/issues/4
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash, JsonSchema)]
pub struct FileIdPath {
    pub id: Uuid,
}

/// Axum service to handle file upload requests
async fn upload_file(
    // State(app): State<Arc<AppState>>,
    multipart: Multipart,
) -> Result<impl IntoApiResponse, AppError> {
    let id = Uuid::new_v4();
    // TODO: use a real db backend
    let mut file = File::create(format!("{}/{}", "/tmp", id)).await.unwrap();

    let _ = stream_to_file(multipart, &mut file)
        .await
        .map_err(|_| async {
            // NOTE: https://docs.rs/tokio/latest/tokio/fs/struct.File.html may need to flush... if we care about saving partial uploads
            drop(file);
            let file_delete = tokio::fs::remove_file(id.to_string()).await;
            if let Err(_) = file_delete {
                return AppError::new("File removal failed! FS unclean!");
            };
            return AppError::new("File upload interupted, upload deleted.");
        });

    Ok((StatusCode::CREATED, Json(FileWrapper { id })))
}

/// Axum service to handle file download requests
/// FIXME: On integration, we need to do auth via token `Session` extrating on the request
async fn download_file(
    // State(app): State<Arc<AppState>>,
    Path(filename): Path<FileIdPath>,
    range: Option<TypedHeader<Range>>,
) -> Result<impl IntoApiResponse, AppError> {
    // FIXME: use app state to set a file store location on disc
    let filepath = format!("{}/{}", "/tmp", filename.id);

    // FIXME: need to only load the bytes we need from disk to memory, not whole file
    // Ideally get a page that is optimal for any client (is there one?) and/or web
    // See https://github.com/tokio-rs/axum/discussions/1638
    // https://lib.rs/crates/axum-streams
    // how does axum handle a last packet failing? (not mid stream where it can be detected for sure)
    let file = match File::open(&filepath).await {
        Ok(file) => file,
        Err(_) => return Err(AppError::new("File not found").with_status(StatusCode::NOT_FOUND)),
    };

    let body = KnownSize::file(file).await.unwrap();
    let range = range.map(|TypedHeader(range)| range);

    let r = Ranged::new(range, body);
    r.try_respond().map_err(|_| {
        AppError::new("Malformed file download request").with_status(StatusCode::NOT_ACCEPTABLE)
    })

    // let mut response = (
    //     StatusCode::PARTIAL_CONTENT,
    //     [
    //         (
    //             header::CONTENT_RANGE,
    //             format!("bytes {}-{}/{}", read_range.0, read_range.1, file_size),
    //         ),
    //         (
    //             header::CONTENT_LENGTH,
    //             (read_range.1 - read_range.0 + 1).to_string(),
    //         ),
    //         (header::CONTENT_TYPE, "application/octet-stream".to_string()),
    //         (
    //             header::CONTENT_DISPOSITION,
    //             format!("attachment; filename=\"{}\"", filename.id),
    //         ),
    //     ],
    //     buffer,
    // )
    //     .into_response();
    // response.headers_mut().insert(
    //     header::ACCEPT_RANGES,
    //     header::HeaderValue::from_static("bytes"),
    // );
    // return Ok(response);

    // Err(AppError::new("Malformed file download request").with_status(StatusCode::NOT_ACCEPTABLE))
}

/// Stream uploaded data to a given file on disk.
async fn stream_to_file(
    mut multipart: Multipart,
    file: &mut tokio::fs::File,
) -> Result<(), AppError> {
    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|err| AppError::new(err.to_string().as_str()))?
    {
        if field.name().unwrap() == "data" {
            while let Some(chunk) = field
                .chunk()
                .await
                .map_err(|err| AppError::new(err.to_string().as_str()))?
            {
                file.write_all(&chunk)
                    .await
                    .map_err(|err| AppError::new(err.to_string().as_str()))?
            }
        }
    }

    Ok(())
}

fn upload_file_docs(op: TransformOperation) -> TransformOperation {
    op.description("Upload a file.")
        .response::<201, Json<FileWrapper>>()
        // TODO: what is correct way to handle/report error docs in upload?
        .response::<500, Json<FileWrapper>>()
}

// TODO correct docs
fn download_file_docs(op: TransformOperation) -> TransformOperation {
    op.description("Download a file.")
        .response::<206, String>()
        // TODO: what is correct way to handle/report error docs in upload?
        .response::<406, String>()
}
