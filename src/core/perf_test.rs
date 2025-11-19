use std::time::{Duration, Instant};

/// Performance test result
#[derive(Debug, Clone)]
pub struct PerfResult {
    pub name: String,
    pub iterations: usize,
    pub total_duration: Duration,
    pub avg_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub std_dev: f64,
}

impl PerfResult {
    pub fn throughput(&self, operations_per_iter: usize) -> f64 {
        let ops_per_sec = operations_per_iter as f64 / self.avg_duration.as_secs_f64();
        ops_per_sec
    }

    pub fn print_summary(&self) {
        println!("\n=== {} ===", self.name);
        println!("Iterations: {}", self.iterations);
        println!("Total:      {:?}", self.total_duration);
        println!("Average:    {:?}", self.avg_duration);
        println!("Min:        {:?}", self.min_duration);
        println!("Max:        {:?}", self.max_duration);
        println!("Std Dev:    {:.2} µs", self.std_dev * 1_000_000.0);
    }

    pub fn print_comparison(&self, baseline: &PerfResult) {
        let speedup = baseline.avg_duration.as_secs_f64() / self.avg_duration.as_secs_f64();
        println!(
            "{:30} {:>12.2} µs ({:>6.2}x)",
            self.name,
            self.avg_duration.as_secs_f64() * 1_000_000.0,
            speedup
        );
    }
}

/// Performance test runner
pub struct PerfTest {
    name: String,
    warmup_iterations: usize,
    test_iterations: usize,
}

impl PerfTest {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            warmup_iterations: 10,
            test_iterations: 100,
        }
    }

    pub fn with_warmup(mut self, iterations: usize) -> Self {
        self.warmup_iterations = iterations;
        self
    }

    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.test_iterations = iterations;
        self
    }

    /// Run benchmark with warmup
    pub fn run<F>(&self, mut test_fn: F) -> PerfResult
    where
        F: FnMut(),
    {
        // Warmup phase
        for _ in 0..self.warmup_iterations {
            test_fn();
        }

        // Actual measurements
        let mut durations = Vec::with_capacity(self.test_iterations);

        for _ in 0..self.test_iterations {
            let start = Instant::now();
            test_fn();
            let duration = start.elapsed();
            durations.push(duration);
        }

        self.calculate_stats(&durations)
    }

    fn calculate_stats(&self, durations: &[Duration]) -> PerfResult {
        let total: Duration = durations.iter().sum();
        let avg = total / durations.len() as u32;
        let min = *durations.iter().min().unwrap();
        let max = *durations.iter().max().unwrap();

        // Calculate standard deviation
        let avg_secs = avg.as_secs_f64();
        let variance: f64 = durations
            .iter()
            .map(|d| {
                let diff = d.as_secs_f64() - avg_secs;
                diff * diff
            })
            .sum::<f64>()
            / durations.len() as f64;
        let std_dev = variance.sqrt();

        PerfResult {
            name: self.name.clone(),
            iterations: durations.len(),
            total_duration: total,
            avg_duration: avg,
            min_duration: min,
            max_duration: max,
            std_dev,
        }
    }
}

/// Comparison suite for multiple benchmarks
pub struct PerfSuite {
    name: String,
    results: Vec<PerfResult>,
}

impl PerfSuite {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            results: Vec::new(),
        }
    }

    pub fn add_result(&mut self, result: PerfResult) {
        self.results.push(result);
    }

    pub fn print_comparison(&self) {
        if self.results.is_empty() {
            println!("No results to compare");
            return;
        }

        println!("\n╔══════════════════════════════════════════════════════╗");
        println!("║  {}  ║", self.name);
        println!("╠══════════════════════════════════════════════════════╣");
        println!("║ {:30} {:>12} {:>7} ║", "Method", "Avg Time", "Speedup");
        println!("╠══════════════════════════════════════════════════════╣");

        let baseline = &self.results[0];
        for result in &self.results {
            let speedup = baseline.avg_duration.as_secs_f64() / result.avg_duration.as_secs_f64();
            println!(
                "║ {:30} {:>9.2} µs {:>6.2}x ║",
                result.name,
                result.avg_duration.as_secs_f64() * 1_000_000.0,
                speedup
            );
        }
        println!("╚══════════════════════════════════════════════════════╝");
    }

    pub fn find_fastest(&self) -> Option<&PerfResult> {
        self.results
            .iter()
            .min_by(|a, b| a.avg_duration.cmp(&b.avg_duration))
    }

    pub fn find_slowest(&self) -> Option<&PerfResult> {
        self.results
            .iter()
            .max_by(|a, b| a.avg_duration.cmp(&b.avg_duration))
    }
}

