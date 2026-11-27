#[cfg(windows)]
fn embed_windows_resources() -> std::io::Result<()> {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return Ok(());
    }
    println!("cargo:rerun-if-changed=icons/icon.ico");
    winresource::WindowsResource::new()
        .set_icon("icons/icon.ico")
        .set("ProductName", "Stashden")
        .set("FileDescription", "Stashden")
        .set("CompanyName", "FerrLabs")
        .set("OriginalFilename", "Stashden.exe")
        .compile()
}

fn main() -> std::io::Result<()> {
    #[cfg(windows)]
    embed_windows_resources()?;
    Ok(())
}
