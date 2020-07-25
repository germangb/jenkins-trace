use jenkins_trace::{Config, CrumbUrl, JenkinsTrace};
use std::io::Cursor;
use structopt::StructOpt;
use tokio::io::{copy, stdout};
use url::Url;

/// Command-line arguments.
#[derive(Debug, StructOpt)]
struct Opt {
    /// Jenkins host.
    #[structopt(short, long)]
    host: String,
    /// Jenkins project name.
    #[structopt(short, long)]
    job: String,
    /// Numeric ID of the build.
    #[structopt(short, long)]
    build: u64,
    /// Use HTML output.
    #[structopt(short = "H", long)]
    html: bool,
    /// Jenkins login credentials.
    #[structopt(short, long)]
    user: Option<String>,
}

impl Opt {
    /// Return job progressive log endpoint.
    fn url(&self) -> String {
        let endpoint = if self.html {
            "progressiveHtml"
        } else {
            "progressiveText"
        };
        let url = format!(
            "{host}/job/{job}/{build}/logText/{endpoint}",
            host = self.host,
            job = self.job,
            build = self.build,
            endpoint = endpoint,
        );
        Url::parse(&url)
            .map(Url::into_string)
            .expect("Error parsing Jenkins log URL")
    }

    /// Return crumb endpoint.
    fn crumb(&self) -> CrumbUrl {
        let url = format!("{}/crumbIssuer/api/json", self.host);
        CrumbUrl::Json(
            Url::parse(&url)
                .map(Url::into_string)
                .expect("Error parsing Jenkins crumbIssuer URL"),
        )
    }

    /// Return Basic auth.
    fn auth(&self) -> Option<(String, Option<String>)> {
        if let Some(auth) = &self.user {
            let mut split = auth.split(':');
            let user = split.next().unwrap().to_string();
            let pass = split.next().map(|p| p.to_string());
            Some((user, pass))
        } else {
            None
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    let config = Config {
        url: opt.url(),
        crumb_url: opt.crumb(),
        auth: opt.auth(),
    };

    let mut trace = JenkinsTrace::new(config);

    while let Some(bytes) = trace.next_trace().await? {
        copy(&mut Cursor::new(bytes), &mut stdout()).await?;
    }

    Ok(())
}
