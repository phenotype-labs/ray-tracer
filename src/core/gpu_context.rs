use std::sync::Arc;
use wgpu::{Device, Queue, Instance, Surface, Adapter, Features, Limits, DeviceDescriptor, Buffer};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Shared GPU context for multiple layers
///
/// This provides a shared Device and Queue that can be cloned cheaply (Arc)
/// and used by multiple layers without each needing their own GPU context.
#[derive(Clone)]
pub struct GpuContext {
    device: Arc<Device>,
    queue: Arc<Queue>,
}

impl GpuContext {
    /// Create a new GPU context without a surface (for offscreen rendering)
    ///
    /// This is useful for compute-only workloads where no window is needed.
    pub async fn new() -> Result<Self> {
        let instance = Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let adapter = Self::request_adapter_headless(&instance).await?;
        let (device, queue) = Self::request_device(&adapter).await?;

        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
        })
    }

    /// Create a GPU context compatible with a surface (for window rendering)
    ///
    /// This ensures the adapter is compatible with the provided surface.
    pub async fn new_with_surface(surface: &Surface<'_>) -> Result<Self> {
        let instance = Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let adapter = Self::request_adapter(&instance, surface).await?;
        let (device, queue) = Self::request_device(&adapter).await?;

        Ok(Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
        })
    }

    /// Get reference to the device
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Get reference to the queue
    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    /// Synchronously read data from a buffer
    ///
    /// IMPORTANT: This is a blocking operation that polls the device.
    /// Use sparingly and only when necessary (e.g., for layer pixel readback).
    pub async fn read_buffer(&self, buffer: &Buffer, _size: u64) -> Result<Vec<u8>> {
        let buffer_slice = buffer.slice(..);

        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).ok();
        });

        self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        }).ok();

        match receiver.recv() {
            Ok(Ok(())) => {
                let data = buffer_slice.get_mapped_range();
                let result = data.to_vec();
                drop(data);
                buffer.unmap();
                Ok(result)
            }
            Ok(Err(e)) => Err(format!("Buffer mapping failed: {:?}", e).into()),
            Err(_) => Err("Channel closed before receiving result".into()),
        }
    }

    /// Synchronously read data from a buffer (blocking version)
    ///
    /// WARNING: This blocks the current thread. Prefer read_buffer() in async contexts.
    pub fn read_buffer_sync(&self, buffer: &Buffer) -> Result<Vec<u8>> {
        let buffer_slice = buffer.slice(..);

        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).ok();
        });

        self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        }).ok();

        match receiver.recv() {
            Ok(Ok(())) => {
                let data = buffer_slice.get_mapped_range();
                let result = data.to_vec();
                drop(data);
                buffer.unmap();
                Ok(result)
            }
            Ok(Err(e)) => Err(format!("Buffer mapping failed: {:?}", e).into()),
            Err(_) => Err("Channel closed before receiving result".into()),
        }
    }

    /// Request adapter with surface compatibility
    async fn request_adapter(instance: &Instance, surface: &Surface<'_>) -> Result<Adapter> {
        instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| format!("Failed to find appropriate adapter: {:?}", e).into())
    }

    /// Request adapter without surface (headless)
    async fn request_adapter_headless(instance: &Instance) -> Result<Adapter> {
        instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| format!("Failed to find appropriate adapter: {:?}", e).into())
    }

    /// Request device and queue
    async fn request_device(adapter: &Adapter) -> Result<(Device, Queue)> {
        let supported_features = adapter.features();
        let mut requested_features = Features::empty();

        // Request timestamp queries if available (for profiling)
        if supported_features.contains(Features::TIMESTAMP_QUERY) {
            requested_features |= Features::TIMESTAMP_QUERY;
        }

        // Request texture adapter specific format query
        if supported_features.contains(Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES) {
            requested_features |= Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES;
        }

        let limits = Limits {
            max_storage_buffer_binding_size: adapter.limits().max_storage_buffer_binding_size,
            max_buffer_size: adapter.limits().max_buffer_size,
            ..Default::default()
        };

        adapter
            .request_device(&DeviceDescriptor {
                label: Some("GPU Context Device"),
                required_features: requested_features,
                required_limits: limits,
                memory_hints: Default::default(),
                experimental_features: Default::default(),
                trace: Default::default(),
            })
            .await
            .map_err(|e| format!("Failed to create device: {:?}", e).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_context_creation() {
        // GpuContext creation requires actual GPU hardware
        // These tests would be run in integration tests with a real device
        // For now, we just test the type exists and can be cloned
        assert!(std::mem::size_of::<GpuContext>() > 0);
    }

    #[test]
    fn test_clone_semantics() {
        // Test that Arc cloning works as expected (compile-time check)
        fn assert_clone<T: Clone>() {}
        assert_clone::<GpuContext>();
    }
}
