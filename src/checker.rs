use crate::models::{MaigretCheckResult, MaigretCheckStatus, MaigretDatabase, Site};
use regex::Regex;
use reqwest::Client;
use std::collections::HashMap;
use std::time::Instant;

pub struct Maigret {
    pub db: MaigretDatabase,
    pub client: Client,
}

use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};

impl Maigret {
    pub async fn search(
        &self,
        initial_usernames: Vec<String>,
        concurrency: usize,
    ) -> Vec<MaigretCheckResult> {
        let mut all_results = Vec::new();
        let mut searched_usernames = std::collections::HashSet::new();
        let mut usernames_to_search = initial_usernames;

        while !usernames_to_search.is_empty() {
            let mut next_usernames = std::collections::HashSet::new();
            let current_usernames = usernames_to_search.clone();
            usernames_to_search.clear();

            for username in current_usernames {
                if searched_usernames.contains(&username) {
                    continue;
                }
                searched_usernames.insert(username.clone());

                println!("\nSearching for username: {}", username);
                let sites: Vec<_> = self
                    .db
                    .sites
                    .iter()
                    .filter(|(_, s)| !s.disabled.unwrap_or(false))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                let pb = ProgressBar::new(sites.len() as u64);
                pb.set_style(ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                    .unwrap()
                    .progress_chars("#>-"));

                let results = futures::stream::iter(sites)
                    .map(|(site_name, site_data)| {
                        let username = username.clone();
                        let pb = pb.clone();
                        async move {
                            let result = self.check_site(&site_name, &site_data, &username).await;
                            pb.inc(1);
                            result
                        }
                    })
                    .buffer_unordered(concurrency)
                    .collect::<Vec<_>>()
                    .await;

                pb.finish_with_message("Search complete");

                for mut result in results {
                    if result.status == MaigretCheckStatus::Claimed {
                        println!("[+] {}: Found!", result.site_name);

                        for link in &result.ids_links {
                            for (_, site_config) in &self.db.sites {
                                if let Some(new_username) =
                                    site_config.extract_username_from_url(link)
                                {
                                    if new_username != username
                                        && !searched_usernames.contains(&new_username)
                                    {
                                        next_usernames.insert(new_username.clone());
                                        result
                                            .ids_data
                                            .insert(new_username, "username".to_string());
                                    }
                                }
                            }
                        }
                    }
                    all_results.push(result);
                }
            }
            usernames_to_search.extend(next_usernames);
        }

        all_results
    }

