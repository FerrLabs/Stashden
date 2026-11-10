use stashden_core::grant::Received;
use stashden_core::node::{Node, NodeKind};

pub const SHARED_WITH_ME: &str = "Shared with me";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Folder(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Where<'a> {
    Own,
    Shelf,
    Shared { mount: &'a str },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crumb {
    pub name: String,
    pub folder: Folder,
}

impl Folder {
    pub fn root() -> Self {
        Self::default()
    }

    pub fn new(path: &str) -> Self {
        Self(
            path.split('/')
                .filter(|segment| !segment.is_empty())
                .collect::<Vec<_>>()
                .join("/"),
        )
    }

    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn remote(&self) -> String {
        format!("/{}", self.0)
    }

    pub fn child(&self, name: &str) -> String {
        if self.0.is_empty() {
            Self::new(name).0
        } else {
            Self::new(&format!("{}/{name}", self.0)).0
        }
    }

    pub fn enter(&self, name: &str) -> Self {
        Self(self.child(name))
    }

    pub fn parent(&self) -> Option<Self> {
        if self.0.is_empty() {
            return None;
        }
        Some(
            self.0
                .rsplit_once('/')
                .map_or_else(Self::root, |(parent, _)| Self(parent.to_owned())),
        )
    }

    pub fn crumbs(&self) -> Vec<Crumb> {
        let mut folder = Self::root();
        self.0
            .split('/')
            .filter(|segment| !segment.is_empty())
            .map(|name| {
                folder = folder.enter(name);
                Crumb {
                    name: name.to_owned(),
                    folder: folder.clone(),
                }
            })
            .collect()
    }

    pub fn place(&self) -> Where<'_> {
        let mut segments = self.0.split('/');
        if segments.next() != Some(SHARED_WITH_ME) {
            return Where::Own;
        }
        segments
            .next()
            .map_or(Where::Shelf, |mount| Where::Shared { mount })
    }

    pub fn mount<'a>(&self, received: &'a [Received]) -> Option<&'a Received> {
        match self.place() {
            Where::Shared { mount } => received.iter().find(|grant| grant.name == mount),
            Where::Own | Where::Shelf => None,
        }
    }

    pub fn is_shelf_entry(&self, node: &Node) -> bool {
        self.is_root() && node.kind == NodeKind::Directory && node.name == SHARED_WITH_ME
    }
}

pub fn parent_folder_of(path: &str) -> Folder {
    Folder::new(path).parent().unwrap_or_default()
}

pub fn sorted(mut nodes: Vec<Node>) -> Vec<Node> {
    nodes.sort_by(|left, right| {
        let directory_first = |node: &Node| node.kind != NodeKind::Directory;
        directory_first(left)
            .cmp(&directory_first(right))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    nodes
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use stashden_core::grant::Access;
    use uuid::Uuid;

    use super::*;

    fn node(name: &str, kind: NodeKind) -> Node {
        Node {
            id: Uuid::now_v7(),
            owner_id: Uuid::now_v7(),
            parent_id: None,
            name: name.to_owned(),
            kind,
            blob_hash: None,
            size: 0,
            etag: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    fn received(name: &str) -> Received {
        Received {
            id: Uuid::now_v7(),
            name: name.to_owned(),
            kind: NodeKind::Directory,
            access: Access::Read,
            owner_email: "o@x".to_owned(),
            owner_name: String::new(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn stray_slashes_collapse() {
        assert_eq!(Folder::new("//docs///2024/").as_str(), "docs/2024");
        assert_eq!(Folder::new("/").remote(), "/");
        assert_eq!(Folder::new("docs").remote(), "/docs");
    }

    #[test]
    fn a_child_of_the_root_has_no_leading_slash_and_a_typed_path_moves_deeper() {
        assert_eq!(Folder::root().child("a.txt"), "a.txt");
        assert_eq!(Folder::new("docs").child("a.txt"), "docs/a.txt");
        assert_eq!(Folder::new("docs").child("old/a.txt"), "docs/old/a.txt");
        assert_eq!(Folder::new("docs").child("/old//a.txt"), "docs/old/a.txt");
    }

    #[test]
    fn the_root_has_no_parent_and_a_top_folder_returns_to_it() {
        assert_eq!(Folder::root().parent(), None);
        assert_eq!(Folder::new("docs").parent(), Some(Folder::root()));
        assert_eq!(Folder::new("docs/2024").parent(), Some(Folder::new("docs")));
        assert_eq!(
            parent_folder_of("/docs/2024/a.txt"),
            Folder::new("docs/2024")
        );
        assert_eq!(parent_folder_of("a.txt"), Folder::root());
    }

    #[test]
    fn crumbs_lead_back_up_one_folder_at_a_time() {
        let crumbs = Folder::new("a/b/c").crumbs();
        let folders: Vec<_> = crumbs.iter().map(|crumb| crumb.folder.as_str()).collect();
        assert_eq!(folders, ["a", "a/b", "a/b/c"]);
        assert_eq!(crumbs[1].name, "b");
        assert_eq!(Folder::root().crumbs(), Vec::<Crumb>::new());
    }

    #[test]
    fn the_shelf_and_the_mounts_under_it_are_told_apart_from_own_files() {
        assert_eq!(Folder::new("docs").place(), Where::Own);
        assert_eq!(Folder::new(SHARED_WITH_ME).place(), Where::Shelf);
        assert_eq!(
            Folder::new("Shared with me/Plans/2024").place(),
            Where::Shared { mount: "Plans" }
        );
        assert_eq!(Folder::new("Shared with meh").place(), Where::Own);
    }

    #[test]
    fn a_mount_is_found_by_its_name_only_inside_the_shelf() {
        let grants = [received("Plans"), received("Photos")];
        let inside = Folder::new("Shared with me/Photos/2024");
        assert_eq!(
            inside.mount(&grants).map(|grant| grant.name.as_str()),
            Some("Photos")
        );
        assert!(Folder::new("Photos").mount(&grants).is_none());
        assert!(Folder::new("Shared with me/Gone").mount(&grants).is_none());
    }

    #[test]
    fn only_the_shelf_directory_at_the_root_counts_as_the_shelf() {
        let shelf = node(SHARED_WITH_ME, NodeKind::Directory);
        assert!(Folder::root().is_shelf_entry(&shelf));
        assert!(!Folder::new("docs").is_shelf_entry(&shelf));
        assert!(!Folder::root().is_shelf_entry(&node(SHARED_WITH_ME, NodeKind::File)));
    }

    #[test]
    fn folders_come_before_files_then_names_ignore_case() {
        let listing = sorted(vec![
            node("b.txt", NodeKind::File),
            node("Zeta", NodeKind::Directory),
            node("A.txt", NodeKind::File),
            node("alpha", NodeKind::Directory),
        ]);
        let names: Vec<_> = listing.iter().map(|node| node.name.as_str()).collect();
        assert_eq!(names, ["alpha", "Zeta", "A.txt", "b.txt"]);
    }
}
