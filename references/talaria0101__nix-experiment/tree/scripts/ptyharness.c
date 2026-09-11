/* Mimics nix 2.3.18's exact pty usage in startBuilder/commonChildInit:
   parent: posix_openpt -> ptsname -> open(slave) -> tcgetattr/tcsetattr(raw)
           -> fork -> child: setsid, dup2(slave,2), closeMostFDs-ish, exec sh
           parent: close slave, read master until EOF */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <pty.h>
#include <termios.h>
#include <fcntl.h>
#include <sys/wait.h>

int main(void) {
    int master = posix_openpt(O_RDWR | O_NOCTTY);
    if (master < 0) { perror("posix_openpt"); return 1; }
    char slaveName[128];
    if (ptsname_r(master, slaveName, sizeof slaveName) != 0) { perror("ptsname_r"); return 1; }
    printf("harness: emulated slave path = %s\n", slaveName);
    if (unlockpt(master) < 0) { perror("unlockpt"); return 1; }
    int slave = open(slaveName, O_RDWR | O_NOCTTY);
    if (slave < 0) { perror("open slave"); return 1; }
    struct termios t;
    if (tcgetattr(slave, &t) < 0) { perror("tcgetattr"); return 1; }
    cfmakeraw(&t);
    if (tcsetattr(slave, TCSANOW, &t) < 0) { perror("tcsetattr"); return 1; }

    pid_t pid = fork();
    if (pid == 0) {
        setsid();
        dup2(slave, STDERR_FILENO);
        dup2(STDERR_FILENO, STDOUT_FILENO);
        int nul = open("/dev/null", O_RDWR);
        if (nul >= 0) { dup2(nul, STDIN_FILENO); if (nul > 2) close(nul); }
        if (slave > 2) close(slave);
        if (master > 2) close(master);
        execl("/bin/sh", "sh", "-c", "echo pty-shim-works-from-execed-child >&2", (char *)0);
        _exit(127);
    }
    close(slave); /* nix does this too, right after childStarted */
    char buf[256];
    ssize_t n;
    size_t total = 0;
    while ((n = read(master, buf, sizeof buf)) > 0) { total += n; fwrite(buf, 1, n, stdout); }
    int st; waitpid(pid, &st, 0);
    printf("harness: eof after %zu bytes, child exit=%d\n", total, WEXITSTATUS(st));
    return total > 0 ? 0 : 2;
}
