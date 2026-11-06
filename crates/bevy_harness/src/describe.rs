//! How an entity reads in a failure: by its name if it has one, and what
//! it's made of.

use bevy::ecs::entity::Entity;
use bevy::ecs::name::Name;
use bevy::ecs::world::World;
use bevy::utils::DebugName;

/// `T`'s name, short: `Health`, not its path.
pub(crate) fn short<T>() -> String {
    DebugName::type_name::<T>().shortname().to_string()
}

/// `entity` as a failure names it: `"player" (7v1)`, or its id alone.
pub(crate) fn label(world: &World, entity: Entity) -> String {
    match world.get::<Name>(entity) {
        Some(name) => format!("{:?} ({entity})", name.as_str()),
        None => entity.to_string(),
    }
}

/// `entity`'s components, by their short names, sorted; or that it isn't
/// there.
pub(crate) fn components(world: &World, entity: Entity) -> String {
    let Ok(infos) = world.inspect_entity(entity) else {
        return "it isn't spawned".to_owned();
    };
    let mut names: Vec<String> = infos
        .map(|(_, info)| info.name().shortname().to_string())
        .collect();
    names.sort();
    format!("it has [{}]", names.join(", "))
}
