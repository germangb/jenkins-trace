use jenkins_trace::{Config, CrumbUrl, JenkinsTrace};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // config specific job
    let config = Config {
        url: "http://localhost:8080/job/foo/5/logText/progressiveText".to_string(),
        crumb_url: CrumbUrl::Json("http://localhost:8080/crumbIssuer/api/json".to_string()),
        auth: Some(("root".to_string(), Some("root".to_string()))),
    };

    // create jenkins tracer
    let mut trace = JenkinsTrace::new(config);

    loop {
        match trace.next_trace().await? {
            Some(bytes) => {
                let mut read = std::io::Cursor::new(bytes);
                let mut write = tokio::io::stdout();

                tokio::io::copy(&mut read, &mut write).await?;
            }
            None => break,
        }
    }

    Ok(())
}
