/*
 * Simple Crackme v2.0  --  C edition
 *
 * A tiny crackme: it accepts exactly one input string.
 * Build:  see ../tools/build.ps1   (or: cc -O2 -s -o crackme crackme.c)
 */
#include <stdio.h>
#include <string.h>

#define FLAG_LEN 32
#define KEY_LEN  16

/* --- flag #1: the plaintext string everyone finds first ------------------ */
static const char legacy_flag[] = "flag{strings_are_not_enough_you_know}";

/* --- the XOR key. Rotates over the flag, one byte at a time -------------- */
static const char key_str[KEY_LEN + 1] = "C_CR4CK_ME_2024!";

/* --- the flag, shuffled into this order before encryption ---------------- */
static const unsigned char perm[FLAG_LEN] = {
	 2, 25, 17, 18, 26, 15,  9, 12, 23,  1, 14, 10, 19, 22,  6,  0,
	 5, 31, 13, 30, 20, 28,  3, 21, 24,  7,  4, 16, 29, 27, 11,  8,
};

/* --- and here it is, encrypted ------------------------------------------
 *   enc[i] = ((flag[perm[i]] ^ key_str[i % KEY_LEN]) + perm[i]) & 0xFF
 */
static const unsigned char enc[FLAG_LEN] = {
	0x24, 0x50, 0x81, 0x48, 0x1f, 0x82, 0x36, 0x47, 0x29, 0x2a, 0x35, 0x68,
	0x82, 0x6c, 0x62, 0x47, 0x35, 0x41, 0x29, 0x54, 0x14, 0x53, 0x2f, 0x46,
	0x56, 0x37, 0x28, 0x50, 0x20, 0x6f, 0x12, 0x4f,
};

/*
 * Returns 1 only for the real flag.
 *   1. the candidate must be exactly FLAG_LEN bytes long,
 *   2. every index i must satisfy the relation that produced enc[i].
 */
static int verify(const char *cand)
{
	size_t i;

	if (strlen(cand) != FLAG_LEN)
		return 0;

	for (i = 0; i < FLAG_LEN; i++) {
		unsigned char v;

		v = (unsigned char)cand[perm[i]] ^ (unsigned char)key_str[i % KEY_LEN];
		v = (unsigned char)(v + perm[i]);

		if (v != enc[i])
			return 0;
	}
	return 1;
}

int main(void)
{
	char buf[256];
	int tries;

	puts("==========================================");
	puts("  Simple Crackme v2.0  (C edition)");
	puts("  flag format: flag{...}");
	puts("==========================================");

	for (tries = 0; tries < 3; tries++) {
		char *nl;

		printf("Enter flag: ");
		fflush(stdout);

		if (fgets(buf, sizeof(buf), stdin) == NULL)
			break;

		nl = strpbrk(buf, "\r\n");
		if (nl != NULL)
			*nl = '\0';

		/* Values accepted by the previous release no longer count. */
		if (strcmp(buf, legacy_flag) == 0) {
			puts("[-] Wrong flag. Try again.");
			continue;
		}

		if (verify(buf)) {
			puts("[+] Correct! Nice reversing.");
			return 0;
		}
		puts("[-] Wrong flag. Try again.");
	}

	puts("[?] Hint: it is shuffled, xored, and shifted.");
	return 1;
}
