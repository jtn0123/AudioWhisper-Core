//! The safety gate a model's correction of a transcript must pass before it replaces what was
//! dictated, shared by AudioWhisper's Mac app and its Android keyboard: negation, digits,
//! literal flags, addresses, code and names kept, no greeting, sign-off or subject line invented,
//! and (for cleanup) a bounded amount of change.
//!
//! It began as the Mac app's Swift (`CorrectionIntegrity`, `CorrectionContentComparison`,
//! `SemanticCorrectionService.safeMerge` and `DictationWriting.accepted`) and keeps its exact
//! answers, ratios to the last bit included. The Swift's patterns ran on ICU, and run here, as
//! written, on `apple_text::icu`.

mod content;
mod integrity;
mod merge;

pub use content::ratio;
pub use integrity::allows;
pub use merge::{
    MAX_CHANGE_RATIO, Operation, SAFE_MERGE_LENGTH_CAP, accepted, normalized_edit_distance,
    safe_merge,
};
