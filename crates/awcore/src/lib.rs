//! The transcript rules behind a C interface, which the Mac app's Swift calls through
//! `include/awcore.h`. Text crosses as UTF-8 bytes with a length, so a NUL inside it survives.
//! Every string this library returns is the caller's to free with [`aw_string_free`], every
//! vocabulary with [`aw_vocabulary_free`] and every list with [`aw_strings_free`].

use std::borrow::Cow;
use std::ptr;

use transcript::{Options, Replacement, Vocabulary, cursor, formatter, vocabulary};

/// A UTF-8 string this library allocated. `ptr` is null only for "no string".
#[repr(C)]
#[derive(Debug)]
pub struct AwString {
    pub ptr: *mut u8,
    pub len: usize,
}

impl AwString {
    const NONE: Self = Self {
        ptr: ptr::null_mut(),
        len: 0,
    };

    fn new(text: String) -> Self {
        let bytes = Box::into_raw(text.into_bytes().into_boxed_slice());
        Self {
            ptr: bytes.cast(),
            len: bytes.len(),
        }
    }
}

/// The text at `bytes`, invalid UTF-8 replaced.
///
/// # Safety
/// `bytes` must point to `len` readable bytes, or be null with `len` 0.
unsafe fn text<'a>(bytes: *const u8, len: usize) -> Cow<'a, str> {
    if bytes.is_null() || len == 0 {
        return Cow::Borrowed("");
    }
    // SAFETY: as the caller promises.
    String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(bytes, len) })
}

/// Frees a string this library returned. Null does nothing.
///
/// # Safety
/// `string` must be one this library returned, not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_string_free(string: AwString) {
    if string.ptr.is_null() {
        return;
    }
    let slice = ptr::slice_from_raw_parts_mut(string.ptr, string.len);
    // SAFETY: it came from `AwString::new`'s boxed slice of this length.
    drop(unsafe { Box::from_raw(slice) });
}

/// An empty vocabulary.
#[unsafe(no_mangle)]
pub extern "C" fn aw_vocabulary_new() -> *mut Vocabulary {
    Box::into_raw(Box::default())
}

/// The vocabulary written one entry per line.
///
/// # Safety
/// `bytes` must point to `len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_vocabulary_parse(bytes: *const u8, len: usize) -> *mut Vocabulary {
    // SAFETY: as the caller promises.
    let text = unsafe { text(bytes, len) };
    Box::into_raw(Box::new(Vocabulary::parse(&text)))
}

/// Frees a vocabulary. Null does nothing.
///
/// # Safety
/// `vocabulary` must be one this library returned, not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_vocabulary_free(vocabulary: *mut Vocabulary) {
    if !vocabulary.is_null() {
        // SAFETY: as the caller promises.
        drop(unsafe { Box::from_raw(vocabulary) });
    }
}

/// Appends a term.
///
/// # Safety
/// `vocabulary` must be live, and `bytes` point to `len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_vocabulary_add_term(
    vocabulary: *mut Vocabulary,
    bytes: *const u8,
    len: usize,
) {
    // SAFETY: as the caller promises.
    let (vocabulary, term) = unsafe { (&mut *vocabulary, text(bytes, len)) };
    vocabulary.terms.push(term.into_owned());
}

/// Appends a replacement.
///
/// # Safety
/// `vocabulary` must be live, and each pointer point to its length of readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_vocabulary_add_replacement(
    vocabulary: *mut Vocabulary,
    heard: *const u8,
    heard_len: usize,
    written: *const u8,
    written_len: usize,
) {
    // SAFETY: as the caller promises.
    let (vocabulary, heard, written) = unsafe {
        (
            &mut *vocabulary,
            text(heard, heard_len),
            text(written, written_len),
        )
    };
    vocabulary.replacements.push(Replacement {
        heard: heard.into_owned(),
        written: written.into_owned(),
    });
}

/// How many terms there are.
///
/// # Safety
/// `vocabulary` must be live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_vocabulary_term_count(vocabulary: *const Vocabulary) -> usize {
    // SAFETY: as the caller promises.
    unsafe { &*vocabulary }.terms.len()
}

/// Term `index`, or no string past the end.
///
/// # Safety
/// `vocabulary` must be live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_vocabulary_term(
    vocabulary: *const Vocabulary,
    index: usize,
) -> AwString {
    // SAFETY: as the caller promises.
    let terms = &unsafe { &*vocabulary }.terms;
    terms
        .get(index)
        .map_or(AwString::NONE, |it| AwString::new(it.clone()))
}

/// How many replacements there are.
///
/// # Safety
/// `vocabulary` must be live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_vocabulary_replacement_count(vocabulary: *const Vocabulary) -> usize {
    // SAFETY: as the caller promises.
    unsafe { &*vocabulary }.replacements.len()
}

/// Replacement `index`'s heard (`written` false) or written (`written` true) side, or no string
/// past the end.
///
/// # Safety
/// `vocabulary` must be live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_vocabulary_replacement(
    vocabulary: *const Vocabulary,
    index: usize,
    written: bool,
) -> AwString {
    // SAFETY: as the caller promises.
    let replacements = &unsafe { &*vocabulary }.replacements;
    replacements.get(index).map_or(AwString::NONE, |it| {
        AwString::new(if written { &it.written } else { &it.heard }.clone())
    })
}

