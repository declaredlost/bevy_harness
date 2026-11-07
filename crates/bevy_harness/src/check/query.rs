//! Asserting on what a query matches.

use bevy::ecs::query::{QueryFilter, QuerySingleError, QueryState, ROQueryItem, ReadOnlyQueryData};
use bevy::ecs::world::World;

use crate::describe;

/// What a query matches, to assert on.
pub struct QueryCheck<'w, D: ReadOnlyQueryData + 'static, F: QueryFilter + 'static> {
    world: &'w World,
    state: QueryState<D, F>,
}

impl<'w, D: ReadOnlyQueryData + 'static, F: QueryFilter + 'static> QueryCheck<'w, D, F> {
    pub(crate) fn new(world: &'w World, state: QueryState<D, F>) -> Self {
        Self { world, state }
    }

    /// What it matches.
    fn matches(&mut self) -> impl Iterator<Item = ROQueryItem<'w, '_, D>> {
        self.state.iter(self.world)
    }

    /// How many it matches.
    pub fn len(&mut self) -> usize {
        self.matches().count()
    }

    /// Whether it matches none.
    pub fn is_empty(&mut self) -> bool {
        self.len() == 0
    }

    /// What it matches, if it matches exactly one.
    ///
    /// # Panics
    ///
    /// If it matches none, or more than one, saying how many.
    #[track_caller]
    pub fn single(&mut self) -> ROQueryItem<'w, '_, D> {
        let found = self.len();
        match self.state.single(self.world) {
            Ok(item) => item,
            Err(QuerySingleError::NoEntities(_) | QuerySingleError::MultipleEntities(_)) => {
                panic!("{} matches {found}, not one", describe::short::<D>())
            }
        }
    }

    /// Asserts it matches `n`.
    ///
    /// # Panics
    ///
    /// If it matches any other number, saying how many.
    #[track_caller]
    pub fn assert_len(&mut self, n: usize) -> &mut Self {
        let found = self.len();
        assert_eq!(
            found,
            n,
            "{} matches {found}, not {n}",
            describe::short::<D>()
        );
        self
    }

    /// Asserts every match is as `holds` says (none matching passes).
    ///
    /// # Panics
    ///
    /// If any match isn't, saying how many of how many.
    #[track_caller]
    pub fn assert_all(
        &mut self,
        mut holds: impl FnMut(ROQueryItem<'_, '_, D>) -> bool,
    ) -> &mut Self {
        let (mut found, mut failed) = (0, 0);
        for item in self.matches() {
            found += 1;
            failed += usize::from(!holds(item));
        }
        assert!(
            failed == 0,
            "{failed} of {found} {} aren't as expected",
            describe::short::<D>()
        );
        self
    }

    /// Asserts at least one match is as `holds` says.
    ///
    /// # Panics
    ///
    /// If none is, saying how many there were.
    #[track_caller]
    pub fn assert_any(&mut self, holds: impl FnMut(ROQueryItem<'_, '_, D>) -> bool) -> &mut Self {
        let found = self.len();
        let any = self.matches().any(holds);
        assert!(
            any,
            "none of {found} {} is as expected",
            describe::short::<D>()
        );
        self
    }

    /// Asserts no match is as `holds` says.
    ///
    /// # Panics
    ///
    /// If any is, saying how many of how many.
    #[track_caller]
    pub fn assert_none(
        &mut self,
        mut holds: impl FnMut(ROQueryItem<'_, '_, D>) -> bool,
    ) -> &mut Self {
        let (mut found, mut matched) = (0, 0);
        for item in self.matches() {
            found += 1;
            matched += usize::from(holds(item));
        }
        assert!(
            matched == 0,
            "{matched} of {found} {} are as they shouldn't be",
            describe::short::<D>()
        );
        self
    }
}
