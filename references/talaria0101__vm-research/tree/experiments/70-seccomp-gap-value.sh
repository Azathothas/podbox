#!/usr/bin/env bash
# Question: does the seccomp filter gap (clone/clone3 with namespace flags are
# unfiltered, 60-) have PRACTICAL value? Three demonstrations, each a facility
# the sandbox denies at unshare-level:
#   a) private NET namespace owned by the nested userns -> veth pair + addresses
#      + loopback + in-ns ping (CAP_NET_ADMIN over the netns OWNER userns; the
#      parent userns does NOT own the host netns, so this is impossible outside
#      the gap);
#   b) private PID namespace -> PID1 child, isolated process tree;
#   c) chroot into a prepared Alpine rootfs (its own /etc/passwd — fixes the
#      sandbox's missing-passwd problem for anything running inside).
# Exit codes: 0 at least one demonstration succeeded, 1 all failed, 2 could not run.
set -u
. "$(dirname "$0")/lib.sh"
LOG="$VR_LOGS/70-seccomp-gap-value.log"
{ echo "## conditions"; conditions seccomp-gap-value; } > "$LOG"

# prepare an alpine chroot appliance (its own /etc/passwd)
CH="$VR_WORK/alpine-chroot"
if [ ! -x "$CH/bin/sh" ]; then
  MR="$VR_WORK/alpine-minirootfs-3.22.5-x86_64.tar.gz"
  [ -f "$MR" ] || curl -sL --max-time 300 -o "$MR" \
    "https://dl-cdn.alpinelinux.org/alpine/v3.22/releases/x86_64/alpine-minirootfs-3.22.5-x86_64.tar.gz"
  mkdir -p "$CH"; tar --no-same-owner -xzf "$MR" -C "$CH"
  mkdir -p "$CH"/{dev,proc,sys,tmp}
  echo 'root:x:0:0:root:/root:/bin/sh' > "$CH/etc/passwd"
  echo 'root:x:0:' > "$CH/etc/group"
fi

cat > "$VR_WORK/nsvalue2.c" <<'EOF'
/* Practical value of the seccomp gap (clone3 NEWUSER|NEWNET unfiltered):
   a process that enters its own userns+netns in-process holds CAP_NET_ADMIN
   over that netns and can build a PRIVATE NETWORK FABRIC (veth pair, addresses,
   ICMP across it) with no exec (exec would drop caps: uid is unmapped).
   No host root, no host capabilities involved. */
#define _GNU_SOURCE
#include <stdio.h>
#include <string.h>
#include <errno.h>
#include <unistd.h>
#include <sched.h>
#include <stdlib.h>
#include <sys/wait.h>
#include <sys/socket.h>
#include <sys/ioctl.h>
#include <netinet/in.h>
#include <netinet/ip_icmp.h>
#include <arpa/inet.h>
#include <linux/netlink.h>
#include <linux/rtnetlink.h>

#include <linux/veth.h>
#include <linux/if_link.h>
#include <net/if.h>
#include <time.h>
#define STACK (256*1024)
static char stk[STACK];

