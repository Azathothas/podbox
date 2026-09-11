// spawn_matrix.go — which os/exec SysProcAttr shapes make Go call
// setgroups in the child, measured against a runtime where setgroups
// is denied.
//
// Go's syscall.forkExec issues setgroups in the child whenever
// Credential is non-nil, unless (GidMappings != nil &&
// !GidMappingsEnableSetgroups && ngroups == 0) or Credential.NoSetGroups
// is set. On a runtime whose user namespace denies setgroups
// (setgroups=deny in the userns contract), a non-nil Credential fails
// the spawn with a bare "operation not permitted" — the same string a
// refused clone produces, which is why the two are so easy to confuse.
//
// The one-field fix is NoSetGroups: true — a no-op on unrestricted
// hosts. Dropping Credential is NOT required and changes behaviour.
//
// Exit: 0 matrix taken. Run: go run spawn_matrix.go
package main

import (
	"fmt"
	"os"
	"os/exec"
	"syscall"
)

type row struct {
	name string
	mk   func() *exec.Cmd
}

func cmdWith(attr func(*syscall.SysProcAttr)) *exec.Cmd {
	c := exec.Command("/bin/true")
	c.SysProcAttr = &syscall.SysProcAttr{}
	attr(c.SysProcAttr)
	return c
}

func main() {
	fmt.Println("## Go os/exec spawn matrix against setgroups-denied runtime")
	fmt.Println("## (results below; identical failure string from setgroups and")
	fmt.Println("##  from a refused clone is the ambiguity this matrix documents)")
	rows := []row{
		{"nil SysProcAttr", func() *exec.Cmd { return exec.Command("/bin/true") }},
		{"{} no Credential", func() *exec.Cmd { return cmdWith(func(s *syscall.SysProcAttr) {}) }},
		{"Credential{0,0}", func() *exec.Cmd {
			return cmdWith(func(s *syscall.SysProcAttr) { s.Credential = &syscall.Credential{Uid: 0, Gid: 0} })
		}},
		{"Credential{0,0} + EnableSetgroups", func() *exec.Cmd {
			return cmdWith(func(s *syscall.SysProcAttr) {
				s.Credential = &syscall.Credential{Uid: 0, Gid: 0}
				s.GidMappingsEnableSetgroups = true
			})
		}},
		{"Credential{0,0,NoSetGroups:true}", func() *exec.Cmd {
			return cmdWith(func(s *syscall.SysProcAttr) {
				s.Credential = &syscall.Credential{Uid: 0, Gid: 0, NoSetGroups: true}
			})
		}},
		{"Credential{0,0,NoSetGroups:true}+Pdeathsig", func() *exec.Cmd {
			return cmdWith(func(s *syscall.SysProcAttr) {
				s.Credential = &syscall.Credential{Uid: 0, Gid: 0, NoSetGroups: true}
				s.Pdeathsig = syscall.SIGTERM
			})
		}},
		{"Cloneflags(CLONE_NEWNS) only", func() *exec.Cmd {
			return cmdWith(func(s *syscall.SysProcAttr) { s.Cloneflags = syscall.CLONE_NEWNS })
		}},
		{"Cloneflags(CLONE_NEWNS|NEWUTS) only", func() *exec.Cmd {
			return cmdWith(func(s *syscall.SysProcAttr) { s.Cloneflags = syscall.CLONE_NEWNS | syscall.CLONE_NEWUTS })
		}},
		{"Cloneflags(CLONE_NEWNS) + Credential{0,0}", func() *exec.Cmd {
			return cmdWith(func(s *syscall.SysProcAttr) {
				s.Cloneflags = syscall.CLONE_NEWNS
				s.Credential = &syscall.Credential{Uid: 0, Gid: 0}
			})
		}},
		{"Cloneflags(CLONE_NEWNS) + Credential{0,0,NoSetGroups}", func() *exec.Cmd {
			return cmdWith(func(s *syscall.SysProcAttr) {
				s.Cloneflags = syscall.CLONE_NEWNS
				s.Credential = &syscall.Credential{Uid: 0, Gid: 0, NoSetGroups: true}
			})
		}},
	}
	failed := 0
	for _, r := range rows {
		c := r.mk()
		err := c.Run()
		if err != nil {
			failed++
			fmt.Printf("%-46s FAIL %v\n", r.name, err)
		} else {
			fmt.Printf("%-46s OK\n", r.name)
		}
	}
	fmt.Printf("\n%d of %d rows failed (rows that fail are the setgroups callers)\n", failed, len(rows))
	os.Exit(0)
}
