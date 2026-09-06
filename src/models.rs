use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaigretDatabase {
    pub sites: HashMap<String, Site>,
    pub engines: HashMap<String, Engine>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Engine {
    pub name: String,
    pub site: Site,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Site {
    pub tags: Option<Vec<String>>,
    pub check_type: Option<String>, // message, response_url, status_code
    pub url_main: Option<String>,
    pub url: Option<String>,
    pub url_probe: Option<String>,
    pub regex_check: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    pub presense_strs: Option<Vec<String>>,
    pub absence_strs: Option<Vec<String>>,
    pub errors: Option<HashMap<String, String>>,
    pub alexa_rank: Option<u64>,
    pub username_claimed: Option<String>,
    pub username_unclaimed: Option<String>,
    pub disabled: Option<bool>,
    pub engine: Option<String>,
    pub source: Option<String>,
    pub activation: Option<Activation>,
    pub request_method: Option<String>,
    pub request_payload: Option<serde_json::Value>,
    pub request_head_only: Option<bool>,
    pub get_params: Option<HashMap<String, String>>,
    pub ignore403: Option<bool>,
    pub similar_search: Option<bool>,
    pub url_subpath: Option<String>,
    pub protocol: Option<String>,
    pub protection: Option<Vec<String>>,
    pub stats: Option<serde_json::Value>,
}

impl Site {
    pub fn merge(&mut self, other: &Site) {
        if let Some(other_tags) = &other.tags {
            let mut tags = self.tags.take().unwrap_or_default();
            tags.extend(other_tags.clone());
            self.tags = Some(tags);
        }
        if self.check_type.is_none() {
            self.check_type = other.check_type.clone();
        }
        if self.url_main.is_none() {
            self.url_main = other.url_main.clone();
        }
        if self.url.is_none() {
            self.url = other.url.clone();
        }
        if self.url_probe.is_none() {
            self.url_probe = other.url_probe.clone();
        }
        if self.regex_check.is_none() {
            self.regex_check = other.regex_check.clone();
        }
        if let Some(other_headers) = &other.headers {
            let mut headers = self.headers.take().unwrap_or_default();
            for (k, v) in other_headers {
                headers.insert(k.clone(), v.clone());
            }
            self.headers = Some(headers);
        }
        if let Some(other_presense) = &other.presense_strs {
            let mut presense = self.presense_strs.take().unwrap_or_default();
            presense.extend(other_presense.clone());
            self.presense_strs = Some(presense);
        }
        if let Some(other_absence) = &other.absence_strs {
            let mut absence = self.absence_strs.take().unwrap_or_default();
            absence.extend(other_absence.clone());
            self.absence_strs = Some(absence);
        }
        if let Some(other_errors) = &other.errors {
            let mut errors = self.errors.take().unwrap_or_default();
            for (k, v) in other_errors {
                errors.insert(k.clone(), v.clone());
            }
            self.errors = Some(errors);
        }
        if self.alexa_rank.is_none() {
            self.alexa_rank = other.alexa_rank;
        }
        if self.username_claimed.is_none() {
            self.username_claimed = other.username_claimed.clone();
        }
        if self.username_unclaimed.is_none() {
            self.username_unclaimed = other.username_unclaimed.clone();
        }
        if self.disabled.is_none() {
            self.disabled = other.disabled;
        }
        if self.engine.is_none() {
            self.engine = other.engine.clone();
        }
        if self.source.is_none() {
            self.source = other.source.clone();
        }
        if self.activation.is_none() {
            self.activation = other.activation.clone();
        }
        if self.request_method.is_none() {
            self.request_method = other.request_method.clone();
        }
        if self.request_payload.is_none() {
            self.request_payload = other.request_payload.clone();
        }
        if self.request_head_only.is_none() {
            self.request_head_only = other.request_head_only;
        }
        if let Some(other_params) = &other.get_params {
            let mut params = self.get_params.take().unwrap_or_default();
            for (k, v) in other_params {
                params.insert(k.clone(), v.clone());
            }
            self.get_params = Some(params);
        }
        if self.ignore403.is_none() {
            self.ignore403 = other.ignore403;
        }
        if self.similar_search.is_none() {
            self.similar_search = other.similar_search;
        }
        if self.url_subpath.is_none() {
            self.url_subpath = other.url_subpath.clone();
        }
        if self.protocol.is_none() {
            self.protocol = other.protocol.clone();
        }
        if let Some(other_protection) = &other.protection {
            let mut protection = self.protection.take().unwrap_or_default();
            protection.extend(other_protection.clone());
            self.protection = Some(protection);
        }
    }

    pub fn extract_username_from_url(&self, test_url: &str) -> Option<String> {
        if let Some(url_template) = &self.url {
            let parts: Vec<&str> = url_template.split("{username}").collect();
            if parts.len() == 2 {
                let prefix = parts[0];
                let suffix = parts[1];
                if test_url.len() >= prefix.len() + suffix.len()
                    && test_url.starts_with(prefix)
                    && test_url.ends_with(suffix)
                {
                    let mut username = test_url[prefix.len()..].to_string();
                    if !suffix.is_empty() {
                        username = username[..username.len() - suffix.len()].to_string();
                    }
                    if !username.is_empty() && !username.contains('/') {
                        return Some(username);
                    }
                }
            }
        }
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Activation {
    pub method: String,
    pub marks: Option<Vec<String>>,
    pub url: Option<String>,
    pub src: Option<String>,
    pub dst: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MaigretCheckStatus {
    Claimed,
    Available,
    Unknown,
    Illegal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaigretCheckResult {
    pub username: String,
    pub site_name: String,
    pub site_url_user: String,
    pub status: MaigretCheckStatus,
    pub ids_data: HashMap<String, String>,
    pub ids_links: Vec<String>,
    pub query_time: Option<f64>,
    pub context: Option<String>,
    pub error: Option<String>,
    pub tags: Vec<String>,
}
