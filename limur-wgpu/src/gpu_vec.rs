use crate::GraphicsContext;

pub(crate) struct GpuVec<T> {
    data: Vec<T>,
    gpu_buffer: Option<wgpu::Buffer>,
    gpu_buffer_len: usize,
    gpu_buffer_capacity: usize,
    buffer_resized: bool,
    dirty: bool,
    usage: wgpu::BufferUsages,
}

impl<T: Default + Clone + bytemuck::Pod + bytemuck::Zeroable> GpuVec<T> {
    pub(crate) fn new(capacity: usize, usage: wgpu::BufferUsages) -> Self {
        Self {
            data: vec![T::default(); capacity],
            gpu_buffer: None,
            gpu_buffer_len: 0,
            gpu_buffer_capacity: 0,
            buffer_resized: false,
            dirty: false,
            usage,
        }
    }

    pub(crate) fn gpu_buffer(&self) -> &wgpu::Buffer {
        self.gpu_buffer
            .as_ref()
            .expect("Buffer has not been created")
    }

    pub(crate) fn data(&self) -> &[T] {
        &self.data[0..self.gpu_buffer_len]
    }

    pub(crate) fn take_buffer_resized(&mut self) -> bool {
        let was = self.buffer_resized;
        self.buffer_resized = false;

        was
    }

    pub(crate) fn len(&self) -> usize {
        self.gpu_buffer_len
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.gpu_buffer_len == 0
    }

    pub(crate) fn push(&mut self, data: T) {
        let index = self.gpu_buffer_len;
        self.gpu_buffer_len += 1;

        if index >= self.data.len() {
            self.data.resize(self.data.len() * 2, T::default());
        }

        self.data[index] = data;
        self.dirty = true;
    }

    pub(crate) fn update(&mut self, index: usize, data: T) {
        // debug_assert!(self.ids.contains(&id), "Slot has not been allocated");
        if index >= self.gpu_buffer_len {
            panic!(
                "Out of bound, index: {}, len: {}",
                index, self.gpu_buffer_len
            );
        }

        self.data[index] = data;
        self.dirty = true;
    }

    pub(crate) fn clear(&mut self) {
        self.gpu_buffer_len = 0;
    }

    /// Ensures the GPU buffer exists and has enough capacity.
    ///
    /// On resize, flushes current data to the old buffer first so any draw
    /// commands already recorded against it see up-to-date values. wgpu's
    /// write_buffer staging guarantee ensures the write lands before those
    /// commands execute. The old buffer handle is then dropped — wgpu's
    /// internal refcount keeps the GPU resource alive until commands finish.
    pub(crate) fn ensure_capacity(&mut self, context: &GraphicsContext<'_>) {
        if self.gpu_buffer.is_none() || self.gpu_buffer_len > self.gpu_buffer_capacity {
            // Write current data to the old buffer before replacing it.
            if let Some(old_buffer) = &self.gpu_buffer {
                context.queue.write_buffer(
                    old_buffer,
                    0,
                    bytemuck::cast_slice(&self.data[0..self.gpu_buffer_capacity]),
                );
            }

            self.gpu_buffer = Some(context.device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: (self.data.len() * std::mem::size_of::<T>()) as wgpu::BufferAddress,
                usage: self.usage,
                mapped_at_creation: false,
            }));
            self.gpu_buffer_capacity = self.data.len();
            self.buffer_resized = true;
            self.dirty = true;
        }
    }

    /// Ensures the buffer exists and uploads dirty data to the GPU.
    pub(crate) fn flush(&mut self, context: &GraphicsContext<'_>) {
        self.ensure_capacity(context);

        if !self.dirty {
            return;
        }

        context.queue.write_buffer(
            self.gpu_buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(&self.data[0..self.gpu_buffer_len]),
        );

        self.dirty = false;
    }
}
