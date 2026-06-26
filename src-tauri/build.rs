fn main() {
    println!("cargo:rerun-if-changed=../dist");
    println!("cargo:rerun-if-env-changed=CLEANERX_DISTRIBUTION");
    tauri_build::build();
}
