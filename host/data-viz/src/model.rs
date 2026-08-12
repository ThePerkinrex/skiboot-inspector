use std::ops::Range;

use cgmath::{Matrix4, Quaternion, Vector3};
use wgpu::{BindGroup, BindGroupLayoutDescriptor, Buffer};

pub trait Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

impl Vertex for ModelVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub struct ModelTransform {
    pub translate: cgmath::Vector3<f32>,
    pub rotate: cgmath::Quaternion<f32>,
    pub scale: cgmath::Vector3<f32>,
}

impl ModelTransform {
    pub fn build_transform_matrix(&self) -> cgmath::Matrix4<f32> {
        Matrix4::from(self.rotate)
            * Matrix4::from_nonuniform_scale(self.scale.x, self.scale.y, self.scale.z)
            * Matrix4::from_translation(self.translate)
    }

    pub fn build_normal_matrix(&self) -> cgmath::Matrix3<f32> {
        cgmath::Matrix3::from(self.rotate).into()
    }
}

// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelTransformUniform {
    transform: [[f32; 4]; 4],
    normal: [[f32; 4]; 3], // pad each column to vec4 for WGSL alignment
}

impl ModelTransformUniform {
    pub fn new(transform: &ModelTransform) -> Self {
        let normal3: cgmath::Matrix3<f32> = transform.build_normal_matrix();
        let normal = [
            [normal3.x.x, normal3.x.y, normal3.x.z, 0.0],
            [normal3.y.x, normal3.y.y, normal3.y.z, 0.0],
            [normal3.z.x, normal3.z.y, normal3.z.z, 0.0],
        ];
        Self {
            transform: transform.build_transform_matrix().into(),
            normal,
        }
    }

    /// returns whether the view has updated
    pub fn update_transform(&mut self, transform: &ModelTransform) -> bool {
        let normal3: cgmath::Matrix3<f32> = transform.build_normal_matrix();
        let new_normal = [
            [normal3.x.x, normal3.x.y, normal3.x.z, 0.0],
            [normal3.y.x, normal3.y.y, normal3.y.z, 0.0],
            [normal3.z.x, normal3.z.y, normal3.z.z, 0.0],
        ];
        let new_transform = transform.build_transform_matrix().into();

        if self.transform == new_transform && self.normal == new_normal {
            return false;
        }
        self.transform = new_transform;
        self.normal = new_normal;
        true
    }
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    pub transform: ModelTransform,
    pub uniform: ModelTransformUniform,
    pub bind_group: BindGroup,
    pub buffer: Buffer,
}

impl Model {
    pub const fn bind_group_layout_desc() -> BindGroupLayoutDescriptor<'static> {
        wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("model_bind_group_layout"),
        }
    }
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

// model.rs
pub trait DrawModel<'a> {
    fn draw_mesh(
        &mut self,
        mesh: &'a Mesh,
        camera_bind_group: &'a wgpu::BindGroup,
        model_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        model_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_model(&mut self, model: &'a Model, camera_bind_group: &'a wgpu::BindGroup);
    fn draw_model_instanced(
        &mut self,
        model: &'a Model,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
    );
}
impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        camera_bind_group: &'a wgpu::BindGroup,
        model_bind_group: &'a wgpu::BindGroup,
    ) {
        self.draw_mesh_instanced(mesh, 0..1, camera_bind_group, model_bind_group);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        model_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, camera_bind_group, &[]);
        self.set_bind_group(1, model_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_model(&mut self, model: &'b Model, camera_bind_group: &'b wgpu::BindGroup) {
        self.draw_model_instanced(model, 0..1, camera_bind_group);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            // let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(
                mesh,
                instances.clone(),
                camera_bind_group,
                &model.bind_group,
            );
        }
    }
}
