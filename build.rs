//! Compiles `notefetch.rc` into the executable: the manifest and version now,
//! and the dialog templates from milestone 3. `embed-resource` runs rc.exe at
//! build time; nothing of it ships.
fn main() {
    embed_resource::compile("notefetch.rc", embed_resource::NONE)
        .manifest_required()
        .unwrap();
    println!("cargo:rerun-if-changed=notefetch.rc");
    println!("cargo:rerun-if-changed=notefetch.manifest");
}
