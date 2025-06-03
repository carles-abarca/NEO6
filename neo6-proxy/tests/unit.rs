use axum::http::{StatusCode, Request};
use axum::body::Body;
use axum::Router;
use serde_json::json;
use tower::util::ServiceExt; // for .oneshot()

// Import from the local crate
use neo6_proxy::proxy::router::create_router;
use neo6_proxy::cics::mapping::load_transaction_map;

#[tokio::test]
async fn test_health_endpoint() {
    let app = create_router();
    let response = axum::http::request::Builder::new()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(response).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_invoke_endpoint() {
    let app = create_router();
    let payload = json!({
        "transaction_id": "TX01",
        "parameters": { "account_number": "1234567890", "amount": 500.25 }
    });
    let request = Request::builder()
        .method("POST")
        .uri("/invoke")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_invoke_async_endpoint() {
    let app = create_router();
    let payload = json!({
        "transaction_id": "TX01",
        "parameters": { "account_number": "1234567890", "amount": 500.25 }
    });
    let request = Request::builder()
        .method("POST")
        .uri("/invoke-async")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_status_endpoint() {
    let app = create_router();
    let request = Request::builder()
        .method("GET")
        .uri("/status/tx123")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_metrics_endpoint() {
    let app = create_router();
    let request = Request::builder()
        .method("GET")
        .uri("/metrics")
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_invoke_tx01_valid() {
    let app = create_router();
    let payload = json!({
        "transaction_id": "TX01",
        "parameters": { "account_number": "1234567890", "amount": 500.25 }
    });
    let request = Request::builder()
        .method("POST")
        .uri("/invoke")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    // Optionally, check the response body for expected fields
}

#[tokio::test]
async fn test_invoke_tx02_valid() {
    let app = create_router();
    let payload = json!({
        "transaction_id": "TX02",
        "parameters": { "customer_id": "CUST001" }
    });
    let request = Request::builder()
        .method("POST")
        .uri("/invoke")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    // Optionally, check the response body for expected fields
}

#[tokio::test]
async fn test_invoke_invalid_transaction() {
    let app = create_router();
    let payload = json!({
        "transaction_id": "INVALID_TX",
        "parameters": { "foo": "bar" }
    });
    let request = Request::builder()
        .method("POST")
        .uri("/invoke")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    // Optionally, check the response body for error message
}
