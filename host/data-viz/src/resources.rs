pub mod static_loader;

pub trait Loader {
    type Error: std::error::Error + Send + Sync;
    type Reader<'a>: tokio::io::AsyncRead + Unpin
    where
        Self: 'a;
    async fn get_file<P: AsRef<Path>>(&self, path: P) -> Result<Self::Reader<'_>, Self::Error>;
}

// pub async fn load_texture(
//     file_name: &str,
//     device: &wgpu::Device,
//     queue: &wgpu::Queue,
// ) -> anyhow::Result<crate::texture::Texture> {
//     // let data = load_binary(file_name).await?;
//     // texture::Texture::from_bytes(device, queue, &data, file_name)
// }

use std::{io::Cursor, path::Path};

use cgmath::{Matrix4, One, Quaternion, Vector3, Zero};
use tokio::io::BufReader;
use tracing::info;
use wgpu::util::DeviceExt;

use crate::model;

pub async fn load_model<P, L>(
    file_name: P,
    loader: &L,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    scale: Vector3<f32>,
    translate: Vector3<f32>,
    rotate: Quaternion<f32>,
    // layout: &wgpu::BindGroupLayout,
) -> anyhow::Result<model::Model>
where
    P: AsRef<Path>,
    L: Loader,
    L::Error: 'static,
{
    let data = loader.get_file(&file_name).await?;
    let mut obj_reader = BufReader::new(data);

    let (models, obj_materials) = tobj::tokio::load_obj_buf(
        &mut obj_reader,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |p| async move {
            let mat_text = loader.get_file(&p).await.unwrap();
            tobj::tokio::load_mtl_buf(&mut BufReader::new(mat_text)).await
        },
    )
    .await?;

    // let mut materials = Vec::new();
    for m in obj_materials? {
        // let diffuse_texture = load_texture(&m.diffuse_texture, device, queue).await?;
        // let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     layout,
        //     entries: &[
        //         wgpu::BindGroupEntry {
        //             binding: 0,
        //             resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
        //         },
        //         wgpu::BindGroupEntry {
        //             binding: 1,
        //             resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
        //         },
        //     ],
        //     label: None,
        // });

        // materials.push(model::Material {
        //     name: m.name,
        //     diffuse_texture,
        //     bind_group,
        // })
    }
    let mut maxmin = (
        (-f32::INFINITY, f32::INFINITY),
        (-f32::INFINITY, f32::INFINITY),
        (-f32::INFINITY, f32::INFINITY),
    );

    let meshes = models
        .into_iter()
        .map(|m| {
            let vertices = (0..m.mesh.positions.len() / 3)
                .map(|i| {
                    const fn update_maxmin(a: (f32, f32), b: f32) -> (f32, f32) {
                        (a.0.max(b), a.1.min(b))
                    }

                    maxmin = (
                        update_maxmin(maxmin.0, m.mesh.positions[i * 3]),
                        update_maxmin(maxmin.1, m.mesh.positions[i * 3 + 1]),
                        update_maxmin(maxmin.2, m.mesh.positions[i * 3 + 2]),
                    );
                    if m.mesh.normals.is_empty() {
                        model::ModelVertex {
                            position: [
                                m.mesh.positions[i * 3],
                                m.mesh.positions[i * 3 + 1],
                                m.mesh.positions[i * 3 + 2],
                            ],
                            // tex_coords: [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]],
                            normal: [0.0, 0.0, 0.0],
                        }
                    } else {
                        model::ModelVertex {
                            position: [
                                m.mesh.positions[i * 3],
                                m.mesh.positions[i * 3 + 1],
                                m.mesh.positions[i * 3 + 2],
                            ],
                            // tex_coords: [m.mesh.texcoords[i * 2], 1.0 - m.mesh.texcoords[i * 2 + 1]],
                            normal: [
                                m.mesh.normals[i * 3],
                                m.mesh.normals[i * 3 + 1],
                                m.mesh.normals[i * 3 + 2],
                            ],
                        }
                    }
                })
                .collect::<Vec<_>>();

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Vertex Buffer", file_name.as_ref().display())),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Index Buffer", file_name.as_ref().display())),
                contents: bytemuck::cast_slice(&m.mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            model::Mesh {
                name: file_name.as_ref().display().to_string(),
                vertex_buffer,
                index_buffer,
                num_elements: m.mesh.indices.len() as u32,
                material: m.mesh.material_id.unwrap_or(0),
            }
        })
        .collect::<Vec<_>>();
    info!("model {} maxmins: {maxmin:?}", file_name.as_ref().display());

    let transform = Matrix4::from_translation(translate)
        * Matrix4::from(rotate)
        * Matrix4::from_nonuniform_scale(scale.x, scale.y, scale.z);

    Ok(model::Model { meshes, transform })
}
