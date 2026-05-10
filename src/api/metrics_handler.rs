//! `GET /metrics` handler — Prometheus text format export.
//!
//! RFC 401: Prometheus metrics endpoint.
//! RFC 822: access restricted to `server.monitoring_cidrs` (default: loopback only).
//! When ConnectInfo is absent (test/oneshot), defaults to 127.0.0.1 which is allowed.

use std::{net::{IpAddr, Ipv4Addr, SocketAddr}, sync::Arc};

use axum::{
    extract::State,
    http::{Request, StatusCode},
    response::IntoResponse,
};
use ipnet::IpNet;

use crate::{metrics, AppState};

pub async fn metrics_handler_no_ci(
    State(state): State<Arc<AppState>>,
    request:      Request<axum::body::Body>,
) -> impl IntoResponse {
    // Extract ConnectInfo if available (production with into_make_service_with_connect_info).
    // Fall back to loopback when absent (test/oneshot context).
    let client_ip: IpAddr = request
        .extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip())
        .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));

    let cfg = state.config();

    // RFC 822: restrict to monitoring_cidrs
    let allowed = cfg.server.monitoring_cidrs.iter().any(|cidr| {
        cidr.parse::<IpNet>().ok().map_or(false, |net| net.contains(&client_ip))
    });

    if !allowed {
        return StatusCode::FORBIDDEN.into_response();
    }

    match metrics::encode(&state.metrics.registry) {
        Ok(body) => (
            StatusCode::OK,
            [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
            body,
        )
            .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}
