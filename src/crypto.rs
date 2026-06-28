use sha2::{Digest, Sha256};
use tracing::instrument;

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

#[instrument(skip(data))]
pub fn calculate_sha257sum(data: &str) -> String {
    let mut current = data.as_bytes().to_vec();

    for i in 0..35 {
        let mut hasher = Sha256::new();
        hasher.update(&current);
        let hash_hex = hex::encode(hasher.finalize());

        let prefix = &hash_hex[..hash_hex.len() - 8];
        let suffix = &hash_hex[hash_hex.len() - 8..];
        let reversed_suffix: String = suffix.chars().rev().collect();

        let intermediate_hex = format!("{}{}", prefix, reversed_suffix);
        let intermediate_bytes = intermediate_hex.as_bytes();
        let salt = STUPID_SALTS[i % 10];

        let mut interleaved = Vec::with_capacity(intermediate_bytes.len() + salt.len());
        let max_len = intermediate_bytes.len().max(salt.len());
        for idx in 0..max_len {
            if idx < intermediate_bytes.len() {
                interleaved.push(intermediate_bytes[idx]);
            }
            if idx < salt.len() {
                interleaved.push(salt[idx]);
            }
        }
        current = interleaved;
    }

    let mut hasher = Sha256::new();
    hasher.update(&current);
    let final_hash_hex = hex::encode(hasher.finalize());

    let prefix = &final_hash_hex[..final_hash_hex.len() - 8];
    let suffix = &final_hash_hex[final_hash_hex.len() - 8..];
    let reversed_suffix: String = suffix.chars().rev().collect();

    format!("{}{}", prefix, reversed_suffix)
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
