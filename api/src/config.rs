use std::env::{self, VarError};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum BlobBackend {
    Local { root: PathBuf },
    S3(S3Config),
}

#[derive(Debug, Clone)]
pub struct S3Config {
    pub bucket: String,
    pub region: Option<String>,
    pub endpoint: Option<String>,
    pub prefix: String,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub database_url: String,
    pub blobs: BlobBackend,
    pub upload_root: PathBuf,
    pub web_root: Option<PathBuf>,
    pub jwt_secret: String,
    pub cors_allowed_origins: Vec<String>,
    pub default_quota_bytes: i64,
    pub versions_kept: i64,
    pub trash_retention_days: u32,
    pub session_ttl_seconds: i64,
    pub blob_sweep_interval_seconds: u64,
    pub blob_grace_period_seconds: u64,
    pub oidc: Option<OidcConfig>,
    pub metrics_token: Option<String>,
    pub bootstrap_admin: Option<BootstrapAdmin>,
    pub mail: Option<MailConfig>,
}

#[derive(Debug, Clone)]
pub struct MailConfig {
    pub smtp_url: String,
    pub from: String,
    pub public_url: String,
}

#[derive(Debug, Clone)]
pub struct OidcConfig {
    pub issuer: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
    /// Whether a verified address nobody has an account for becomes one. A deployment that invites
    /// people through its provider wants this; one with a fixed roster does not.
    pub create_accounts: bool,
}

#[derive(Debug, Clone)]
pub struct BootstrapAdmin {
    pub email: String,
    pub password: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{0} is not set")]
    Missing(&'static str),
    #[error("{name} is not valid: {reason}")]
    Invalid {
        name: &'static str,
        reason: &'static str,
    },
}

pub const MIN_JWT_SECRET_CHARS: usize = 32;

const DEFAULT_QUOTA_BYTES: i64 = 10 * 1024 * 1024 * 1024;
const VERSIONS_KEPT: i64 = 10;
const DEFAULT_SESSION_TTL_SECONDS: i64 = 12 * 60 * 60;
const DEFAULT_BLOB_SWEEP_INTERVAL_SECONDS: u64 = 60 * 60;
const DEFAULT_BLOB_GRACE_PERIOD_SECONDS: u64 = 24 * 60 * 60;

#[must_use]
pub fn jwt_secret_is_short(secret: &str) -> bool {
    secret.chars().count() < MIN_JWT_SECRET_CHARS
}

pub fn database_url() -> Result<String, ConfigError> {
    required(&process_env, "DATABASE_URL")
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(&process_env)
    }

