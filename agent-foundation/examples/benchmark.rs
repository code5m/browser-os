//! Run the identical synthetic three-task suite under both engines locally.
//! Examples do not load production credentials or invoke native capabilities.
use browseros_agent_foundation::benchmark::run_benchmark_suite;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let report = run_benchmark_suite(5).await?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
