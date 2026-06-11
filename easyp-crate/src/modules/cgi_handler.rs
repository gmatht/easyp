use std::path::Path;

pub async fn execute_cgi_script(
    _script_path: &Path,
    _env: &crate::cgi_env::CgiEnv,
    _body: Option<&[u8]>,
    _timeout_secs: u64,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    Err("CGI handler not implemented".into())
}
