//! The command registry. Ids follow Magic's domain structure (`domain.verb`); every user-visible
//! action is a command dispatched through [`crate::Session::execute`].

pub mod character;
pub mod commit;
pub mod film;
pub mod motion;
pub mod project;
pub mod song;
pub mod storyboard;
pub mod words;
