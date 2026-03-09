use anyhow::Result;

#[allow(clippy::unnecessary_wraps)] // Skeleton — real main will have fallible calls
fn main() -> Result<()> {
    env_logger::init();

    println!("Hello from linecop!");

    Ok(())
}
