#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <tree_sitter/parser.h>

enum TokenType {
  NEWLINE,
  INDENT,
  DEDENT,
};

#define MAX_INDENT_DEPTH 256

typedef struct {
  uint16_t indent_stack[MAX_INDENT_DEPTH];
  uint32_t stack_size;
  uint32_t dedent_count;
  int32_t pending_indent;
} Scanner;

static void scanner_reset(Scanner *scanner) {
  scanner->indent_stack[0] = 0;
  scanner->stack_size = 1;
  scanner->dedent_count = 0;
  scanner->pending_indent = -1;
}

static inline bool is_newline(int32_t character) {
  return character == '\n' || character == '\r';
}

static inline void advance_newline(TSLexer *lexer) {
  if (lexer->lookahead == '\r') {
    lexer->advance(lexer, true);
    if (lexer->lookahead == '\n') {
      lexer->advance(lexer, true);
    }
  } else {
    lexer->advance(lexer, true);
  }
}

void *tree_sitter_patto_external_scanner_create() {
  Scanner *scanner = (Scanner *)calloc(1, sizeof(Scanner));
  scanner_reset(scanner);
  return scanner;
}

void tree_sitter_patto_external_scanner_destroy(void *payload) {
  free(payload);
}

unsigned tree_sitter_patto_external_scanner_serialize(void *payload, char *buffer) {
  Scanner *scanner = (Scanner *)payload;
  uint32_t size = 0;

  memcpy(buffer + size, &scanner->stack_size, sizeof(scanner->stack_size));
  size += sizeof(scanner->stack_size);

  uint32_t stack_bytes = scanner->stack_size * sizeof(uint16_t);
  memcpy(buffer + size, scanner->indent_stack, stack_bytes);
  size += stack_bytes;

  memcpy(buffer + size, &scanner->dedent_count, sizeof(scanner->dedent_count));
  size += sizeof(scanner->dedent_count);

  memcpy(buffer + size, &scanner->pending_indent, sizeof(scanner->pending_indent));
  size += sizeof(scanner->pending_indent);

  return size;
}

void tree_sitter_patto_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {
  Scanner *scanner = (Scanner *)payload;
  scanner_reset(scanner);

  if (length == 0) {
    return;
  }

  uint32_t size = 0;

  if (size + sizeof(scanner->stack_size) > length) {
    return;
  }
  memcpy(&scanner->stack_size, buffer + size, sizeof(scanner->stack_size));
  size += sizeof(scanner->stack_size);

  if (scanner->stack_size == 0 || scanner->stack_size > MAX_INDENT_DEPTH) {
    scanner_reset(scanner);
    return;
  }

  uint32_t stack_bytes = scanner->stack_size * sizeof(uint16_t);
  if (size + stack_bytes > length) {
    scanner_reset(scanner);
    return;
  }
  memcpy(scanner->indent_stack, buffer + size, stack_bytes);
  size += stack_bytes;

  if (size + sizeof(scanner->dedent_count) > length) {
    scanner_reset(scanner);
    return;
  }
  memcpy(&scanner->dedent_count, buffer + size, sizeof(scanner->dedent_count));
  size += sizeof(scanner->dedent_count);

  if (size + sizeof(scanner->pending_indent) > length) {
    scanner_reset(scanner);
    return;
  }
  memcpy(&scanner->pending_indent, buffer + size, sizeof(scanner->pending_indent));
}

bool tree_sitter_patto_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid_symbols) {
  Scanner *scanner = (Scanner *)payload;

  if (scanner->dedent_count > 0) {
    if (valid_symbols[DEDENT]) {
      scanner->dedent_count--;
      lexer->result_symbol = DEDENT;
      return true;
    }
  }

  if (scanner->pending_indent >= 0) {
    if (valid_symbols[INDENT]) {
      if (scanner->stack_size < MAX_INDENT_DEPTH) {
        scanner->indent_stack[scanner->stack_size++] = (uint16_t)scanner->pending_indent;
      }
      scanner->pending_indent = -1;
      lexer->result_symbol = INDENT;
      return true;
    }
  }

  if (lexer->eof(lexer)) {
    if (valid_symbols[DEDENT] && scanner->stack_size > 1) {
      scanner->stack_size--;
      lexer->result_symbol = DEDENT;
      return true;
    }
    return false;
  }

  if (!is_newline(lexer->lookahead) || !valid_symbols[NEWLINE]) {
    return false;
  }

  advance_newline(lexer);
  lexer->result_symbol = NEWLINE;

  while (lexer->lookahead == '\r') {
    advance_newline(lexer);
  }

  uint32_t indent_length = 0;
  while (lexer->lookahead == '\t') {
    indent_length++;
    lexer->advance(lexer, true);
  }

  int32_t next_char = lexer->lookahead;
  bool line_is_blank = next_char == '\n' || next_char == '\r' || next_char == 0;

  if (!line_is_blank) {
    uint32_t current_indent = scanner->indent_stack[scanner->stack_size - 1];
    if (indent_length > current_indent) {
      scanner->pending_indent = (int32_t)indent_length;
    } else if (indent_length < current_indent) {
      while (scanner->stack_size > 1 && indent_length < scanner->indent_stack[scanner->stack_size - 1]) {
        scanner->stack_size--;
        scanner->dedent_count++;
      }
      if (indent_length > scanner->indent_stack[scanner->stack_size - 1]) {
        scanner->pending_indent = (int32_t)indent_length;
      }
    }
  }

  return true;
}
