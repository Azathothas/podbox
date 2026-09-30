# Shared by the guest proof and the controlled process fixture.
# KVM is the caller's owned, absolute scratch directory.
owned_emulators() {
  process_table=$(ps -eo pid,args) || return 2
  printf '%s\n' "$process_table" | awk -v owned="$KVM/" \
    '$2 ~ /(^|\/)qemu-system-x86_64$/ && index($0, owned) {print $1}'
}
stop_owned_emulators() {
  selected=$(owned_emulators) || return 2
  for pid in $selected; do
    kill "$pid" 2>/dev/null || true
    n=0
    while kill -0 "$pid" 2>/dev/null && [ "$n" -lt 10 ]; do
      sleep 1
      n=$((n + 1))
    done
    kill -KILL "$pid" 2>/dev/null || true
  done
}
