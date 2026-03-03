/*
 * External scanner for tree-sitter-patto.
 * Emits NEWLINE at \n and exactly once at EOF.
 */
#include "tree_sitter/parser.h"
#include <stdlib.h>
#include <stdbool.h>

enum TokenType { NEWLINE };

typedef struct {
    bool eof_emitted;
} Scanner;

void *tree_sitter_patto_external_scanner_create(void) {
    Scanner *s = calloc(1, sizeof(Scanner));
    return s;
}

void tree_sitter_patto_external_scanner_destroy(void *p) {
    free(p);
}

unsigned tree_sitter_patto_external_scanner_serialize(void *p, char *buf) {
    Scanner *s = (Scanner *)p;
    buf[0] = s->eof_emitted ? 1 : 0;
    return 1;
}

void tree_sitter_patto_external_scanner_deserialize(void *p, const char *buf, unsigned len) {
    Scanner *s = (Scanner *)p;
    s->eof_emitted = (len > 0 && buf[0]) ? true : false;
}

bool tree_sitter_patto_external_scanner_scan(
    void *payload, TSLexer *lexer, const bool *valid_symbols
) {
    Scanner *s = (Scanner *)payload;
    if (!valid_symbols[NEWLINE]) return false;

    if (lexer->eof(lexer)) {
        if (s->eof_emitted) return false;
        s->eof_emitted = true;
        lexer->result_symbol = NEWLINE;
        return true;
    }

    /* Reset eof_emitted on non-EOF input (for incremental parsing) */
    s->eof_emitted = false;

    if (lexer->lookahead == '\n') {
        lexer->advance(lexer, false);
        lexer->result_symbol = NEWLINE;
        return true;
    }
    if (lexer->lookahead == '\r') {
        lexer->advance(lexer, false);
        if (lexer->lookahead == '\n') lexer->advance(lexer, false);
        lexer->result_symbol = NEWLINE;
        return true;
    }

    return false;
}
