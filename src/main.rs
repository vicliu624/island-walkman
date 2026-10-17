mod database;
mod metadata;
mod protocol;
mod service;
mod sony_artwork;
mod space;
fn main() -> anyhow::Result<()> {
    service::run()
}
