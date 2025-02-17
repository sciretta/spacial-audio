use hyper::{Body, Request, Response};
use random_string::generate;
use serde_json::json;
use std::convert::Infallible;
use std::sync::Arc;

use crate::{AppState, AudioData};

pub async fn create_session(
    req: Request<Body>,
    state: Arc<AppState>,
) -> Result<Response<Body>, Infallible> {
    let code_charset = "1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let metadata: String = req
        .headers()
        .get("metadata")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let session_code = generate(8, code_charset);

    // Store the received audio buffer and metadata in memory
    let mut sessions = state.sessions.lock().await;
    sessions.insert(
        session_code.clone(),
        AudioData {
            metadata,
            audio_buffer: vec![],
        },
    );

    let response_body = json!({
        "session_code": session_code
    });

    Ok(Response::new(Body::from(response_body.to_string())))
}

pub async fn add_buffer(
    req: Request<Body>,
    state: Arc<AppState>,
) -> Result<Response<Body>, Infallible> {
    let metadata = req
        .headers()
        .get("metadata")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let session_code = req
        .headers()
        .get("sessioncode")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let whole_body = hyper::body::to_bytes(req.into_body()).await.unwrap();

    // Store the received audio buffer and metadata in memory
    let mut sessions = state.sessions.lock().await;
    let session = sessions.get_mut(&session_code);

    match session {
        Some(session) => {
            session.audio_buffer.push(whole_body.to_vec());
            Ok(Response::new(Body::from(
                "Audio buffer and metadata added successfully to the session",
            )))
        }
        None => return Ok(Response::new(Body::from("Session not found"))),
    }
}

pub async fn get_session(
    req: Request<Body>,
    state: Arc<AppState>,
) -> Result<Response<Body>, Infallible> {
    let session_code = req
        .headers()
        .get("sessioncode")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    let sessions = state.sessions.lock().await;
    let session = sessions.get(&session_code);

    match session {
        Some(session) => {
            let mut audio_buffer = vec![];
            for buffer in &session.audio_buffer {
                audio_buffer.extend_from_slice(buffer);
                audio_buffer.extend_from_slice(&[0x3d, 0x3d, 0x3d, 0x3d, 0x3d]);
            }
            Ok(Response::new(Body::from(audio_buffer)))
        }
        None => Ok(Response::new(Body::from("Session not found"))),
    }
}
