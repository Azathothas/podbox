#include <stdio.h>
#include <pwd.h>
int main(void) {
    struct passwd *p = getpwnam("podvmsupplied");
    puts(p ? "FOUND" : "NOTFOUND");
    return p ? 0 : 1;
}
