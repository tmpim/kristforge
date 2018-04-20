// @formatter:off
#ifndef VECSIZE
	#error "Vector size not defined"
#elif VECSIZE == 2 || VECSIZE == 4 || VECSIZE == 8 || VECSIZE == 16
	// two levels of indirection necessary because the c preprocessor is not smart
	#define _PASTE(x, y) x ## y
	#define PASTE(x, y) _PASTE(x, y)

	// types
	#define UCHARV PASTE(uchar, VECSIZE)
	#define UINTV PASTE(uint, VECSIZE)
	#define LONGV PASTE(long, VECSIZE)

	// functions
	#define CONVERT(t, x) PASTE(convert_, t)(x)
	#define VLOAD(x, y) PASTE(vload, VECSIZE)((x), (y))
	#define VSTORE(x, y, z) PASTE(vstore, VECSIZE)((x), (y), (z))
#elif VECSIZE == 1
	// types
	#define UCHARV uchar
	#define UINTV uint
	#define LONGV long

	// functions
	#define CONVERT(t, x) (t)(x)
	#define VLOAD(x, y) (y)[(x)]
	#define VSTORE(x, y, z) (z)[(y)] = (x)
#else
	#error "Invalid vector size"
#endif
// @formatter:on

// right rotate macro
#define RR(x, y) rotate((UINTV)(x), -((UINTV)(y)))

// sha256 macros
#define CH(x, y, z) bitselect((z),(y),(x))
#define MAJ(x, y, z) bitselect((x),(y),(z)^(x))
#define EP0(x) (RR((x),2) ^ RR((x),13) ^ RR((x),22))
#define EP1(x) (RR((x),6) ^ RR((x),11) ^ RR((x),25))
#define SIG0(x) (RR((x),7) ^ RR((x),18) ^ ((x) >> 3))
#define SIG1(x) (RR((x),17) ^ RR((x),19) ^ ((x) >> 10))

// sha256 initial hash values
#define H0 0x6a09e667
#define H1 0xbb67ae85
#define H2 0x3c6ef372
#define H3 0xa54ff53a
#define H4 0x510e527f
#define H5 0x9b05688c
#define H6 0x1f83d9ab
#define H7 0x5be0cd19

// sha256 round constants
// @formatter:off
__constant uint K[64] = {
		0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
		0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
		0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
		0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
		0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
		0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
		0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
		0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2 };
// @formatter:on

// perform a single round of sha256 transformation on the given data
void sha256_transform(UCHARV *data, UINTV *H) {
	int i;
	UINTV a, b, c, d, e, f, g, h, t1, t2, m[64];

#pragma unroll
	for (i = 0; i < 16; i++) {
		m[i] = (CONVERT(UINTV, data[i * 4]) << 24) |
		       (CONVERT(UINTV, data[i * 4 + 1]) << 16) |
		       (CONVERT(UINTV, data[i * 4 + 2]) << 8) |
		       (CONVERT(UINTV, data[i * 4 + 3]));
	}

#pragma unroll
	for (i = 16; i < 64; i++) m[i] = SIG1(m[i - 2]) + m[i - 7] + SIG0(m[i - 15]) + m[i - 16];

	a = H[0];
	b = H[1];
	c = H[2];
	d = H[3];
	e = H[4];
	f = H[5];
	g = H[6];
	h = H[7];

#pragma unroll
	for (i = 0; i < 64; i++) {
		t1 = h + EP1(e) + CH(e, f, g) + K[i] + m[i];
		t2 = EP0(a) + MAJ(a, b, c);
		h = g;
		g = f;
		f = e;
		e = d + t1;
		d = c;
		c = b;
		b = a;
		a = t1 + t2;
	}

	H[0] += a;
	H[1] += b;
	H[2] += c;
	H[3] += d;
	H[4] += e;
	H[5] += f;
	H[6] += g;
	H[7] += h;
}

void sha256_finish(UINTV *H, UCHARV *hash) {
	int l;

#pragma unroll
	for (int i = 0; i < 4; i++) {
		l = 24 - i * 8;
		hash[i] = CONVERT(UCHARV, (H[0] >> l) & 0x000000ff);
		hash[i + 4] = CONVERT(UCHARV, (H[1] >> l) & 0x000000ff);
		hash[i + 8] = CONVERT(UCHARV, (H[2] >> l) & 0x000000ff);
		hash[i + 12] = CONVERT(UCHARV, (H[3] >> l) & 0x000000ff);
		hash[i + 16] = CONVERT(UCHARV, (H[4] >> l) & 0x000000ff);
		hash[i + 20] = CONVERT(UCHARV, (H[5] >> l) & 0x000000ff);
		hash[i + 24] = CONVERT(UCHARV, (H[6] >> l) & 0x000000ff);
		hash[i + 28] = CONVERT(UCHARV, (H[7] >> l) & 0x000000ff);
	}
}

