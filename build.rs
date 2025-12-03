fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile the proto files
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir("src/api")
        .compile_protos(
            &["proto/lastmile/v1/analytics.proto"],
            &["proto"],
        )?;

    Ok(())
}
