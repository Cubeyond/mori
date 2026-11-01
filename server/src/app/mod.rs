//! Validated runtime configuration, shared resources, and secure identifiers.

use crate::infra::{db, storage_settings::StorageManager};
use anyhow::{Context, bail};
use clap::Parser;
use rand::{RngCore, rngs::OsRng};
use sqlx::SqlitePool;
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::Semaphore;
use url::Url;

/// Command-line and environment-backed runtime settings.
#[derive(Clone, Parser)]
pub struct Config {
    /// Internal HTTP socket address.
    #[arg(long, env = "MORI_LISTEN", default_value = "0.0.0.0:3000")]
    pub listen: String,
    #[arg(
        long,
        env = "MORI_DATABASE_URL",
        default_value = "sqlite://data/mori.db"
    )]
    /// SQLite connection URL for metadata and durable tasks.
    pub database_url: String,
    /// Canonical browser origin used for CSRF checks and sharing URLs.
    #[arg(long, env = "MORI_PUBLIC_URL", default_value = "http://localhost:3000")]
    pub public_url: String,
    /// Optional S3 endpoint override for isolated protocol tests.
    #[arg(long, env = "MORI_S3_ENDPOINT")]
    pub s3_endpoint: Option<String>,
    /// Optional `CloudFront` endpoint override for isolated protocol previews.
    #[arg(long, env = "MORI_CLOUDFRONT_ENDPOINT")]
    pub cloudfront_endpoint: Option<String>,
    /// Directory containing the immutable static frontend build.
    #[arg(long, env = "MORI_WEB_DIR", default_value = "web/build")]
    pub web_dir: String,
    /// Single administrator name, provisioned through Compose environment settings.
    #[arg(long, env = "MORI_ADMIN_USERNAME", default_value = "root")]
    pub admin_username: String,
    /// Optional environment-provisioned password; plaintext is discarded after startup.
    #[arg(long, env = "MORI_ADMIN_PASSWORD", hide_env_values = true)]
    pub admin_password: Option<String>,
    /// Maximum bytes per upload; the default is one decimal gigabyte.
    #[arg(long, env = "MORI_MAX_UPLOAD_BYTES", default_value_t = 1_000_000_000)]
    pub max_upload_bytes: i64,
    /// Allowed filename extensions; codecs are never restricted or transcoded.
    #[arg(
        long,
        env = "MORI_VIDEO_EXTENSIONS",
        value_delimiter = ',',
        default_value = "mp4,m4v,mov,webm,mkv,avi,ogv,mpeg,mpg,ts,mts,m2ts,3gp,wmv,flv"
    )]
    pub video_extensions: Vec<String>,
    /// Allowed image containers; originals are retained even when a browser cannot decode them.
    #[arg(
        long,
        env = "MORI_IMAGE_EXTENSIONS",
        value_delimiter = ',',
        default_value = "jpg,jpeg,png,apng,gif,webp,avif,bmp,ico,tif,tiff,heic,heif,jxl,svg,dng,cr2,cr3,nef,arw,raf,rw2,orf,pef,psd"
    )]
    pub image_extensions: Vec<String>,
    /// Optional path to the `CloudFront` origin-header secret.
    #[arg(long, env = "MORI_ORIGIN_SECRET_FILE")]
    pub origin_secret_file: Option<String>,
}

impl Config {
    /// Normalize the public origin and reject unsupported or unsafe configuration.
    ///
    /// # Errors
    /// Returns invalid origin, transport, or upload-limit configuration.
    pub fn validate(&mut self) -> anyhow::Result<()> {
        let url = Url::parse(&self.public_url)?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            bail!("MORI_PUBLIC_URL must be an HTTP(S) origin");
        }
        self.public_url = url.origin().ascii_serialization();
        if !(16..=5_000_000_000).contains(&self.max_upload_bytes) {
            bail!("MORI_MAX_UPLOAD_BYTES must be 16..=5000000000 for S3 CopyObject publication");
        }
        if self.admin_username.is_empty()
            || self.admin_username.len() > 64
            || self.admin_username.chars().any(char::is_control)
        {
            bail!("MORI_ADMIN_USERNAME must contain 1-64 bytes without control characters");
        }
        if self
            .admin_password
            .as_ref()
            .is_some_and(|password| !(12..=1024).contains(&password.len()))
        {
            bail!("MORI_ADMIN_PASSWORD must contain 12-1024 bytes");
        }
        for extension in &mut self.video_extensions {
            *extension = extension.trim().to_ascii_lowercase();
            if extension.is_empty()
                || extension.len() > 12
                || extension == "jpg"
                || !extension
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
            {
                bail!(
                    "MORI_VIDEO_EXTENSIONS must contain safe video extensions; configure jpg with MORI_IMAGE_EXTENSIONS"
                );
            }
        }
        self.video_extensions.sort();
        self.video_extensions.dedup();
        if self.video_extensions.is_empty() {
            bail!("MORI_VIDEO_EXTENSIONS must not be empty");
        }
        for extension in &mut self.image_extensions {
            *extension = extension.trim().to_ascii_lowercase();
            if extension.is_empty()
                || extension.len() > 12
                || !extension
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
                || self.video_extensions.contains(extension)
            {
                bail!(
                    "MORI_IMAGE_EXTENSIONS must contain safe extensions distinct from video extensions"
                );
            }
        }
        self.image_extensions.sort();
        self.image_extensions.dedup();
        if self.image_extensions.is_empty() {
            bail!("MORI_IMAGE_EXTENSIONS must not be empty");
        }
        if url.scheme() != "https" && !matches!(url.host_str(), Some("localhost" | "127.0.0.1")) {
            bail!("HTTPS is required outside localhost");
        }
        Ok(())
    }
}

