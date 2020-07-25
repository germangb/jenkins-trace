#![deny(unused)]

use bytes::Bytes;
use reqwest::{multipart::Form, Client, RequestBuilder, Response};
use std::fmt;

/// CSRF Crumb request endpoint.
///
/// Must be one of:
/// - http://<server>/crumbIssuer/api/json
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CrumbUrl {
    Json(String),
}

impl CrumbUrl {
    fn as_str(&self) -> &str {
        match self {
            CrumbUrl::Json(url) => url.as_str(),
        }
    }
}

/// Jenkins trace error.
#[derive(Debug)]
pub enum Error {
    /// Some error related to the HTTP request.
    Reqwest(reqwest::Error),
    /// Some error related to Jenkins service.
    Jenkins(&'static str),
    /// JSON error when parsing the CSRF crumb.
    Json(serde_json::Error),
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Error::Reqwest(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Error::Json(error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Reqwest(req) => req.fmt(f),
            Error::Jenkins(jen) => jen.fmt(f),
            Error::Json(json) => json.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

/// Basic auth username & password.
pub type Auth = (String, Option<String>);
type Crumb = (String, String);

/// Jenkins job
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Config {
    /// Job url. Must be one of either:
    /// - `http://<server>/job/<project>/<build_id>/progressiveText`
    /// - `http://<server>/job/<project>/<build_id>/progressiveHtml`
    pub url: String,
    /// Crumb endpoint.
    pub crumb_url: CrumbUrl,
    /// HTTP basic auth.
    pub auth: Option<Auth>,
}

/// A type to read the log of a given jenkins build.
pub struct JenkinsTrace {
    config: Config,
    client: Client,
    // To keep track of the # of bytes read so far.
    offset: u64,
    ended: bool,
    crumb: Option<Crumb>,
}

impl JenkinsTrace {
    const MORE_DATA_FIELD: &'static str = "X-More-Data";
    const TEXT_SIZE_FIELD: &'static str = "X-Text-Size";

    /// Create a new jenkins trace with the given job parameters.
    pub fn new(config: Config) -> Self {
        Self {
            config,
            client: Client::new(),
            offset: 0,
            ended: false,
            crumb: None,
        }
    }

    /// Returns a future that resolves to the next log trace.
    /// Returns None if the trace has ended.
    pub async fn next_trace(&mut self) -> Result<Option<Bytes>, Error> {
        if self.ended {
            return Ok(None);
        }

        // issue request to progressive log endpoint.
        let response = self.trace_request_future().await?;
        // X-More-Data header field
        let more_data = response.headers().contains_key(Self::MORE_DATA_FIELD);
        // X-Text-Size header field
        let text_size = response
            .headers()
            .get(Self::TEXT_SIZE_FIELD)
            .ok_or(Error::Jenkins("Missing X-Text-Size header"))?
            .to_str()
            .unwrap()
            .parse::<u64>()
            .map_err(|_| Error::Jenkins("Invalid X-Text-Size value"))?;

        self.offset = text_size;
        self.ended = !more_data;
        Ok(Some(response.bytes().await?))
    }

    // create request with basic auth
    fn base_request(&self, url: &str) -> RequestBuilder {
        let req = self.client.get(url);
        if let Some((user, passwd)) = &self.config.auth {
            req.basic_auth(user, passwd.as_ref())
        } else {
            req
        }
    }

    async fn csrf_crumb_future(&mut self) -> Result<Crumb, Error> {
        if let Some(crumb) = &self.crumb {
            return Ok(crumb.clone());
        }

        #[derive(serde::Deserialize)]
        struct C {
            crumb: String,
            #[serde(rename = "crumbRequestField")]
            crumb_request_field: String,
        }

        // issue crumb issuer request
        // fail if result is not 2xx
        let res = self
            .base_request(self.config.crumb_url.as_str())
            .send()
            .await
            .and_then(Response::error_for_status)?;
        let body = res.text().await?;

        let C {
            crumb,
            crumb_request_field,
        } = match self.config.crumb_url {
            CrumbUrl::Json(_) => serde_json::from_str(&body)?,
        };

        self.crumb = Some((crumb_request_field, crumb));
        Ok(self.crumb.clone().unwrap())
    }

    async fn trace_request_future(&mut self) -> Result<Response, Error> {
        // request CSRF crumb
        let (crumb_field, crumb) = self.csrf_crumb_future().await?;

        // request next log
        // fails if response code isn't 2xx
        Ok(self
            .base_request(&self.config.url)
            .multipart(Form::new().text("start", format!("{}", self.offset)))
            .header(&crumb_field, &crumb)
            .send()
            .await
            .and_then(Response::error_for_status)?)
    }
}
