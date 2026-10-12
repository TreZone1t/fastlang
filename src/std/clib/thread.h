#ifndef FASTLANG_CLIB_THREAD_H
#define FASTLANG_CLIB_THREAD_H

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>

#ifdef _WIN32
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <winsock2.h>
#include <windows.h>
#else
#include <pthread.h>
#include <unistd.h>
#include <sched.h>
#endif

#ifdef __cplusplus
#include <type_traits>
#include <utility>

extern "C" {
#endif

// ==========================================
// C-ABI Thread Primitives
// ==========================================

static inline void* fast_thread_create(void* (*func)(void*), void* arg) {
#ifdef _WIN32
    DWORD thread_id = 0;
    HANDLE h = CreateThread(NULL, 0, (LPTHREAD_START_ROUTINE)func, arg, 0, &thread_id);
    return (void*)h;
#else
    pthread_t* tid = (pthread_t*)malloc(sizeof(pthread_t));
    if (!tid) return NULL;
    if (pthread_create(tid, NULL, func, arg) != 0) {
        free(tid);
        return NULL;
    }
    return (void*)tid;
#endif
}

static inline int fast_thread_join(void* handle) {
    if (!handle) return -1;
#ifdef _WIN32
    WaitForSingleObject((HANDLE)handle, INFINITE);
    CloseHandle((HANDLE)handle);
    return 0;
#else
    pthread_t* tid = (pthread_t*)handle;
    pthread_join(*tid, NULL);
    free(tid);
    return 0;
#endif
}

static inline void fast_sleep_ms(int ms) {
    if (ms <= 0) return;
#ifdef _WIN32
    Sleep((DWORD)ms);
#else
    usleep((useconds_t)(ms * 1000));
#endif
}

static inline void fast_thread_yield(void) {
#ifdef _WIN32
    SwitchToThread();
#else
    sched_yield();
#endif
}

// ==========================================
// C-ABI Mutex Primitives
// ==========================================

static inline void* fast_mutex_create(void) {
#ifdef _WIN32
    CRITICAL_SECTION* cs = (CRITICAL_SECTION*)malloc(sizeof(CRITICAL_SECTION));
    if (cs) {
        InitializeCriticalSection(cs);
    }
    return (void*)cs;
#else
    pthread_mutex_t* m = (pthread_mutex_t*)malloc(sizeof(pthread_mutex_t));
    if (m) {
        pthread_mutex_init(m, NULL);
    }
    return (void*)m;
#endif
}

static inline void fast_mutex_lock(void* m) {
    if (!m) return;
#ifdef _WIN32
    EnterCriticalSection((CRITICAL_SECTION*)m);
#else
    pthread_mutex_lock((pthread_mutex_t*)m);
#endif
}

static inline void fast_mutex_unlock(void* m) {
    if (!m) return;
#ifdef _WIN32
    LeaveCriticalSection((CRITICAL_SECTION*)m);
#else
    pthread_mutex_unlock((pthread_mutex_t*)m);
#endif
}

static inline void fast_mutex_destroy(void* m) {
    if (!m) return;
#ifdef _WIN32
    DeleteCriticalSection((CRITICAL_SECTION*)m);
    free(m);
#else
    pthread_mutex_destroy((pthread_mutex_t*)m);
    free(m);
#endif
}

#ifdef __cplusplus
} // extern "C"

// ==========================================
// C++ Callable / Lambda Spawn Helper
// ==========================================
template <typename F>
static inline void* fast_thread_spawn(F&& f) {
    using DecayedF = std::decay_t<F>;
    DecayedF* fn_ptr = new DecayedF(std::forward<F>(f));
#ifdef _WIN32
    auto trampoline = [](void* arg) -> DWORD {
        DecayedF* callable = static_cast<DecayedF*>(arg);
        (*callable)();
        delete callable;
        return 0;
    };
    DWORD thread_id = 0;
    HANDLE h = CreateThread(NULL, 0, (LPTHREAD_START_ROUTINE)+trampoline, fn_ptr, 0, &thread_id);
    return (void*)h;
#else
    auto posix_trampoline = [](void* arg) -> void* {
        DecayedF* callable = static_cast<DecayedF*>(arg);
        (*callable)();
        delete callable;
        return NULL;
    };
    pthread_t* tid = (pthread_t*)malloc(sizeof(pthread_t));
    if (!tid) { delete fn_ptr; return NULL; }
    if (pthread_create(tid, NULL, posix_trampoline, fn_ptr) != 0) {
        free(tid);
        delete fn_ptr;
        return NULL;
    }
    return (void*)tid;
#endif
}
#endif

#endif // FASTLANG_CLIB_THREAD_H
