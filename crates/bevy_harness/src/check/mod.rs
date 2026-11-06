//! Asserting on what an app holds: one entity, or what a query matches.

mod entity;
mod query;

pub use self::entity::EntityCheck;
pub use self::query::QueryCheck;
