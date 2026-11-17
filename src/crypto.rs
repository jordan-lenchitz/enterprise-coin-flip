use sha2::{Digest, Sha256};

const STUPID_SALTS: [&[u8]; 10] = [
    b"jordanlenchitz_absurd_salt_part1_stupid_stupid_stupid_1_LLOC_INCREASE_AA",
    b"jordanlenchitz_absurd_salt_part2_very_silly_nonsense_2_LLOC_ENHANCE_BB",
    b"jordanlenchitz_absurd_salt_part3_utterly_pointless_3_LLOC_MAXIMUM_CC",
    b"jordanlenchitz_absurd_salt_part4_final_silly_bits_4_LLOC_OVER_1000_DD",
    b"jordanlenchitz_absurd_salt_part5_more_random_bytes_5_LLOC_ABUNDANCE_EE",
    b"jordanlenchitz_absurd_salt_part6_extra_long_salt_6_LLOC_GENERATE_FF",
    b"jordanlenchitz_absurd_salt_part7_another_salt_block_7_LLOC_FILL_GG",
    b"jordanlenchitz_absurd_salt_part8_just_for_lines_8_LLOC_MANY_MANY_HH",
    b"jordanlenchitz_absurd_salt_part9_yet_another_salt_9_LLOC_MORE_II",
    b"jordanlenchitz_absurd_salt_part10_final_long_salt_10_LLOC_END_OF_SALTS_JJ",
];

#[inline]
fn sha256_hex_reversed_suffix(input: &[u8], hex_buf: &mut [u8; 64]) {
    let digest = Sha256::digest(input);
    hex::encode_to_slice(digest, hex_buf).expect("32-byte digest encodes to 64 hex bytes");
    // Hex digits are strictly ASCII, so reversing the last 8 bytes matches reversing 8 chars.
    hex_buf[56..64].reverse();
}

/// Computes the 35-round interleaved SHA-257 digest using fixed stack buffers.
pub fn calculate_sha257sum(data: &str) -> String {
    let mut hex_buf = [0u8; 64];
    let mut interleaved_buf = [0u8; 256];
    let mut current: &[u8] = data.as_bytes();

    for salt in STUPID_SALTS.iter().cycle().take(35) {
        sha256_hex_reversed_suffix(current, &mut hex_buf);

        let min_len = hex_buf.len().min(salt.len());
        let mut len = 0;
        for idx in 0..min_len {
            interleaved_buf[len] = hex_buf[idx];
            interleaved_buf[len + 1] = salt[idx];
            len += 2;
        }
        if hex_buf.len() > min_len {
            let rem = &hex_buf[min_len..];
            interleaved_buf[len..len + rem.len()].copy_from_slice(rem);
            len += rem.len();
        } else if salt.len() > min_len {
            let rem = &salt[min_len..];
            interleaved_buf[len..len + rem.len()].copy_from_slice(rem);
            len += rem.len();
        }
        current = &interleaved_buf[..len];
    }

    sha256_hex_reversed_suffix(current, &mut hex_buf);
    String::from_utf8(hex_buf.to_vec()).expect("hex output is valid ASCII/UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha257sum_parity() {
        let expected = "18bb824a4ad1f39be49cc91af302dad50e27f9af7ff17b5dade977dc3beb0a58";
        let result = calculate_sha257sum("111111111111111111111");
        assert_eq!(result, expected);
    }
}