static int nlsock(void){
  int fd = socket(AF_NETLINK, SOCK_RAW|SOCK_CLOEXEC, NETLINK_ROUTE);
  struct sockaddr_nl sa = { .nl_family = AF_NETLINK };
  bind(fd, (struct sockaddr*)&sa, sizeof sa);
  return fd;
}
static int add_attr(struct nlmsghdr *n, int max, int type, const void *d, int len){
  struct rtattr *r = (void*)n + NLMSG_ALIGN(n->nlmsg_len);
  int ta = RTA_LENGTH(len);
  if (n->nlmsg_len + NLMSG_ALIGN(ta) > max) return -1;
  r->rta_type = type; r->rta_len = ta; memcpy(RTA_DATA(r), d, len);
  n->nlmsg_len += NLMSG_ALIGN(ta);
  return 0;
}
static int add_attr_nested(struct nlmsghdr *n, int max, int type){
  struct rtattr *r = (void*)n + NLMSG_ALIGN(n->nlmsg_len);
  if (n->nlmsg_len + NLMSG_ALIGN(RTA_LENGTH(0)) > max) return -1;
  r->rta_type = type; r->rta_len = RTA_LENGTH(0);
  n->nlmsg_len += NLMSG_ALIGN(RTA_LENGTH(0));
  return 0;
}
static void end_nested(struct nlmsghdr *n, struct rtattr *r){
  r->rta_len = (void*)n + NLMSG_ALIGN(n->nlmsg_len) - (void*)r;
}
static int create_veth(int fd){
  char buf[4096]; struct nlmsghdr *n = (void*)buf;
  memset(buf, 0, sizeof buf);
  n->nlmsg_len = NLMSG_LENGTH(sizeof(struct ifinfomsg));
  n->nlmsg_type = RTM_NEWLINK;
  n->nlmsg_flags = NLM_F_REQUEST | NLM_F_ACK | NLM_F_CREATE | NLM_F_EXCL;
  struct ifinfomsg *ifi = NLMSG_DATA(n);
  ifi->ifi_family = AF_UNSPEC;
  /* IFLA_LINKINFO nest */
  struct rtattr *li = (void*)n + NLMSG_ALIGN(n->nlmsg_len);
  li->rta_type = IFLA_LINKINFO; li->rta_len = RTA_LENGTH(0);
  n->nlmsg_len += NLMSG_ALIGN(li->rta_len);
  /* IFLA_INFO_KIND = "veth" */
  struct rtattr *k = (void*)n + NLMSG_ALIGN(n->nlmsg_len);
  k->rta_type = IFLA_INFO_KIND; k->rta_len = RTA_LENGTH(5);
  memcpy(RTA_DATA(k), "veth", 5);
  n->nlmsg_len += NLMSG_ALIGN(k->rta_len);
  /* IFLA_INFO_DATA nest */
  struct rtattr *idata = (void*)n + NLMSG_ALIGN(n->nlmsg_len);
  idata->rta_type = IFLA_INFO_DATA; idata->rta_len = RTA_LENGTH(0);
  n->nlmsg_len += NLMSG_ALIGN(idata->rta_len);
  /* VETH_INFO_PEER: payload = ifinfomsg + IFLA_IFNAME */
  struct rtattr *peer = (void*)n + NLMSG_ALIGN(n->nlmsg_len);
  struct ifinfomsg pifi = { .ifi_family = AF_UNSPEC };
  peer->rta_type = VETH_INFO_PEER;
  peer->rta_len = RTA_LENGTH(sizeof pifi + RTA_LENGTH(6));
  memcpy(RTA_DATA(peer), &pifi, sizeof pifi);
  n->nlmsg_len += NLMSG_ALIGN(peer->rta_len);
  struct rtattr *in = (void*)RTA_DATA(peer) + sizeof pifi;
  in->rta_type = IFLA_IFNAME; in->rta_len = RTA_LENGTH(6);
  memcpy(RTA_DATA(in), "veth1", 6);
  /* close nests (reverse order) */
  idata->rta_len = (void*)n + NLMSG_ALIGN(n->nlmsg_len) - (void*)idata;
  n->nlmsg_len = NLMSG_ALIGN(n->nlmsg_len);
  li->rta_len = (void*)n + NLMSG_ALIGN(n->nlmsg_len) - (void*)li;
  n->nlmsg_len = NLMSG_ALIGN(n->nlmsg_len);

  struct sockaddr_nl sa = { .nl_family = AF_NETLINK };
  sendto(fd, n, n->nlmsg_len, 0, (struct sockaddr*)&sa, sizeof sa);
  char resp[4096]; int r = recv(fd, resp, sizeof resp, 0);
  struct nlmsghdr *rn = (void*)resp;
  if (r < 0 || rn->nlmsg_type == NLMSG_ERROR) {
    struct nlmsgerr *e = NLMSG_DATA(rn);
    return e->error;
  }
  return 0;
}
static int ifup_addr(const char *name, const char *ip){
  int fd = socket(AF_INET, SOCK_DGRAM, 0);
  struct ifreq ir; memset(&ir,0,sizeof ir);
  snprintf(ir.ifr_name, IFNAMSIZ, "%s", name);
  struct sockaddr_in *a = (void*)&ir.ifr_addr;
  a->sin_family = AF_INET; inet_pton(AF_INET, ip, &a->sin_addr);
  ioctl(fd, SIOCSIFADDR, &ir);
  a->sin_addr.s_addr = htonl(0xFFFFFF00); /* 255.255.255.0 */
  ioctl(fd, SIOCSIFNETMASK, &ir);
  ioctl(fd, SIOCGIFFLAGS, &ir);
  ir.ifr_flags |= IFF_UP|IFF_RUNNING;
  ioctl(fd, SIOCSIFFLAGS, &ir);
  close(fd);
  return 0;
}
static int ping_icmp(const char *dst){
  int fd = socket(AF_INET, SOCK_RAW, IPPROTO_ICMP);
  struct sockaddr_in d = { .sin_family = AF_INET };
  inet_pton(AF_INET, dst, &d.sin_addr);
  char pkt[64]; memset(pkt, 0, sizeof pkt);
  struct icmphdr *h = (void*)pkt;
  h->type = ICMP_ECHO;
  for (int i = 8; i < 64; i++) pkt[i] = i;
  h->checksum = 0;
  unsigned sum = 0;
  for (int i = 0; i < 64; i += 2) sum += (unsigned char)pkt[i] << 8 | (unsigned char)pkt[i+1];
  sum = (sum >> 16) + (sum & 0xffff); sum += sum >> 16;
  h->checksum = htons((unsigned short)(~sum & 0xffff));
  struct timespec t0; clock_gettime(CLOCK_MONOTONIC, &t0);
  printf("[netns] hdr: %02x %02x ck=%04x  first16: ", h->type, h->code, h->checksum);
  for (int i=0;i<16;i++) printf("%02x", (unsigned char)pkt[i]);
  printf("\n");
  int sr = sendto(fd, pkt, 64, 0, (void*)&d, sizeof d);
  printf("[netns] sendto=%d (%s)\n", sr, sr < 0 ? strerror(errno) : "ok");
  struct timeval tv = { .tv_sec = 3, .tv_usec = 0 };
  setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof tv);
  char rb[512];
  int n, got = 0;
  while ((n = recv(fd, rb, sizeof rb, 0)) > 0) {
    struct iphdr *ip = (void*)rb;
    struct icmphdr *ih = (void*)(rb + ip->ihl*4);
    unsigned sum2 = 0; unsigned char *b = (void*)ih;
    ih->checksum = 0;
    for (int i = 0; i < (int)(n - ip->ihl*4); i += 2)
      sum2 += (unsigned char)b[i] << 8 | (unsigned char)b[i+1];
    sum2 = (sum2 >> 16) + (sum2 & 0xffff); sum2 += sum2 >> 16;
    unsigned ck = (~sum2) & 0xffff;
    printf("[netns] rx icmp type=%d from=%x checksum_field=%04x recomputed=%04x %s\n",
           ih->type, ip->saddr, ih->checksum, ck, ck==0?"VALID":"INVALID");
    if (ih->type == ICMP_ECHOREPLY) { got = 1; break; }
  }
  struct timespec t1; clock_gettime(CLOCK_MONOTONIC, &t1);
  printf("[netns] ICMP echo to %s over private veth: %s (%.3fs)\n", dst,
         got ? "REPLY RECEIVED" : "no reply",
         (t1.tv_sec-t0.tv_sec)+(t1.tv_nsec-t0.tv_nsec)/1e9);
  close(fd);
  return got ? 0 : 1;
}
static int child(void *a){
  printf("[netns] inside private netns (in-process, no exec)\n");
  int fd = nlsock();
  int rc = create_veth(fd);
  printf("[netns] veth create rc=%d (%s)\n", rc, rc==0?"OK":strerror(-rc));
  if (rc != 0) return 1;
  ifup_addr("veth0", "10.200.0.1");
  ifup_addr("veth1", "10.200.0.2");
  printf("[netns] addresses set on both ends\n");
  { int fd = socket(AF_INET, SOCK_DGRAM, 0);
    struct ifreq ir; memset(&ir,0,sizeof ir); snprintf(ir.ifr_name,IFNAMSIZ,"veth0");
    ioctl(fd, SIOCGIFFLAGS, &ir); printf("[dbg] veth0 flags=%04x\n", ir.ifr_flags);
    memset(&ir,0,sizeof ir); snprintf(ir.ifr_name,IFNAMSIZ,"veth1");
    ioctl(fd, SIOCGIFFLAGS, &ir); printf("[dbg] veth1 flags=%04x\n", ir.ifr_flags);
    close(fd); }
  { FILE *f = fopen("/proc/net/route","r"); char l[256];
    printf("[dbg] routes:\n");
    while (f && fgets(l,sizeof l,f)) printf("  %s", l);
    if (f) fclose(f); }
  ifup_addr("lo", "127.0.0.1");
  ping_icmp("127.0.0.1");
  { FILE *f = fopen("/proc/net/snmp","r");
    char l[512];
    while (f && fgets(l,sizeof l,f)) {
      if (l[0]=='I' && l[1]=='c' && l[2]=='m') printf("[snmp] %s", l);
    }
    if (f) fclose(f); }
  ping_icmp("10.200.0.2");
  fflush(stdout);
  return 0;
}
int main(void){ setvbuf(stdout, NULL, _IONBF, 0);
  pid_t p = clone(child, stk+STACK, CLONE_NEWUSER|CLONE_NEWNET|SIGCHLD, NULL);
  if (p < 0){ printf("clone: %s\n", strerror(errno)); return 1; }
  int st; waitpid(p, &st, 0);
  printf("[p] child exit=%d\n", WEXITSTATUS(st));
  return 0;
}
EOF
gcc -O0 -o "$VR_WORK/nsvalue2" "$VR_WORK/nsvalue2.c" 2>> "$LOG" || { echo "build failed" >> "$LOG"; exit 2; }
"$VR_WORK/nsvalue2" >> "$LOG" 2>&1
RC=$?
echo "--- summary:" >> "$LOG"
grep -aE 'veth add rc|packet loss|VMR-CHROOT|pidns|clone\(' "$LOG" | head -10 >> "$LOG"
grep -aE 'REPLY RECEIVED|VMR-CHROOT-OK|pidns\] PID' "$LOG" > /dev/null && { tail -10 "$LOG"; exit 0; }
tail -8 "$LOG"; exit 1
