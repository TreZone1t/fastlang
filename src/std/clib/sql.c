#include "sql.h"

void* fast_sql_open(const char* path) {
    sqlite3* db = NULL;
    int rc = sqlite3_open(path, &db);
    if (rc != SQLITE_OK) {
        if (db) sqlite3_close(db);
        return NULL;
    }
    return (void*)db;
}

void fast_sql_close(void* db) {
    if (db) {
        sqlite3_close_v2((sqlite3*)db);
    }
}

int32_t fast_sql_exec(void* db, const char* sql) {
    if (!db || !sql) return -1;
    char* err_msg = NULL;
    int rc = sqlite3_exec((sqlite3*)db, sql, NULL, NULL, &err_msg);
    if (err_msg) {
        sqlite3_free(err_msg);
    }
    return rc;
}

char* fast_sql_errmsg(void* db) {
    if (!db) return fast_sql_dup_str("Null database handle");
    const char* msg = sqlite3_errmsg((sqlite3*)db);
    return fast_sql_dup_str(msg);
}

void* fast_sql_prepare(void* db, const char* sql) {
    if (!db || !sql) return NULL;
    sqlite3_stmt* stmt = NULL;
    int rc = sqlite3_prepare_v2((sqlite3*)db, sql, -1, &stmt, NULL);
    if (rc != SQLITE_OK) {
        return NULL;
    }
    return (void*)stmt;
}

int32_t fast_sql_step(void* stmt) {
    if (!stmt) return -1;
    return sqlite3_step((sqlite3_stmt*)stmt);
}

int32_t fast_sql_reset(void* stmt) {
    if (!stmt) return -1;
    return sqlite3_reset((sqlite3_stmt*)stmt);
}

void fast_sql_finalize(void* stmt) {
    if (stmt) {
        sqlite3_finalize((sqlite3_stmt*)stmt);
    }
}

int32_t fast_sql_bind_int(void* stmt, int32_t index, int32_t value) {
    if (!stmt) return -1;
    return sqlite3_bind_int((sqlite3_stmt*)stmt, index, value);
}

int32_t fast_sql_bind_int64(void* stmt, int32_t index, int64_t value) {
    if (!stmt) return -1;
    return sqlite3_bind_int64((sqlite3_stmt*)stmt, index, value);
}

int32_t fast_sql_bind_double(void* stmt, int32_t index, double value) {
    if (!stmt) return -1;
    return sqlite3_bind_double((sqlite3_stmt*)stmt, index, value);
}

int32_t fast_sql_bind_text(void* stmt, int32_t index, const char* text) {
    if (!stmt) return -1;
    return sqlite3_bind_text((sqlite3_stmt*)stmt, index, text ? text : "", -1, SQLITE_TRANSIENT);
}

int32_t fast_sql_bind_null(void* stmt, int32_t index) {
    if (!stmt) return -1;
    return sqlite3_bind_null((sqlite3_stmt*)stmt, index);
}

int32_t fast_sql_column_count(void* stmt) {
    if (!stmt) return 0;
    return sqlite3_column_count((sqlite3_stmt*)stmt);
}

char* fast_sql_column_name(void* stmt, int32_t col) {
    if (!stmt) return fast_sql_dup_str("");
    const char* name = sqlite3_column_name((sqlite3_stmt*)stmt, col);
    return fast_sql_dup_str(name);
}

int32_t fast_sql_column_type(void* stmt, int32_t col) {
    if (!stmt) return 5;
    return sqlite3_column_type((sqlite3_stmt*)stmt, col);
}

int32_t fast_sql_column_int(void* stmt, int32_t col) {
    if (!stmt) return 0;
    return sqlite3_column_int((sqlite3_stmt*)stmt, col);
}

int64_t fast_sql_column_int64(void* stmt, int32_t col) {
    if (!stmt) return 0;
    return sqlite3_column_int64((sqlite3_stmt*)stmt, col);
}

double fast_sql_column_double(void* stmt, int32_t col) {
    if (!stmt) return 0.0;
    return sqlite3_column_double((sqlite3_stmt*)stmt, col);
}

char* fast_sql_column_text(void* stmt, int32_t col) {
    if (!stmt) return fast_sql_dup_str("");
    const unsigned char* text = sqlite3_column_text((sqlite3_stmt*)stmt, col);
    return fast_sql_dup_str(text ? (const char*)text : "");
}

int64_t fast_sql_last_insert_rowid(void* db) {
    if (!db) return 0;
    return sqlite3_last_insert_rowid((sqlite3*)db);
}

int32_t fast_sql_changes(void* db) {
    if (!db) return 0;
    return sqlite3_changes((sqlite3*)db);
}