    pub fn new(db: MaigretDatabase) -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap();
        Self { db, client }
    }

    pub fn validate_username(&self, site: &Site, username: &str) -> bool {
        if let Some(regex_str) = &site.regex_check {
            if let Ok(re) = Regex::new(regex_str) {
                return re.is_match(username);
            }
        }
        true
    }

    pub fn extract_metadata(&self, html: &str) -> (HashMap<String, String>, Vec<String>) {
        let ids_data = HashMap::new();
        let mut ids_links = Vec::new();

        // Disabled temporary overly-aggressive extractor to prevent runaway recursive searches.
        // A proper port of `socid-extractor` is needed for accurate link harvesting.
        /*
        if let Ok(re) = Regex::new(r#"href=["'](https?://[^"']+)["']"#) {
            for cap in re.captures_iter(html) {
                if let Some(m) = cap.get(1) {
                    ids_links.push(m.as_str().to_string());
                }
            }
        }
        */

        // Remove duplicates
        ids_links.sort();
        ids_links.dedup();

        (ids_data, ids_links)
    }

    pub async fn check_site(
        &self,
        site_name: &str,
        site: &Site,
        username: &str,
    ) -> MaigretCheckResult {
        let start_time = Instant::now();
        let mut site_data = site.clone();
        if let Some(engine_name) = &site.engine {
            if let Some(engine) = self.db.engines.get(engine_name) {
                site_data.merge(&engine.site);
            }
        }

        let url_template = match &site_data.url {
            Some(u) => u,
            None => {
                return MaigretCheckResult {
                    username: username.to_string(),
                    site_name: site_name.to_string(),
                    site_url_user: "".to_string(),
                    status: MaigretCheckStatus::Unknown,
                    ids_data: HashMap::new(),
                    ids_links: Vec::new(),
                    query_time: Some(start_time.elapsed().as_secs_f64()),
                    context: None,
                    error: Some("No URL template".to_string()),
                    tags: site_data.tags.clone().unwrap_or_default(),
                };
            }
        };

        let site_url_user = url_template.replace("{username}", username);

        if !self.validate_username(&site_data, username) {
            return MaigretCheckResult {
                username: username.to_string(),
                site_name: site_name.to_string(),
                site_url_user,
                status: MaigretCheckStatus::Illegal,
                ids_data: HashMap::new(),
                ids_links: Vec::new(),
                query_time: Some(start_time.elapsed().as_secs_f64()),
                context: None,
                error: Some("Unsupported username format".to_string()),
                tags: site_data.tags.clone().unwrap_or_default(),
            };
        }

        let mut request = self.client.get(&site_url_user);
        if let Some(headers) = &site_data.headers {
            for (k, v) in headers {
                request = request.header(k, v);
            }
        }

        let response = match request.send().await {
            Ok(res) => res,
            Err(e) => {
                return MaigretCheckResult {
                    username: username.to_string(),
                    site_name: site_name.to_string(),
                    site_url_user,
                    status: MaigretCheckStatus::Unknown,
                    ids_data: HashMap::new(),
                    ids_links: Vec::new(),
                    query_time: Some(start_time.elapsed().as_secs_f64()),
                    context: None,
                    error: Some(e.to_string()),
                    tags: site_data.tags.clone().unwrap_or_default(),
                };
            }
        };

        let status_code = response.status();
        let ignore403 = site_data.ignore403.unwrap_or(false);
        let final_url = response.url().to_string();

        let text = match response.text().await {
            Ok(t) => t,
            Err(e) => {
                return MaigretCheckResult {
                    username: username.to_string(),
                    site_name: site_name.to_string(),
                    site_url_user,
                    status: MaigretCheckStatus::Unknown,
                    ids_data: HashMap::new(),
                    ids_links: Vec::new(),
                    query_time: Some(start_time.elapsed().as_secs_f64()),
                    context: None,
                    error: Some(e.to_string()),
                    tags: site_data.tags.clone().unwrap_or_default(),
                };
            }
        };

        let mut check_status = MaigretCheckStatus::Available;
        let mut error = None;

        if status_code == 403 && !ignore403 {
            check_status = MaigretCheckStatus::Available;
        } else {
            let check_type = site_data.check_type.as_deref().unwrap_or("status_code");
            match check_type {
                "status_code" => {
                    if status_code.is_success() || (status_code == 403 && ignore403) {
                        check_status = MaigretCheckStatus::Claimed;
                    }
                }
                "message" => {
                    let mut found = false;
                    if let Some(presense) = &site_data.presense_strs {
                        for s in presense {
                            if text.contains(s) {
                                found = true;
                                break;
                            }
                        }
                    } else {
                        found = status_code.is_success();
                    }

                    if let Some(absence) = &site_data.absence_strs {
                        for s in absence {
                            if text.contains(s) {
                                found = false;
                                break;
                            }
                        }
                    }

                    if found {
                        check_status = MaigretCheckStatus::Claimed;
                    }
                }
                "response_url" => {
                    if final_url == site_url_user {
                        check_status = MaigretCheckStatus::Claimed;
                    } else {
                        if !final_url.contains("404")
                            && !final_url.contains("not-found")
                            && !final_url.contains("error")
                        {
                            check_status = MaigretCheckStatus::Claimed;
                        }
                    }
                }
                _ => {
                    check_status = MaigretCheckStatus::Unknown;
                    error = Some(format!("Unknown check type: {}", check_type));
                }
            }
        }

        let (ids_data, ids_links) = if check_status == MaigretCheckStatus::Claimed {
            self.extract_metadata(&text)
        } else {
            (HashMap::new(), Vec::new())
        };
        MaigretCheckResult {
            username: username.to_string(),
            site_name: site_name.to_string(),
            site_url_user,
            status: check_status,
            ids_data,
            ids_links,
            query_time: Some(start_time.elapsed().as_secs_f64()),
            context: None,
            error,
            tags: site_data.tags.clone().unwrap_or_default(),
        }
    }
}