/// The recogniser prompt, or no string if there is nothing to prompt with.
///
/// # Safety
/// `vocabulary` must be live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_vocabulary_prompt_text(vocabulary: *const Vocabulary) -> AwString {
    // SAFETY: as the caller promises.
    unsafe { &*vocabulary }
        .prompt_text()
        .map_or(AwString::NONE, AwString::new)
}

/// The text with the vocabulary applied.
///
/// # Safety
/// `vocabulary` must be live, and `bytes` point to `len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_vocabulary_apply(
    vocabulary: *const Vocabulary,
    bytes: *const u8,
    len: usize,
) -> AwString {
    // SAFETY: as the caller promises.
    let (vocabulary, text) = unsafe { (&*vocabulary, text(bytes, len)) };
    AwString::new(vocabulary.apply(&text))
}

/// Every whole-word, case-insensitive match of the phrase in the text replaced.
///
/// # Safety
/// Each pointer must point to its length of readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_replace_whole_phrase(
    phrase: *const u8,
    phrase_len: usize,
    replacement: *const u8,
    replacement_len: usize,
    bytes: *const u8,
    len: usize,
    matching_split_camel_case: bool,
) -> AwString {
    // SAFETY: as the caller promises.
    let (phrase, replacement, text) = unsafe {
        (
            text(phrase, phrase_len),
            text(replacement, replacement_len),
            text(bytes, len),
        )
    };
    let replaced =
        vocabulary::replace_whole_phrase(&phrase, &replacement, &text, matching_split_camel_case);
    AwString::new(replaced)
}

/// Spoken punctuation, then fillers, then the vocabulary (which may be null), as asked.
///
/// # Safety
/// `bytes` must point to `len` readable bytes, and `vocabulary` be null or live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_format(
    bytes: *const u8,
    len: usize,
    vocabulary: *const Vocabulary,
    remove_fillers: bool,
    spoken_punctuation: bool,
) -> AwString {
    // SAFETY: as the caller promises.
    let (text, vocabulary) = unsafe { (text(bytes, len), vocabulary.as_ref()) };
    let options = Options {
        vocabulary: vocabulary.cloned().unwrap_or_default(),
        remove_fillers,
        spoken_punctuation,
    };
    AwString::new(transcript::format(&text, &options))
}

/// One of the formatter's stages.
///
/// # Safety
/// `bytes` must point to `len` readable bytes.
unsafe fn stage(bytes: *const u8, len: usize, rule: fn(&str) -> String) -> AwString {
    // SAFETY: as the caller promises.
    AwString::new(rule(&unsafe { text(bytes, len) }))
}

/// Spoken punctuation turned into marks.
///
/// # Safety
/// `bytes` must point to `len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_spoken_punctuation(bytes: *const u8, len: usize) -> AwString {
    // SAFETY: as the caller promises.
    unsafe { stage(bytes, len, formatter::apply_spoken_punctuation) }
}

/// Filler words removed.
///
/// # Safety
/// `bytes` must point to `len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_remove_fillers(bytes: *const u8, len: usize) -> AwString {
    // SAFETY: as the caller promises.
    unsafe { stage(bytes, len, formatter::remove_fillers) }
}

/// The gaps a removal leaves collapsed.
///
/// # Safety
/// `bytes` must point to `len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_tidy(bytes: *const u8, len: usize) -> AwString {
    // SAFETY: as the caller promises.
    unsafe { stage(bytes, len, formatter::tidy) }
}

/// The text fitted to what precedes the cursor, if `has_preceding`.
///
/// # Safety
/// Each pointer must point to its length of readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_fit(
    bytes: *const u8,
    len: usize,
    has_preceding: bool,
    preceding: *const u8,
    preceding_len: usize,
) -> AwString {
    // SAFETY: as the caller promises.
    let text = unsafe { text(bytes, len) };
    let before = has_preceding.then(|| {
        // SAFETY: as the caller promises.
        unsafe { self::text(preceding, preceding_len) }
    });
    AwString::new(cursor::fit(&text, before.as_deref()))
}

/// A list of strings this library made.
#[derive(Debug)]
pub struct AwStrings(Vec<String>);

/// A CamelCase term's parts: "AudioWhisper" gives "Audio" and "Whisper".
///
/// # Safety
/// `bytes` must point to `len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_camel_case_parts(bytes: *const u8, len: usize) -> *mut AwStrings {
    // SAFETY: as the caller promises.
    let term = unsafe { text(bytes, len) };
    Box::into_raw(Box::new(AwStrings(vocabulary::camel_case_parts(&term))))
}

/// How many strings there are.
///
/// # Safety
/// `strings` must be live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_strings_count(strings: *const AwStrings) -> usize {
    // SAFETY: as the caller promises.
    unsafe { &*strings }.0.len()
}

/// String `index`, or no string past the end.
///
/// # Safety
/// `strings` must be live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_strings_get(strings: *const AwStrings, index: usize) -> AwString {
    // SAFETY: as the caller promises.
    let strings = &unsafe { &*strings }.0;
    strings
        .get(index)
        .map_or(AwString::NONE, |it| AwString::new(it.clone()))
}

/// Frees a list. Null does nothing.
///
/// # Safety
/// `strings` must be one this library returned, not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aw_strings_free(strings: *mut AwStrings) {
    if !strings.is_null() {
        // SAFETY: as the caller promises.
        drop(unsafe { Box::from_raw(strings) });
    }
}
