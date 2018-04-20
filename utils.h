#pragma once

#include <string>
#include <sstream>
#include <stdexcept>

/** Converts the given binary data to a hex string */
std::string toHex(const unsigned char *data, size_t len);

/** Converts the binary data of the string to a hex string */
std::string toHex(const std::string &data);

/** Compute SHA256 of given string and return hex representation */
std::string sha256hex(const std::string &data);

/** Throw an exception if given inputs aren't equal */
template<typename T>
void assertEquals(const T &expected, const T &got, const std::string &message) {
	if (!(expected == got)) {
		std::ostringstream msgStream;
		msgStream << message << " - got " << got << ", expected " << expected;
		throw std::runtime_error(msgStream.str());
	}
}

/** Convert binary data to a string */
std::string mkString(const unsigned char *data, size_t len);