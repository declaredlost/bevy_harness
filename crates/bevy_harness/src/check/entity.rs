//! Asserting on one entity.

use core::fmt::Debug;

use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::world::World;

use crate::describe;

/// One entity, to assert on: each failure names it and says what it has.
pub struct EntityCheck<'w> {
    world: &'w World,
    entity: Entity,
}

impl<'w> EntityCheck<'w> {
    pub(crate) fn new(world: &'w World, entity: Entity) -> Self {
        Self { world, entity }
    }

    /// Its `C`.
    ///
    /// # Panics
    ///
    /// If it has no `C`, naming what it has instead.
    #[track_caller]
    #[must_use]
    pub fn get<C: Component>(&self) -> &'w C {
        self.component()
    }

    /// Its `C`, failing the test if it has none.
    #[track_caller]
    fn component<C: Component>(&self) -> &'w C {
        self.world.get::<C>(self.entity).unwrap_or_else(|| {
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
    pub fn assert_has<C: Component>(&mut self) -> &mut Self {
        self.component::<C>();
        self
    }

    /// Asserts it has no `C`.
    ///
    /// # Panics
    ///
    /// If it has a `C`, naming what it has.
    #[track_caller]
    pub fn assert_lacks<C: Component>(&mut self) -> &mut Self {
        assert!(
            self.world.get::<C>(self.entity).is_none(),
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
    pub fn assert<C: Component + Debug>(&mut self, holds: impl FnOnce(&C) -> bool) -> &mut Self {
        let found = self.component::<C>();
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
    pub fn assert_eq<C: Component + Debug + PartialEq>(&mut self, expected: C) -> &mut Self {
        let found = self.component::<C>();
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
            self.world.get_entity(self.entity).is_err(),
            "{} is still there: {}",
            describe::label(self.world, self.entity),
            describe::components(self.world, self.entity)
        );
    }
}
