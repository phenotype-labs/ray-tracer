// cli.rs - Command-line interface configuration
use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "ray-tracer")]
#[command(about = "WebGPU Ray Tracer", long_about = None)]
pub struct Cli {
    /// Disable UI elements and console output
    #[arg(long = "no-ui", default_value = "false")]
    pub no_ui: bool,
}
