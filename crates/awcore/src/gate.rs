//! The correction gate (the `correction` crate) behind the C interface.

use correction::Operation;

use crate::{AwString, text};

/// Whether the correction keeps what the original must not lose; see `correction::allows`.
///
/// # Safety
/// Each pointer must point to its length of readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_correction_allows(
    original: *const u8,
    original_len: usize,
    corrected: *const u8,
    corrected_len: usize,
    allow_framing: bool,
) -> bool {
    // SAFETY: as the caller promises.
    let (original, corrected) =
        unsafe { (text(original, original_len), text(corrected, corrected_len)) };
    correction::allows(&original, &corrected, allow_framing)
}

/// The bounded comparison's change ratio, written to `ratio`; false (and `ratio` untouched) when
/// it can't tell.
///
/// # Safety
/// Each pointer must point to its length of readable bytes, and `ratio` to a writable double.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_correction_ratio(
    original: *const u8,
    original_len: usize,
    corrected: *const u8,
    corrected_len: usize,
    ratio: *mut f64,
) -> bool {
    // SAFETY: as the caller promises.
    let (original, corrected) =
        unsafe { (text(original, original_len), text(corrected, corrected_len)) };
    let Some(value) = correction::ratio(&original, &corrected) else {
        return false;
    };
    // SAFETY: as the caller promises.
    unsafe { ratio.write(value) };
    true
}

/// The correction, trimmed, if it passes the gate and changes at most `max_change_ratio`; else
/// the original.
///
/// # Safety
/// Each pointer must point to its length of readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_correction_safe_merge(
    original: *const u8,
    original_len: usize,
    corrected: *const u8,
    corrected_len: usize,
    max_change_ratio: f64,
) -> AwString {
    // SAFETY: as the caller promises.
    let (original, corrected) =
        unsafe { (text(original, original_len), text(corrected, corrected_len)) };
    AwString::new(correction::safe_merge(
        &original,
        &corrected,
        max_change_ratio,
    ))
}

/// The edit distance in characters over the longer length.
///
/// # Safety
/// Each pointer must point to its length of readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_correction_edit_distance(
    lhs: *const u8,
    lhs_len: usize,
    rhs: *const u8,
    rhs_len: usize,
) -> f64 {
    // SAFETY: as the caller promises.
    let (lhs, rhs) = unsafe { (text(lhs, lhs_len), text(rhs, rhs_len)) };
    correction::normalized_edit_distance(&lhs, &rhs)
}

/// The model's output if it may replace the dictation (`rewrite` false: a cleanup), else the
/// original.
///
/// # Safety
/// Each pointer must point to its length of readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_correction_accepted(
    original: *const u8,
    original_len: usize,
    output: *const u8,
    output_len: usize,
    rewrite: bool,
) -> AwString {
    // SAFETY: as the caller promises.
    let (original, output) = unsafe { (text(original, original_len), text(output, output_len)) };
    let operation = if rewrite {
        Operation::Rewrite
    } else {
        Operation::Cleanup
    };
    AwString::new(correction::accepted(&original, &output, operation))
}
