#ifndef FASTLANG_CLIB_NET_H
#define FASTLANG_CLIB_NET_H

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

#ifdef _WIN32
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <winsock2.h>
#include <ws2tcpip.h>
#include <windows.h>
#ifdef _MSC_VER
#pragma comment(lib, "ws2_32.lib")
#endif
#else
#include <sys/types.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <unistd.h>
#include <fcntl.h>
#endif

#ifdef __cplusplus
extern "C" {
#endif

static inline void fast_net_init(void) {
#ifdef _WIN32
    static int initialized = 0;
    if (!initialized) {
        WSADATA wsa;
        WSAStartup(MAKEWORD(2, 2), &wsa);
        initialized = 1;
    }
#endif
}

static inline int32_t fast_net_listen(int32_t port) {
    fast_net_init();
#ifdef _WIN32
    SOCKET s = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (s == INVALID_SOCKET) return -1;
    int opt = 1;
    setsockopt(s, SOL_SOCKET, SO_REUSEADDR, (const char*)&opt, sizeof(opt));
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = INADDR_ANY;
    addr.sin_port = htons((uint16_t)port);
    if (bind(s, (struct sockaddr*)&addr, sizeof(addr)) == SOCKET_ERROR) {
        closesocket(s);
        return -1;
    }
    if (listen(s, 128) == SOCKET_ERROR) {
        closesocket(s);
        return -1;
    }
    return (int32_t)s;
#else
    int s = socket(AF_INET, SOCK_STREAM, 0);
    if (s < 0) return -1;
    int opt = 1;
    setsockopt(s, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = INADDR_ANY;
    addr.sin_port = htons((uint16_t)port);
    if (bind(s, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        close(s);
        return -1;
    }
    if (listen(s, 128) < 0) {
        close(s);
        return -1;
    }
    return s;
#endif
}

static inline int32_t fast_net_connect(const char* host, int32_t port) {
    fast_net_init();
#ifdef _WIN32
    SOCKET s = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP);
    if (s == INVALID_SOCKET) return -1;
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = inet_addr((host && *host) ? host : "127.0.0.1");
    addr.sin_port = htons((uint16_t)port);
    if (connect(s, (struct sockaddr*)&addr, sizeof(addr)) == SOCKET_ERROR) {
        closesocket(s);
        return -1;
    }
    return (int32_t)s;
#else
    int s = socket(AF_INET, SOCK_STREAM, 0);
    if (s < 0) return -1;
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = inet_addr((host && *host) ? host : "127.0.0.1");
    addr.sin_port = htons((uint16_t)port);
    if (connect(s, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        close(s);
        return -1;
    }
    return s;
#endif
}

static inline int32_t fast_net_accept(int32_t server_fd) {
#ifdef _WIN32
    struct sockaddr_in client_addr;
    int len = sizeof(client_addr);
    SOCKET client = accept((SOCKET)server_fd, (struct sockaddr*)&client_addr, &len);
    if (client == INVALID_SOCKET) return -1;
    return (int32_t)client;
#else
    struct sockaddr_in client_addr;
    socklen_t len = sizeof(client_addr);
    int client = accept(server_fd, (struct sockaddr*)&client_addr, &len);
    return client;
#endif
}

static inline int32_t fast_net_read(int32_t fd, char* buf, int32_t max_len) {
    if (!buf || max_len <= 0) return 0;
#ifdef _WIN32
    int bytes = recv((SOCKET)fd, buf, max_len, 0);
    return bytes < 0 ? 0 : bytes;
#else
    ssize_t bytes = recv(fd, buf, max_len, 0);
    return bytes < 0 ? 0 : (int32_t)bytes;
#endif
}

static inline int32_t fast_net_write(int32_t fd, const char* buf, int32_t len) {
    if (!buf || len <= 0) return 0;
#ifdef _WIN32
    int sent = send((SOCKET)fd, buf, len, 0);
    return sent < 0 ? 0 : sent;
#else
    ssize_t sent = send(fd, buf, len, 0);
    return sent < 0 ? 0 : (int32_t)sent;
#endif
}

static inline void fast_net_close(int32_t fd) {
#ifdef _WIN32
    closesocket((SOCKET)fd);
#else
    close(fd);
#endif
}

#ifdef __cplusplus
}
#endif

#endif // FASTLANG_CLIB_NET_H
