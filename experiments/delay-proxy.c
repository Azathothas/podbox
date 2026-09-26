/* delay-proxy.c - a per-fetch latency injector for experiments/190.
 *
 * TODO/image.md T-0207 needs a latency-bound shape beside the loopback one:
 * small layers with an injected delay. Shaping with tc needs a privilege no
 * driver container holds, and a sleep at connection setup collapses under
 * keep-alive (one setup serves many blobs). So this proxy delays every
 * request-response exchange instead: it reads one request's headers,
 * forces the connection closed (so each fetch pays its own setup, the way
 * it does across a high-RTT link), forwards the request, sleeps, then
 * forwards the response until the server closes.
 *
 * It forwards bytes, never semantics: TLS passes through opaquely, and the
 * only bytes it alters are the Connection header it must own to bound the
 * exchange. Anything it cannot parse it refuses by closing, never by
 * guessing: a half-forwarded exchange is a corruption, not a delay.
 *
 * Usage: delay-proxy <listen-port> <upstream-host> <upstream-port> <delay-ms>
 *
 * Exit: 0 only on a signal stop; 2 on a bad invocation. A refused exchange
 * closes its connection; the proxy itself keeps serving.
 *
 * ⛔ Fast refusal for TLS handshakes. A client that tries `https://`
 * first against a plain-HTTP upstream would hang in its handshake
 * timeout (measured: ~13 s a fetch, serialised or not, which buries
 * the injected delay under timeouts). The proxy peeks the first bytes
 * and closes a handshake at once, so the client's plain-HTTP fallback
 * runs and the measured exchanges are the delayed ones.
 */
#define _POSIX_C_SOURCE 200809L

#include <arpa/inet.h>
#include <errno.h>
#include <netdb.h>
#include <pthread.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <time.h>
#include <unistd.h>

/* The largest request head this proxy will bound: GET headers are small,
 * and anything larger is not a fetch this experiment stages. Failing
 * closed past it is the code.md rule (validate the shape, fail loud). */
#define HEAD_MAX (64 * 1024)
#define CHUNK 8192

static char *upstream_host;
static char *upstream_port;
static unsigned delay_ms;

static void *handle(void *arg);
static ssize_t write_all(int fd, const char *buf, size_t n);
static int read_head(int fd, char *head, size_t *len);
static int force_close(char *head, size_t *len);

int main(int argc, char **argv) {
    if (argc != 5) {
        fprintf(stderr, "usage: %s <listen-port> <upstream-host> <upstream-port> <delay-ms>\n",
                argv[0]);
        return 2;
    }
    upstream_host = argv[2];
    upstream_port = argv[3];
    delay_ms = (unsigned)strtoul(argv[4], NULL, 10);

    signal(SIGPIPE, SIG_IGN);

    int ls = socket(AF_INET, SOCK_STREAM, 0);
    if (ls < 0) {
        perror("socket");
        return 2;
    }
    int one = 1;
    setsockopt(ls, SOL_SOCKET, SO_REUSEADDR, &one, sizeof one);
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof addr);
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    addr.sin_port = htons((uint16_t)strtoul(argv[1], NULL, 10));
    if (bind(ls, (struct sockaddr *)&addr, sizeof addr) < 0) {
        perror("bind");
        return 2;
    }
    if (listen(ls, 64) < 0) {
        perror("listen");
        return 2;
    }
    for (;;) {
        int c = accept(ls, NULL, NULL);
        if (c < 0) {
            if (errno == EINTR) {
                continue;
            }
            perror("accept");
            return 2;
        }
        pthread_t th;
        int *pfd = malloc(sizeof *pfd);
        if (pfd == NULL) {
            close(c);
            continue;
        }
        *pfd = c;
        if (pthread_create(&th, NULL, handle, pfd) != 0) {
            close(c);
            free(pfd);
            continue;
        }
        pthread_detach(th);
    }
}

