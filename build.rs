fn main() {
    println!("cargo:rerun-if-changed=resources.rc");
    println!("cargo:rerun-if-changed=assets/parker.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_resource::compile("resources.rc", embed_resource::NONE)
            .manifest_required()
            .expect("failed to compile Parker Windows resources");
    }
}
