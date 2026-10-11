pub mod zip;

use axum::body::Body;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use futures::StreamExt;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use sqlx::PgPool;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

use crate::db::node_columns;
use crate::error::ApiError;
use crate::state::AppState;
use stashden_core::node::{Node, NodeKind};

pub const MAX_ENTRIES: usize = 100_000;
const BACKLOG: usize = 4;

#[derive(sqlx::FromRow)]
struct Below {
    #[sqlx(flatten)]
    node: Node,
    rel_path: Vec<String>,
}

pub struct Member {
    pub path: String,
    pub node: Node,
}

pub async fn members(pool: &PgPool, root: &Node) -> Result<Vec<Member>, ApiError> {
    if root.kind != NodeKind::Directory {
        return Err(ApiError::WrongKind {
            expected: "directory",
        });
    }

    let limit = i64::try_from(MAX_ENTRIES + 1).unwrap_or(i64::MAX);
    let rows = sqlx::query_as::<_, Below>(concat!(
        "WITH RECURSIVE tree AS (
             SELECT id AS node_id, ARRAY[name] AS rel_path
             FROM nodes WHERE parent_id = $1 AND deleted_at IS NULL
             UNION ALL
             SELECT child.id, tree.rel_path || child.name
             FROM nodes child JOIN tree ON child.parent_id = tree.node_id
             WHERE child.deleted_at IS NULL
         )
         SELECT ",
        node_columns!(),
        ", tree.rel_path FROM nodes JOIN tree ON nodes.id = tree.node_id
         ORDER BY tree.rel_path LIMIT $2"
    ))
    .bind(root.id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    if rows.len() > MAX_ENTRIES {
        return Err(ApiError::TooLarge {
            what: "folder, which has more files than one archive carries",
        });
    }

    Ok(rows
        .into_iter()
        .map(|row| Member {
            path: format!("{}/{}", root.name, row.rel_path.join("/")),
            node: row.node,
        })
        .collect())
}

#[must_use]
pub fn respond(state: &AppState, root: &Node, members: Vec<Member>) -> Response {
    let (sender, receiver) = mpsc::channel::<Result<Bytes, std::io::Error>>(BACKLOG);
    let download_name = root.name.clone();
    let state = state.clone();
    let root = Member {
        path: root.name.clone(),
        node: root.clone(),
    };

    tokio::spawn(async move {
        match write(&state, &root, members, &sender).await {
            Ok(()) | Err(Stop::Closed) => {}
            Err(failure) => {
                tracing::warn!(%failure, "an archive stopped part way");
                let _ = sender
                    .send(Err(std::io::Error::other(failure.to_string())))
                    .await;
            }
        }
    });

    let body = Body::from_stream(futures::stream::unfold(
        receiver,
        |mut receiver| async move { receiver.recv().await.map(|item| (item, receiver)) },
    ));

    let name = format!("{download_name}.zip");
    (
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/zip"),
            ),
            (
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            ),
            (
                header::CONTENT_DISPOSITION,
                HeaderValue::from_str(&disposition(&name))
                    .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
            ),
        ],
        body,
    )
        .into_response()
}

fn disposition(name: &str) -> String {
    let fallback: String = name
        .chars()
        .map(|c| {
            if c.is_ascii() && !c.is_ascii_control() && c != '"' && c != '\\' && c != '/' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!(
        "attachment; filename=\"{fallback}\"; filename*=UTF-8''{}",
        utf8_percent_encode(name, NON_ALPHANUMERIC)
    )
}

#[derive(Debug, thiserror::Error)]
enum Stop {
    #[error("the client went away")]
    Closed,
    #[error("a blob could not be read: {0}")]
    Storage(#[from] crate::storage::StorageError),
    #[error("{0} is not the size its node says")]
    Size(Uuid),
}

struct Output<'a> {
    sender: &'a mpsc::Sender<Result<Bytes, std::io::Error>>,
    written: u64,
}

impl Output<'_> {
    async fn send(&mut self, bytes: Vec<u8>) -> Result<(), Stop> {
        self.written += bytes.len() as u64;
        self.sender
            .send(Ok(Bytes::from(bytes)))
            .await
            .map_err(|_| Stop::Closed)
    }
}

async fn write(
    state: &AppState,
    root: &Member,
    members: Vec<Member>,
    sender: &mpsc::Sender<Result<Bytes, std::io::Error>>,
) -> Result<(), Stop> {
    let mut out = Output { sender, written: 0 };
    let mut index: Vec<zip::Entry> = Vec::with_capacity(members.len() + 1);

    let top = zip::Entry {
        name: format!("{}/", root.path),
        directory: true,
        size: 0,
        crc: 0,
        offset: 0,
        modified: root.node.updated_at,
    };
    out.send(zip::local_header(&top)).await?;
    index.push(top);

    for member in members {
        let node = &member.node;
        let directory = node.kind == NodeKind::Directory;
        let size = if directory {
            0
        } else {
            u64::try_from(node.size).unwrap_or(0)
        };
        let mut entry = zip::Entry {
            name: if directory {
                format!("{}/", member.path)
            } else {
                member.path.clone()
            },
            directory,
            size,
            crc: 0,
            offset: out.written,
            modified: node.updated_at,
        };

        out.send(zip::local_header(&entry)).await?;

        if !directory {
            let mut crc = crc32fast::Hasher::new();
            let mut seen = 0_u64;
            if let Some(hash) = node.blob_hash {
                let mut stream = ReaderStream::new(state.blobs.read(hash).await?.take(size + 1));
                while let Some(chunk) = stream.next().await {
                    let chunk = chunk.map_err(crate::storage::StorageError::Io)?;
                    crc.update(&chunk);
                    seen += chunk.len() as u64;
                    out.written += chunk.len() as u64;
                    out.sender.send(Ok(chunk)).await.map_err(|_| Stop::Closed)?;
                }
            }
            if seen != size {
                return Err(Stop::Size(node.id));
            }
            entry.crc = crc.finalize();
            out.send(zip::data_descriptor(size, entry.crc)).await?;
        }
        index.push(entry);
    }

    let start = out.written;
    out.send(zip::central_directory(&index, start)).await
}
