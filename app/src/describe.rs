use chrono::{DateTime, Local, NaiveDate, TimeZone, Utc};
use stashden_client::{Report, Status};
use stashden_core::grant::{Access, Received};
use stashden_core::role::Role;
use stashden_core::share::Share;

const UNITS: [&str; 5] = ["B", "kB", "MB", "GB", "TB"];

#[allow(clippy::cast_precision_loss)]
pub fn size(bytes: i64) -> String {
    let mut value = bytes.max(0) as f64;
    let mut unit = 0;
    while value.round() >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 || (value * 10.0).round() >= 100.0 {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

pub fn day(at: DateTime<Utc>) -> String {
    at.with_timezone(&Local).format("%-d %b %Y").to_string()
}

pub fn moment(at: DateTime<Utc>) -> String {
    at.with_timezone(&Local)
        .format("%-d %b %Y, %H:%M")
        .to_string()
}

pub fn role(role: Role) -> &'static str {
    match role {
        Role::Admin => "Administrator",
        Role::Member => "Member",
        Role::Reader => "Read only",
    }
}

pub fn access(access: Access) -> &'static str {
    match access {
        Access::Write => "can edit",
        Access::Read => "view only",
    }
}

pub fn origin(received: &Received) -> String {
    format!("From {}, {}", owner(received), access(received.access))
}

pub fn owner(received: &Received) -> &str {
    if received.owner_name.is_empty() {
        &received.owner_email
    } else {
        &received.owner_name
    }
}

pub fn link(share: &Share, now: DateTime<Utc>) -> String {
    let expiry = match share.expires_at {
        None => "No expiry".to_owned(),
        Some(at) if at <= now => format!("Expired {}", day(at)),
        Some(at) => format!("Expires {}", day(at)),
    };
    let used = share.last_used_at.map_or_else(
        || "Never opened".to_owned(),
        |at| format!("Last opened {}", day(at)),
    );
    let mut parts = vec![expiry, used];
    if share.has_password {
        parts.push("Password protected".to_owned());
    }
    parts.join(", ")
}

pub fn share_url(server: &str, token: &str) -> String {
    format!("{}/#/s/{token}", server.trim_end_matches('/'))
}

pub fn end_of_day(typed: &str) -> Result<Option<DateTime<Utc>>, String> {
    let typed = typed.trim();
    if typed.is_empty() {
        return Ok(None);
    }
    let day = NaiveDate::parse_from_str(typed, "%Y-%m-%d")
        .map_err(|_| format!("{typed} is not a date, write it as 2026-12-31"))?;
    let last_second = day.and_hms_opt(23, 59, 59).expect("23:59:59 exists");
    Local
        .from_local_datetime(&last_second)
        .latest()
        .map(|at| Some(at.with_timezone(&Utc)))
        .ok_or_else(|| format!("{typed} has no end of day in this time zone"))
}

pub fn server_address(typed: &str) -> Option<String> {
    let typed = typed.trim();
    let (scheme, rest) = ["http", "https"]
        .into_iter()
        .find_map(|scheme| {
            typed
                .strip_prefix(scheme)
                .and_then(|rest| rest.strip_prefix("://"))
                .map(|rest| (scheme, rest))
        })
        .unwrap_or(("https", typed));
    let rest = rest.trim_end_matches('/');
    let host = rest.split(['/', ':']).next().unwrap_or_default();
    (!host.is_empty()).then(|| format!("{scheme}://{rest}"))
}

pub fn status(status: &Status) -> String {
    match status {
        Status::Idle => "Watching for changes".to_owned(),
        Status::Syncing => "Syncing".to_owned(),
        Status::Synced(_) => "In step with the server".to_owned(),
        Status::Failed { reason } => format!("The last pass failed: {reason}"),
        Status::Paused => "Paused, changes wait until you resume".to_owned(),
        Status::Stopped => "Stopped".to_owned(),
    }
}

pub fn summary(report: &Report) -> String {
    format!(
        "{} up, {} down, {} deleted here, {} deleted on the server",
        report.uploaded, report.downloaded, report.deleted_locally, report.deleted_remotely
    )
}

pub fn attention(report: &Report) -> Vec<(&'static str, String)> {
    let mut lines = Vec::new();
    lines.extend(
        report
            .conflicts
            .iter()
            .map(|path| ("conflict", format!("{path}, both copies kept"))),
    );
    lines.extend(report.blocked.iter().map(|path| {
        (
            "blocked",
            format!("{path} is a file on one side and a directory on the other"),
        )
    }));
    lines.extend(
        report
            .held
            .iter()
            .map(|path| ("held", format!("{path} was not changed on the server"))),
    );
    lines.extend(report.unsupported.iter().map(|unsupported| {
        (
            "not here",
            format!("{} ({})", unsupported.path, unsupported.reason),
        )
    }));
    lines.extend(report.skipped.iter().map(|path| ("skipped", path.clone())));
    lines.extend(
        report
            .failures
            .iter()
            .map(|failed| ("failed", format!("{} ({})", failed.path, failed.reason))),
    );
    lines
}

