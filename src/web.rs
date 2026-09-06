use askama::Template;
use axum::{
    Form, Router,
    extract::{Path as AxumPath, State},
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tower_http::services::ServeDir;

use crate::checker::Maigret;
use crate::models::{MaigretCheckStatus, MaigretDatabase};

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    site_options: Vec<String>,
    error: Option<String>,
}

#[derive(Template)]
#[template(path = "status.html")]
struct StatusTemplate {
    timestamp: String,
}

#[derive(Template)]
#[template(path = "results.html")]
struct ResultsTemplate {
    message: Option<String>,
    graph_file: Option<String>,
    individual_reports: Vec<IndividualReport>,
}

#[derive(Clone)]
struct IndividualReport {
    username: String,
    csv_file: String,
    json_file: String,
    pdf_file: String,
    html_file: String,
    claimed_profiles: Vec<ClaimedProfile>,
}

#[derive(Clone)]
struct ClaimedProfile {
    site_name: String,
    url: String,
    tags: Vec<String>,
}

#[derive(Deserialize)]
struct SearchForm {
    usernames: String,
    concurrency: Option<usize>,
}

#[derive(Clone)]
struct AppState {
    db: MaigretDatabase,
    // Timestamp -> Status string
    jobs: Arc<Mutex<HashMap<String, String>>>,
    // Timestamp -> Results
    results: Arc<Mutex<HashMap<String, Vec<IndividualReport>>>>,
}

pub async fn start_server(db: MaigretDatabase, port: u16) {
    let state = AppState {
        db,
        jobs: Arc::new(Mutex::new(HashMap::new())),
        results: Arc::new(Mutex::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/", get(index))
        .route("/search", post(search))
        .route("/status/{timestamp}", get(status))
        .route("/results/{timestamp}", get(results_page))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .unwrap();
    println!("Web server listening on http://127.0.0.1:{}", port);
    axum::serve(listener, app).await.unwrap();
}

async fn index(State(state): State<AppState>) -> impl IntoResponse {
    let mut site_options: Vec<String> = state.db.sites.keys().cloned().collect();
    site_options.sort();

    let template = IndexTemplate {
        site_options,
        error: None,
    };
    Html(template.render().unwrap())
}

async fn search(State(state): State<AppState>, Form(input): Form<SearchForm>) -> impl IntoResponse {
    let usernames: Vec<String> = input
        .usernames
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    if usernames.is_empty() {
        return Redirect::to("/").into_response();
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();

    {
        let mut jobs = state.jobs.lock().await;
        jobs.insert(timestamp.clone(), "running".to_string());
    }

    let state_clone = state.clone();
    let ts_clone = timestamp.clone();
    let concurrency = input.concurrency.unwrap_or(50);

    tokio::spawn(async move {
        let maigret = Maigret::new(state_clone.db.clone());
        let search_results = maigret.search(usernames.clone(), concurrency).await;

        let mut individual_reports = Vec::new();
        for username in usernames {
            let mut claimed = Vec::new();
            for res in &search_results {
                if res.username == username && res.status == MaigretCheckStatus::Claimed {
                    claimed.push(ClaimedProfile {
                        site_name: res.site_name.clone(),
                        url: res.site_url_user.clone(),
                        tags: res.tags.clone(),
                    });
                }
            }

            individual_reports.push(IndividualReport {
                username: username.clone(),
                csv_file: format!("report_{}.csv", username),
                json_file: format!("report_{}.json", username),
                pdf_file: format!("report_{}.pdf", username),
                html_file: format!("report_{}.html", username),
                claimed_profiles: claimed,
            });
        }

        {
            let mut results = state_clone.results.lock().await;
            results.insert(ts_clone.clone(), individual_reports);
        }

        {
            let mut jobs = state_clone.jobs.lock().await;
            jobs.insert(ts_clone, "completed".to_string());
        }
    });

    Redirect::to(&format!("/status/{}", timestamp)).into_response()
}

async fn status(
    State(state): State<AppState>,
    AxumPath(timestamp): AxumPath<String>,
) -> impl IntoResponse {
    let status = {
        let jobs = state.jobs.lock().await;
        jobs.get(&timestamp).cloned()
    };

    match status.as_deref() {
        Some("completed") => Redirect::to(&format!("/results/{}", timestamp)).into_response(),
        Some("running") => {
            let template = StatusTemplate { timestamp };
            Html(template.render().unwrap()).into_response()
        }
        _ => Redirect::to("/").into_response(),
    }
}

async fn results_page(
    State(state): State<AppState>,
    AxumPath(timestamp): AxumPath<String>,
) -> impl IntoResponse {
    let reports = {
        let results = state.results.lock().await;
        results.get(&timestamp).cloned().unwrap_or_default()
    };

    let template = ResultsTemplate {
        message: Some("Search finished!".to_string()),
        graph_file: None,
        individual_reports: reports,
    };

    Html(template.render().unwrap()).into_response()
}
