// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (c) 2026 ReCraft contributors

// M1 corpus: a small Go binary used to validate pclntab function/name recovery.

package main

import (
	"fmt"
	"strings"
)

func helper(value int) int {
	if value < 0 {
		return -value
	}
	return value * 2
}

func add(left, right int) int {
	return left + right
}

func classify(value int) string {
	switch {
	case value < 0:
		return "negative"
	case value == 0:
		return "zero"
	default:
		return "positive"
	}
}

func joinParts(parts ...string) string {
	return strings.Join(parts, "/")
}

func main() {
	total := add(helper(-3), helper(5))
	fmt.Println(total, classify(total), joinParts("recraft", "go", "corpus"))
}
