#![allow(
    clippy::unreadable_literal,
    clippy::cast_ptr_alignment,
    overflowing_literals
)]

use super::{score_ab, Kernel, ScalarKernelInput};
use std::arch::x86_64::*;

pub struct SHA;
impl Kernel for SHA {
    type Input = ScalarKernelInput;

    #[inline(always)]
    fn score(&self, input: &ScalarKernelInput) -> u64 {
        let [a, b, _, _, _, _, _, _] = digest(input.data_block());
        score_ab(a, b)
    }
}

/// SHA256 digest of a single padded block of data.
#[inline(always)]
fn digest(data: &[u8; 64]) -> [u32; 8] {
    // initial state
    let mut state = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    unsafe { process(&mut state, &data) };

    state
}

/// Process multiple blocks. The caller is responsible for setting the initial
/// state, and the caller is responsible for padding the final block.
#[target_feature(enable = "sha")]
#[target_feature(enable = "sse4.1")]
unsafe fn process(state: &mut [u32; 8], data: &[u8; 64]) {
    let mask = _mm_set_epi64x(0x0c0d0e0f08090a0b, 0x0405060700010203);

    let mut state0: __m128i;
    let mut state1: __m128i;
    let mut msg: __m128i;
    let mut tmp: __m128i;
    let mut msg0: __m128i;
    let mut msg1: __m128i;
    let mut msg2: __m128i;
    let mut msg3: __m128i;
    let abef_save: __m128i;
    let cdgh_save: __m128i;

    /* Load initial values */
    tmp = _mm_loadu_si128(state.as_ptr() as _);
    state1 = _mm_loadu_si128(state.as_ptr().add(4) as _);

    tmp = _mm_shuffle_epi32(tmp, 0xb1); /* CDAB */
    state1 = _mm_shuffle_epi32(state1, 0x1b); /* EFGH */
    state0 = _mm_alignr_epi8(tmp, state1, 8); /* ABEF */
    state1 = _mm_blend_epi16(state1, tmp, 0xf0); /* CDGH */

    /* Save current state */
    abef_save = state0;
    cdgh_save = state1;

    /* Rounds 0-3 */
    msg = _mm_loadu_si128(data.as_ptr().add(0) as _);
    msg0 = _mm_shuffle_epi8(msg, mask);
    msg = _mm_add_epi32(msg0, _mm_set_epi64x(0xE9B5DBA5B5C0FBCF, 0x71374491428A2F98));
    state1 = _mm_sha256rnds2_epu32(state1, state0, msg);
    msg = _mm_shuffle_epi32(msg, 0x0E);
    state0 = _mm_sha256rnds2_epu32(state0, state1, msg);

    /* Rounds 4-7 */
    msg1 = _mm_loadu_si128(data.as_ptr().add(16) as _);
    msg1 = _mm_shuffle_epi8(msg1, mask);
    msg = _mm_add_epi32(msg1, _mm_set_epi64x(0xAB1C5ED5923F82A4, 0x59F111F13956C25B));
    state1 = _mm_sha256rnds2_epu32(state1, state0, msg);
    msg = _mm_shuffle_epi32(msg, 0x0E);
    state0 = _mm_sha256rnds2_epu32(state0, state1, msg);
    msg0 = _mm_sha256msg1_epu32(msg0, msg1);

    /* Rounds 8-11 */
    msg2 = _mm_loadu_si128(data.as_ptr().add(32) as _);
    msg2 = _mm_shuffle_epi8(msg2, mask);
    msg = _mm_add_epi32(msg2, _mm_set_epi64x(0x550C7DC3243185BE, 0x12835B01D807AA98));
    state1 = _mm_sha256rnds2_epu32(state1, state0, msg);
    msg = _mm_shuffle_epi32(msg, 0x0E);
    state0 = _mm_sha256rnds2_epu32(state0, state1, msg);
    msg1 = _mm_sha256msg1_epu32(msg1, msg2);

    /* Rounds 12-15 */
    msg3 = _mm_loadu_si128(data.as_ptr().add(48) as _);
    msg3 = _mm_shuffle_epi8(msg3, mask);
    msg = _mm_add_epi32(msg3, _mm_set_epi64x(0xC19BF1749BDC06A7, 0x80DEB1FE72BE5D74));
    state1 = _mm_sha256rnds2_epu32(state1, state0, msg);
    tmp = _mm_alignr_epi8(msg3, msg2, 4);
    msg0 = _mm_add_epi32(msg0, tmp);
    msg0 = _mm_sha256msg2_epu32(msg0, msg3);
    msg = _mm_shuffle_epi32(msg, 0x0E);
    state0 = _mm_sha256rnds2_epu32(state0, state1, msg);
    msg2 = _mm_sha256msg1_epu32(msg2, msg3);

    /* Rounds 16-19 */
    msg = _mm_add_epi32(msg0, _mm_set_epi64x(0x240CA1CC0FC19DC6, 0xEFBE4786E49B69C1));
    state1 = _mm_sha256rnds2_epu32(state1, state0, msg);
    tmp = _mm_alignr_epi8(msg0, msg3, 4);
    msg1 = _mm_add_epi32(msg1, tmp);
    msg1 = _mm_sha256msg2_epu32(msg1, msg0);
    msg = _mm_shuffle_epi32(msg, 0x0E);
    state0 = _mm_sha256rnds2_epu32(state0, state1, msg);
    msg3 = _mm_sha256msg1_epu32(msg3, msg0);

    /* Rounds 20-23 */
    msg = _mm_add_epi32(msg1, _mm_set_epi64x(0x76F988DA5CB0A9DC, 0x4A7484AA2DE92C6F));
    state1 = _mm_sha256rnds2_epu32(state1, state0, msg);
    tmp = _mm_alignr_epi8(msg1, msg0, 4);
    msg2 = _mm_add_epi32(msg2, tmp);
    msg2 = _mm_sha256msg2_epu32(msg2, msg1);
    msg = _mm_shuffle_epi32(msg, 0x0E);
    state0 = _mm_sha256rnds2_epu32(state0, state1, msg);
    msg0 = _mm_sha256msg1_epu32(msg0, msg1);

    /* Rounds 24-27 */
    msg = _mm_add_epi32(msg2, _mm_set_epi64x(0xBF597FC7B00327C8, 0xA831C66D983E5152));
    state1 = _mm_sha256rnds2_epu32(state1, state0, msg);
    tmp = _mm_alignr_epi8(msg2, msg1, 4);
    msg3 = _mm_add_epi32(msg3, tmp);
    msg3 = _mm_sha256msg2_epu32(msg3, msg2);
    msg = _mm_shuffle_epi32(msg, 0x0E);
    state0 = _mm_sha256rnds2_epu32(state0, state1, msg);
    msg1 = _mm_sha256msg1_epu32(msg1, msg2);

    /* Rounds 28-31 */
    msg = _mm_add_epi32(msg3, _mm_set_epi64x(0x1429296706CA6351, 0xD5A79147C6E00BF3));
    state1 = _mm_sha256rnds2_epu32(state1, state0, msg);
    tmp = _mm_alignr_epi8(msg3, msg2, 4);
    msg0 = _mm_add_epi32(msg0, tmp);
    msg0 = _mm_sha256msg2_epu32(msg0, msg3);
    msg = _mm_shuffle_epi32(msg, 0x0E);
    state0 = _mm_sha256rnds2_epu32(state0, state1, msg);
    msg2 = _mm_sha256msg1_epu32(msg2, msg3);

    /* Rounds 32-35 */
    msg = _mm_add_epi32(msg0, _mm_set_epi64x(0x53380D134D2C6DFC, 0x2E1B213827B70A85));
    state1 = _mm_sha256rnds2_epu32(state1, state0, msg);
    tmp = _mm_alignr_epi8(msg0, msg3, 4);
    msg1 = _mm_add_epi32(msg1, tmp);
    msg1 = _mm_sha256msg2_epu32(msg1, msg0);
    msg = _mm_shuffle_epi32(msg, 0x0E);
    state0 = _mm_sha256rnds2_epu32(state0, state1, msg);
    msg3 = _mm_sha256msg1_epu32(msg3, msg0);

    /* Rounds 36-39 */
    msg = _mm_add_epi32(msg1, _mm_set_epi64x(0x92722C8581C2C92E, 0x766A0ABB650A7354));
    state1 = _mm_sha256rnds2_epu32(state1, state0, msg);
    tmp = _mm_alignr_epi8(msg1, msg0, 4);
    msg2 = _mm_add_epi32(msg2, tmp);
    msg2 = _mm_sha256msg2_epu32(msg2, msg1);
    msg = _mm_shuffle_epi32(msg, 0x0E);
    state0 = _mm_sha256rnds2_epu32(state0, state1, msg);
    msg0 = _mm_sha256msg1_epu32(msg0, msg1);

    /* Rounds 40-43 */
    msg = _mm_add_epi32(msg2, _mm_set_epi64x(0xC76C51A3C24B8B70, 0xA81A664BA2BFE8A1));
    state1 = _mm_sha256rnds2_epu32(state1, state0, msg);
    tmp = _mm_alignr_epi8(msg2, msg1, 4);
    msg3 = _mm_add_epi32(msg3, tmp);
    msg3 = _mm_sha256msg2_epu32(msg3, msg2);
    msg = _mm_shuffle_epi32(msg, 0x0E);
    state0 = _mm_sha256rnds2_epu32(state0, state1, msg);
    msg1 = _mm_sha256msg1_epu32(msg1, msg2);

    /* Rounds 44-47 */
    msg = _mm_add_epi32(msg3, _mm_set_epi64x(0x106AA070F40E3585, 0xD6990624D192E819));
    state1 = _mm_sha256rnds2_epu32(state1, state0, msg);
    tmp = _mm_alignr_epi8(msg3, msg2, 4);
    msg0 = _mm_add_epi32(msg0, tmp);
    msg0 = _mm_sha256msg2_epu32(msg0, msg3);
    msg = _mm_shuffle_epi32(msg, 0x0E);
    state0 = _mm_sha256rnds2_epu32(state0, state1, msg);
    msg2 = _mm_sha256msg1_epu32(msg2, msg3);

    /* Rounds 48-51 */
    msg = _mm_add_epi32(msg0, _mm_set_epi64x(0x34B0BCB52748774C, 0x1E376C0819A4C116));
    state1 = _mm_sha256rnds2_epu32(state1, state0, msg);
    tmp = _mm_alignr_epi8(msg0, msg3, 4);
    msg1 = _mm_add_epi32(msg1, tmp);
    msg1 = _mm_sha256msg2_epu32(msg1, msg0);
    msg = _mm_shuffle_epi32(msg, 0x0E);
    state0 = _mm_sha256rnds2_epu32(state0, state1, msg);
    msg3 = _mm_sha256msg1_epu32(msg3, msg0);

    /* Rounds 52-55 */
    msg = _mm_add_epi32(msg1, _mm_set_epi64x(0x682E6FF35B9CCA4F, 0x4ED8AA4A391C0CB3));
    state1 = _mm_sha256rnds2_epu32(state1, state0, msg);
    tmp = _mm_alignr_epi8(msg1, msg0, 4);
    msg2 = _mm_add_epi32(msg2, tmp);
    msg2 = _mm_sha256msg2_epu32(msg2, msg1);
    msg = _mm_shuffle_epi32(msg, 0x0E);
    state0 = _mm_sha256rnds2_epu32(state0, state1, msg);

    /* Rounds 56-59 */
    msg = _mm_add_epi32(msg2, _mm_set_epi64x(0x8CC7020884C87814, 0x78A5636F748F82EE));
    state1 = _mm_sha256rnds2_epu32(state1, state0, msg);
    tmp = _mm_alignr_epi8(msg2, msg1, 4);
    msg3 = _mm_add_epi32(msg3, tmp);
    msg3 = _mm_sha256msg2_epu32(msg3, msg2);
    msg = _mm_shuffle_epi32(msg, 0x0E);
    state0 = _mm_sha256rnds2_epu32(state0, state1, msg);

    /* Rounds 60-63 */
    msg = _mm_add_epi32(msg3, _mm_set_epi64x(0xC67178F2BEF9A3F7, 0xA4506CEB90BEFFFA));
    state1 = _mm_sha256rnds2_epu32(state1, state0, msg);
    msg = _mm_shuffle_epi32(msg, 0x0E);
    state0 = _mm_sha256rnds2_epu32(state0, state1, msg);

    /* Combine state  */
    state0 = _mm_add_epi32(state0, abef_save);
    state1 = _mm_add_epi32(state1, cdgh_save);

    tmp = _mm_shuffle_epi32(state0, 0x1B); /* FEBA */
    state1 = _mm_shuffle_epi32(state1, 0xB1); /* DCHG */
    state0 = _mm_blend_epi16(tmp, state1, 0xF0); /* DCBA */
    state1 = _mm_alignr_epi8(state1, tmp, 8); /* ABEF */

    /* Save state */
    _mm_storeu_si128(state.as_ptr() as _, state0);
    _mm_storeu_si128(state.as_ptr().add(4) as _, state1);
}
