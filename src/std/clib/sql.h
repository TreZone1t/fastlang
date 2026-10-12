#ifndef FASTLANG_CLIB_SQL_H
#define FASTLANG_CLIB_SQL_H

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include "sqlite3.h"

#ifdef __cplusplus
extern "C" {
#endif

// FastLang native string layout: size_t length prefix stored at (char*)[-1]
// Built without <string.h> using direct native byte operations
static inline char* fast_sql_dup_str(const char* s) {
    if (!s) s = "";
    size_t len = 0;
    while (s[len] != '\0') {
        len++;
    }
    size_t* raw = (size_t*)malloc(sizeof(size_t) + len + 1);
    if (!raw) return NULL;
    raw[0] = len;
    char* data = (char*)(raw + 1);
    for (size_t i = 0; i < len; ++i) {
        data[i] = s[i];
    }
    data[len] = '\0';
    return data;
}

// Database lifecycle
void* fast_sql_open(const char* path);
void fast_sql_close(void* db);

// Execution
int32_t fast_sql_exec(void* db, const char* sql);
char* fast_sql_errmsg(void* db);

// Statement preparation & step
void* fast_sql_prepare(void* db, const char* sql);
int32_t fast_sql_step(void* stmt);
int32_t fast_sql_reset(void* stmt);
void fast_sql_finalize(void* stmt);

// Parameter binding (1-based index)
int32_t fast_sql_bind_int(void* stmt, int32_t index, int32_t value);
int32_t fast_sql_bind_int64(void* stmt, int32_t index, int64_t value);
int32_t fast_sql_bind_double(void* stmt, int32_t index, double value);
int32_t fast_sql_bind_text(void* stmt, int32_t index, const char* text);
int32_t fast_sql_bind_null(void* stmt, int32_t index);

// Metadata & columns (0-based index)
int32_t fast_sql_column_count(void* stmt);
char* fast_sql_column_name(void* stmt, int32_t col);
int32_t fast_sql_column_type(void* stmt, int32_t col);
int32_t fast_sql_column_int(void* stmt, int32_t col);
int64_t fast_sql_column_int64(void* stmt, int32_t col);
double fast_sql_column_double(void* stmt, int32_t col);
char* fast_sql_column_text(void* stmt, int32_t col);

// Utilities
int64_t fast_sql_last_insert_rowid(void* db);
int32_t fast_sql_changes(void* db);

#ifdef __cplusplus
}
#endif

#endif // FASTLANG_CLIB_SQL_H
