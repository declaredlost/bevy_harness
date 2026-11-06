//! Asserting on what a query matches.

use bevy::ecs::query::{QueryFilter, QuerySingleError, QueryState, ROQueryItem, ReadOnlyQueryData};
use bevy::ecs::world::World;

use crate::describe;

/// What a query matches, to assert on. One naming something never
/// registered in the world matches nothing.
pub struct QueryCheck<'w, D: ReadOnlyQueryData + 'static, F: QueryFilter + 'static> {
    world: &'w World,
    /// `None` if what it names was never registered.
    state: Option<QueryState<D, F>>,
}

impl<'w, D: ReadOnlyQueryData + 'static, F: QueryFilter + 'static> QueryCheck<'w, D, F> {
    pub(crate) fn new(world: &'w World) -> Self {
        Self {
            world,
            state: world.try_query_filtered::<D, F>(),
        }
    }

    /// What it matches: nothing, if what it names was never registered.
    fn matches(&mut self) -> impl Iterator<Item = ROQueryItem<'w, '_, D>> {
        let world = self.world;
        self.state
            .as_mut()
            .into_iter()
            .flat_map(move |state| state.iter(world))
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
        let single = self.state.as_mut().map(|state| state.single(self.world));
        match single {
            Some(Ok(item)) => item,
            Some(Err(QuerySingleError::NoEntities(_) | QuerySingleError::MultipleEntities(_)))
            | None => panic!("{} matches {found}, not one", describe::short::<D>()),
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
