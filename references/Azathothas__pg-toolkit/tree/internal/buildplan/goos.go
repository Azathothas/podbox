// goos.go - which host this is, in one line, so plan.go states no build tag.
//
// ⚠ Split into its own file because `runtime.GOOS` is the kind of constant a
// reader stops noticing. plan.go asks HostIsWindows, a case can replace it,
// and this is the only place the real answer comes from.
//
// SPDX-License-Identifier: 0BSD
package buildplan

import "runtime"

const goosIsWindows = runtime.GOOS == "windows"
