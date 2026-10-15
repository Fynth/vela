fn main() {
    // Stable Rust does not make sqlx::migrate! watch newly added directory entries.
    // Track the directory so incremental binaries embed every new migration.
    println!("cargo:rerun-if-changed=../../migrations");
}
