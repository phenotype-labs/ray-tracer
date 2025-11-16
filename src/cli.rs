// cli.rs - Command-line interface and frame capture configuration
use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "ray-tracer")]
#[command(about = "WebGPU Ray Tracer with Frame Capture", long_about = None)]
pub struct Cli {
    /// Capture specific frames (e.g., "1..3", "5", "10..20", "1,5,10")
    ///
    /// Supports:
    ///   - Single frame: "5"
    ///   - Range: "1..10" (captures frames 1 through 10)
    ///   - Multiple: "1,5,10" (captures frames 1, 5, and 10)
    ///   - Combined: "1..3,5,10..12"
    #[arg(long = "capture-frame", value_name = "RANGE")]
    pub capture_frame: Option<String>,

    /// Auto-export captured frames to JSON
    #[arg(long = "export-json", default_value = "true")]
    pub export_json: bool,

    /// Auto-export captured frames to Chrome Tracing format
    #[arg(long = "export-chrome", default_value = "false")]
    pub export_chrome: bool,

    /// Output directory for exported frames
    #[arg(long = "output-dir", value_name = "DIR", default_value = ".")]
    pub output_dir: String,

    /// Exit after capturing all specified frames
    #[arg(long = "exit-after-capture", default_value = "false")]
    pub exit_after_capture: bool,

    /// Disable frame capture UI (headless mode)
    #[arg(long = "headless", default_value = "false")]
    pub headless: bool,

    /// Disable UI elements and console output
    #[arg(long = "no-ui", default_value = "false")]
    pub no_ui: bool,
}

#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub enabled: bool,
    pub frames: Vec<u64>,
    pub export_json: bool,
    pub export_chrome: bool,
    pub output_dir: String,
    pub exit_after_capture: bool,
    pub headless: bool,
    pub no_ui: bool,
}

impl CaptureConfig {
    pub fn from_cli(cli: &Cli) -> Result<Self, String> {
        if let Some(range_str) = &cli.capture_frame {
            let frames = parse_frame_range(range_str)?;
            Ok(Self {
                enabled: true,
                frames,
                export_json: cli.export_json,
                export_chrome: cli.export_chrome,
                output_dir: cli.output_dir.clone(),
                exit_after_capture: cli.exit_after_capture,
                headless: cli.headless,
                no_ui: cli.no_ui,
            })
        } else {
            Ok(Self {
                enabled: false,
                frames: Vec::new(),
                export_json: false,
                export_chrome: false,
                output_dir: ".".to_string(),
                exit_after_capture: false,
                headless: false,
                no_ui: false,
            })
        }
    }

    pub fn should_capture_frame(&self, frame_number: u64) -> bool {
        self.enabled && self.frames.contains(&frame_number)
    }

    pub fn all_frames_captured(&self, captured_frames: &[u64]) -> bool {
        if !self.enabled {
            return false;
        }
        self.frames.iter().all(|f| captured_frames.contains(f))
    }
}

/// Parse frame range string into list of frame numbers
///
/// Examples:
///   - "5" -> [5]
///   - "1..3" -> [1, 2, 3]
///   - "1,5,10" -> [1, 5, 10]
///   - "1..3,5,10..12" -> [1, 2, 3, 5, 10, 11, 12]
pub fn parse_frame_range(input: &str) -> Result<Vec<u64>, String> {
    let mut frames = Vec::new();

    // Split by comma for multiple ranges/values
    for part in input.split(',') {
        let part = part.trim();

        if part.contains("..") {
            // Range syntax: "1..10"
            let parts: Vec<&str> = part.split("..").collect();
            if parts.len() != 2 {
                return Err(format!("Invalid range syntax: '{}'. Expected 'start..end'", part));
            }

            let start: u64 = parts[0].trim().parse()
                .map_err(|_| format!("Invalid start number: '{}'", parts[0]))?;
            let end: u64 = parts[1].trim().parse()
                .map_err(|_| format!("Invalid end number: '{}'", parts[1]))?;

            if start > end {
                return Err(format!("Invalid range: {} > {}. Start must be <= end", start, end));
            }

            frames.extend(start..=end);
        } else {
            // Single frame: "5"
            let frame: u64 = part.parse()
                .map_err(|_| format!("Invalid frame number: '{}'", part))?;
            frames.push(frame);
        }
    }

    // Remove duplicates and sort
    frames.sort_unstable();
    frames.dedup();

    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_frame() {
        assert_eq!(parse_frame_range("5").unwrap(), vec![5]);
    }

    #[test]
    fn test_parse_range() {
        assert_eq!(parse_frame_range("1..3").unwrap(), vec![1, 2, 3]);
        assert_eq!(parse_frame_range("10..15").unwrap(), vec![10, 11, 12, 13, 14, 15]);
    }

    #[test]
    fn test_parse_multiple() {
        assert_eq!(parse_frame_range("1,5,10").unwrap(), vec![1, 5, 10]);
    }

    #[test]
    fn test_parse_combined() {
        assert_eq!(
            parse_frame_range("1..3,5,10..12").unwrap(),
            vec![1, 2, 3, 5, 10, 11, 12]
        );
    }

    #[test]
    fn test_parse_duplicates() {
        // Should remove duplicates
        assert_eq!(parse_frame_range("1,2,2,3").unwrap(), vec![1, 2, 3]);
        assert_eq!(parse_frame_range("1..3,2..4").unwrap(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_parse_invalid() {
        assert!(parse_frame_range("abc").is_err());
        assert!(parse_frame_range("1..").is_err());
        assert!(parse_frame_range("..5").is_err());
        assert!(parse_frame_range("5..1").is_err()); // start > end
    }
}