/// Cloneable handles to validated resources and concurrency budgets.
#[derive(Clone)]
pub struct AppState {
    /// Bounded pool of SQLite metadata connections.
    pub db: SqlitePool,
    /// Configured AWS storage and distribution adapter.
    pub storage: StorageManager,
    /// Validated immutable runtime configuration.
    pub config: Arc<Config>,
    /// Concurrency budget for memory-intensive password verification.
    pub password_slots: Arc<Semaphore>,
    /// Concurrency budget retained for each private response stream.
    pub preview_slots: Arc<Semaphore>,
    /// Separate bounded poster budget so gallery covers cannot exhaust video streams.
    pub poster_slots: Arc<Semaphore>,
    /// Optional expected CloudFront-to-origin authentication value.
    pub origin_secret: Option<Arc<String>>,
    /// Hashes authorizing only the trusted frontend bootstrap scripts.
    pub script_csp: Arc<String>,
    /// Bounded login-failure count and earliest permitted retry timestamp.
    pub login_backoff: Arc<tokio::sync::Mutex<(u32, i64)>>,
    /// Wake the single storage worker when an HTTP mutation schedules new work.
    pub job_wakeup: Arc<tokio::sync::Notify>,
}

impl AppState {
    /// Initialize validated application resources and bounded external clients.
    ///
    /// # Errors
    /// Returns invalid configuration, secret-file, database, or AWS credential initialization
    /// errors.
    pub async fn new(mut config: Config) -> anyhow::Result<Self> {
        config.validate()?;
        let origin_secret = if let Some(path) = &config.origin_secret_file {
            let value = tokio::fs::read_to_string(path)
                .await
                .context("reading origin secret")?;
            if value.trim().len() < 32 {
                bail!("origin secret requires at least 32 characters");
            }
            Some(Arc::new(value.trim().to_owned()))
        } else {
            None
        };
        let html = tokio::fs::read_to_string(format!("{}/index.html", config.web_dir))
            .await
            .unwrap_or_default();
        let script_csp = Arc::new(inline_script_hashes(&html));
        let db = db::connect(&config.database_url).await?;
        crate::features::auth::configure(&db, &config.admin_username, config.admin_password.take())
            .await?;
        let storage = StorageManager::load(&db, &config).await?;
        Ok(Self {
            script_csp,
            db,
            storage,
            config: Arc::new(config),
            password_slots: Arc::new(Semaphore::new(1)),
            preview_slots: Arc::new(Semaphore::new(2)),
            poster_slots: Arc::new(Semaphore::new(4)),
            origin_secret,
            login_backoff: Arc::new(tokio::sync::Mutex::new((0, 0))),
            job_wakeup: Arc::new(tokio::sync::Notify::new()),
        })
    }
}

/// Return Unix seconds, saturating clocks outside the representable range.
#[must_use]
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_secs()).unwrap_or(i64::MAX)
        })
}
/// Generate an independent 128-bit operating-system random identifier.
///
/// # Panics
/// Panics if the operating system cannot provide secure entropy.
#[must_use]
pub fn random_id() -> String {
    let mut bytes = [0; 16];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}
/// Check the canonical lowercase hexadecimal public-token representation.
#[must_use]
pub fn valid_token(s: &str) -> bool {
    s.len() == 32
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
/// Validate an editable display name without accepting control characters.
pub fn valid_name(s: &str) -> bool {
    !s.trim().is_empty() && s.chars().count() <= 200 && !s.chars().any(char::is_control)
}

/// Hash only the trusted build's inline bootstrap scripts; no unsafe-inline JS.
#[must_use]
pub fn inline_script_hashes(html: &str) -> String {
    use base64::Engine;
    use sha2::{Digest, Sha256};
    let mut rest = html;
    let mut hashes = Vec::new();
    while let Some(start) = rest.find("<script") {
        rest = &rest[start..];
        let Some(open) = rest.find('>') else { break };
        rest = &rest[open + 1..];
        let Some(close) = rest.find("</script>") else {
            break;
        };
        hashes.push(format!(
            "'sha256-{}'",
            base64::engine::general_purpose::STANDARD
                .encode(Sha256::digest(&rest.as_bytes()[..close]))
        ));
        rest = &rest[close + 9..];
    }
    hashes.join(" ")
}
