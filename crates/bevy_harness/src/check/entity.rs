//! Asserting on one entity, or asking about it.

use core::fmt::Debug;

use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::world::World;

use crate::describe;

/// One entity, to assert on or ask about: each failure names it and says
/// what it has.
///
/// The questions ([`has`](Self::has), [`try_get`](Self::try_get),
/// [`is_spawned`](Self::is_spawned)) never fail, for a condition
/// ([`until`](crate::TestApp::until)) or an assertion of a test's own.
pub struct EntityCheck<'w> {
    world: &'w World,
    entity: Entity,
}

impl<'w> EntityCheck<'w> {
    pub(crate) fn new(world: &'w World, entity: Entity) -> Self {
        Self { world, entity }
    }

    /// Whether it's spawned: not despawned, nor never spawned.
    #[must_use]
    pub fn is_spawned(&self) -> bool {
        self.world.get_entity(self.entity).is_ok()
    }

    /// Whether it has a `C`; not, if it isn't spawned.
    #[must_use]
    pub fn has<C: Component>(&self) -> bool {
        self.world.get::<C>(self.entity).is_some()
    }

    /// Its `C`, if it has one.
    #[must_use]
    pub fn try_get<C: Component>(&self) -> Option<&'w C> {
        self.world.get::<C>(self.entity)
    }

    /// Its `C`.
    ///
    /// # Panics
    ///
    /// If it has no `C`, naming what it has instead.
    #[track_caller]
    #[must_use]
    pub fn get<C: Component>(&self) -> &'w C {
        self.try_get::<C>().unwrap_or_else(|| {
            panic!(
                "{} has no {}: {}",
                describe::label(self.world, self.entity),
                describe::short::<C>(),
                describe::components(self.world, self.entity)
            )
        })
    }

    /// Asserts it has a `C`.
    ///
    /// # Panics
    ///
    /// If it has no `C`, naming what it has instead.
    #[track_caller]
    #[expect(
        clippy::must_use_candidate,
        reason = "it's returned to chain on: an assertion alone is its own use"
    )]
    pub fn assert_has<C: Component>(&self) -> &Self {
        let _ = self.get::<C>();
        self
    }

    /// Asserts it has no `C`.
    ///
    /// # Panics
    ///
    /// If it has a `C`, naming what it has.
    #[track_caller]
    #[expect(
        clippy::must_use_candidate,
        reason = "it's returned to chain on: an assertion alone is its own use"
    )]
    pub fn assert_lacks<C: Component>(&self) -> &Self {
        assert!(
            !self.has::<C>(),
            "{} has a {}: {}",
            describe::label(self.world, self.entity),
            describe::short::<C>(),
            describe::components(self.world, self.entity)
        );
        self
    }

    /// Asserts its `C` is as `holds` says.
    ///
    /// # Panics
    ///
    /// If it has no `C`, or `holds` says its `C` isn't as expected, printing
    /// the `C` it has.
    #[track_caller]
    pub fn assert<C: Component + Debug>(&self, holds: impl FnOnce(&C) -> bool) -> &Self {
        let found = self.get::<C>();
        assert!(
            holds(found),
            "{}'s {} isn't as expected: {found:#?}",
            describe::label(self.world, self.entity),
            describe::short::<C>()
        );
        self
    }

    /// Asserts its `C` is `expected`.
    ///
    /// # Panics
    ///
    /// If it has no `C`, or its `C` isn't `expected`.
    #[track_caller]
    pub fn assert_eq<C: Component + Debug + PartialEq>(&self, expected: C) -> &Self {
        let found = self.get::<C>();
        assert_eq!(
            found,
            &expected,
            "{}'s {}",
            describe::label(self.world, self.entity),
            describe::short::<C>()
        );
        self
    }

    /// Asserts it's gone: despawned.
    ///
    /// # Panics
    ///
    /// If it's still there, naming what it has.
    #[track_caller]
    pub fn assert_despawned(&self) {
        assert!(
            !self.is_spawned(),
            "{} is still there: {}",
            describe::label(self.world, self.entity),
            describe::components(self.world, self.entity)
        );
    }
}
