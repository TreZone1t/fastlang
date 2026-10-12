#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <string.h>
#include <fcntl.h>

#ifdef _WIN32
#include <windows.h>
#include <io.h>
#else
#include <unistd.h>
#endif

#ifndef FASTLANG_UTF8_DEFINED
#define FASTLANG_UTF8_DEFINED
static void fastlang_enable_utf8_output(void) {
#ifdef _WIN32
    static int initialized = 0;
    if (!initialized) {
        SetConsoleOutputCP(CP_UTF8);
        initialized = 1;
    }
#endif
}
#endif

// In C mode, these provide extern C linkage if not inlined
#ifndef __cplusplus
void* fast_alloc(size_t size) {
    void* ptr = malloc(size);
    if (ptr) {
        memset(ptr, 0, size);
    }
    return ptr;
}

void fast_free(void* ptr) {
    if (ptr) {
        free(ptr);
    }
}

int fast_open(const char* path, int flags, int mode) {
#ifdef _WIN32
    return _open(path, flags, mode);
#else
    return open(path, flags, mode);
#endif
}

int fast_close(int fd) {
#ifdef _WIN32
    return _close(fd);
#else
    return close(fd);
#endif
}

size_t fast_read(int fd, void* buf, size_t count) {
    if (!buf || count == 0) return 0;
    if (fd == 0) {
        size_t read_bytes = fread(buf, 1, count, stdin);
        return read_bytes;
    }
#ifdef _WIN32
    int bytes = _read(fd, buf, (unsigned int)count);
    return bytes < 0 ? 0 : (size_t)bytes;
#else
    ssize_t bytes = read(fd, buf, count);
    return bytes < 0 ? 0 : (size_t)bytes;
#endif
}

size_t fast_write(int fd, const void* buf, size_t count) {
    fastlang_enable_utf8_output();
    if (!buf || count == 0) return 0;
    if (fd == 1) {
        size_t written = fwrite(buf, 1, count, stdout);
        fflush(stdout);
        return written;
    } else if (fd == 2) {
        size_t written = fwrite(buf, 1, count, stderr);
        fflush(stderr);
        return written;
    }
#ifdef _WIN32
    int bytes = _write(fd, buf, (unsigned int)count);
    return bytes < 0 ? 0 : (size_t)bytes;
#else
    ssize_t bytes = write(fd, buf, count);
    return bytes < 0 ? 0 : (size_t)bytes;
#endif
}

void fast_exit(int code) {
    exit(code);
}
#endif
