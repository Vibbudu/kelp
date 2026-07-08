fn main() {
    // Only compile resource icon on Windows targets
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        embed_resource::compile("resources.rc", embed_resource::NONE);
    }
}
