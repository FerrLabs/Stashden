use stashden_core::node::{Node, NodeKind, Trashed};

pub fn listing_line(node: &Node) -> String {
    let marker = match node.kind {
        NodeKind::Directory => "/",
        NodeKind::File => "",
    };
    format!("{:>12}  {}{marker}", node.size, node.name)
}

pub fn trash_line(trashed: &Trashed) -> String {
    let node = &trashed.node;
    let deleted = node
        .deleted_at
        .map(|at| at.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default();
    format!("{}  {deleted}  {}", node.id, node.name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    fn node(name: &str, kind: NodeKind, size: i64) -> Node {
        let moment = Utc.with_ymd_and_hms(2026, 3, 4, 5, 6, 7).unwrap();
        Node {
            id: Uuid::nil(),
            owner_id: Uuid::nil(),
            parent_id: None,
            name: name.to_owned(),
            kind,
            blob_hash: None,
            size,
            etag: String::new(),
            created_at: moment,
            updated_at: moment,
            deleted_at: None,
        }
    }

    #[test]
    fn a_file_is_listed_by_size_and_name() {
        assert_eq!(
            listing_line(&node("a.txt", NodeKind::File, 42)),
            "          42  a.txt"
        );
    }

    #[test]
    fn a_directory_gets_a_trailing_slash() {
        assert_eq!(
            listing_line(&node("docs", NodeKind::Directory, 0)),
            "           0  docs/"
        );
    }

    #[test]
    fn a_large_size_still_keeps_the_name_column_aligned_after_two_spaces() {
        let line = listing_line(&node("big", NodeKind::File, 123_456_789_012));
        assert_eq!(line, "123456789012  big");
    }

    #[test]
    fn a_trashed_node_shows_its_id_deletion_minute_and_name() {
        let mut gone = node("old.txt", NodeKind::File, 1);
        gone.deleted_at = Some(Utc.with_ymd_and_hms(2026, 5, 6, 7, 8, 59).unwrap());
        let line = trash_line(&Trashed {
            node: gone,
            expires_at: None,
        });
        assert_eq!(line, format!("{}  2026-05-06 07:08  old.txt", Uuid::nil()));
    }

    #[test]
    fn a_trashed_node_without_a_deletion_time_leaves_the_column_empty() {
        let line = trash_line(&Trashed {
            node: node("old.txt", NodeKind::File, 1),
            expires_at: None,
        });
        assert_eq!(line, format!("{}    old.txt", Uuid::nil()));
    }
}