static void *handle(void *arg) {
    int down = *(int *)arg;
    free(arg);
    char *head = malloc(HEAD_MAX);
    int up = -1;
    if (head == NULL) {
        goto close_down;
    }
    /* Fast refusal for a TLS handshake: the proxy forwards bytes and
     * cannot complete one, and holding it open parks the client in its
     * handshake timeout. A ClientHello opens 0x16 0x03; anything shorter
     * than three bytes is no request either. Either way the close is at
     * once, and the client's fallback (or failure) is its own. */
    {
        unsigned char peek[3];
        ssize_t np = recv(down, (char *)peek, sizeof peek, MSG_PEEK);
        if (np < 3 || (peek[0] == 0x16 && peek[1] == 0x03)) {
            goto out;
        }
    }
    size_t len = 0;
    /* One exchange per connection: read the request head, force its close,
     * forward it, sleep, then forward the response to the server's close.
     * Anything unparsable closes the downstream side having sent nothing:
     * the client's retry (or failure) is honest, a half answer is not. */
    if (read_head(down, head, &len) != 0) {
        goto out;
    }
    if (force_close(head, &len) != 0) {
        goto out;
    }
    struct addrinfo hints;
    memset(&hints, 0, sizeof hints);
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    struct addrinfo *ai = NULL;
    if (getaddrinfo(upstream_host, upstream_port, &hints, &ai) != 0) {
        goto out;
    }
    up = socket(ai->ai_family, ai->ai_socktype, ai->ai_protocol);
    if (up < 0) {
        freeaddrinfo(ai);
        goto out;
    }
    if (connect(up, ai->ai_addr, ai->ai_addrlen) != 0) {
        freeaddrinfo(ai);
        goto out;
    }
    freeaddrinfo(ai);
    if (write_all(up, head, len) < 0) {
        goto out;
    }
    /* The injected delay: every fetch pays it, serialized or overlapped
     * according to the shape under test. Nanosleep, not sleep: the unit
     * is milliseconds and the contract is at least that long. */
    struct timespec ts;
    ts.tv_sec = (time_t)(delay_ms / 1000);
    ts.tv_nsec = (long)(delay_ms % 1000) * 1000000L;
    nanosleep(&ts, NULL);
    for (;;) {
        char buf[CHUNK];
        ssize_t n = read(up, buf, sizeof buf);
        if (n <= 0) {
            break;
        }
        if (write_all(down, buf, (size_t)n) < 0) {
            break;
        }
    }
out:
    free(head);
    if (up >= 0) {
        close(up);
    }
close_down:
    close(down);
    return NULL;
}

static ssize_t write_all(int fd, const char *buf, size_t n) {
    size_t at = 0;
    while (at < n) {
        ssize_t w = write(fd, buf + at, n - at);
        if (w < 0) {
            if (errno == EINTR) {
                continue;
            }
            return -1;
        }
        at += (size_t)w;
    }
    return (ssize_t)at;
}

/* Read until the header terminator, bounded. 0 complete, -1 anything else:
 * EOF, error, or a head past HEAD_MAX (fail closed, per the header note). */
static int read_head(int fd, char *head, size_t *len) {
    size_t at = 0;
    for (;;) {
        if (at >= HEAD_MAX) {
            return -1;
        }
        ssize_t n = read(fd, head + at, HEAD_MAX - at);
        if (n <= 0) {
            return -1;
        }
        at += (size_t)n;
        if (at >= 4) {
            size_t i;
            for (i = 3; i < at; i++) {
                if (head[i - 3] == '\r' && head[i - 2] == '\n' && head[i - 1] == '\r' &&
                    head[i] == '\n') {
                    *len = at;
                    return 0;
                }
            }
        }
    }
}

/* Own the Connection header so the exchange stays one request and one
 * response: replace its value with `close`, or insert it before the
 * terminator where absent. 0 bounded correctly, -1 where the edit would
 * not fit (fail closed, as above). The comparison is case-insensitive
 * over the header name; values are opaque. */
static int force_close(char *head, size_t *len) {
    static const char CLOSE_LINE[] = "Connection: close\r\n";
    size_t n = *len;
    size_t i = 0;
    /* Skip the request line. */
    while (i + 1 < n && !(head[i] == '\r' && head[i + 1] == '\n')) {
        i++;
    }
    if (i + 1 >= n) {
        return -1;
    }
    i += 2;
    while (i + 1 < n) {
        if (head[i] == '\r' && head[i + 1] == '\n') {
            /* End of headers: insert before it. */
            if (n + sizeof CLOSE_LINE - 1 > HEAD_MAX) {
                return -1;
            }
            memmove(head + i + sizeof CLOSE_LINE - 1, head + i, n - i);
            memcpy(head + i, CLOSE_LINE, sizeof CLOSE_LINE - 1);
            *len = n + sizeof CLOSE_LINE - 1;
            return 0;
        }
        size_t eol = i;
        while (eol + 1 < n && !(head[eol] == '\r' && head[eol + 1] == '\n')) {
            eol++;
        }
        if (eol + 1 >= n) {
            return -1;
        }
        size_t namelen = eol - i;
        const char *colon = memchr(head + i, ':', namelen);
        if (colon != NULL) {
            size_t k = (size_t)(colon - (head + i));
            if (k == 10 &&
                (head[i] | 32) == 'c' && (head[i + 1] | 32) == 'o' &&
                (head[i + 2] | 32) == 'n' && (head[i + 3] | 32) == 'n' &&
                (head[i + 4] | 32) == 'e' && (head[i + 5] | 32) == 'c' &&
                (head[i + 6] | 32) == 't' && (head[i + 7] | 32) == 'i' &&
                (head[i + 8] | 32) == 'o' && (head[i + 9] | 32) == 'n') {
                /* Found it: replace the whole line. `close` is shorter
                 * than any legal value this could hold... unless the
                 * value is shorter than `close`, so shift either way. */
                size_t linelen = eol - i + 2;
                size_t want = sizeof CLOSE_LINE - 1;
                if (n - linelen + want > HEAD_MAX) {
                    return -1;
                }
                memmove(head + i + want, head + eol + 2, n - (eol + 2));
                memcpy(head + i, CLOSE_LINE, want);
                *len = n - linelen + want;
                return 0;
            }
        }
        i = eol + 2;
    }
    return -1;
}
