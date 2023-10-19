use std::fs::read_to_string;

use wgpu::{
    util::DeviceExt, Adapter, AdapterInfo, BindGroupEntry, Buffer, Device, Instance, Queue,
    ShaderModule,
};

// macro_rules! all_files {
// 	($($file:expr),*) => {
// 		{String::new()$(+include_str!($file)+"\n")*}
// 	};
// }

pub struct Bindings {
    input_output: Vec<u32>,
    shared_memory: Vec<u32>,
    global_memory: Vec<u32>,
    output_vec: Vec<u32>,
}

impl Bindings {
    pub fn initialize_one(input_output: Vec<u32>) -> Self {
        Bindings {
            input_output,
            shared_memory: <_>::default(),
            global_memory: <_>::default(),
            output_vec: <_>::default(),
        }
    }

    pub fn initialize_two(input_output: Vec<u32>, shared_memory: Vec<u32>) -> Self {
        Bindings {
            input_output,
            shared_memory,
            global_memory: <_>::default(),
            output_vec: <_>::default(),
        }
    }

    pub fn initialize_three(
        input_output: Vec<u32>,
        shared_memory: Vec<u32>,
        global_memory: Vec<u32>,
    ) -> Self {
        Bindings {
            input_output,
            shared_memory,
            global_memory,
            output_vec: <_>::default(),
        }
    }

    pub fn initialize_four(
        input_vec: Vec<u32>,
        start: Vec<u32>,
        end: Vec<u32>,
        output_vec: Vec<u32>,
    ) -> Self {
        Bindings {
            input_output: input_vec,
            shared_memory: start,
            global_memory: end,
            output_vec,
        }
    }
}

pub struct BufCoder {
    staging_buffer: Buffer,
}

impl BufCoder {
    pub fn initialize(
        gpu: &GpuConsts,
        numbers: &mut Bindings,
        func_name: &str,
        binding_number: u32,
    ) -> BufCoder {
        // Gets the size in bytes of the buffer.
        let slice_size = numbers.input_output.len() * std::mem::size_of::<u32>();
        let size = slice_size as wgpu::BufferAddress;

        // Instantiates buffer without data.
        // `usage` of buffer specifies how it can be used:
        //   `BufferUsages::MAP_READ` allows it to be read (outside the shader).
        //   `BufferUsages::COPY_DST` allows it to be the destination of the copy.
        let staging_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Instantiates buffer with data (`numbers`).
        // Usage allowing the buffer to be:
        //   A storage buffer (can be bound within a bind group and thus available to a shader).
        //   The destination of a copy.
        //   The source of a copy.
        let storage_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Storage Buffer"),
                contents: bytemuck::cast_slice(&numbers.input_output),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
            });

        // A bind group defines how buffers are accessed by shaders.
        // It is to WebGPU what a descriptor set is to Vulkan.
        // `binding` here refers to the `binding` of a buffer in the shader (`layout(set = 0, binding = 0) buffer`).

        // A pipeline specifies the operation of a shader

        // Instantiates the pipeline.
        let compute_pipeline =
            gpu.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: None,
                    layout: None,
                    module: &gpu.cs_module,
                    entry_point: func_name,
                });

        // Instantiates the bind group, once again specifying the binding of buffers.
        let bind_group_layout = compute_pipeline.get_bind_group_layout(0);

        let mut new_binding_entries: Vec<BindGroupEntry> = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: storage_buffer.as_entire_binding(),
        }];

        let storage_buffer2 = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Shared Memory Buffer"),
                contents: bytemuck::cast_slice(&numbers.shared_memory),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let storage_buffer3 = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Global Memory Buffer"),
                contents: bytemuck::cast_slice(&numbers.global_memory),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let storage_buffer4 = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Global Memory Buffer"),
                contents: bytemuck::cast_slice(&numbers.output_vec),
                usage: wgpu::BufferUsages::STORAGE,
            });

        if binding_number > 1 {
            new_binding_entries.push(wgpu::BindGroupEntry {
                binding: 1,
                resource: storage_buffer2.as_entire_binding(),
            });

            if binding_number > 2 {
                new_binding_entries.push(wgpu::BindGroupEntry {
                    binding: 2,
                    resource: storage_buffer3.as_entire_binding(),
                });

                if binding_number > 3 {
                    new_binding_entries.push(wgpu::BindGroupEntry {
                        binding: 3,
                        resource: storage_buffer4.as_entire_binding(),
                    });
                }
            }
        }

        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &new_binding_entries,
        });

        // A command encoder executes one or many pipelines.
        // It is to WebGPU what a command buffer is to Vulkan.
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut cpass =
                encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
            cpass.set_pipeline(&compute_pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.insert_debug_marker("compute collatz iterations");
            cpass.dispatch_workgroups(256, 1, 1); // Number of cells to run, the (x,y,z) size of item being processed
        }
        // Sets adds copy operation to command encoder.
        // Will copy data from storage buffer on GPU to staging buffer on CPU.
        encoder.copy_buffer_to_buffer(&storage_buffer, 0, &staging_buffer, 0, size);

        // Submits command encoder for processing
        gpu.queue.submit(Some(encoder.finish()));

        BufCoder { staging_buffer }
    }
}

