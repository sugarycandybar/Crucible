use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use ash::vk;

const BUF_SIZE: usize = 1 << 20;
const WORKGROUP_SIZE: u32 = 256;

const SHADER_GLSL: &str = r#"
#version 460
layout(local_size_x = 256, local_size_y = 1, local_size_z = 1) in;
layout(std430, set = 0, binding = 0) buffer Data { float values[]; } buf;
void main() {
    uint idx = gl_GlobalInvocationID.x;
    float v = buf.values[idx];
    for (int i = 0; i < 100; i++) {
        v = sin(v * 3.14159) * cos(v * 0.57721) + sqrt(abs(v));
    }
    buf.values[idx] = v;
}
"#;

fn compile_shader() -> Result<Vec<u32>, String> {
    let mut frontend = naga::front::glsl::Frontend::default();
    let options = naga::front::glsl::Options {
        stage: naga::ShaderStage::Compute,
        defines: Default::default(),
    };
    let module = frontend
        .parse(&options, SHADER_GLSL)
        .map_err(|e| format!("GLSL parse error: {e}"))?;

    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .map_err(|e| format!("Validation error: {e}"))?;

    let spv =
        naga::back::spv::write_vec(&module, &info, &naga::back::spv::Options::default(), None)
            .map_err(|e| format!("SPIR-V generation error: {e}"))?;
    Ok(spv)
}

struct VulkanContext {
    instance: ash::Instance,
    device: ash::Device,
    _physical_device: vk::PhysicalDevice,
    queue: vk::Queue,
    _queue_family_index: u32,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    descriptor_set_layout: vk::DescriptorSetLayout,
    shader_module: vk::ShaderModule,
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
    buffer: vk::Buffer,
    buffer_memory: vk::DeviceMemory,
    _buffer_size: vk::DeviceSize,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set: vk::DescriptorSet,
}

impl VulkanContext {
    fn create(entry: &ash::Entry) -> Result<Self, String> {
        let app_name =
            std::ffi::CString::new("Crucible GPU Stress").map_err(|e| format!("CString: {e}"))?;
        let engine_name =
            std::ffi::CString::new("Crucible").map_err(|e| format!("CString: {e}"))?;

        let app_info = vk::ApplicationInfo::default()
            .application_name(app_name.as_c_str())
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(engine_name.as_c_str())
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(vk::API_VERSION_1_0);

        let instance_create_info = vk::InstanceCreateInfo::default().application_info(&app_info);

        let instance = unsafe {
            entry
                .create_instance(&instance_create_info, None)
                .map_err(|e| format!("Failed to create Vulkan instance: {e}"))?
        };

        let physical_devices = unsafe {
            instance
                .enumerate_physical_devices()
                .map_err(|e| format!("Failed to enumerate devices: {e}"))?
        };

        let (physical_device, queue_family_index) =
            Self::pick_compute_device(&instance, &physical_devices)
                .ok_or_else(|| "No Vulkan device with compute support found".to_string())?;

        let queue_priorities = [1.0f32];
        let queue_create_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family_index)
            .queue_priorities(&queue_priorities);

        let queue_create_infos = [queue_create_info];
        let device_create_info =
            vk::DeviceCreateInfo::default().queue_create_infos(&queue_create_infos);

        let device = unsafe {
            instance
                .create_device(physical_device, &device_create_info, None)
                .map_err(|e| format!("Failed to create logical device: {e}"))?
        };

        let queue = unsafe { device.get_device_queue(queue_family_index, 0) };

        let spv = compile_shader()?;
        let shader_module_create_info = vk::ShaderModuleCreateInfo::default().code(&spv);
        let shader_module = unsafe {
            device
                .create_shader_module(&shader_module_create_info, None)
                .map_err(|e| format!("Failed to create shader module: {e}"))?
        };

