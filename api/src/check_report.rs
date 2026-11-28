use crate::check::{Findings, Options};

pub fn options_from(flags: &[String]) -> anyhow::Result<Options> {
    let mut options = Options::default();
    for flag in flags {
        match flag.as_str() {
            "--verify-content" => options.verify_content = true,
            "--repair" => options.repair = true,
            other => anyhow::bail!(
                "unknown option {other}; the ones there are: --verify-content, --repair"
            ),
        }
    }
    Ok(options)
}

#[must_use]
pub fn report(findings: &Findings, options: Options) -> Vec<String> {
    let mut lines = Vec::new();
    for hash in &findings.missing {
        lines.push(format!(
            "blob {}: referenced, but not in the store",
            hash.to_hex()
        ));
    }
    for hash in &findings.corrupt {
        lines.push(format!(
            "blob {}: its bytes no longer match its hash",
            hash.to_hex()
        ));
    }
    for wrong in &findings.wrong_counts {
        lines.push(format!(
            "blob {}: reference count {}, should be {}",
            wrong.hash.to_hex(),
            wrong.stored,
            wrong.expected
        ));
    }
    for wrong in &findings.wrong_usage {
        lines.push(format!(
            "account {}: usage {} bytes, should be {}",
            wrong.owner_id, wrong.stored, wrong.expected
        ));
    }

    let content = if options.verify_content {
        "bytes and hashes"
    } else {
        "presence"
    };
    lines.push(format!(
        "checked {} blobs for {content}",
        findings.checked_blobs
    ));
    if findings.repaired {
        lines.push("reference counts and usage corrected".to_owned());
    }
    lines
}

pub fn verdict(findings: &Findings) -> anyhow::Result<()> {
    if findings.needs_a_person() {
        anyhow::bail!(
            "{} blobs are missing and {} corrupt: restore them from a backup",
            findings.missing.len(),
            findings.corrupt.len()
        );
    }
    if !findings.is_clean() {
        anyhow::bail!("counts or usage are off: run it again with --repair");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{WrongCount, WrongUsage};
    use stashden_core::blob::BlobHash;
    use uuid::Uuid;

    fn hash(byte: u8) -> BlobHash {
        BlobHash::from_bytes([byte; 32])
    }

    fn flags(given: &[&str]) -> Vec<String> {
        given.iter().map(|flag| (*flag).to_owned()).collect()
    }

    #[test]
    fn no_flags_means_a_presence_check_without_repair() {
        let options = options_from(&[]).unwrap();
        assert!(!options.verify_content);
        assert!(!options.repair);
    }

    #[test]
    fn each_flag_turns_on_only_its_own_option() {
        let verify = options_from(&flags(&["--verify-content"])).unwrap();
        assert!(verify.verify_content && !verify.repair);
        let repair = options_from(&flags(&["--repair"])).unwrap();
        assert!(repair.repair && !repair.verify_content);
        let both = options_from(&flags(&["--repair", "--verify-content"])).unwrap();
        assert!(both.repair && both.verify_content);
    }

    #[test]
    fn an_unknown_flag_is_refused_by_name() {
        let error = options_from(&flags(&["--repair", "--fix"])).unwrap_err();
        assert!(error.to_string().contains("--fix"));
    }

    #[test]
    fn a_clean_instance_reports_only_what_it_checked() {
        let findings = Findings {
            checked_blobs: 7,
            ..Findings::default()
        };
        assert_eq!(
            report(&findings, Options::default()),
            vec!["checked 7 blobs for presence"]
        );
        let verified = Options {
            verify_content: true,
            repair: false,
        };
        assert_eq!(
            report(&findings, verified),
            vec!["checked 7 blobs for bytes and hashes"]
        );
    }

    #[test]
    fn every_kind_of_finding_gets_its_own_line_before_the_summary() {
        let owner = Uuid::nil();
        let findings = Findings {
            missing: vec![hash(1)],
            corrupt: vec![hash(2)],
            wrong_counts: vec![WrongCount {
                hash: hash(3),
                stored: 5,
                expected: 4,
            }],
            wrong_usage: vec![WrongUsage {
                owner_id: owner,
                stored: 10,
                expected: 20,
            }],
            checked_blobs: 3,
            repaired: true,
        };
        let lines = report(&findings, Options::default());
        assert_eq!(lines.len(), 6);
        assert_eq!(
            lines[0],
            format!(
                "blob {}: referenced, but not in the store",
                hash(1).to_hex()
            )
        );
        assert_eq!(
            lines[1],
            format!(
                "blob {}: its bytes no longer match its hash",
                hash(2).to_hex()
            )
        );
        assert_eq!(
            lines[2],
            format!("blob {}: reference count 5, should be 4", hash(3).to_hex())
        );
        assert_eq!(
            lines[3],
            format!("account {owner}: usage 10 bytes, should be 20")
        );
        assert_eq!(lines[4], "checked 3 blobs for presence");
        assert_eq!(lines[5], "reference counts and usage corrected");
    }

    #[test]
    fn a_clean_instance_passes_the_verdict() {
        assert!(verdict(&Findings::default()).is_ok());
    }

    #[test]
    fn missing_or_corrupt_blobs_need_a_backup_even_when_counts_are_off_too() {
        let findings = Findings {
            missing: vec![hash(1), hash(2)],
            corrupt: vec![hash(3)],
            wrong_usage: vec![WrongUsage {
                owner_id: Uuid::nil(),
                stored: 1,
                expected: 2,
            }],
            ..Findings::default()
        };
        let message = verdict(&findings).unwrap_err().to_string();
        assert!(message.contains("2 blobs are missing and 1 corrupt"));
        assert!(message.contains("backup"));
    }

    #[test]
    fn wrong_counts_alone_ask_for_a_repair_run() {
        let findings = Findings {
            wrong_counts: vec![WrongCount {
                hash: hash(1),
                stored: 2,
                expected: 1,
            }],
            ..Findings::default()
        };
        assert!(
            verdict(&findings)
                .unwrap_err()
                .to_string()
                .contains("--repair")
        );
    }

    #[test]
    fn wrong_usage_alone_asks_for_a_repair_run() {
        let findings = Findings {
            wrong_usage: vec![WrongUsage {
                owner_id: Uuid::nil(),
                stored: 2,
                expected: 1,
            }],
            ..Findings::default()
        };
        assert!(
            verdict(&findings)
                .unwrap_err()
                .to_string()
                .contains("--repair")
        );
    }
}