    fn from_lookup(lookup: Lookup) -> Result<Self, ConfigError> {
        let backend = blobs(lookup)?;
        let blob_sweep_interval_seconds = parse_or(
            lookup,
            "BLOB_SWEEP_INTERVAL_SECONDS",
            DEFAULT_BLOB_SWEEP_INTERVAL_SECONDS,
        )?;
        Ok(Self {
            port: parse_or(lookup, "PORT", 3001)?,
            database_url: required(lookup, "DATABASE_URL")?,
            blobs: backend.clone(),
            upload_root: upload_root(lookup, &backend)?,
            web_root: optional(lookup, "WEB_ROOT").map(PathBuf::from),
            jwt_secret: required(lookup, "JWT_SECRET")?,
            cors_allowed_origins: optional(lookup, "CORS_ALLOWED_ORIGINS")
                .unwrap_or_default()
                .split(',')
                .map(str::trim)
                .filter(|origin| !origin.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
            default_quota_bytes: parse_or(lookup, "DEFAULT_QUOTA_BYTES", DEFAULT_QUOTA_BYTES)?,
            versions_kept: parse_or(lookup, "VERSIONS_KEPT", VERSIONS_KEPT)?,
            trash_retention_days: trash_retention(
                parse_or(lookup, "TRASH_RETENTION_DAYS", 0)?,
                blob_sweep_interval_seconds,
            )?,
            session_ttl_seconds: parse_or(
                lookup,
                "SESSION_TTL_SECONDS",
                DEFAULT_SESSION_TTL_SECONDS,
            )?,
            blob_sweep_interval_seconds,
            blob_grace_period_seconds: parse_or(
                lookup,
                "BLOB_GRACE_PERIOD_SECONDS",
                DEFAULT_BLOB_GRACE_PERIOD_SECONDS,
            )?,
            oidc: oidc(lookup),
            metrics_token: optional(lookup, "METRICS_TOKEN"),
            bootstrap_admin: bootstrap_admin(lookup),
            mail: mail(lookup)?,
        })
    }
}

/// Beside the blobs by default, because that is the one directory a deployment has already had to
/// make writable. An object store deployment has no such directory, so it has to say where.
fn upload_root(lookup: Lookup, blobs: &BlobBackend) -> Result<PathBuf, ConfigError> {
    if let Some(configured) = optional(lookup, "UPLOAD_ROOT") {
        return Ok(PathBuf::from(configured));
    }
    match blobs {
        BlobBackend::Local { root } => Ok(root.join("uploads")),
        // Falling back to a relative path would stage uploads on the container's ephemeral disk,
        // which is the one place this design says the bytes must not live, and the deployment
        // would find out when the disk filled rather than at startup.
        BlobBackend::S3(_) => Err(ConfigError::Missing("UPLOAD_ROOT")),
    }
}

fn trash_retention(days: u32, sweep_interval_seconds: u64) -> Result<u32, ConfigError> {
    if days > 0 && sweep_interval_seconds == 0 {
        return Err(ConfigError::Invalid {
            name: "TRASH_RETENTION_DAYS",
            reason: "needs BLOB_SWEEP_INTERVAL_SECONDS above 0, which is what purges it",
        });
    }
    Ok(days)
}

/// Local disk unless `BLOB_BACKEND=s3`. An unknown value is refused rather than quietly falling
/// back, because a deployment that meant S3 and got local disk loses every upload when the pod
/// restarts.
fn blobs(lookup: Lookup) -> Result<BlobBackend, ConfigError> {
    match optional(lookup, "BLOB_BACKEND").as_deref() {
        None | Some("local") => Ok(BlobBackend::Local {
            root: PathBuf::from(
                optional(lookup, "BLOB_ROOT").unwrap_or_else(|| "./data".to_owned()),
            ),
        }),
        Some("s3") => Ok(BlobBackend::S3(S3Config {
            bucket: required(lookup, "S3_BUCKET")?,
            region: optional(lookup, "S3_REGION"),
            endpoint: optional(lookup, "S3_ENDPOINT"),
            prefix: optional(lookup, "S3_PREFIX").unwrap_or_default(),
            access_key_id: optional(lookup, "S3_ACCESS_KEY_ID"),
            secret_access_key: optional(lookup, "S3_SECRET_ACCESS_KEY"),
        })),
        Some(_) => Err(ConfigError::Invalid {
            name: "BLOB_BACKEND",
            reason: "expected local or s3",
        }),
    }
}

fn oidc(lookup: Lookup) -> Option<OidcConfig> {
    Some(OidcConfig {
        issuer: optional(lookup, "OIDC_ISSUER")?,
        client_id: optional(lookup, "OIDC_CLIENT_ID")?,
        client_secret: optional(lookup, "OIDC_CLIENT_SECRET")?,
        redirect_url: optional(lookup, "OIDC_REDIRECT_URL")?,
        create_accounts: optional(lookup, "OIDC_CREATE_ACCOUNTS")
            .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes")),
    })
}

fn mail(lookup: Lookup) -> Result<Option<MailConfig>, ConfigError> {
    let Some(smtp_url) = optional(lookup, "SMTP_URL") else {
        return Ok(None);
    };
    Ok(Some(MailConfig {
        smtp_url,
        from: required(lookup, "MAIL_FROM")?,
        public_url: required(lookup, "PUBLIC_URL")?,
    }))
}

fn bootstrap_admin(lookup: Lookup) -> Option<BootstrapAdmin> {
    Some(BootstrapAdmin {
        email: optional(lookup, "BOOTSTRAP_ADMIN_EMAIL")?,
        password: optional(lookup, "BOOTSTRAP_ADMIN_PASSWORD")?,
    })
}

type Lookup<'a> = &'a dyn Fn(&str) -> Result<String, VarError>;

fn process_env(name: &str) -> Result<String, VarError> {
    env::var(name)
}

fn required(lookup: Lookup, name: &'static str) -> Result<String, ConfigError> {
    match lookup(name) {
        Ok(value) if !value.is_empty() => Ok(value),
        Ok(_) | Err(VarError::NotPresent) => Err(ConfigError::Missing(name)),
        Err(VarError::NotUnicode(_)) => Err(ConfigError::Invalid {
            name,
            reason: "not valid unicode",
        }),
    }
}

fn optional(lookup: Lookup, name: &str) -> Option<String> {
    lookup(name).ok().filter(|value| !value.is_empty())
}

fn parse_or<T: std::str::FromStr>(
    lookup: Lookup,
    name: &'static str,
    fallback: T,
) -> Result<T, ConfigError> {
    match optional(lookup, name) {
        None => Ok(fallback),
        Some(raw) => raw.parse().map_err(|_| ConfigError::Invalid {
            name,
            reason: "not a number",
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::ffi::OsString;

    use super::*;

    fn config_from(pairs: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let vars: HashMap<String, String> = pairs
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect();
        Config::from_lookup(&|name| vars.get(name).cloned().ok_or(VarError::NotPresent))
    }

    const MINIMAL: [(&str, &str); 2] = [("DATABASE_URL", "postgres://db"), ("JWT_SECRET", "s")];

    fn with(extra: &[(&'static str, &'static str)]) -> Result<Config, ConfigError> {
        let mut pairs = MINIMAL.to_vec();
        pairs.extend_from_slice(extra);
        config_from(&pairs)
    }

    #[test]
    fn a_minimal_environment_gets_the_documented_defaults() {
        let config = with(&[]).unwrap();
        assert_eq!(config.port, 3001);
        assert_eq!(config.default_quota_bytes, DEFAULT_QUOTA_BYTES);
        assert_eq!(config.versions_kept, VERSIONS_KEPT);
        assert_eq!(config.session_ttl_seconds, DEFAULT_SESSION_TTL_SECONDS);
        assert_eq!(config.trash_retention_days, 0);
        assert_eq!(config.cors_allowed_origins, Vec::<String>::new());
        assert!(config.oidc.is_none());
        assert!(config.mail.is_none());
        assert!(config.bootstrap_admin.is_none());
        assert!(config.metrics_token.is_none());
        assert!(config.web_root.is_none());
        assert_eq!(config.upload_root, PathBuf::from("./data/uploads"));
        assert!(matches!(config.blobs, BlobBackend::Local { .. }));
    }

    #[test]
    fn a_missing_database_url_or_secret_is_named() {
        assert!(matches!(
            config_from(&[("JWT_SECRET", "s")]),
            Err(ConfigError::Missing("DATABASE_URL"))
        ));
        assert!(matches!(
            config_from(&[("DATABASE_URL", "x")]),
            Err(ConfigError::Missing("JWT_SECRET"))
        ));
    }

    #[test]
    fn an_empty_value_counts_as_missing() {
        assert!(matches!(
            config_from(&[("DATABASE_URL", ""), ("JWT_SECRET", "s")]),
            Err(ConfigError::Missing("DATABASE_URL"))
        ));
    }

    #[test]
    fn a_value_that_is_not_unicode_is_invalid_rather_than_missing() {
        let result = required(
            &|_| Err(VarError::NotUnicode(OsString::from("x"))),
            "DATABASE_URL",
        );
        assert!(matches!(
            result,
            Err(ConfigError::Invalid {
                name: "DATABASE_URL",
                reason: "not valid unicode"
            })
        ));
    }

    #[test]
    fn a_number_that_does_not_parse_is_refused() {
        assert!(matches!(
            with(&[("PORT", "http")]),
            Err(ConfigError::Invalid { name: "PORT", .. })
        ));
        assert_eq!(with(&[("PORT", "8080")]).unwrap().port, 8080);
    }

    #[test]
    fn cors_origins_are_split_trimmed_and_stripped_of_blanks() {
        let config = with(&[(
            "CORS_ALLOWED_ORIGINS",
            " https://a.example , ,https://b.example,",
        )])
        .unwrap();
        assert_eq!(
            config.cors_allowed_origins,
            vec!["https://a.example", "https://b.example"]
        );
    }

    #[test]
    fn an_unknown_blob_backend_is_refused_instead_of_falling_back_to_disk() {
        assert!(matches!(
            with(&[("BLOB_BACKEND", "gcs")]),
            Err(ConfigError::Invalid {
                name: "BLOB_BACKEND",
                ..
            })
        ));
    }

    #[test]
    fn the_local_backend_honours_its_root_and_stages_uploads_beside_it() {
        let config = with(&[("BLOB_BACKEND", "local"), ("BLOB_ROOT", "/srv/blobs")]).unwrap();
        assert_eq!(config.upload_root, PathBuf::from("/srv/blobs/uploads"));
        let config = with(&[("UPLOAD_ROOT", "/var/staging")]).unwrap();
        assert_eq!(config.upload_root, PathBuf::from("/var/staging"));
    }

    #[test]
    fn the_s3_backend_needs_a_bucket_and_an_explicit_upload_root() {
        assert!(matches!(
            with(&[("BLOB_BACKEND", "s3")]),
            Err(ConfigError::Missing("S3_BUCKET"))
        ));
        assert!(matches!(
            with(&[("BLOB_BACKEND", "s3"), ("S3_BUCKET", "b")]),
            Err(ConfigError::Missing("UPLOAD_ROOT"))
        ));
        let config = with(&[
            ("BLOB_BACKEND", "s3"),
            ("S3_BUCKET", "b"),
            ("S3_PREFIX", "p/"),
            ("S3_REGION", "eu-west-3"),
            ("UPLOAD_ROOT", "/var/staging"),
        ])
        .unwrap();
        match config.blobs {
            BlobBackend::S3(s3) => {
                assert_eq!(s3.bucket, "b");
                assert_eq!(s3.prefix, "p/");
                assert_eq!(s3.region.as_deref(), Some("eu-west-3"));
                assert!(s3.endpoint.is_none());
            }
            BlobBackend::Local { .. } => panic!("expected the s3 backend"),
        }
    }

    #[test]
    fn a_retention_without_a_sweeper_fails_the_whole_config() {
        assert!(matches!(
            with(&[
                ("TRASH_RETENTION_DAYS", "30"),
                ("BLOB_SWEEP_INTERVAL_SECONDS", "0")
            ]),
            Err(ConfigError::Invalid {
                name: "TRASH_RETENTION_DAYS",
                ..
            })
        ));
    }

    #[test]
    fn oidc_needs_all_four_settings_and_creates_accounts_only_when_asked() {
        let full = [
            ("OIDC_ISSUER", "https://idp"),
            ("OIDC_CLIENT_ID", "id"),
            ("OIDC_CLIENT_SECRET", "secret"),
            ("OIDC_REDIRECT_URL", "https://app/cb"),
        ];
        assert!(with(&full[..3]).unwrap().oidc.is_none());
        let oidc = with(&full).unwrap().oidc.unwrap();
        assert!(!oidc.create_accounts);
        let mut asked = full.to_vec();
        asked.push(("OIDC_CREATE_ACCOUNTS", "yes"));
        assert!(with(&asked).unwrap().oidc.unwrap().create_accounts);
        let mut refused = full.to_vec();
        refused.push(("OIDC_CREATE_ACCOUNTS", "no"));
        assert!(!with(&refused).unwrap().oidc.unwrap().create_accounts);
    }

    #[test]
    fn mail_needs_a_sender_and_a_public_url_once_smtp_is_set() {
        assert!(matches!(
            with(&[("SMTP_URL", "smtp://relay")]),
            Err(ConfigError::Missing("MAIL_FROM"))
        ));
        assert!(matches!(
            with(&[("SMTP_URL", "smtp://relay"), ("MAIL_FROM", "a@b.c")]),
            Err(ConfigError::Missing("PUBLIC_URL"))
        ));
        let mail = with(&[
            ("SMTP_URL", "smtp://relay"),
            ("MAIL_FROM", "a@b.c"),
            ("PUBLIC_URL", "https://files"),
        ])
        .unwrap()
        .mail
        .unwrap();
        assert_eq!(mail.from, "a@b.c");
    }

    #[test]
    fn the_bootstrap_admin_needs_both_halves() {
        assert!(
            with(&[("BOOTSTRAP_ADMIN_EMAIL", "a@b.c")])
                .unwrap()
                .bootstrap_admin
                .is_none()
        );
        let admin = with(&[
            ("BOOTSTRAP_ADMIN_EMAIL", "a@b.c"),
            ("BOOTSTRAP_ADMIN_PASSWORD", "pw"),
        ])
        .unwrap()
        .bootstrap_admin
        .unwrap();
        assert_eq!(admin.email, "a@b.c");
    }

    #[test]
    fn a_retention_nothing_would_honour_is_refused() {
        assert!(matches!(
            trash_retention(30, 0),
            Err(ConfigError::Invalid {
                name: "TRASH_RETENTION_DAYS",
                ..
            })
        ));
    }

    #[test]
    fn a_secret_shorter_than_the_minimum_is_flagged() {
        assert!(jwt_secret_is_short("dev-secret"));
        assert!(jwt_secret_is_short(&"x".repeat(MIN_JWT_SECRET_CHARS - 1)));
    }

    #[test]
    fn a_generated_secret_is_not() {
        assert!(!jwt_secret_is_short(&"x".repeat(MIN_JWT_SECRET_CHARS)));
        assert!(!jwt_secret_is_short(
            "9f2c7a4be01d38c65a7b94e0f1d2a3c4b5e6f708192a3b4c5d6e7f8091a2b3c4"
        ));
    }

    #[test]
    fn a_retention_with_a_sweep_or_no_retention_at_all_is_accepted() {
        assert!(matches!(trash_retention(30, 3600), Ok(30)));
        assert!(matches!(trash_retention(0, 0), Ok(0)));
    }
}