#[cfg(test)]
mod tests {
    use chrono::Duration;
    use uuid::Uuid;

    use super::*;

    fn share(expires_at: Option<DateTime<Utc>>, has_password: bool) -> Share {
        Share {
            id: Uuid::now_v7(),
            node_id: Uuid::now_v7(),
            name: "a.txt".to_owned(),
            has_password,
            expires_at,
            created_at: Utc::now(),
            last_used_at: None,
        }
    }

    #[test]
    fn sizes_round_to_one_decimal_only_below_ten_units() {
        assert_eq!(size(0), "0 B");
        assert_eq!(size(999), "999 B");
        assert_eq!(size(1_000), "1.0 kB");
        assert_eq!(size(1_500), "1.5 kB");
        assert_eq!(size(12_345), "12 kB");
        assert_eq!(size(3_200_000_000), "3.2 GB");
        assert_eq!(size(5_000_000_000_000_000), "5000 TB");
        assert_eq!(size(-4), "0 B");
    }

    #[test]
    fn a_size_that_rounds_up_to_a_thousand_moves_to_the_next_unit() {
        assert_eq!(size(999_499), "999 kB");
        assert_eq!(size(999_500), "1.0 MB");
        assert_eq!(size(999_999), "1.0 MB");
        assert_eq!(size(9_949), "9.9 kB");
        assert_eq!(size(9_950), "10 kB");
        assert_eq!(size(999_999_999), "1.0 GB");
    }

    #[test]
    fn a_link_reads_as_expired_once_its_date_passed() {
        let now = Utc::now();
        assert!(link(&share(None, false), now).starts_with("No expiry, Never opened"));
        assert!(link(&share(Some(now - Duration::days(1)), false), now).starts_with("Expired "));
        assert!(link(&share(Some(now + Duration::days(1)), false), now).starts_with("Expires "));
        assert!(link(&share(None, true), now).ends_with(", Password protected"));
    }

    #[test]
    fn a_share_url_points_at_the_web_app_route_on_the_server() {
        assert_eq!(
            share_url("https://files.example/", "abc"),
            "https://files.example/#/s/abc"
        );
    }

    #[test]
    fn an_expiry_is_the_last_second_of_the_typed_day_or_nothing() {
        assert_eq!(end_of_day("  "), Ok(None));
        let at = end_of_day("2030-06-15").unwrap().unwrap();
        let local = at.with_timezone(&Local);
        assert_eq!(
            local.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2030-06-15 23:59:59"
        );
        assert!(end_of_day("15/06/2030").is_err());
        assert!(end_of_day("2030-02-30").is_err());
    }

    #[test]
    fn a_bare_host_is_reached_over_https_and_a_trailing_slash_is_dropped() {
        assert_eq!(
            server_address(" files.example/ ").as_deref(),
            Some("https://files.example")
        );
        assert_eq!(
            server_address("http://localhost:3001").as_deref(),
            Some("http://localhost:3001")
        );
        assert_eq!(
            server_address("https://a.example//").as_deref(),
            Some("https://a.example")
        );
    }

    #[test]
    fn an_address_without_a_host_is_refused() {
        for typed in [
            "",
            "  ",
            "/",
            "https://",
            "http:///",
            "https:///path",
            ":3001",
        ] {
            assert_eq!(server_address(typed), None, "{typed:?}");
        }
    }

    #[test]
    fn an_owner_without_a_display_name_is_named_by_address() {
        let mut received = Received {
            id: Uuid::now_v7(),
            name: "Plans".to_owned(),
            kind: stashden_core::node::NodeKind::Directory,
            access: Access::Write,
            owner_email: "o@x".to_owned(),
            owner_name: String::new(),
            created_at: Utc::now(),
        };
        assert_eq!(origin(&received), "From o@x, can edit");
        received.owner_name = "Olga".to_owned();
        received.access = Access::Read;
        assert_eq!(origin(&received), "From Olga, view only");
    }

    #[test]
    fn a_clean_pass_needs_no_attention_and_each_problem_is_listed_with_its_tag() {
        let mut report = Report::default();
        assert_eq!(attention(&report), Vec::<(&str, String)>::new());
        assert_eq!(
            summary(&report),
            "0 up, 0 down, 0 deleted here, 0 deleted on the server"
        );

        report.skipped.push("huge.iso".to_owned());
        let tags: Vec<_> = attention(&report).into_iter().map(|(tag, _)| tag).collect();
        assert_eq!(tags, ["skipped"]);
    }

    #[test]
    fn a_failed_pass_says_why() {
        let failed = Status::Failed {
            reason: "offline".to_owned(),
        };
        assert_eq!(status(&failed), "The last pass failed: offline");
        assert_eq!(
            status(&Status::Paused),
            "Paused, changes wait until you resume"
        );
    }
}
