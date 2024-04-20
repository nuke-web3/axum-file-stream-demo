use aide::{
    axum::{routing::post_with, ApiRouter, IntoApiResponse},
    transform::TransformOperation,
};
use axum::{
    extract::{DefaultBodyLimit, Multipart},
    http::StatusCode,
};
use schemars::JsonSchema;
use serde::Serialize;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::{errors::AppError, extractors::Json, state::AppState};

pub fn file_uploader_routes(state: AppState) -> ApiRouter {
    ApiRouter::new()
        .api_route("/upload", post_with(upload_file, upload_file_docs))
        // FIXME: want a sensible max file upload size! (2 MB is the default https://docs.rs/axum/latest/axum/extract/struct.Multipart.html)
        .layer(DefaultBodyLimit::disable())
        .with_state(state)
}

/// New File details.
#[derive(Serialize, JsonSchema)]
struct FileUploaded {
    /// The ID of the new file.
    id: Uuid,
}

async fn upload_file(
    // State(app): State<AppState>,
    multipart: Multipart,
) -> Result<impl IntoApiResponse, AppError> {
    let id = Uuid::new_v4();
    // TODO: use a real db backend
    let mut file = File::create(format!("/tmp/{}", id)).await.unwrap();

    // TODO: get map_err() to work with async instead of match
    match stream_to_file(multipart, &mut file).await {
        Ok(_) => println!("💾 Streamed data written to disk"),
        Err(_) => {
            println!("❌ Interupted upload! Drop & delete file uploaded...");
            // NOTE: https://docs.rs/tokio/latest/tokio/fs/struct.File.html may need to flush... if we care about saving partial uploads
            drop(file);
            tokio::fs::remove_file(id.to_string()).await.unwrap();
            return Err(AppError::new("File upload interupted"));
        }
    }

    Ok((StatusCode::CREATED, Json(FileUploaded { id })))
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
        .response::<201, Json<FileUploaded>>()
        // TODO: what is correct way to handle/reprort error docs in upload?
        .response::<500, Json<FileUploaded>>()
}
