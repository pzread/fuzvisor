fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().compile(
        &[
            "../proto/control_flow_graph.proto",
            "../proto/collector_service.proto",
        ],
        &["../proto"],
    )?;
    Ok(())
}