// sha256 digest of up to 55 bytes of input
// uchar data[64] - input bytes - will be modified
// uint inputLen - input length (in bytes)
// uchar hash[32] - output bytes - will be modified
void digest55(UCHARV *data, uint len, UCHARV *hash) {
	// pad input
	data[len] = 0x80;
	data[62] = (len * 8) >> 8;
	data[63] = len * 8;

	// init hash state
	UINTV H[8] = {H0, H1, H2, H3, H4, H5, H6, H7};

	// transform
	sha256_transform(data, H);

	// finish
	sha256_finish(H, hash);
}

// @formatter:off
__kernel void testDigest55(__global uchar *input, uint len, __global uchar *output) {
	UCHARV in[64], out[32];

#pragma unroll
	for (int i = 0; i < 64; i++) in[i] = VLOAD(i, input);

	digest55(in, len, out);

#pragma unroll
	for (int i = 0; i < 32; i++) VSTORE(out[i], i, output);
}

LONGV score_hash(UCHARV *hash) {
	return (CONVERT(LONGV, hash[5])) +
	       (CONVERT(LONGV, hash[4]) << 8) +
	       (CONVERT(LONGV, hash[3]) << 16) +
	       (CONVERT(LONGV, hash[2]) << 24) +
	       (CONVERT(LONGV, hash[1]) << 32) +
	       (CONVERT(LONGV, hash[0]) << 40);
}

long score_hash_scalar(uchar *hash) {
	return (hash[5]) +
	       (hash[4] << 8) +
	       (hash[3] << 16) +
	       (((long)hash[2]) << 24) +
	       (((long)hash[1]) << 32) +
	       (((long)hash[0]) << 40);
}

__kernel void testScore(__global uchar *hash, __global long *scores) {
	UCHARV in[32];

#pragma unroll
	for (int i = 0; i < 32; i++) in[i] = VLOAD(i, hash);

	LONGV score = score_hash(in);

	VSTORE(score, 0, scores);
}

__constant union {
	long scalars[16];
	LONGV vec;
} nonceOffset = { .scalars = { 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15 }};

__constant uchar hex[16] = "0123456789abcdef";

union vectorExtractor {
	UCHARV vector;
	uchar components[VECSIZE];
};

__kernel
__attribute__((vec_type_hint(UINTV)))
void kristMiner(
		__global const uchar *kristAddress,         // 10 bytes
		__global const uchar *block,                // 12 bytes
		__global const uchar *prefix,               // 2 bytes
		const long offset,
		const long work,                            // convert to 13 bytes
		__global uchar *solution) {                  // 15 bytes (prefix + nonce)

	// TODO: figure out why this is slower?
	const LONGV nonce = nonceOffset.vec + (LONGV)(get_global_id(0) * VECSIZE + offset);

	UCHARV input[64] = {0}, hashed[32] = {0};

#pragma unroll
	for (int i = 0; i < 10; i++) input[i] = kristAddress[i];

#pragma unroll
	for (int i = 0; i < 12; i++) input[i+10] = block[i];

#pragma unroll
	for (int i = 0; i < 2; i++) input[i+22] = prefix[i];

#pragma unroll
	for (int i = 0; i < 13; i++) input[i+24] = CONVERT(UCHARV, ((nonce >> (i * 5)) & 0b11111) + 48);

	digest55(input, 37, hashed);

	LONGV score = score_hash(hashed);

#if VECSIZE == 1
	if (score < work) {
#pragma unroll
		for (int i = 0; i < 15; i++) {
			solution[i] = input[i+22];
		}
	}
#else
	if (any(score < work)) {
#pragma unroll
		for (int i = 0; i < VECSIZE; i++) {
			union vectorExtractor *hash = (union vectorExtractor*) hashed;

			uchar start[6] = {0};

#pragma unroll
			for (int j = 0; j < 6; j++) start[j] = hash[j].components[i];

			if (score_hash_scalar(start) < work) {
#pragma unroll
				for (int k = 0; k < 15; k++) solution[k] = ((union vectorExtractor)input[22 + k]).components[i];
			}
		}
	}
#endif
}