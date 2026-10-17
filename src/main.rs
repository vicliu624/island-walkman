mod database;
mod metadata;
mod protocol;
mod service;
mod sony_artwork;
fn main() -> anyhow::Result<()> {
    service::run()
}
