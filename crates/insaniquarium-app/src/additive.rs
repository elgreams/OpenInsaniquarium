//! SexyApp draw mode 1 ("additive", `Graphics::SetDrawMode(1)`): the
//! source colour, times its tint and alpha, is added to the destination.
//! An extension of Bevy's `ColorMaterial` that only swaps the blend state;
//! the cel, tint and mirroring come from the base material.

use bevy::asset::Asset;
use bevy::material::descriptor::RenderPipelineDescriptor;
use bevy::material::specialize::SpecializedMeshPipelineError;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState,
};
use bevy::sprite_render::{
    AlphaMode2d, ExtendedMaterial2d, Material2dKey, Material2dPipeline, MaterialExtension2d,
};

/// `ColorMaterial` drawn additively.
pub type AdditiveMaterial = ExtendedMaterial2d<ColorMaterial, Additive>;

#[derive(Asset, AsBindGroup, Reflect, Clone, Default)]
pub struct Additive {}

impl MaterialExtension2d for Additive {
    fn alpha_mode(&self) -> Option<AlphaMode2d> {
        Some(AlphaMode2d::Blend)
    }

    fn specialize(
        _pipeline: &Material2dPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(f) = descriptor.fragment.as_mut() {
            for t in f.targets.iter_mut().flatten() {
                t.blend = Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::Zero,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                });
            }
        }
        Ok(())
    }
}
