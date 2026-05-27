use askama::Template;
use axum::{
    Router,
    extract::{Query, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse},
    routing::get,
};
use serde::Deserialize;
use std::{env, net::SocketAddr};

#[derive(Clone)]
struct AppState {
    client: reqwest::Client,
    default_user: String,
}

#[derive(Deserialize)]
struct HomeParams {
    user: Option<String>,
}

#[derive(Deserialize, Clone)]
struct GitHubProfile {
    login: String,
    name: Option<String>,
    bio: Option<String>,
    followers: u64,
    following: u64,
    public_repos: u64,
    created_at: String,
    avatar_url: String,
    html_url: String,
}

#[derive(Deserialize)]
struct GitHubRepo {
    name: String,
    description: Option<String>,
    language: Option<String>,
    stargazers_count: u64,
    forks_count: u64,
    html_url: String,
}

#[derive(Clone)]
struct RepoCard {
    name: String,
    description: String,
    language: String,
    stars: u64,
    forks: u64,
    html_url: String,
}

#[derive(Template)]
#[template(path = "index.html")]
struct HomeTemplate {
    profile: GitHubProfile,
    repos: Vec<RepoCard>,
    profile_name: String,
    member_since: String,
    stats_card_url: String,
    streak_card_url: String,
    languages_card_url: String,
    contributions_chart_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let default_user = env::var("GITHUB_USERNAME").unwrap_or_else(|_| "octocat".to_string());
    let client = reqwest::Client::builder()
        .user_agent("website-test-rust-profile-site")
        .build()?;

    let state = AppState {
        client,
        default_user,
    };

    let app = Router::new()
        .route("/", get(home))
        .route("/styles.css", get(styles_css))
        .with_state(state);

    let port = env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    println!("Server running on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn styles_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../styles.css"),
    )
}

async fn home(State(state): State<AppState>, Query(params): Query<HomeParams>) -> impl IntoResponse {
    let requested_user = params.user.as_deref().unwrap_or(&state.default_user);
    let username = sanitize_username(requested_user).unwrap_or_else(|| state.default_user.clone());

    let (profile_result, repos_result) = tokio::join!(
        fetch_profile(&state.client, &username),
        fetch_repos(&state.client, &username)
    );

    let profile = profile_result.unwrap_or_else(|_| fallback_profile(&username));
    let repos = repos_result
        .unwrap_or_default()
        .into_iter()
        .map(|repo| RepoCard {
            name: repo.name,
            description: repo
                .description
                .unwrap_or_else(|| "No description provided.".to_string()),
            language: repo.language.unwrap_or_else(|| "Code".to_string()),
            stars: repo.stargazers_count,
            forks: repo.forks_count,
            html_url: repo.html_url,
        })
        .collect::<Vec<_>>();

    let profile_name = profile.name.clone().unwrap_or_else(|| profile.login.clone());
    let member_since = profile
        .created_at
        .split('-')
        .next()
        .unwrap_or("N/A")
        .to_string();

    let template = HomeTemplate {
        profile,
        repos,
        profile_name,
        member_since,
        stats_card_url: format!(
            "https://github-readme-stats.vercel.app/api?username={username}&show_icons=true&hide_border=true&title_color=9f1f1f&text_color=2b2118&icon_color=9f1f1f&bg_color=f4ead2"
        ),
        streak_card_url: format!(
            "https://streak-stats.demolab.com?user={username}&hide_border=true&background=F4EAD2&ring=9F1F1F&fire=9F1F1F&currStreakLabel=2B2118"
        ),
        languages_card_url: format!(
            "https://github-readme-stats.vercel.app/api/top-langs/?username={username}&layout=compact&hide_border=true&title_color=9f1f1f&text_color=2b2118&bg_color=f4ead2"
        ),
        contributions_chart_url: format!("https://ghchart.rshah.org/9f1f1f/{username}"),
    };

    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to render page template",
        )
            .into_response(),
    }
}

fn sanitize_username(input: &str) -> Option<String> {
    let cleaned = input.trim();
    if cleaned.is_empty() {
        return None;
    }

    if cleaned
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        Some(cleaned.to_ascii_lowercase())
    } else {
        None
    }
}

async fn fetch_profile(client: &reqwest::Client, username: &str) -> Result<GitHubProfile, reqwest::Error> {
    client
        .get(format!("https://api.github.com/users/{username}"))
        .send()
        .await?
        .error_for_status()?
        .json::<GitHubProfile>()
        .await
}

async fn fetch_repos(client: &reqwest::Client, username: &str) -> Result<Vec<GitHubRepo>, reqwest::Error> {
    client
        .get(format!(
            "https://api.github.com/users/{username}/repos?sort=updated&per_page=6"
        ))
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<GitHubRepo>>()
        .await
}

fn fallback_profile(username: &str) -> GitHubProfile {
    GitHubProfile {
        login: username.to_string(),
        name: None,
        bio: Some("Unable to load GitHub profile right now.".to_string()),
        followers: 0,
        following: 0,
        public_repos: 0,
        created_at: "N/A".to_string(),
        avatar_url: "https://avatars.githubusercontent.com/u/583231?v=4".to_string(),
        html_url: format!("https://github.com/{username}"),
    }
}
