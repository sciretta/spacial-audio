use crate::AppState;
use crate::AudioData;
use hyper::{Body, Request, Response};
use random_string::generate;
use serde_json::json;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::broadcast;

pub async fn create_session(
    _req: Request<Body>,
    state: Arc<AppState>,
) -> Result<Response<Body>, Infallible> {
    let code_charset = "1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let session_code = generate(8, code_charset);

    let mut sessions = state.sessions.lock().await;
    sessions.insert(
        session_code.clone(),
        AudioData {
            is_session_finished: false,
            connected_guests: 0,
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
    let session_code = req
        .headers()
        .get("sessioncode")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let whole_body = hyper::body::to_bytes(req.into_body()).await.unwrap();

    let mut sessions = state.sessions.lock().await;
    let session = sessions.get_mut(&session_code);

    match session {
        Some(session) => {
            if session.connected_guests == (session.audio_buffer.len() + 1) as u8 {
                session.is_session_finished = true;
            }
            session.audio_buffer.push(whole_body.to_vec());
            Ok(Response::new(Body::from(
                "Audio buffer added successfully to the session",
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
    let session: Option<&AudioData> = sessions.get(&session_code);

    match session {
        Some(session) => {
            println!("SESSION {:?}", session);
            if !session.is_session_finished {
                return Ok(Response::new(Body::from("Session not finished yet")));
            }
            let mut audio_buffer = vec![];
            for buffer in &session.audio_buffer {
                audio_buffer.extend_from_slice(buffer);
                audio_buffer.extend_from_slice(&[0x3d, 0x3d, 0x3d, 0x3d, 0x3d]);
            }
            remove_session_private_method(session_code, state.clone()).await;
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

    let subscribers = state.subscribers.lock().await;
    let mut sessions = state.sessions.lock().await;
    let session = sessions.get_mut(&session_code);

    match session {
        Some(session) => {
            session.connected_guests = session.connected_guests + 1;
        }
        None => {}
    }

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
    if let Some(_) = sessions.get_mut(&session_code) {
        let subscribers = state.subscribers.lock().await;
        if let Some(tx) = subscribers.get(&session_code) {
            let _ = tx.send("session_finished".to_string());
        }

        Ok(Response::new(Body::from("Session finished")))
    } else {
        Ok(Response::new(Body::from("Session not found")))
    }
}

async fn remove_session_private_method(session_code: String, state: Arc<AppState>) {
    // let mut sessions = state.sessions.lock().await;
    // sessions.remove(&session_code);
    // let mut subscribers = state.subscribers.lock().await;
    // subscribers.remove(&session_code);

    println!("REMOVE SESSION {}", session_code);
}
