use crate::AppState;
use crate::AudioData;
use hyper::{Body, Request, Response};
use random_string::generate;
use serde_json::json;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::Mutex as TokioMutex;

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
            finished: false,
            audio_buffer: vec![],
        },
    );

    // Create a broadcast channel for the session
    let (tx, _rx) = broadcast::channel(100);
    let mut subscribers = state.subscribers.lock().await;
    subscribers.insert(session_code.clone(), tx);

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
            }
            Ok(Response::new(Body::from(audio_buffer)))
        }
        None => Ok(Response::new(Body::from("Session not found"))),
    }
}

pub async fn sse_handler(
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

    let mut subscribers = state.subscribers.lock().await;
    if let Some(tx) = subscribers.get(&session_code) {
        let mut rx = tx.subscribe();

        let (mut sender, body) = Body::channel();

        tokio::spawn(async move {
            while let Ok(message) = rx.recv().await {
                if message == "session_finished" {
                    let _ = sender.send_data("data: session_finished\n\n".into()).await;
                    break;
                }
                if sender
                    .send_data(format!("data: {}\n\n", message).into())
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });

        Ok(Response::builder()
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .body(body)
            .unwrap())
    } else {
        Ok(Response::new(Body::from("Session not found")))
    }
}

pub async fn set_finished(
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

    let mut sessions = state.sessions.lock().await;
    if let Some(session) = sessions.get_mut(&session_code) {
        session.finished = true;

        // Notify all subscribers
        let mut subscribers = state.subscribers.lock().await;
        if let Some(tx) = subscribers.get(&session_code) {
            let _ = tx.send("session_finished".to_string());
        }

        Ok(Response::new(Body::from("Session finished")))
    } else {
        Ok(Response::new(Body::from("Session not found")))
    }
}
