use uuid::Uuid;

use super::super::engine::SyncError;
use super::super::state::StateError;
use super::*;

const SERVER: &str = "https://files.example.com";

fn user(number: u128) -> Account {
    Account::new(SERVER, Uuid::from_u128(number))
}

fn open(local: &Path, server: &Path, account: &Account) -> Result<Engine<FakeServer>, SyncError> {
    Engine::open(
        local.to_path_buf(),
        FakeServer {
            root: server.to_path_buf(),
            held: Held::default(),
            before_upload: None,
            fingerprinting: false,
            walks: std::sync::Arc::default(),
        },
        account,
    )
}

fn storage_of(pair: &Pair, name: &str) -> PathBuf {
    let storage = pair.server.with_file_name(name);
    fs::create_dir_all(&storage).expect("another account's storage");
    storage
}

fn state_file(pair: &Pair) -> PathBuf {
    pair.local.join(STATE_FILE_NAME)
}

fn refusal(result: Result<Engine<FakeServer>, SyncError>) -> String {
    match result {
        Err(error @ SyncError::State(StateError::OtherAccount { .. })) => error.to_string(),
        Err(other) => panic!("expected a refusal, got {other}"),
        Ok(_) => panic!("expected a refusal, the folder opened"),
    }
}

async fn synced_with_first_account(name: &str) -> Pair {
    let pair = Pair::new(name);
    pair.write_server("a.txt", b"first account");
    open(&pair.local, &pair.server, &user(1))
        .expect("first account opens")
        .sync_once()
        .await
        .expect("first account syncs");
    assert_eq!(
        pair.read_local("a.txt").as_deref(),
        Some(&b"first account"[..])
    );
    pair
}

#[tokio::test]
async fn another_account_is_refused_before_anything_is_transferred_or_deleted() {
    let pair = synced_with_first_account("binding-refused").await;
    let second = storage_of(&pair, "binding-refused-second");
    let state_before = fs::read(state_file(&pair)).expect("state");

    let message = refusal(open(&pair.local, &second, &user(2)));

    assert_eq!(
        pair.read_local("a.txt").as_deref(),
        Some(&b"first account"[..])
    );
    assert_eq!(
        pair.read_server("a.txt").as_deref(),
        Some(&b"first account"[..])
    );
    assert_eq!(names(&second), Vec::<String>::new());
    assert_eq!(fs::read(state_file(&pair)).expect("state"), state_before);
    assert!(
        message.contains(&Uuid::from_u128(1).to_string()),
        "{message}"
    );
    assert!(
        message.contains(&Uuid::from_u128(2).to_string()),
        "{message}"
    );
    assert!(message.contains(STATE_FILE_NAME), "{message}");
    assert!(message.contains("deletes nothing"), "{message}");
}

#[tokio::test]
async fn deleting_the_state_file_compares_in_full_and_deletes_nothing() {
    let pair = synced_with_first_account("binding-reset").await;
    let second = storage_of(&pair, "binding-reset-second");
    pair.write_server("only-on-first.txt", b"never seen locally");
    pair.write_local("new-here.txt", b"made after");
    fs::remove_file(state_file(&pair)).expect("deleting the state");

    let report = open(&pair.local, &second, &user(2))
        .expect("an unbound folder opens")
        .sync_once()
        .await
        .expect("second account syncs");

    assert_eq!(report.deleted_locally, 0, "{report:?}");
    assert_eq!(report.deleted_remotely, 0, "{report:?}");
    assert_eq!(
        pair.read_local("a.txt").as_deref(),
        Some(&b"first account"[..])
    );
    assert_eq!(
        fs::read(second.join("a.txt")).expect("uploaded").as_slice(),
        b"first account"
    );
}

