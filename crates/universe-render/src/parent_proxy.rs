//! Coarse, non-contact parent cover while the one detailed stack is preparing.
//! This reference-radius fallback is not terrain/collision or appearance proof.

use crate::patches::{GroundSurface, PatchTasks, PatchTree};
use crate::resource_diagnostic::{AssetKey, Descriptor, Role};
use bevy::mesh::primitives::{SphereKind, SphereMeshBuilder};
use bevy::prelude::*;
use universe_core::moon_catalog::PlanetIdentity;
use universe_core::surface_observation::SurfaceIdentity;

#[derive(Component)]
pub(crate) struct ParentProxy(pub(crate) PlanetIdentity);

/// Coarse cover takes the first shared application unit; no extra allowance.
/// Retirement uses the actual published detailed owner, never work interest.
#[allow(clippy::too_many_arguments, reason = "Bevy system engine inputs")]
pub(crate) fn sync_parent_proxy(
    mut commands: Commands,
    universe: Res<crate::Universe>,
    origin: Res<crate::style::RenderOrigin>,
    tree: Res<PatchTree>,
    ground: Res<GroundSurface>,
    atlas: Res<crate::patch_grid::GridAtlas>,
    store: Res<crate::settings_ui::SettingsStore>,
    effective: Option<Res<crate::effective::EffectiveGraphics>>,
    mut tasks: ResMut<PatchTasks>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    owned: Query<(
        Entity,
        &ParentProxy,
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    diagnostics: Option<Res<crate::LoadingDiagnostics>>,
) {
    tasks.apply_started = Some(std::time::Instant::now());
    tasks.apply_units = 0;
    tasks.apply_uploads = 0;
    tasks.apply_work_share = effective
        .as_ref()
        .map_or(0.10, |e| e.0.terrain_work_share());
    tasks.apply_budget_ms = tasks.apply_work_share * store.0.target_fps.interval_ms();
    let host = if universe.level().get() == 10 {
        tree.recipe
            .map(|key| key.owner().host())
            .filter(|host| {
                universe.root == host.seed() && universe.path.matches_indices(host.chain())
            })
            .or_else(|| PlanetIdentity::from_open_cell(&universe).ok())
    } else {
        None
    };
    let detailed = ground
        .0
        .as_ref()
        .and_then(|drawn| drawn.owner_frame())
        .map(|frame| frame.owner());
    let cap = if tasks.cap_override == 0 {
        store.0.ground_memory.bytes()
    } else {
        tasks.cap_override
    };
    let rebuilding = tree.tree.is_none() || atlas.pages != crate::patch_grid::pages_for(cap);
    let needed = host.filter(|host| rebuilding || detailed != Some(SurfaceIdentity::Planet(*host)));
    let witness = diagnostics.as_ref().map(|d| d.context(&universe));
    let mut held = false;
    for (entity, proxy, mesh, material) in &owned {
        if needed == Some(proxy.0) {
            held = true;
            continue;
        }
        commands.entity(entity).despawn();
        let removed_mesh = meshes.remove(&mesh.0).is_some();
        let removed_material = materials.remove(&material.0).is_some();
        if let Some(witness) = &witness {
            witness.removed(AssetKey::Mesh(mesh.id()), removed_mesh);
            witness.removed(AssetKey::Standard(material.id()), removed_material);
        }
    }
    let Some(host) = needed.filter(|_| !held) else {
        return;
    };
    if !crate::surface_work::may_prepare(&tasks) {
        return;
    }
    let unit = std::time::Instant::now();
    let Some(frame) = universe_core::nav::planet_frame(&universe) else {
        return;
    };
    let Ok(recipe) =
        universe_core::surface_recipe::SurfaceRecipe::for_owner(SurfaceIdentity::Planet(host))
    else {
        return;
    };
    let kind = recipe.kind();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "E-CAST: positive finite radius and palette narrowed for rendering"
    )]
    let (radius, colour) = {
        let [r, g, b] = kind.palette().low;
        (
            (frame.radius_m() / universe_core::planet_frame::metres_per_cell_l10()) as f32,
            Color::srgb(r as f32, g as f32, b as f32),
        )
    };
    // A bounded emergency cover, not the final parent appearance (T5).
    let mesh = SphereMeshBuilder::new(radius, SphereKind::Ico { subdivisions: 3 }).build();
    let descriptor = witness.as_ref().map(|_| Descriptor::mesh(&mesh));
    let mesh = meshes.add(mesh);
    let material = materials.add(StandardMaterial {
        base_color: colour,
        perceptual_roughness: 1.0,
        reflectance: 0.0,
        ..default()
    });
    if let Some(witness) = &witness {
        if let Some(descriptor) = descriptor {
            witness.created(AssetKey::Mesh(mesh.id()), Role::ParentProxy, descriptor);
        }
        witness.created(
            AssetKey::Standard(material.id()),
            Role::ParentMaterial,
            Descriptor::Material,
        );
    }
    commands.spawn((
        ParentProxy(host),
        Name::new("planet-coarse-cover"),
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(origin.at([0.0; 3])).with_scale(Vec3::splat(origin.len(1.0))),
        crate::style::Anchored([0.0; 3]),
        crate::style::CellSized,
    ));
    crate::surface_work::record_proxy(&mut tasks, unit, witness.as_ref());
}

/// Runs after detailed publication and command application, so coarse and fine
/// cannot remain simultaneous opaque copies at the render extraction boundary.
pub(crate) fn retire_parent_proxy(
    mut commands: Commands,
    ground: Res<GroundSurface>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    owned: Query<(
        Entity,
        &ParentProxy,
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    universe: Res<crate::Universe>,
    diagnostics: Option<Res<crate::LoadingDiagnostics>>,
) {
    let detailed = ground
        .0
        .as_ref()
        .and_then(|s| s.owner_frame())
        .map(|s| s.owner());
    for (entity, proxy, mesh, material) in &owned {
        if detailed != Some(SurfaceIdentity::Planet(proxy.0)) {
            continue;
        }
        commands.entity(entity).despawn();
        let removed_mesh = meshes.remove(&mesh.0).is_some();
        let removed_material = materials.remove(&material.0).is_some();
        if let Some(diagnostics) = &diagnostics {
            let witness = diagnostics.context(&universe);
            witness.removed(AssetKey::Mesh(mesh.id()), removed_mesh);
            witness.removed(AssetKey::Standard(material.id()), removed_material);
        }
    }
}

#[cfg(test)]
#[path = "parent_proxy_tests.rs"]
mod tests;
