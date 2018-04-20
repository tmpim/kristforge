#include "utils.h"

#include <openssl/sha.h>

static const char hex[] = "0123456789abcdef";

/** Convert binary data to hex representation */
std::string toHex(const unsigned char *data, size_t len) {
	std::string output;

	for (int i = 0; i < len; i++) {
		output += hex[data[i] >> 4];
		output += hex[data[i] & 0xf];
	}

	return output;
}

/** Convert binary string to hex representation */
std::string toHex(const std::string &input) {
	std::string output;

	for (const unsigned char c : input) {
		output += hex[c >> 4];
		output += hex[c & 0xf];
	}

	return output;
}

/** Compute sha256 and return hex representation */
std::string sha256hex(const std::string &data) {
	unsigned char hashed[SHA256_DIGEST_LENGTH];
	SHA256(reinterpret_cast<const unsigned char *>(data.data()), data.size(), hashed);
	return toHex(hashed, SHA256_DIGEST_LENGTH);
}

std::string mkString(const unsigned char *data, size_t len) {
	std::string out(len, ' ');
	for (int i = 0; i < len; i++) out[i] = data[i];
	return out;
}