#[tokio::test]
async fn a_legacy_state_is_adopted_and_written_back_with_its_tracked_files() {
    let pair = Pair::new("binding-legacy");
    pair.write_server("a.txt", b"agreed");
    pair.engine().sync_once().await.expect("syncs");
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(state_file(&pair)).expect("state")).expect("json");
    value.as_object_mut().expect("object").remove("account");
    fs::write(state_file(&pair), value.to_string()).expect("a state without an account");
    let legacy = SyncState::load(&state_file(&pair)).expect("state");

    let adopted = SyncState::load_bound(&state_file(&pair), &user(7)).expect("adopts");

    assert_eq!(adopted.base(), legacy.base());
    assert!(!adopted.base().is_empty());
    let written = fs::read_to_string(state_file(&pair)).expect("state text");
    assert!(
        written.contains(&Uuid::from_u128(7).to_string()),
        "{written}"
    );
    assert!(
        SyncState::load_bound(&state_file(&pair), &user(7)).is_ok(),
        "the adopted binding is accepted next time"
    );
    refusal(open(&pair.local, &pair.server, &user(8)));
}

#[test]
fn a_folder_with_no_state_is_bound_by_its_first_open() {
    let pair = Pair::new("binding-fresh");
    assert!(!state_file(&pair).exists());

    open(&pair.local, &pair.server, &user(1)).expect("opens");

    assert!(state_file(&pair).exists());
    refusal(open(&pair.local, &pair.server, &user(2)));
}

#[test]
fn the_same_account_matches_however_the_server_is_written() {
    let pair = Pair::new("binding-spelling");
    let id = Uuid::from_u128(1);
    open(
        &pair.local,
        &pair.server,
        &Account::new("https://files.example.com", id),
    )
    .expect("binds");

    for spelling in [
        "https://files.example.com/",
        "https://Files.Example.COM",
        "HTTPS://FILES.EXAMPLE.COM//",
    ] {
        open(&pair.local, &pair.server, &Account::new(spelling, id))
            .unwrap_or_else(|error| panic!("{spelling} was refused: {error}"));
    }
}

#[test]
fn the_same_user_id_on_another_instance_is_another_account() {
    let pair = Pair::new("binding-instances");
    let id = Uuid::from_u128(1);
    open(
        &pair.local,
        &pair.server,
        &Account::new("https://one.example.com", id),
    )
    .expect("binds");

    refusal(open(
        &pair.local,
        &pair.server,
        &Account::new("https://two.example.com", id),
    ));
}

#[test]
fn two_folders_bound_to_different_accounts_each_accept_only_their_own() {
    let first = Pair::new("binding-two-folders-first");
    let second = Pair::new("binding-two-folders-second");
    open(&first.local, &first.server, &user(1)).expect("binds the first");
    open(&second.local, &second.server, &user(2)).expect("binds the second");

    open(&first.local, &first.server, &user(1)).expect("the first keeps its own");
    open(&second.local, &second.server, &user(2)).expect("the second keeps its own");
    refusal(open(&first.local, &first.server, &user(2)));
    refusal(open(&second.local, &second.server, &user(1)));
}

#[tokio::test]
async fn a_sync_pass_keeps_the_binding() {
    let pair = Pair::new("binding-kept");
    let mut engine = open(&pair.local, &pair.server, &user(1)).expect("opens");
    pair.write_local("a.txt", b"mine");
    engine.sync_once().await.expect("syncs");
    drop(engine);

    refusal(open(&pair.local, &pair.server, &user(2)));
}

#[test]
fn an_unreadable_state_is_reported_and_left_alone_rather_than_bound() {
    let pair = Pair::new("binding-corrupt");
    fs::write(state_file(&pair), b"{ not json").expect("writes");

    assert!(matches!(
        open(&pair.local, &pair.server, &user(1)),
        Err(SyncError::State(StateError::Corrupt { .. }))
    ));
    assert_eq!(fs::read(state_file(&pair)).expect("state"), b"{ not json");
}

#[test]
fn an_account_that_is_not_a_user_id_is_reported_as_unreadable() {
    let pair = Pair::new("binding-bad-account");
    let broken =
        r#"{"account":{"server":"https://files.example.com","user_id":"nobody"},"tracked":{}}"#;
    fs::write(state_file(&pair), broken).expect("writes");

    assert!(matches!(
        open(&pair.local, &pair.server, &user(1)),
        Err(SyncError::State(StateError::Corrupt { .. }))
    ));
    assert_eq!(
        fs::read_to_string(state_file(&pair)).expect("state"),
        broken
    );
}
