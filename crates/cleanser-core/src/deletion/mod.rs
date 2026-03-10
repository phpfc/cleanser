//! Deletion strategies module.
//!
//! This module provides different strategies for deleting files:
//! - Standard: Regular filesystem deletion
//! - Secure: Overwrite data before deletion (DoD 5220.22-M)
//! - Trash: Move to system trash for recovery

mod journal;
mod secure;
mod strategy;
mod trash;

pub use journal::{TrashEntry, TrashJournal};
pub use secure::SecureDeleter;
pub use strategy::{DeletionMethod, DeletionProgress, SecureDeleteConfig, SecureDeletePattern};
pub use trash::{TrashConfig, TrashManager};
