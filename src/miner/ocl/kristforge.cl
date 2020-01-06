// right rotate macro
#define RR(x, y) rotate((uint)(x), -(uint)(y))

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
constant uint K[64] = {
	0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
	0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
	0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
	0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
	0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
	0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
	0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
	0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
};

kernel void mine(
	constant const uchar *input,    // address + prev block - 22 bytes
	const ulong work,               // target work
	const ulong offset,             // id offset
	global uchar *solution          // solution nonce - 11 bytes
) {
	// initialize hash input array
	uchar text[64] = { 0 };

	// fill first 22 bytes of hash input
#pragma unroll
	for (int i = 0; i < 22; i++) text[i] = input[i];

	// expand id into next 11 bytes
	ulong id = get_global_id(0) + offset;

#pragma unroll
	for (int i = 0; i < 11; i++) { text[i + 22] = ((id >> (i * 6)) & 0x3f) + 32; }

	// padding - digest input is 33 bytes
	text[33] = 0x80;
	text[62] = ((33 * 8) >> 8) & 0xff;
	text[63] =  (33 * 8)       & 0xff;

	uint a, b, c, d, e, f, g, h, t1, t2, m[64];

	// message extension
#pragma unroll
	for (int i = 0; i < 16; i++) {
		m[i] =  text[i * 4    ] << 24 |
				text[i * 4 + 1] << 16 |
				text[i * 4 + 2] << 8 |
				text[i * 4 + 3];
	}

#pragma unroll
	for (int i = 16; i < 64; i++) {
		m[i] = SIG1(m[i - 2]) + m[i - 7] + SIG0(m[i - 15]) + m[i - 16];
	}

	// transformation
	a = H0;
	b = H1;
	c = H2;
	d = H3;
	e = H4;
	f = H5;
	g = H6;
	h = H7;

#pragma unroll
	for (int i = 0; i < 64; i++) {
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

	// only need the first 6 bytes of the hash output
	a += H0;
	b += H1;

	ulong score =   (((ulong)(a >> 24)) & 0xff) << 40 |
					(((ulong)(a >> 16)) & 0xff) << 32 |
					(((ulong)(a >>  8)) & 0xff) << 24 |
					(((ulong)(a      )) & 0xff) << 16 |
					(((ulong)(b >> 24)) & 0xff) << 8  |
					(((ulong)(b >> 16)) & 0xff);

	if (score <= work) {
		// solution found!
		// copy nonce to solution buffer
#pragma unroll
		for (int i = 0; i < 11; i++) {
			solution[i] = text[i + 22];
		}
	}
}
