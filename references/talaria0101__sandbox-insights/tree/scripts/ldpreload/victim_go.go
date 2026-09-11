// victim_go.go — a Go victim: os.Chown and os.OpenFile issue raw
// syscalls, with or without cgo, so no libc interposer sees them.
// Build: go build -o victim_go victim_go.go
package main

import (
	"fmt"
	"os"
)

func main() {
	f, err := os.OpenFile("/tmp/.ldpreload-victim", os.O_CREATE|os.O_WRONLY, 0644)
	if err == nil {
		f.Close()
	}
	err = os.Lchown("/tmp/.ldpreload-victim", 0, 0)
	fmt.Printf("victim_go: Lchown -> %v (interposer sees none of this)\n", err == nil)
	os.Remove("/tmp/.ldpreload-victim")
}
