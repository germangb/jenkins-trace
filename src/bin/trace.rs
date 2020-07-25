use jenkins_trace::{Config, CrumbUrl, JenkinsTrace};
use std::{io::Cursor, time::Duration};
use structopt::StructOpt;
use tokio::{
    io::{copy, stdout},
    time::delay_for,
};
use url::Url;

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
    /// Jenkins login credentials (username:password).
    #[structopt(short, long)]
    user: Option<String>,
    /// Delay between requests in seconds.
    #[structopt(short, long, default_value = "1.0")]
    delay: f64,
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
    fn crumb_url(&self) -> CrumbUrl {
        let url = format!("{}/crumbIssuer/api/json", self.host);
        CrumbUrl::Json(
            Url::parse(&url)
                .map(Url::into_string)
                .expect("Error parsing Jenkins crumbIssuer URL"),
        )
    }

    /// Return Basic auth.
    fn auth(&self) -> Option<(String, Option<String>)> {
        self.user.as_ref().map(|auth| {
            let mut split = auth.split(':');
            let user = split.next().unwrap().to_string();
            let pass = split.next().map(|p| p.to_string());
            (user, pass)
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    let mut trace = JenkinsTrace::new(Config {
        url: opt.url(),
        crumb_url: opt.crumb_url(),
        auth: opt.auth(),
    });

    while let Some(bytes) = trace.next_trace().await? {
        copy(&mut Cursor::new(bytes), &mut stdout()).await?;
        delay_for(Duration::from_secs_f64(opt.delay)).await;
    }

    Ok(())
}
