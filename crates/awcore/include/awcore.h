// AudioWhisper's transcript rules, from the Rust crate `awcore` (crates/awcore/src/lib.rs).
//
// Text crosses as UTF-8 bytes with a length. Every AwString returned is the caller's to free
// with aw_string_free, every vocabulary with aw_vocabulary_free and every list with
// aw_strings_free. A null `ptr` in an AwString means "no string" (Swift's nil), not an empty one.

#ifndef AWCORE_H
#define AWCORE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct AwString {
    uint8_t *ptr;
    size_t len;
} AwString;

typedef struct AwVocabulary AwVocabulary;
typedef struct AwStrings AwStrings;

void aw_string_free(AwString string);

AwVocabulary *aw_vocabulary_new(void);
AwVocabulary *aw_vocabulary_parse(const uint8_t *bytes, size_t len);
void aw_vocabulary_free(AwVocabulary *vocabulary);
void aw_vocabulary_add_term(AwVocabulary *vocabulary, const uint8_t *bytes, size_t len);
void aw_vocabulary_add_replacement(AwVocabulary *vocabulary, const uint8_t *heard,
                                   size_t heard_len, const uint8_t *written, size_t written_len);
size_t aw_vocabulary_term_count(const AwVocabulary *vocabulary);
AwString aw_vocabulary_term(const AwVocabulary *vocabulary, size_t index);
size_t aw_vocabulary_replacement_count(const AwVocabulary *vocabulary);
AwString aw_vocabulary_replacement(const AwVocabulary *vocabulary, size_t index, bool written);
AwString aw_vocabulary_prompt_text(const AwVocabulary *vocabulary);
AwString aw_vocabulary_apply(const AwVocabulary *vocabulary, const uint8_t *bytes, size_t len);

AwString aw_replace_whole_phrase(const uint8_t *phrase, size_t phrase_len,
                                 const uint8_t *replacement, size_t replacement_len,
                                 const uint8_t *bytes, size_t len, bool matching_split_camel_case);

AwString aw_format(const uint8_t *bytes, size_t len, const AwVocabulary *vocabulary,
                   bool remove_fillers, bool spoken_punctuation);
AwString aw_spoken_punctuation(const uint8_t *bytes, size_t len);
AwString aw_remove_fillers(const uint8_t *bytes, size_t len);
AwString aw_tidy(const uint8_t *bytes, size_t len);
AwStrings *aw_camel_case_parts(const uint8_t *bytes, size_t len);
size_t aw_strings_count(const AwStrings *strings);
AwString aw_strings_get(const AwStrings *strings, size_t index);
void aw_strings_free(AwStrings *strings);

AwString aw_fit(const uint8_t *bytes, size_t len, bool has_preceding, const uint8_t *preceding,
                size_t preceding_len);

#ifdef __cplusplus
}
#endif

#endif