pub struct GpuConsts {
    _instance: Instance,
    _adapter: Adapter,
    device: Device,
    queue: Queue,
    _info: AdapterInfo,
    cs_module: ShaderModule,
}

impl GpuConsts {
    pub async fn initialaze(filename: &str) -> Result<GpuConsts, String> {
        // Instantiates instance of WebGPU
        let instance = wgpu::Instance::default();

        // `request_adapter` instantiates the general connection to the GPU
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .ok_or_else(|| "adapter error")?;

        // `request_device` instantiates the feature specific connection to the GPU, defining some parameters,
        //  `features` being the available features.
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::downlevel_defaults(),
                },
                None,
            )
            .await
            .unwrap();

        let info = adapter.get_info();

        if info.vendor == 0x10005 {
            return Err("info error".to_string());
        }

        let cs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(read_to_string(filename).unwrap().into()),
        });

        Ok(GpuConsts {
            _instance: instance,
            _adapter: adapter,
            device,
            queue,
            _info: info,
            cs_module,
        })
    }

    pub async fn run(&self, bufcoder: &BufCoder) -> Option<Vec<u32>> {
        // Note that we're not calling `.await` here.
        let buffer_slice = bufcoder.staging_buffer.slice(..);
        // Sets the buffer up for mapping, sending over the result of the mapping back to us when it is finished.
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());

        // Poll the device in a blocking manner so that our future resolves.
        // In an actual application, `device.poll(...)` should
        // be called in an event loop or on another thread.
        self.device.poll(wgpu::Maintain::Wait);

        // Awaits until `buffer_future` can be read from
        if let Some(Ok(())) = receiver.receive().await {
            // Gets contents of buffer
            let data = buffer_slice.get_mapped_range();
            // Since contents are got in bytes, this converts these bytes back to u32
            let result = bytemuck::cast_slice(&data).to_vec();

            // With the current interface, we have to make sure all mapped views are
            // dropped before we unmap the buffer.
            drop(data);
            bufcoder.staging_buffer.unmap(); // Unmaps buffer from memory
                                             // If you are familiar with C++ these 2 lines can be thought of similarly to:
                                             //   delete myPointer;
                                             //   myPointer = NULL;
                                             // It effectively frees the memory

            // Returns data from buffer
            Some(result)
        } else {
            panic!("failed to run compute on gpu!")
        }
    }
}

pub fn add_two_vec(a: &[u32], b: &[u32], cap: usize) -> Vec<u32> {
    let mut res = Vec::with_capacity(cap);

    for i in 0..cap {
        res.push(a[i] + b[i]);
    }

    return res;
}

pub fn batch_add_two_vec(a: &[u32], b: &[u32], cap: usize, batch: u32) {
    for _ in 0..batch {
        add_two_vec(a, b, cap);
    }
}

pub fn sum_vec(a: &[u32], cap: usize) -> u32 {
    let mut res = 0;

    for i in 0..cap {
        res += a[i];
    }

    return res;
}

pub fn batch_sum_vec(a: &[u32], cap: usize, batch: u32) {
    for _ in 0..batch {
        sum_vec(a, cap);
    }
}

pub fn optimized_sum_vec(arr: &[u32], start: usize, end: usize) -> u32 {
    if end == start {
        return arr[end];
    }
    if end - start == 1 {
        return arr[start] + arr[end];
    } else {
        return optimized_sum_vec(arr, start, (end - start) / 2 + start)
            + optimized_sum_vec(arr, (end - start) / 2 + start + 1, end);
    }
}

pub fn batch_optimized_sum_vec(arr: &[u32], start: usize, end: usize, batch: u32) {
    for _ in 0..batch {
        optimized_sum_vec(arr, start, end);
    }
}