        let layout_binding = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE);

        let layout_bindings = [layout_binding];
        let descriptor_set_layout_create_info =
            vk::DescriptorSetLayoutCreateInfo::default().bindings(&layout_bindings);
        let descriptor_set_layout = unsafe {
            device
                .create_descriptor_set_layout(&descriptor_set_layout_create_info, None)
                .map_err(|e| format!("Failed to create descriptor set layout: {e}"))?
        };

        let set_layouts = [descriptor_set_layout];
        let pipeline_layout_create_info =
            vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layouts);
        let pipeline_layout = unsafe {
            device
                .create_pipeline_layout(&pipeline_layout_create_info, None)
                .map_err(|e| format!("Failed to create pipeline layout: {e}"))?
        };

        let entry_point_name = std::ffi::CString::new("main").unwrap();
        let shader_stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(shader_module)
            .name(entry_point_name.as_c_str());

        let compute_pipeline_create_info = vk::ComputePipelineCreateInfo::default()
            .stage(shader_stage)
            .layout(pipeline_layout);

        let pipeline = unsafe {
            device
                .create_compute_pipelines(
                    vk::PipelineCache::null(),
                    &[compute_pipeline_create_info],
                    None,
                )
                .map_err(|(_, e)| format!("Failed to create compute pipeline: {e:?}"))?
                .remove(0)
        };

        let buffer_size = (BUF_SIZE as vk::DeviceSize * 4).max(256);
        let buffer_create_info = vk::BufferCreateInfo::default()
            .size(buffer_size)
            .usage(vk::BufferUsageFlags::STORAGE_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let buffer = unsafe {
            device
                .create_buffer(&buffer_create_info, None)
                .map_err(|e| format!("Failed to create buffer: {e}"))?
        };

        let mem_reqs = unsafe { device.get_buffer_memory_requirements(buffer) };
        let mem_properties =
            unsafe { instance.get_physical_device_memory_properties(physical_device) };
        // Pick a host-visible memory type that the buffer is actually allowed to
        // use. `memory_type_bits` is indexed by memory *type* index (and the
        // array's tail past `memory_type_count` is uninitialized, so only the
        // valid entries are considered).
        let mem_type_index = mem_properties
            .memory_types
            .iter()
            .take(mem_properties.memory_type_count as usize)
            .enumerate()
            .find(|(i, m)| {
                (mem_reqs.memory_type_bits & (1u32 << i)) != 0
                    && m.property_flags
                        .contains(vk::MemoryPropertyFlags::HOST_VISIBLE)
            })
            .map(|(i, _)| i as u32)
            .ok_or("No suitable memory type found")?;

        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_reqs.size)
            .memory_type_index(mem_type_index);
        let buffer_memory = unsafe {
            device
                .allocate_memory(&alloc_info, None)
                .map_err(|e| format!("Failed to allocate memory: {e}"))?
        };
        unsafe {
            device
                .bind_buffer_memory(buffer, buffer_memory, 0)
                .map_err(|e| format!("Failed to bind buffer memory: {e}"))?;
        }

        let pool_size = vk::DescriptorPoolSize {
            ty: vk::DescriptorType::STORAGE_BUFFER,
            descriptor_count: 1,
        };
        let pool_sizes = [pool_size];
        let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::default()
            .pool_sizes(&pool_sizes)
            .max_sets(1);
        let descriptor_pool = unsafe {
            device
                .create_descriptor_pool(&descriptor_pool_create_info, None)
                .map_err(|e| format!("Failed to create descriptor pool: {e}"))?
        };

        let descriptor_layouts = [descriptor_set_layout];
        let descriptor_alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&descriptor_layouts);
        let descriptor_set = unsafe {
            device
                .allocate_descriptor_sets(&descriptor_alloc_info)
                .map_err(|e| format!("Failed to allocate descriptor sets: {e}"))?
                .remove(0)
        };

        let buffer_info = vk::DescriptorBufferInfo::default()
            .buffer(buffer)
            .offset(0)
            .range(buffer_size);
        let buffer_infos = [buffer_info];
        let write_desc = vk::WriteDescriptorSet::default()
            .dst_set(descriptor_set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&buffer_infos);
        unsafe {
            device.update_descriptor_sets(&[write_desc], &[]);
        }

        let command_pool_create_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let command_pool = unsafe {
            device
                .create_command_pool(&command_pool_create_info, None)
                .map_err(|e| format!("Failed to create command pool: {e}"))?
        };

        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1);
        let command_buffer = unsafe {
            device
                .allocate_command_buffers(&command_buffer_allocate_info)
                .map_err(|e| format!("Failed to allocate command buffers: {e}"))?
                .remove(0)
        };

        Ok(VulkanContext {
            instance,
            device,
            _physical_device: physical_device,
            queue,
            _queue_family_index: queue_family_index,
            pipeline,
            pipeline_layout,
            descriptor_set_layout,
            shader_module,
            command_pool,
            command_buffer,
            buffer,
            buffer_memory,
            _buffer_size: buffer_size,
            descriptor_pool,
            descriptor_set,
        })
    }

    fn pick_compute_device(
        instance: &ash::Instance,
        devices: &[vk::PhysicalDevice],
    ) -> Option<(vk::PhysicalDevice, u32)> {
        let mut discrete: Option<(vk::PhysicalDevice, u32)> = None;
        let mut integrated: Option<(vk::PhysicalDevice, u32)> = None;

        for &device in devices {
            let props = unsafe { instance.get_physical_device_properties(device) };
            let queue_props =
                unsafe { instance.get_physical_device_queue_family_properties(device) };
            for (i, qp) in queue_props.iter().enumerate() {
                if qp.queue_flags.contains(vk::QueueFlags::COMPUTE) {
                    let candidate = (device, i as u32);
                    if props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
                        discrete = Some(candidate);
                    } else if props.device_type != vk::PhysicalDeviceType::CPU {
                        integrated = Some(candidate);
                    }
                    break;
                }
            }
        }

        discrete.or(integrated)
    }

    unsafe fn dispatch(&self) -> Result<(), String> {
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        self.device
            .begin_command_buffer(self.command_buffer, &begin_info)
            .map_err(|e| format!("begin_command_buffer failed: {e:?}"))?;
        self.device.cmd_bind_pipeline(
            self.command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            self.pipeline,
        );
        self.device.cmd_bind_descriptor_sets(
            self.command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            self.pipeline_layout,
            0,
            &[self.descriptor_set],
            &[],
        );
        let groups = (BUF_SIZE as u32).div_ceil(WORKGROUP_SIZE);
        self.device.cmd_dispatch(self.command_buffer, groups, 1, 1);
        self.device
            .end_command_buffer(self.command_buffer)
            .map_err(|e| format!("end_command_buffer failed: {e:?}"))?;

        let cmd_buffers = [self.command_buffer];
        let submit_info = vk::SubmitInfo::default().command_buffers(&cmd_buffers);
        self.device
            .queue_submit(self.queue, &[submit_info], vk::Fence::null())
            .map_err(|e| format!("queue_submit failed: {e:?}"))?;
        // A failure here usually means the GPU hung or the device was lost.
        self.device
            .queue_wait_idle(self.queue)
            .map_err(|e| format!("queue_wait_idle failed (GPU hang or device lost?): {e:?}"))?;
        Ok(())
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.device.destroy_buffer(self.buffer, None);
            self.device.free_memory(self.buffer_memory, None);
            self.device
                .free_command_buffers(self.command_pool, &[self.command_buffer]);
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_pipeline(self.pipeline, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            self.device.destroy_shader_module(self.shader_module, None);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuStressState {
    Idle,
    Running,
    Stopping,
}

pub struct GpuStressManager {
    state: GpuStressState,
    stop_flag: Option<Arc<AtomicBool>>,
    worker_handle: Option<std::thread::JoinHandle<Result<(), String>>>,
    start_time: Option<Instant>,
    duration_seconds: u64,
    last_stop_cause: Option<String>,
    last_elapsed_seconds: f64,
    last_error: Option<String>,
}

impl GpuStressManager {
    pub fn new() -> Self {
        GpuStressManager {
            state: GpuStressState::Idle,
            stop_flag: None,
            worker_handle: None,
            start_time: None,
            duration_seconds: 0,
            last_stop_cause: None,
            last_elapsed_seconds: 0.0,
            last_error: None,
        }
    }

    pub fn state(&mut self) -> GpuStressState {
        if self.state == GpuStressState::Running {
            if let Some(handle) = &self.worker_handle {
                if handle.is_finished() {
                    let elapsed = self
                        .start_time
                        .map(|t| t.elapsed().as_secs_f64())
                        .unwrap_or(0.0);
                    self.last_elapsed_seconds = elapsed.max(0.0);

                    let result = self.worker_handle.take().map(|h| h.join());

                    if let Some(Ok(Err(message))) = result {
                        // The worker bailed out (e.g. device lost, no compute
                        // device, Vulkan init failure). Surface the reason.
                        self.last_error = Some(message);
                        self.last_stop_cause = Some("error".to_string());
                    } else if self.duration_seconds > 0
                        && elapsed >= self.duration_seconds.saturating_sub(1) as f64
                    {
                        self.last_stop_cause = Some("completed".to_string());
                    } else {
                        self.last_stop_cause = Some("exited".to_string());
                    }

                    self.state = GpuStressState::Idle;
                    self.stop_flag = None;
                    self.duration_seconds = 0;
                }
            }
        }
        self.state
    }

    pub fn is_running(&mut self) -> bool {
        self.state() == GpuStressState::Running
    }

    pub fn elapsed_seconds(&self) -> f64 {
        if self.state != GpuStressState::Running {
            return 0.0;
        }
        self.start_time
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0)
    }

    pub fn last_elapsed_seconds(&self) -> f64 {
        self.last_elapsed_seconds
    }

    pub fn last_stop_cause(&self) -> Option<&str> {
        self.last_stop_cause.as_deref()
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn is_available() -> bool {
        let entry = match unsafe { ash::Entry::load() } {
            Ok(entry) => entry,
            Err(_) => return false,
        };

        let app_name = match std::ffi::CString::new("Crucible GPU Stress") {
            Ok(name) => name,
            Err(_) => return false,
        };
        let app_info = vk::ApplicationInfo::default()
            .application_name(app_name.as_c_str())
            .api_version(vk::API_VERSION_1_0);
        let instance_create_info = vk::InstanceCreateInfo::default().application_info(&app_info);

        let instance = match unsafe { entry.create_instance(&instance_create_info, None) } {
            Ok(instance) => instance,
            Err(_) => return false,
        };

        // Same device selection the stress worker uses, so availability can't
        // disagree with whether a test can actually start.
        let available = unsafe { instance.enumerate_physical_devices() }
            .map(|devices| VulkanContext::pick_compute_device(&instance, &devices).is_some())
            .unwrap_or(false);

        unsafe { instance.destroy_instance(None) };
        available
    }

    pub fn start(&mut self, duration_seconds: u64) -> bool {
        if self.state != GpuStressState::Idle {
            return false;
        }

        let stop_flag = Arc::new(AtomicBool::new(false));
        let flag = stop_flag.clone();

        self.start_time = Some(Instant::now());
        self.duration_seconds = duration_seconds;
        self.last_stop_cause = None;
        self.last_error = None;
        self.last_elapsed_seconds = 0.0;

        let handle = std::thread::Builder::new()
            .name("gpu-stress".into())
            .spawn(move || Self::worker_main(flag, duration_seconds));

        match handle {
            Ok(h) => {
                self.stop_flag = Some(stop_flag);
                self.worker_handle = Some(h);
                self.state = GpuStressState::Running;
                true
            }
            Err(_) => {
                self.start_time = None;
                false
            }
        }
    }

    fn worker_main(stop_flag: Arc<AtomicBool>, duration_seconds: u64) -> Result<(), String> {
        let entry =
            unsafe { ash::Entry::load() }.map_err(|e| format!("Vulkan loader unavailable: {e}"))?;

        let ctx = VulkanContext::create(&entry)?;

        let start = Instant::now();
        let duration = std::time::Duration::from_secs(duration_seconds);

        // Run until the requested duration has actually elapsed (wall clock),
        // not a fixed number of dispatches: GPUs differ wildly in how fast a
        // dispatch completes, so a dispatch count makes short tests end early
        // on fast hardware and long tests run forever on slow hardware.
        loop {
            if stop_flag.load(Ordering::Relaxed) {
                break;
            }
            if duration_seconds > 0 && start.elapsed() >= duration {
                break;
            }
            unsafe { ctx.dispatch()? };
        }

        Ok(())
    }

    pub fn stop(&mut self, cause: &str) {
        if self.state != GpuStressState::Running {
            return;
        }

        self.last_elapsed_seconds = self
            .start_time
            .map(|t| t.elapsed().as_secs_f64().max(0.0))
            .unwrap_or(0.0);
        self.last_stop_cause = Some(cause.to_string());
        self.state = GpuStressState::Stopping;

        if let Some(ref flag) = self.stop_flag {
            flag.store(true, Ordering::Relaxed);
        }

        if let Some(handle) = self.worker_handle.take() {
            let deadline = Instant::now() + std::time::Duration::from_secs(3);
            while Instant::now() < deadline {
                if handle.is_finished() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            let _ = handle.join();
        }

        self.stop_flag = None;
        self.state = GpuStressState::Idle;
        self.duration_seconds = 0;
    }

    pub fn kill(&mut self) {
        if self.state != GpuStressState::Running {
            return;
        }

        self.last_elapsed_seconds = self
            .start_time
            .map(|t| t.elapsed().as_secs_f64().max(0.0))
            .unwrap_or(0.0);
        self.last_stop_cause = Some("killed".to_string());

        if let Some(ref flag) = self.stop_flag {
            flag.store(true, Ordering::Relaxed);
        }

        if let Some(handle) = self.worker_handle.take() {
            let deadline = Instant::now() + std::time::Duration::from_secs(2);
            while Instant::now() < deadline {
                if handle.is_finished() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            let _ = handle.join();
        }

        self.stop_flag = None;
        self.state = GpuStressState::Idle;
        self.duration_seconds = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_stress_manager_new() {
        let mut manager = GpuStressManager::new();
        assert_eq!(manager.state(), GpuStressState::Idle);
        assert!(!manager.is_running());
        assert_eq!(manager.elapsed_seconds(), 0.0);
        assert_eq!(manager.last_elapsed_seconds(), 0.0);
        assert_eq!(manager.last_stop_cause(), None);
    }

    #[test]
    fn test_compile_shader() {
        let spv = compile_shader();
        assert!(spv.is_ok(), "Shader compilation failed: {:?}", spv.err());
        let spv = spv.unwrap();
        assert!(!spv.is_empty(), "SPIR-V output is empty");
        assert_eq!(spv[0], 0x07230203, "Invalid SPIR-V magic number");
        let version_major = (spv[1] >> 16) as u8;
        assert!(version_major >= 1, "Expected SPIR-V version >= 1.0");
    }

    #[test]
    fn test_gpu_stress_manager_start_stop() {
        let mut manager = GpuStressManager::new();
        if !GpuStressManager::is_available() {
            println!("Vulkan not available, skipping lifecycle test");
            return;
        }
        assert!(manager.start(2));
        assert_eq!(manager.state(), GpuStressState::Running);
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(manager.elapsed_seconds() > 0.0);
        manager.stop("manual_stop");
        assert_eq!(manager.state(), GpuStressState::Idle);
        assert_eq!(manager.last_stop_cause(), Some("manual_stop"));
    }

    #[test]
    fn test_gpu_stress_runs_for_requested_duration() {
        let mut manager = GpuStressManager::new();
        if !GpuStressManager::is_available() {
            println!("Vulkan not available, skipping duration test");
            return;
        }

        // A short test must actually run to completion rather than ending after
        // a fixed number of dispatches (which finishes far too early on fast
        // GPUs and looks like a failure to the user).
        let duration = 3;
        assert!(manager.start(duration));

        // Wait well past the requested duration for the worker to finish.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(duration + 30);
        while manager.is_running() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        assert!(!manager.is_running(), "test did not finish in time");
        assert_eq!(manager.last_stop_cause(), Some("completed"));
        assert!(
            manager.last_elapsed_seconds() >= (duration - 1) as f64,
            "test ended after only {:.1}s (expected ~{duration}s)",
            manager.last_elapsed_seconds()
        );
    }
}
