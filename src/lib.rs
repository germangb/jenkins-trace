use bytes::Bytes;
use futures::TryFutureExt;
use reqwest::{multipart::Form, Client, Response};
use std::{fmt, future::Future};

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

/// Jenkins job
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Config {
    /// Job url. Must be one of either:
    /// - `http://<server>/job/foo/<build_id>/progressiveText`
    /// - `http://<server>/job/foo/<build_id>/progressiveHtml`
    pub url: String,
    /// Crumb endpoint, must be
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
        }
    }

    /// Returns a future that resolves to the next log trace.
    /// Returns None if the trace has ended.
    pub async fn next_trace(&mut self) -> Result<Option<Bytes>, Error> {
        if self.ended {
            return Ok(None);
        }

        // issue request to progressive log endpoint
        // fail if response is non-2xx
        let response = self.trace_request_future().await?.error_for_status()?;

        // X-More-Data
        let more_data = response.headers().contains_key(Self::MORE_DATA_FIELD);
        // X-Text-Size
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

    async fn csrf_crumb_future(&mut self) -> Result<(String, String), Error> {
        #[derive(serde::Deserialize)]
        struct C {
            crumb: String,
            #[serde(rename = "crumbRequestField")]
            crumb_request_field: String,
        }

        let mut client = self.client.get(self.config.crumb_url.as_str());
        if let Some((user, passwd)) = &self.config.auth {
            client = client.basic_auth(user, passwd.as_ref());
        }
        let body = client.send().and_then(reqwest::Response::text).await?;

        let C {
            crumb,
            crumb_request_field,
        } = match self.config.crumb_url {
            CrumbUrl::Json(_) => serde_json::from_str(&body)?,
        };

        Ok((crumb_request_field, crumb))
    }

    async fn trace_request_future(&mut self) -> Result<Response, Error> {
        // request CSRF crumb
        let (crumb_field, crumb) = self.csrf_crumb_future().await?;

        let form = Form::new().text("start", format!("{}", self.offset));
        let mut client = self
            .client
            .get(&self.config.url)
            .multipart(form)
            .header(&crumb_field, &crumb);
        if let Some((user, passwd)) = &self.config.auth {
            client = client.basic_auth(user, passwd.as_ref());
        }

        Ok(client.send().await?)
    }
}
