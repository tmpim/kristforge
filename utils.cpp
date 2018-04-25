#include "utils.h"

#include <openssl/sha.h>

static const char hex[] = "0123456789abcdef";

std::string toHex(const unsigned char *data, size_t len) {
	std::string output;

	for (int i = 0; i < len; i++) {
		output += hex[data[i] >> 4];
		output += hex[data[i] & 0xf];
	}

	return output;
}

std::string toHex(const std::string &input) {
	std::string output;

	for (const unsigned char c : input) {
		output += hex[c >> 4];
		output += hex[c & 0xf];
	}

	return output;
}

std::string mkString(const unsigned char *data, size_t len) {
	std::string out(len, ' ');
	for (int i = 0; i < len; i++) out[i] = data[i];
	return out;
}

std::string sha256(const std::string &data) {
	unsigned char hashed[SHA256_DIGEST_LENGTH];
	SHA256(reinterpret_cast<const unsigned char *>(data.data()), data.size(), hashed);
	return mkString(hashed, SHA256_DIGEST_LENGTH);
}


std::string sha256hex(const std::string &data) {
	unsigned char hashed[SHA256_DIGEST_LENGTH];
	SHA256(reinterpret_cast<const unsigned char *>(data.data()), data.size(), hashed);
	return toHex(hashed, SHA256_DIGEST_LENGTH);
}

long scoreHash(const std::string &hash) {
	const auto *raw = reinterpret_cast<const unsigned char *>(hash.data());

	return ((long)raw[5]) + (((long)raw[4]) << 8) + (((long)raw[3]) << 16) + (((long)raw[2]) << 24) + (((long) raw[1]) << 32) + (((long) raw[0]) << 40);
}