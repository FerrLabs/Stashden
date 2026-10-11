use axum::http::{HeaderMap, header};

use crate::precondition::text;

#[derive(Debug, PartialEq, Eq)]
pub enum Delivery {
    Whole,
    Partial { start: u64, end: u64 },
    NotModified,
    Unsatisfiable,
}

#[must_use]
pub fn plan(headers: &HeaderMap, etag: &str, size: u64) -> Delivery {
    if unchanged(headers, etag) {
        return Delivery::NotModified;
    }
    if size == 0 {
        return Delivery::Whole;
    }
    let Some(range) = text(headers, header::RANGE) else {
        return Delivery::Whole;
    };
    if text(headers, header::IF_RANGE).is_some_and(|guard| guard != etag || guard.starts_with("W/"))
    {
        return Delivery::Whole;
    }
    parse(range, size)
}

fn unchanged(headers: &HeaderMap, etag: &str) -> bool {
    let Some(offered) = text(headers, header::IF_NONE_MATCH) else {
        return false;
    };
    let ours = etag.strip_prefix("W/").unwrap_or(etag);
    offered.split(',').map(str::trim).any(|candidate| {
        candidate == "*" || candidate.strip_prefix("W/").unwrap_or(candidate) == ours
    })
}

fn parse(range: &str, size: u64) -> Delivery {
    let Some(spec) = range.strip_prefix("bytes=") else {
        return Delivery::Whole;
    };
    if spec.contains(',') {
        return Delivery::Whole;
    }
    let Some((first, last)) = spec.split_once('-') else {
        return Delivery::Whole;
    };
    let last_byte = size - 1;

    match (first.trim(), last.trim()) {
        ("", suffix) => match suffix.parse::<u64>() {
            Ok(0) => Delivery::Unsatisfiable,
            Ok(length) => Delivery::Partial {
                start: size.saturating_sub(length),
                end: last_byte,
            },
            Err(_) => Delivery::Whole,
        },
        (first, "") => match first.parse::<u64>() {
            Ok(start) if start > last_byte => Delivery::Unsatisfiable,
            Ok(start) => Delivery::Partial {
                start,
                end: last_byte,
            },
            Err(_) => Delivery::Whole,
        },
        (first, last) => match (first.parse::<u64>(), last.parse::<u64>()) {
            (Ok(start), Ok(end)) if end >= start => {
                if start > last_byte {
                    Delivery::Unsatisfiable
                } else {
                    Delivery::Partial {
                        start,
                        end: end.min(last_byte),
                    }
                }
            }
            _ => Delivery::Whole,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    const TAG: &str = "\"abc\"";

    fn plan_for(pairs: &[(header::HeaderName, &str)], size: u64) -> Delivery {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.insert(name.clone(), HeaderValue::from_str(value).expect("ascii"));
        }
        plan(&headers, TAG, size)
    }

    fn ranged(value: &str, size: u64) -> Delivery {
        plan_for(&[(header::RANGE, value)], size)
    }

    #[test]
    fn no_headers_means_the_whole_file() {
        assert_eq!(plan_for(&[], 100), Delivery::Whole);
    }

    #[test]
    fn a_closed_range_is_inclusive() {
        assert_eq!(
            ranged("bytes=10-19", 100),
            Delivery::Partial { start: 10, end: 19 }
        );
    }

    #[test]
    fn an_open_range_runs_to_the_last_byte() {
        assert_eq!(
            ranged("bytes=90-", 100),
            Delivery::Partial { start: 90, end: 99 }
        );
    }

    #[test]
    fn a_suffix_range_is_the_tail() {
        assert_eq!(
            ranged("bytes=-10", 100),
            Delivery::Partial { start: 90, end: 99 }
        );
        assert_eq!(
            ranged("bytes=-500", 100),
            Delivery::Partial { start: 0, end: 99 }
        );
    }

    #[test]
    fn an_end_past_the_file_is_clamped() {
        assert_eq!(
            ranged("bytes=50-5000", 100),
            Delivery::Partial { start: 50, end: 99 }
        );
    }

    #[test]
    fn a_start_past_the_file_is_unsatisfiable() {
        assert_eq!(ranged("bytes=100-", 100), Delivery::Unsatisfiable);
        assert_eq!(ranged("bytes=150-200", 100), Delivery::Unsatisfiable);
        assert_eq!(ranged("bytes=-0", 100), Delivery::Unsatisfiable);
    }

    #[test]
    fn what_cannot_be_read_as_one_range_is_served_whole() {
        assert_eq!(ranged("bytes=0-1,5-6", 100), Delivery::Whole);
        assert_eq!(ranged("bytes=20-10", 100), Delivery::Whole);
        assert_eq!(ranged("items=0-10", 100), Delivery::Whole);
        assert_eq!(ranged("bytes=a-b", 100), Delivery::Whole);
    }

    #[test]
    fn an_empty_file_is_always_whole() {
        assert_eq!(ranged("bytes=0-10", 0), Delivery::Whole);
    }

    #[test]
    fn a_matching_if_none_match_is_not_modified() {
        assert_eq!(
            plan_for(&[(header::IF_NONE_MATCH, "\"abc\"")], 100),
            Delivery::NotModified
        );
        assert_eq!(
            plan_for(&[(header::IF_NONE_MATCH, "\"x\", W/\"abc\"")], 100),
            Delivery::NotModified
        );
        assert_eq!(
            plan_for(&[(header::IF_NONE_MATCH, "*")], 100),
            Delivery::NotModified
        );
    }

    #[test]
    fn another_tag_is_served() {
        assert_eq!(
            plan_for(&[(header::IF_NONE_MATCH, "\"other\"")], 100),
            Delivery::Whole
        );
    }

    #[test]
    fn if_range_never_accepts_a_weak_tag() {
        let headers = {
            let mut map = HeaderMap::new();
            map.insert(header::RANGE, HeaderValue::from_static("bytes=0-9"));
            map.insert(header::IF_RANGE, HeaderValue::from_static("W/\"abc\""));
            map
        };
        assert_eq!(plan(&headers, "W/\"abc\"", 100), Delivery::Whole);
    }

    #[test]
    fn if_range_with_a_stale_tag_drops_the_range() {
        assert_eq!(
            plan_for(
                &[(header::RANGE, "bytes=0-9"), (header::IF_RANGE, "\"old\"")],
                100
            ),
            Delivery::Whole
        );
        assert_eq!(
            plan_for(
                &[(header::RANGE, "bytes=0-9"), (header::IF_RANGE, TAG)],
                100
            ),
            Delivery::Partial { start: 0, end: 9 }
        );
    }
}
