fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("./peppermint.proto")?;
    println!("cargo:rerun-if-changed=peppermint.proto");
    Ok(())
}
