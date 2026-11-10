// Dumps what ICU (macOS's libicucore, which NSRegularExpression runs on) says about every scalar:
// its general category, its simple and full case folding, and which scalars case-insensitive
// matching adds to the formatter's word-character classes.
#include <stdint.h>
#include <stdio.h>

typedef uint16_t UChar;
typedef struct USet USet;
int32_t u_strFoldCase(UChar *dest, int32_t capacity, const UChar *src, int32_t length,
                      uint32_t options, int *error);
int32_t u_foldCase(int32_t c, uint32_t options);
int8_t u_charType(int32_t c);
void u_getUnicodeVersion(uint8_t version[4]);
USet *uset_openPattern(const UChar *pattern, int32_t length, int *error);
void uset_closeOver(USet *set, int32_t attributes);
int8_t uset_contains(const USet *set, int32_t c);

enum { USET_CASE_INSENSITIVE = 2 };

static void closure(const char *name, const UChar *pattern, int32_t length) {
    int error = 0;
    USet *plain = uset_openPattern(pattern, length, &error);
    USet *closed = uset_openPattern(pattern, length, &error);
    if (error > 0) {
        fprintf(stderr, "bad set %s\n", name);
        return;
    }
    uset_closeOver(closed, USET_CASE_INSENSITIVE);
    printf("closure %s", name);
    for (int32_t c = 0; c <= 0x10FFFF; c++) {
        if (uset_contains(closed, c) && !uset_contains(plain, c)) printf(" %X", c);
    }
    printf("\n");
}

int main(void) {
    uint8_t version[4];
    u_getUnicodeVersion(version);
    printf("version %d.%d\n", version[0], version[1]);
    for (int32_t c = 0; c <= 0x10FFFF; c++) {
        if (c >= 0xD800 && c <= 0xDFFF) continue;
        UChar src[2];
        int32_t n = 0;
        if (c < 0x10000) {
            src[n++] = (UChar)c;
        } else {
            src[n++] = (UChar)(0xD800 + ((c - 0x10000) >> 10));
            src[n++] = (UChar)(0xDC00 + ((c - 0x10000) & 0x3FF));
        }
        UChar folded[8];
        int error = 0;
        int32_t m = u_strFoldCase(folded, 8, src, n, 0, &error);
        printf("gc %X %d\n", c, u_charType(c));
        if (u_foldCase(c, 0) != c) printf("simple %X %X\n", c, u_foldCase(c, 0));
        int same = m == n && folded[0] == src[0] && (n == 1 || folded[1] == src[1]);
        if (!same) {
            printf("full %X", c);
            for (int32_t i = 0; i < m; i++) printf(" %04X", folded[i]);
            printf("\n");
        }
    }
    const UChar word[] = u"[\\p{L}\\p{N}_]";
    closure("word", word, sizeof word / sizeof *word - 1);
    const UChar filler[] = u"[\\p{L}\\p{N}_'’\\-]";
    closure("filler", filler, sizeof filler / sizeof *filler - 1);
    return 0;
}