/// Memory profiling utilities
pub struct MemoryProfile {
    allocations: usize,
    total_bytes: usize,
}

impl MemoryProfile {
    pub fn new() -> Self {
        Self {
            allocations: 0,
            total_bytes: 0,
        }
    }

    pub fn record_allocation(&mut self, bytes: usize) {
        self.allocations += 1;
        self.total_bytes += bytes;
    }

    pub fn print_summary(&self) {
        println!("Memory Allocations: {}", self.allocations);
        println!("Total Bytes: {} ({:.2} KB)", self.total_bytes, self.total_bytes as f64 / 1024.0);
    }
}

/// Ray tracing specific benchmarks
pub mod ray_tracing {
    use super::*;
    use glam::Vec3;

    /// Generate random rays for testing
    pub fn generate_test_rays(count: usize, seed: u64) -> Vec<(Vec3, Vec3)> {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        let mut rays = Vec::with_capacity(count);
        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);

        for i in 0..count {
            (i as u64).hash(&mut hasher);
            let hash = hasher.finish();

            let theta = ((hash & 0xFFFF) as f32 / 65535.0) * 2.0 * std::f32::consts::PI;
            let phi = (((hash >> 16) & 0xFFFF) as f32 / 65535.0) * std::f32::consts::PI;

            let dir = Vec3::new(
                phi.sin() * theta.cos(),
                phi.sin() * theta.sin(),
                phi.cos(),
            )
            .normalize();

            let origin = Vec3::ZERO;
            rays.push((origin, dir));
        }

        rays
    }

    /// Benchmark ray generation throughput
    pub fn bench_ray_generation(count: usize) -> PerfResult {
        PerfTest::new("Ray Generation")
            .with_warmup(5)
            .with_iterations(50)
            .run(|| {
                let _rays = generate_test_rays(count, 42);
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn test_perf_test_basic() {
        let result = PerfTest::new("test_operation")
            .with_warmup(5)
            .with_iterations(10)
            .run(|| {
                // Simulate some work
                let mut sum = 0;
                for i in 0..100 {
                    sum += i;
                }
                std::hint::black_box(sum);
            });

        assert_eq!(result.iterations, 10);
        assert!(result.avg_duration.as_nanos() > 0);
        assert!(result.min_duration <= result.avg_duration);
        assert!(result.avg_duration <= result.max_duration);
    }

    #[test]
    fn test_perf_suite() {
        let mut suite = PerfSuite::new("Test Suite");

        let result1 = PerfTest::new("Fast Operation")
            .with_iterations(5)
            .run(|| {
                std::hint::black_box(1 + 1);
            });

        let result2 = PerfTest::new("Slow Operation")
            .with_iterations(5)
            .run(|| {
                let mut sum = 0;
                for i in 0..1000 {
                    sum += i;
                }
                std::hint::black_box(sum);
            });

        suite.add_result(result1);
        suite.add_result(result2);

        let fastest = suite.find_fastest().unwrap();
        assert_eq!(fastest.name, "Fast Operation");

        let slowest = suite.find_slowest().unwrap();
        assert_eq!(slowest.name, "Slow Operation");
    }

    #[test]
    fn test_throughput_calculation() {
        let result = PerfResult {
            name: "Test".to_string(),
            iterations: 100,
            total_duration: Duration::from_secs(1),
            avg_duration: Duration::from_millis(10),
            min_duration: Duration::from_millis(9),
            max_duration: Duration::from_millis(11),
            std_dev: 0.001,
        };

        let throughput = result.throughput(1000);
        assert!(throughput > 0.0);
    }

    #[test]
    fn test_memory_profile() {
        let mut profile = MemoryProfile::new();
        profile.record_allocation(1024);
        profile.record_allocation(2048);

        assert_eq!(profile.allocations, 2);
        assert_eq!(profile.total_bytes, 3072);
    }

    #[test]
    fn test_ray_generation() {
        let rays = ray_tracing::generate_test_rays(100, 42);
        assert_eq!(rays.len(), 100);

        for (origin, dir) in rays {
            assert_eq!(origin, Vec3::ZERO);
            assert!((dir.length() - 1.0).abs() < 0.01); // Direction should be normalized
        }
    }

    #[test]
    fn test_stats_calculation() {
        let durations = vec![
            Duration::from_millis(10),
            Duration::from_millis(12),
            Duration::from_millis(11),
            Duration::from_millis(13),
            Duration::from_millis(9),
        ];

        let test = PerfTest::new("stats_test");
        let result = test.calculate_stats(&durations);

        assert_eq!(result.min_duration, Duration::from_millis(9));
        assert_eq!(result.max_duration, Duration::from_millis(13));
        assert!(result.std_dev > 0.0);
    }
}
