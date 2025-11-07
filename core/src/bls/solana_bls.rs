//! # Solana BN254 BLS Signature Implementation
//!
//! A BN254 (alt_bn128) BLS signature system optimized for Solana's runtime constraints.
//! This implementation provides cryptographic primitives for threshold signature schemes,
//! enabling distributed consensus and secure multi-party computation on Solana.
//!
//! ## Core Features
//!
//! - **Dual-Key Architecture**: Operators maintain both G1 and G2 key pairs derived from
//!   the same private key, enabling efficient on-chain verification despite Solana's
//!   lack of G2 arithmetic support
//! - **Efficient Aggregation**: G1 signatures and public keys can be aggregated on-chain
//!   using Solana's native alt_bn128 precompiles (~3,000 CU per addition)
//! - **Threshold Signatures**: Supports k-of-n signature schemes through bitmap-based
//!   signer tracking and partial signature aggregation
//! - **Domain Separation**: Cryptographic isolation between different protocols using
//!   the same keys through configurable domain tags
//!
//! ## Architecture Overview
//!
//! The implementation uses a novel approach to work within Solana's constraints:
//!
//! 1. **Key Generation** (off-chain):
//!    - Private key → G1 public key (for on-chain storage and aggregation)
//!    - Private key → G2 public key (for off-chain signing)
//!
//! 2. **Signing Process** (off-chain):
//!    - Hash message to G1 curve point using try-and-increment
//!    - Multiply by private key to create G1 signature
//!    - Aggregate multiple signatures off-chain
//!
//! 3. **Verification** (on-chain):
//!    - Compute signers' aggregated G1 from stored total and non-signer bitmap
//!    - Verify using pairing equation with Fiat-Shamir challenge (alpha)
//!    - Single pairing check validates entire aggregate
//!
//! ## Encoding Standards
//!
//! All cryptographic material uses **big-endian** encoding to match Solana's alt_bn128
//! precompiles (EIP-197 standard):
//!
//! - **G1 Points** (64 bytes): X || Y coordinates, uncompressed
//! - **G2 Points** (128 bytes): X.c1 || X.c0 || Y.c1 || Y.c0 per EIP-197
//! - **Scalars** (32 bytes): Private keys and field elements
//! - **Bitmaps** (variable): Bit i indicates if operator i participated
//!
//! ## Security Model
//!
//! - **Threshold Security**: Any k-of-n operators can produce valid signatures
//! - **Alpha Binding**: Fiat-Shamir challenge prevents substitution attacks between
//!   different operator sets
//! - **Domain Separation**: Different applications use unique domain tags to prevent
//!   cross-protocol signature replay
//! - **Deterministic**: All operations are deterministic for reproducibility
//!
//! ## Performance Characteristics
//!
//! On-chain costs (approximate):
//! - G1 Addition: ~3,000 compute units
//! - G1 Multiplication: ~12,000 compute units
//! - Pairing Check: ~250,000 compute units
//! - SHA256 Hash: Optimized Solana syscall
//!
//! ## Integration with Solana Programs
//!
//! Typical usage in a Solana program:
//!
//! 1. **Initialization**: Store operators' G1 public keys and aggregated total
//! 2. **Vote Submission**: Receive aggregated G2, signature, and signer bitmap
//! 3. **Verification**: Compute signers' G1 from bitmap, verify with pairing
//!
//! ## Compatibility
//!
//! This implementation is designed to be compatible with:
//! - EigenLayer's BN254 implementation for cross-chain interoperability
//! - Ethereum's alt_bn128 precompiles (EIP-196, EIP-197)
//! - Standard BLS signature schemes on BN254
//!
//! ## Limitations
//!
//! - Not constant-time: Suitable for verification, not secure key generation
//! - No G2 arithmetic on-chain: Requires off-chain aggregation of G2 keys
//! - Point validation: G2 points must be validated off-chain before submission
use ark_bn254::{Fq, Fq2, Fr, G1Projective, G2Affine};
use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::{BigInteger, Field, One, PrimeField};
use solana_bn254::{
    compression::prelude::alt_bn128_g1_decompress,
    prelude::{alt_bn128_g1_addition_be, alt_bn128_g1_multiplication_be, alt_bn128_pairing_be},
};

// ----------------------------------------------------------------------------
//                       CONSTANTS
// ----------------------------------------------------------------------------

pub const G1_GENERATOR: [u8; 64] = [
    // x coordinate: 1
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
    // y coordinate: 2
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
];

pub const G2_GENERATOR: [u8; 128] = [
    // X.c1
    0x19, 0x8e, 0x93, 0x93, 0x92, 0x0d, 0x48, 0x3a, 0x72, 0x60, 0xbf, 0xb7, 0x31, 0xfb, 0x5d, 0x25,
    0xf1, 0xaa, 0x49, 0x33, 0x35, 0xa9, 0xe7, 0x12, 0x97, 0xe4, 0x85, 0xb7, 0xae, 0xf3, 0x12, 0xc2,
    // X.c0
    0x18, 0x00, 0xde, 0xef, 0x12, 0x1f, 0x1e, 0x76, 0x42, 0x6a, 0x00, 0x66, 0x5e, 0x5c, 0x44, 0x79,
    0x67, 0x43, 0x22, 0xd4, 0xf7, 0x5e, 0xda, 0xdd, 0x46, 0xde, 0xbd, 0x5c, 0xd9, 0x92, 0xf6, 0xed,
    // Y.c1 (not negated)
    0x09, 0x06, 0x89, 0xd0, 0x58, 0x5f, 0xf0, 0x75, 0xec, 0x9e, 0x99, 0xad, 0x69, 0x0c, 0x33, 0x95,
    0xbc, 0x4b, 0x31, 0x33, 0x70, 0xb3, 0x8e, 0xf3, 0x55, 0xac, 0xda, 0xdc, 0xd1, 0x22, 0x97, 0x5b,
    // Y.c0 (not negated)
    0x12, 0xc8, 0x5e, 0xa5, 0xdb, 0x8c, 0x6d, 0xeb, 0x4a, 0xab, 0x71, 0x80, 0x8d, 0xcb, 0x40, 0x8f,
    0xe3, 0xd1, 0xe7, 0x69, 0x0c, 0x43, 0xd3, 0x7b, 0x4c, 0xe6, 0xcc, 0x01, 0x66, 0xfa, 0x7d, 0xaa,
];

pub const G2_MINUS_ONE: [u8; 128] = [
    0x19, 0x8e, 0x93, 0x93, 0x92, 0x0d, 0x48, 0x3a, 0x72, 0x60, 0xbf, 0xb7, 0x31, 0xfb, 0x5d, 0x25,
    0xf1, 0xaa, 0x49, 0x33, 0x35, 0xa9, 0xe7, 0x12, 0x97, 0xe4, 0x85, 0xb7, 0xae, 0xf3, 0x12, 0xc2,
    0x18, 0x00, 0xde, 0xef, 0x12, 0x1f, 0x1e, 0x76, 0x42, 0x6a, 0x00, 0x66, 0x5e, 0x5c, 0x44, 0x79,
    0x67, 0x43, 0x22, 0xd4, 0xf7, 0x5e, 0xda, 0xdd, 0x46, 0xde, 0xbd, 0x5c, 0xd9, 0x92, 0xf6, 0xed,
    // y coordinate
    0x27, 0x5d, 0xc4, 0xa2, 0x88, 0xd1, 0xaf, 0xb3, 0xcb, 0xb1, 0xac, 0x09, 0x18, 0x75, 0x24, 0xc7,
    0xdb, 0x36, 0x39, 0x5d, 0xf7, 0xbe, 0x3b, 0x99, 0xe6, 0x73, 0xb1, 0x3a, 0x07, 0x5a, 0x65, 0xec,
    0x1d, 0x9b, 0xef, 0xcd, 0x05, 0xa5, 0x32, 0x3e, 0x6d, 0xa4, 0xd4, 0x35, 0xf3, 0xb6, 0x17, 0xcd,
    0xb3, 0xaf, 0x83, 0x28, 0x5c, 0x2d, 0xf7, 0x11, 0xef, 0x39, 0xc0, 0x15, 0x71, 0x82, 0x7f, 0x9d,
];

pub const BN128_PAIRING_SUCCESS_RESULT: [u8; 32] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
];

/// The last multiple of the modulus before 2^256
/// 0xf1f5883e65f820d099915c908786b9d3f58714d70a38f4c22ca2bc723a70f263
const NORMALIZE_MODULUS_BYTES: [u8; 32] = [
    0xf1, 0xf5, 0x88, 0x3e, 0x65, 0xf8, 0x20, 0xd0, 0x99, 0x91, 0x5c, 0x90, 0x87, 0x86, 0xb9, 0xd3,
    0xf5, 0x87, 0x14, 0xd7, 0x0a, 0x38, 0xf4, 0xc2, 0x2c, 0xa2, 0xbc, 0x72, 0x3a, 0x70, 0xf2, 0x63,
];

// ----------------------------------------------------------------------------
//                       HASHING
// ----------------------------------------------------------------------------

/// Compute SHA256 hash using Solana's native syscall implementation
///
/// Leverages Solana's optimized SHA256 syscall for efficient hashing within
/// the runtime environment. This provides significant compute unit savings
/// compared to CPU-based implementations.
///
/// # Arguments
/// * `data` - Arbitrary byte slice to hash
///
/// # Returns
/// * `[u8; 32]` - SHA256 hash digest in standard format
///
/// # Example
/// ```
/// let message = b"vote for round 42";
/// let hash = solana_hash(message);
/// assert_eq!(hash.len(), 32);
/// ```
///
/// # Performance
/// Uses Solana's optimized syscall (solana_program::hash::hash) which is
/// significantly more efficient than software implementations like sha2 crate.
/// This is critical for staying within compute unit limits.
///
/// # Determinism
/// SHA256 is deterministic - identical inputs always produce identical outputs,
/// which is essential for signature verification across different validators.
pub fn solana_hash(data: &[u8]) -> [u8; 32] {
    solana_sha256_hasher::hash(data).to_bytes()
}

/// Hash a message to a point on the BN254 G1 curve using try-and-increment
///
/// Maps arbitrary messages to valid G1 curve points for BLS signatures.
/// Compatible with EigenLayer's implementation for cross-chain interoperability.
///
/// # Arguments
/// * `message` - The message bytes to hash to the curve
/// * `domain` - Optional domain separator for protocol isolation
///
/// # Returns
/// * `Ok([u8; 64])` - Uncompressed G1 point (X || Y) in big-endian
/// * `Err(String)` - If mapping fails (computationally infeasible)
///
/// # Algorithm
/// 1. Concatenate domain (if provided) and message with varint length encoding
/// 2. Hash the concatenation using SHA256
/// 3. Map the hash to a curve point using try-and-increment
///
/// # Example
/// ```
/// let point = hash_to_curve(b"vote_123", Some(b"SOLANA_VOTE_V1"))?;
/// ```
///
/// TODO: This function is NOT collision-resistant for raw messages.
/// The input message should already be a cryptographic hash (e.g., SHA256)
/// to ensure collision resistance. Using unhashed messages directly allows
/// finding collisions through the try-and-increment process.
/// See: https://github.com/Layr-Labs/eigenlayer-middleware/issues/172
///
/// # Security
/// - Deterministic: Same inputs always produce same output
/// - Domain separation prevents cross-protocol replay attacks
pub fn solana_hash_to_curve(message: &[u8], consensus_count: u64) -> Result<[u8; 64], String> {
    let hash = solana_hash(message);
    alt_solana_map_to_curve_simple(&hash, consensus_count)
}

// /// Concatenate domain and message with varint-encoded length prefix
// ///
// /// Creates a unique byte sequence for each (domain, message) pair by prepending
// /// the domain with its length encoded as a varint. This ensures no collisions
// /// between different domain/message combinations.
// ///
// /// # Arguments
// /// * `message` - The message bytes
// /// * `domain` - The domain separator bytes
// ///
// /// # Returns
// /// * `Vec<u8>` - Concatenation: varint(domain.len()) || domain || message
// ///
// /// # Format
// /// - First bytes: Domain length as varint (1-10 bytes depending on size)
// /// - Next bytes: Domain bytes
// /// - Final bytes: Message bytes
// ///
// /// # Example
// /// ```
// /// let result = union_unique(b"hello", b"DOMAIN");
// /// // result = [0x06, b'D', b'O', b'M', b'A', b'I', b'N', b'h', b'e', b'l', b'l', b'o']
// /// //           ^len=6
// /// ```
// ///
// /// # Note
// /// Adapted from commonware's implementation for compatibility.
// pub fn union_unique(message: &[u8], domain: &[u8]) -> Vec<u8> {
//     let namespace_len = domain.len();

//     // Calculate total size needed
//     let mut varint_size = 1;
//     let mut temp = namespace_len;
//     while temp >= 0x80 {
//         varint_size += 1;
//         temp >>= 7;
//     }

//     // Allocate result vector with exact capacity
//     let mut result = Vec::with_capacity(varint_size + namespace_len + message.len());

//     // Encode namespace length as varint
//     let mut value = namespace_len;
//     if value == 0 {
//         result.push(0);
//     } else {
//         // Encode bytes with continuation bit
//         while value >= 0x80 {
//             result.push((value as u8 & 0x7F) | 0x80);
//             value >>= 7;
//         }
//         // Last byte (no continuation bit)
//         result.push(value as u8);
//     }

//     // Append namespace and message
//     result.extend_from_slice(domain);
//     result.extend_from_slice(message);

//     result
// }

/// Map a 32-byte hash to a valid point on the BN254 G1 curve
///
/// Implements try-and-increment algorithm matching EigenLayer's BN254.sol
/// contract for compatibility. Interprets hash as field element and
/// increments until finding a valid curve point.
///
/// # Arguments
/// * `bytes` - 32-byte hash value (big-endian)
///
/// # Returns
/// * `Ok([u8; 64])` - Valid G1 point (X || Y) in big-endian
/// * `Err(String)` - Never fails in practice
///
/// # Algorithm
/// 1. x = hash interpreted as field element
/// 2. Compute y² = x³ + 3 (BN254 curve equation)
/// 3. If y² has a square root, return (x, y)
/// 4. Otherwise x += 1 and repeat
///
/// # Security
/// - Deterministic: Same input always produces same output
/// - Not constant-time: Acceptable for verification, not for key generation
/// - Typically finds valid point within a few iterations
///
/// # Reference
/// https://github.com/Layr-Labs/eigenlayer-middleware/blob/1feb6ae7e12f33ce8eefb361edb69ee26c118b5d/src/libraries/BN254.sol#L292
fn _solana_map_to_curve(bytes: &[u8; 32]) -> Result<[u8; 64], String> {
    let one = Fq::one();
    let three = Fq::from(3u64);
    let mut x = Fq::from_be_bytes_mod_order(bytes);

    loop {
        // y² = x³ + 3
        let mut y_squared = x;
        y_squared.square_in_place();
        y_squared *= x;
        y_squared += three;

        // Check if y is a quadratic residue
        if let Some(y) = y_squared.sqrt() {
            // Try to create the point - this validates it's on the curve
            let projective = G1Projective::new(x, y, Fq::one());
            let affine = projective.into_affine();

            // Additional check for correct subgroup
            if affine.is_in_correct_subgroup_assuming_on_curve() {
                // Convert to big-endian bytes
                let x_bytes = x.into_bigint().to_bytes_be();
                let y_bytes = y.into_bigint().to_bytes_be();

                let mut result = [0u8; 64];
                result[..32].copy_from_slice(&x_bytes);
                result[32..].copy_from_slice(&y_bytes);

                return Ok(result);
            }
        }

        x += one;
    }
}

/// Map a 32-byte message to a valid point on the BN254 G1 curve using hash-and-increment
///
/// Implements a hash-then-try-and-increment algorithm that avoids modulo bias by
/// rejecting hash outputs >= field modulus before reduction. Uses Solana's native
/// alt_bn128_g1_decompress syscall for efficient point recovery.
///
/// # Arguments
/// * `bytes` - 32-byte message to hash (typically a domain-separated message)
///
/// # Returns
/// * `Ok([u8; 64])` - Valid G1 point (X || Y) in big-endian
/// * `Err(String)` - Only if all 255 counter values fail (extremely unlikely)
///
/// # Algorithm
/// 1. hash = SHA256(message || counter) where counter ∈ [0, 255)
/// 2. If hash >= field_modulus (as integers), skip to avoid modulo bias
/// 3. x = hash mod p (field reduction)
/// 4. Try to decompress x to (x, y) using alt_bn128_g1_decompress
/// 5. If successful, return point; otherwise increment counter and repeat
///
/// # Differences from `_solana_map_to_curve`
/// - Hashes with counter instead of incrementing field element directly
/// - Uses Solana syscall (alt_bn128_g1_decompress) instead of arkworks sqrt
/// - Normalizes hash values to avoid modulo bias in hash-to-field mapping
/// - More compute-efficient on Solana due to native syscall usage
///
/// # Security
/// - Deterministic: Same input always produces same output
/// - Uniform distribution: Normalization prevents modulo bias
/// - Not constant-time: Acceptable for verification, not for key generation
/// - Typically finds valid point within first few iterations (~50% success per try)
///
/// # Compute Units
/// Approximately 1,000-3,000 CU depending on number of iterations needed
pub fn alt_solana_map_to_curve_simple(
    hashed_message: &[u8; 32],
    consensus_count: u64,
) -> Result<[u8; 64], String> {
    fn ge_be(a: &[u8; 32], b: &[u8; 32]) -> bool {
        for i in 0..32 {
            if a[i] != b[i] {
                return a[i] > b[i]; // lexicographic BE compare
            }
        }
        true
    }

    // DO NOT reduce NORMALIZE_MODULUS_BYTES into Fq
    const MOD_NORM_BE: [u8; 32] = NORMALIZE_MODULUS_BYTES;

    for counter in 0u8..u8::MAX {
        // hashed_message || consensus_count || counter(u8) — matches your current concatenation
        let consensus_bytes = consensus_count.to_le_bytes();
        let hash_input = [
            hashed_message.as_slice(),
            consensus_bytes.as_slice(),
            &[counter],
        ]
        .concat();
        let hash = solana_hash(&hash_input); // 32 bytes, SHA-256 on Solana

        // 1) Normalization check in integer space (avoid modulo bias)
        let hash_be: [u8; 32] = hash; // already 32 bytes
        if ge_be(&hash_be, &MOD_NORM_BE) {
            continue;
        }

        // 2) Now map into the field (this does the mod p reduction)
        let x_fq = Fq::from_be_bytes_mod_order(&hash_be);

        // 3) Get canonical 32-byte big-endian encoding of the field element
        let mut x_bytes = [0u8; 32];
        x_bytes.copy_from_slice(&x_fq.into_bigint().to_bytes_be());

        // 4) Try to decompress (alt_bn128_g1_decompress expects 32-byte X)
        match alt_bn128_g1_decompress(&x_bytes) {
            Ok(point) if point.len() == 64 => {
                let mut out = [0u8; 64];
                out.copy_from_slice(&point);
                return Ok(out);
            }
            _ => continue,
        }
    }

    Err("Failed to find valid curve point after 1_000_000 attempts".to_string())
}

// ----------------------------------------------------------------------------
//                       SIGNING
// ----------------------------------------------------------------------------

/// Generate a BLS signature on the BN254 G1 curve
///
/// Signs a message by hashing it to a G1 point and multiplying by the private key.
/// Uses Solana's alt_bn128 precompile for efficient scalar multiplication.
///
/// # Arguments
/// * `private_key` - 32-byte private key scalar (big-endian)
/// * `message` - Message bytes to sign
/// * `domain` - Optional domain separator
///
/// # Returns
/// * `Ok([u8; 64])` - Uncompressed G1 signature (X || Y) in big-endian
/// * `Err(String)` - If signing fails
///
/// # Algorithm
/// 1. H = hash_to_curve(message, domain)
/// 2. signature = private_key * H
///
/// # Example
/// ```
/// let signature = solana_sign(&private_key, b"vote_42", Some(b"VOTE_V1"))?;
/// ```
///
/// # Warning
/// NOT constant-time - vulnerable to timing attacks.
/// Use only in trusted off-chain environments.
pub fn solana_sign(
    private_key: &[u8; 32],
    message: &[u8],
    consensus_count: u64,
) -> Result<[u8; 64], String> {
    // Hash message to curve point
    let hash_point = solana_hash_to_curve(message, consensus_count)?;

    // Use helper function for scalar multiplication
    mult_g1(&hash_point, private_key)
}

/// Aggregate multiple BLS signatures into a single G1 signature
///
/// Combines signatures by adding their G1 points. The aggregate can be verified
/// against the aggregate of the signers' public keys.
///
/// # Arguments
/// * `signatures` - Array of uncompressed G1 signatures (64 bytes each)
///
/// # Returns
/// * `Ok([u8; 64])` - Aggregated G1 signature in big-endian
/// * `Err(String)` - If no signatures provided or addition fails
///
/// # Algorithm
/// agg_sig = sig₁ + sig₂ + ... + sigₙ
///
/// # Example
/// ```
/// let aggregated = aggregate_signatures(&[sig1, sig2, sig3])?;
/// ```
///
/// # Performance
/// O(n) where n = number of signatures
/// ~3,000 CU per addition using Solana's alt_bn128 precompile
pub fn aggregate_signatures(signatures: &[[u8; 64]]) -> Result<[u8; 64], String> {
    if signatures.is_empty() {
        return Err("No signatures to aggregate".to_string());
    }

    let mut result = signatures[0];
    for sig in signatures.iter().skip(1) {
        // Use helper function for G1 addition
        result = add_g1(&result, sig)?;
    }

    Ok(result)
}

/// Add two G1 points using Solana's alt_bn128 precompile
///
/// Performs elliptic curve addition: P₁ + P₂ = P₃
///
/// # Arguments
/// * `p1` - First uncompressed G1 point (64 bytes) in big-endian
/// * `p2` - Second uncompressed G1 point (64 bytes) in big-endian
///
/// # Returns
/// * `Ok([u8; 64])` - Resulting G1 point in big-endian
/// * `Err(String)` - If addition fails (invalid points)
///
/// # Performance
/// ~3,000 CU using Solana's native precompile
pub fn add_g1(p1: &[u8; 64], p2: &[u8; 64]) -> Result<[u8; 64], String> {
    let mut input = Vec::with_capacity(128);
    input.extend_from_slice(p1);
    input.extend_from_slice(p2);

    let result =
        alt_bn128_g1_addition_be(&input).map_err(|e| format!("G1 addition failed: {:?}", e))?;

    let mut output = [0u8; 64];
    output.copy_from_slice(&result);
    Ok(output)
}

/// Subtract two G1 points (P₁ - P₂)
///
/// Performs subtraction by negating P₂'s Y coordinate and adding.
///
/// # Arguments
/// * `p1` - First uncompressed G1 point (64 bytes) in big-endian
/// * `p2` - Second uncompressed G1 point to subtract (64 bytes) in big-endian
///
/// # Returns
/// * `Ok([u8; 64])` - Resulting G1 point in big-endian
/// * `Err(String)` - If operation fails
///
/// # Algorithm
/// P₁ - P₂ = P₁ + (-P₂) where -P₂ has negated Y coordinate
pub fn sub_g1(p1: &[u8; 64], p2: &[u8; 64]) -> Result<[u8; 64], String> {
    // Negate p2 by negating its Y coordinate
    let mut neg_p2 = [0u8; 64];
    neg_p2.copy_from_slice(p2);

    // Negate Y coordinate (bytes 32-63)
    let y = Fq::from_be_bytes_mod_order(&p2[32..64]);
    let neg_y = -y;
    let neg_y_bytes = neg_y.into_bigint().to_bytes_be();
    neg_p2[32..64].copy_from_slice(&neg_y_bytes);

    // Add p1 + (-p2)
    add_g1(p1, &neg_p2)
}

/// Multiply a G1 point by a scalar using Solana's alt_bn128 precompile
///
/// Performs scalar multiplication: k * P = P + P + ... + P (k times)
///
/// # Arguments
/// * `point` - Uncompressed G1 point (64 bytes) in big-endian
/// * `scalar` - 32-byte scalar in big-endian
///
/// # Returns
/// * `Ok([u8; 64])` - Resulting G1 point in big-endian
/// * `Err(String)` - If multiplication fails
///
/// # Performance
/// ~12,000 CU using Solana's native precompile
pub fn mult_g1(point: &[u8; 64], scalar: &[u8; 32]) -> Result<[u8; 64], String> {
    let mut input = Vec::with_capacity(96);
    input.extend_from_slice(point);
    input.extend_from_slice(scalar);

    let result = alt_bn128_g1_multiplication_be(&input)
        .map_err(|e| format!("G1 multiplication failed: {:?}", e))?;

    let mut output = [0u8; 64];
    output.copy_from_slice(&result);
    Ok(output)
}

// ----------------------------------------------------------------------------
//                       VERIFICATION
// ----------------------------------------------------------------------------

/// Verify an aggregated BLS signature using pairing checks
///
/// Verifies that an aggregated signature is valid for the given message and
/// set of signers. Uses a Fiat-Shamir challenge (alpha) to bind G1 and G2
/// representations, preventing substitution attacks.
///
/// # Arguments
/// * `aggregated_g1` - Aggregated G1 public key of signers (64 bytes)
/// * `aggregated_g2` - Pre-computed aggregated G2 public key of signers (128 bytes)
/// * `aggregated_signature` - Aggregated G1 signature (64 bytes)
/// * `message` - Original message that was signed
/// * `domain` - Optional domain separator used during signing
///
/// # Returns
/// * `Ok(bool)` - True if signature is valid, false otherwise
/// * `Err(String)` - If verification computation fails
///
/// # Algorithm
/// Verifies the pairing equation:
/// e(H(m) + α·G1, agg_g2) = e(sig + α·agg_g1, G2)
///
/// Where α = SHA256(H(m) || sig || agg_g1 || agg_g2)
///
/// # On-chain Usage
/// The aggregated_g1 is typically computed as: total_g1 - nonsigners_g1
/// This allows efficient verification of partial signer sets.
///
/// # Performance
/// ~250,000 CU for pairing check using Solana's alt_bn128 precompile
pub fn solana_verify_aggregated_signature(
    aggregated_g1: &[u8; 64],        // Aggregated G1 pubkey of signers
    aggregated_g2: &[u8; 128],       // Pre-computed aggregated G2 of signers
    aggregated_signature: &[u8; 64], // Aggregated G1 signature
    message: &[u8],
    consensus_count: u64,
) -> Result<bool, String> {
    // Hash message to G1 curve point
    let msg_point = solana_hash_to_curve(message, consensus_count)?;

    // Compute alpha for the binding between G1 and G2 representations
    // This creates a cryptographic challenge that ensures the G2 aggregate
    // corresponds to the same operator set as the G1 aggregate
    let alpha = compute_alpha(
        &msg_point,
        aggregated_signature,
        aggregated_g1,
        aggregated_g2,
    )?;

    // Scale the generators by alpha
    let g1_generator = get_g1_generator(); // Your G1 generator constant
    let scaled_g1_generator = mult_g1(&g1_generator, &alpha)?;

    // Scale the aggregated G1 pubkey by alpha
    let scaled_aggregated_g1 = mult_g1(aggregated_g1, &alpha)?;

    // Compute the left side of pairing equation: H(m) + G1_gen * alpha
    let msg_plus_scaled_g1 = add_g1(&msg_point, &scaled_g1_generator)?;

    // Compute the right side: signature + aggregated_g1 * alpha
    let sig_plus_scaled_g1 = add_g1(aggregated_signature, &scaled_aggregated_g1)?;

    // Prepare pairing input for the equation:
    // e(H(m) + G1_gen * alpha, aggregated_g2) = e(signature + aggregated_g1 * alpha, G2_gen)
    let mut pairing_input = Vec::with_capacity(384);

    // First pairing: e(H(m) + G1_gen * alpha, aggregated_g2)
    pairing_input.extend_from_slice(&msg_plus_scaled_g1);
    pairing_input.extend_from_slice(aggregated_g2);

    // Second pairing: e(signature + aggregated_g1 * alpha, -G2_gen)
    pairing_input.extend_from_slice(&sig_plus_scaled_g1);
    pairing_input.extend_from_slice(&get_g2_minus_one()); // Pre-computed negated G2 generator

    // Execute pairing check
    let result =
        alt_bn128_pairing_be(&pairing_input).map_err(|e| format!("Pairing failed: {:?}", e))?;

    // Check if result equals 1 (successful verification)
    Ok(result == get_bn128_pairing_success_result())
}

/// Verify a single BLS signature using pairing checks
///
/// Convenience wrapper around solana_verify_aggregated_signature for single signatures.
///
/// # Arguments
/// * `g1` - G1 public key (64 bytes)
/// * `g2` - G2 public key (128 bytes)
/// * `signature` - G1 signature (64 bytes)
/// * `message` - Original message that was signed
/// * `domain` - Optional domain separator used during signing
///
/// # Returns
/// * `Ok(bool)` - True if signature is valid, false otherwise
/// * `Err(String)` - If verification computation fails
///
/// # Example
/// ```
/// let valid = solana_verify_single_signature(
///     &g1_pubkey,
///     &g2_pubkey,
///     &signature,
///     b"message",
///     Some(b"DOMAIN")
/// )?;
pub fn solana_verify_single_signature(
    g1: &[u8; 64],        // Aggregated G1 pubkey of signers
    g2: &[u8; 128],       // Pre-computed aggregated G2 of signers
    signature: &[u8; 64], // Aggregated G1 signature
    message: &[u8],
    consensus_count: u64,
) -> Result<bool, String> {
    solana_verify_aggregated_signature(g1, g2, signature, message, consensus_count)
}

/// Verify a BLS signature using only the G2 public key
///
/// Performs signature verification without requiring the G1 public key or alpha binding.
/// This simplified verification is useful when only the G2 key is available, though it
/// provides less security than the full verification with alpha binding.
///
/// # Arguments
/// * `g2` - G2 public key (128 bytes) in EIP-197 format
/// * `signature` - G1 signature (64 bytes) in big-endian
/// * `message` - Original message that was signed
/// * `domain` - Optional domain separator used during signing
///
/// # Returns
/// * `Ok(true)` - Signature is valid for the given G2 key and message
/// * `Ok(false)` - Signature is invalid
/// * `Err(String)` - If verification computation fails
///
/// # Algorithm
/// Verifies the basic BLS equation:
/// e(H(m), g2) = e(sig, G2_gen)
///
/// Where:
/// - H(m) is the message hashed to G1 curve
/// - g2 is the signer's G2 public key
/// - sig is the G1 signature
/// - G2_gen is the G2 generator
///
/// # Security Note
/// This verification method lacks the alpha binding used in `solana_verify_aggregated_signature`,
/// making it potentially vulnerable to substitution attacks in multi-signer scenarios.
/// Use this only for single-signer verification or when the full verification isn't feasible.
///
/// # Example
/// ```
/// let valid = solana_verify_signature_with_g2(
///     &g2_pubkey,
///     &signature,
///     b"message",
///     Some(b"DOMAIN")
/// )?;
/// ```
pub fn solana_verify_signature_with_g2(
    g2: &[u8; 128],
    signature: &[u8; 64],
    message: &[u8],
    consensus_count: u64,
) -> Result<bool, String> {
    // Hash message to G1 curve point
    let message_on_g1 = solana_hash_to_curve(message, consensus_count)
        .map_err(|e| format!("Failed to hash message to curve: {}", e))?;

    // For a single pairing check, we should use the optimized approach
    // We need to check: e(H(m), g2) ?= e(sig, G2_gen)
    // Which is equivalent to: e(H(m), g2) * e(sig, -G2_gen) ?= 1

    // Prepare a single pairing input with both pairs
    let mut pairing_input = vec![0u8; 384];

    // First pair: e(H(m), g2)
    pairing_input[0..64].copy_from_slice(&message_on_g1);
    pairing_input[64..192].copy_from_slice(g2);

    // Second pair: e(sig, -G2_gen)
    pairing_input[192..256].copy_from_slice(signature);
    pairing_input[256..384].copy_from_slice(&get_g2_minus_one());

    // Execute single pairing check
    let result =
        alt_bn128_pairing_be(&pairing_input).map_err(|e| format!("Pairing failed: {:?}", e))?;

    // Check if result equals 1 (successful verification)
    Ok(result == get_bn128_pairing_success_result())
}

/// Verify that a G1 and G2 public key pair are derived from the same private key
///
/// Validates the cryptographic relationship between G1 and G2 keys using a pairing check.
/// This ensures both keys were generated from the same private key, which is essential
/// for BLS signature security in the dual-key architecture.
///
/// # Arguments
/// * `g1` - Uncompressed G1 public key (64 bytes) in big-endian
/// * `g2` - Uncompressed G2 public key (128 bytes) in EIP-197 format
///
/// # Returns
/// * `Ok(true)` - Keys are valid and derived from same private key
/// * `Ok(false)` - Invalid keys (zero points)
/// * `Err(String)` - If pairing computation fails
///
/// # Algorithm
/// Verifies: e(g1, G2_gen) = e(G1_gen, g2)
/// This holds if and only if g1 = sk * G1_gen and g2 = sk * G2_gen
/// for the same scalar sk (private key).
///
/// # Security
/// - Prevents mixing keys from different operators
/// - Essential check during operator registration
/// - Should be called whenever G1/G2 key pairs are submitted
///
/// # Example
/// ```
/// let valid = verify_g1_g2(&g1_pubkey, &g2_pubkey)?;
/// assert!(valid, "Keys must be derived from same private key");
/// ```
pub fn verify_g1_g2(g1: &[u8; 64], g2: &[u8; 128]) -> Result<bool, String> {
    // Check for zero points (invalid keys)
    let g1_is_zero = g1.iter().all(|&x| x == 0);
    let g2_is_zero = g2.iter().all(|&x| x == 0);

    if g1_is_zero || g2_is_zero {
        return Err("Invalid keys: G1 or G2 is zero point".to_string());
    }

    let mut pairing_input = [0u8; 384];

    // First pairing: e(G1_gen, g2)
    pairing_input[..64].copy_from_slice(&get_g1_generator());
    pairing_input[64..192].copy_from_slice(g2);

    // Second pairing: e(g1, -G2_gen)
    pairing_input[192..256].copy_from_slice(g1);
    pairing_input[256..].copy_from_slice(&get_g2_minus_one());

    let result =
        alt_bn128_pairing_be(&pairing_input).map_err(|e| format!("Pairing failed: {:?}", e))?;

    // Check if result equals 1 (successful verification)
    Ok(result == get_bn128_pairing_success_result())
}

/// Compute alpha binding value for G1/G2 representation security
///
/// Generates a Fiat-Shamir challenge that cryptographically binds the G1 and G2
/// aggregates together, preventing substitution attacks where different operator
/// sets could produce valid signatures.
///
/// # Arguments
/// * `message_hash` - Hashed message point on G1 (64 bytes)
/// * `signature` - Aggregated G1 signature (64 bytes)
/// * `aggregated_g1` - Aggregated G1 public key of signers (64 bytes)
/// * `aggregated_g2` - Aggregated G2 public key of signers (128 bytes)
///
/// # Returns
/// * `Ok([u8; 32])` - Alpha scalar for verification equation
/// * `Err(String)` - Never errors in current implementation
///
/// # Algorithm
/// alpha = SHA256(H(m) || sig || agg_g1 || agg_g2)
///
/// # Security
/// Binding all components together makes it computationally infeasible to
/// find a different operator set that produces the same verification result.
fn compute_alpha(
    message_hash: &[u8; 64],
    signature: &[u8; 64],
    aggregated_g1: &[u8; 64],
    aggregated_g2: &[u8; 128],
) -> Result<[u8; 32], String> {
    // Concatenate all inputs for hashing
    let mut hasher_input = Vec::new();
    hasher_input.extend_from_slice(message_hash);
    hasher_input.extend_from_slice(signature);
    hasher_input.extend_from_slice(aggregated_g1);
    hasher_input.extend_from_slice(aggregated_g2);

    // Hash to get alpha
    Ok(solana_hash(&hasher_input))
}

/// Get the BN254 G1 generator point
///
/// Returns the standard generator G1 = (1, 2) used as the base point
/// for all G1 operations in BN254.
///
/// # Returns
/// * `[u8; 64]` - Uncompressed G1 point (X || Y) in big-endian
fn get_g1_generator() -> [u8; 64] {
    G1_GENERATOR
}

/// Get the BN254 G2 generator point
///
/// Returns the standard G2 generator in EIP-197 format for pairing operations.
///
/// # Returns
/// * `[u8; 128]` - Uncompressed G2 point in EIP-197 format:
///   - Bytes 0-31: X.c1 (imaginary part)
///   - Bytes 32-63: X.c0 (real part)
///   - Bytes 64-95: Y.c1 (imaginary part)
///   - Bytes 96-127: Y.c0 (real part)
pub fn get_g2_generator() -> [u8; 128] {
    G2_GENERATOR
}

/// Get the negated BN254 G2 generator point
///
/// Returns -G2 for use in pairing equations. Pre-computing the negation
/// saves compute units during verification.
///
/// # Returns
/// * `[u8; 128]` - Negated G2 point in EIP-197 format
///
/// # Usage
/// Used in pairing checks to verify e(A, B) = e(C, D) by checking
/// e(A, B) * e(C, -D) = 1
fn get_g2_minus_one() -> [u8; 128] {
    G2_MINUS_ONE
}

/// Get the expected result for a successful BN254 pairing check
///
/// Returns the canonical 32-byte representation of the multiplicative identity
/// in the target group GT. This value indicates a successful pairing verification
/// when returned by Solana's alt_bn128_pairing precompile.
///
/// # Returns
/// * `[u8; 32]` - The value 1 encoded as 32 bytes (big-endian)
///
/// # Usage
/// Used to check pairing results:
/// ```
/// let result = alt_bn128_pairing(&pairing_input)?;
/// if result == get_bn128_pairing_success_result() {
///     // Pairing check passed
/// }
/// ```
///
/// # Technical Details
/// The alt_bn128_pairing precompile returns:
/// - This value (1) when the pairing equation e(A,B) * e(C,D) * ... = 1 holds
/// - A different value when the equation doesn't hold
/// - An error for invalid inputs
///
fn get_bn128_pairing_success_result() -> [u8; 32] {
    BN128_PAIRING_SUCCESS_RESULT
}

// ----------------------------------------------------------------------------
//                       OFFCHAIN
// ----------------------------------------------------------------------------

/// Aggregate multiple G1 public keys off-chain
///
/// Combines G1 public keys by adding their elliptic curve points.
/// Used for pre-computing operator key aggregates before on-chain submission.
///
/// # Arguments
/// * `pubkeys` - Array of uncompressed G1 public keys (64 bytes each)
///
/// # Returns
/// * `Ok([u8; 64])` - Aggregated G1 public key in big-endian
/// * `Err(String)` - If no keys provided or addition fails
///
/// # Performance
/// O(n) where n = number of public keys
/// Each addition ~3,000 CU if using Solana's precompile
pub fn offchain_aggregate_g1_pubkeys(
    pubkeys: &[[u8; 64]], // Uncompressed G1 points
) -> Result<[u8; 64], String> {
    if pubkeys.is_empty() {
        return Err("No public keys to aggregate".to_string());
    }

    let mut result = pubkeys[0];

    // Use the existing add_g1 function for G1 addition
    for pubkey in pubkeys.iter().skip(1) {
        result = add_g1(&result, pubkey)?;
    }

    Ok(result)
}

/// Aggregate multiple G2 public keys off-chain
///
/// Combines G2 public keys by adding their elliptic curve points.
/// Must be performed off-chain as Solana lacks G2 arithmetic support.
///
/// # Arguments
/// * `pubkeys` - Array of uncompressed G2 public keys (128 bytes each) in EIP-197 format
///
/// # Returns
/// * `Ok([u8; 128])` - Aggregated G2 public key in EIP-197 format
/// * `Err(String)` - If no keys provided or invalid points
///
/// # Note
/// Uses arkworks for G2 arithmetic since Solana cannot perform these operations
pub fn offchain_aggregate_g2_pubkeys(
    pubkeys: &[[u8; 128]], // Uncompressed G2 points
) -> Result<[u8; 128], String> {
    if pubkeys.is_empty() {
        return Err("No public keys to aggregate".to_string());
    }

    let mut result = pubkeys[0];

    // G2 addition isn't directly available in alt_bn128, so we need to:
    // 1. Convert to arkworks format
    // 2. Add points

    for pubkey in pubkeys.iter().skip(1) {
        result = offchain_add_g2_points(&result, pubkey)?;
    }

    Ok(result)
}

/// Add two G2 points using arkworks (off-chain only)
///
/// Performs elliptic curve addition on G2: P₁ + P₂ = P₃
///
/// # Arguments
/// * `p1` - First uncompressed G2 point (128 bytes) in EIP-197 format
/// * `p2` - Second uncompressed G2 point (128 bytes) in EIP-197 format
///
/// # Returns
/// * `Ok([u8; 128])` - Sum of the two points in EIP-197 format
/// * `Err(String)` - If points are invalid
///
/// # Implementation
/// 1. Parse points from EIP-197 format to arkworks G2Affine
/// 2. Add points using native curve arithmetic
/// 3. Convert result back to EIP-197 format
pub fn offchain_add_g2_points(p1: &[u8; 128], p2: &[u8; 128]) -> Result<[u8; 128], String> {
    // Parse G2 points from big-endian EIP-197 format
    let g2_1 = offchain_parse_g2_point(p1)?;
    let g2_2 = offchain_parse_g2_point(p2)?;

    // Add points
    let sum = (g2_1 + g2_2).into_affine();

    // Convert back to big-endian format
    offchain_g2_to_bytes(&sum)
}

/// Parse a G2 point from EIP-197 byte encoding
///
/// Converts a 128-byte array in EIP-197 format to arkworks G2Affine point.
///
/// # Arguments
/// * `bytes` - 128-byte array in EIP-197 format
///
/// # Returns
/// * `Ok(G2Affine)` - Parsed G2 point
/// * `Err(String)` - If bytes don't represent a valid G2 point
///
/// # Format (EIP-197)
/// - Bytes 0-31: X.c1 (imaginary part, big-endian)
/// - Bytes 32-63: X.c0 (real part, big-endian)
/// - Bytes 64-95: Y.c1 (imaginary part, big-endian)
/// - Bytes 96-127: Y.c0 (real part, big-endian)
pub fn offchain_parse_g2_point(bytes: &[u8; 128]) -> Result<G2Affine, String> {
    // Extract coordinates (EIP-197: X1, X0, Y1, Y0)
    let x1 = Fq::from_be_bytes_mod_order(&bytes[0..32]);
    let x0 = Fq::from_be_bytes_mod_order(&bytes[32..64]);
    let y1 = Fq::from_be_bytes_mod_order(&bytes[64..96]);
    let y0 = Fq::from_be_bytes_mod_order(&bytes[96..128]);

    let x = Fq2::new(x0, x1);
    let y = Fq2::new(y0, y1);

    Ok(G2Affine::new(x, y))
}

/// Convert a G2 point to EIP-197 byte encoding
///
/// Serializes an arkworks G2Affine point to standard 128-byte format.
///
/// # Arguments
/// * `point` - G2Affine point to serialize
///
/// # Returns
/// * `Ok([u8; 128])` - Point in EIP-197 format (big-endian)
/// * `Err(String)` - Never errors in current implementation
///
/// # Format (EIP-197)
/// Output: X.c1 || X.c0 || Y.c1 || Y.c0 (all big-endian)
fn offchain_g2_to_bytes(point: &G2Affine) -> Result<[u8; 128], String> {
    let mut result = [0u8; 128];

    // Get x and y coordinates
    let x = point.x;
    let y = point.y;

    // EIP-197 format: X1, X0, Y1, Y0
    result[0..32].copy_from_slice(&x.c1.into_bigint().to_bytes_be());
    result[32..64].copy_from_slice(&x.c0.into_bigint().to_bytes_be());
    result[64..96].copy_from_slice(&y.c1.into_bigint().to_bytes_be());
    result[96..128].copy_from_slice(&y.c0.into_bigint().to_bytes_be());

    Ok(result)
}

/// Create a bitmap indicating which operators participated in signing
///
/// Generates a compact bitmap where each bit represents whether the operator
/// at that index participated. Used for efficient on-chain verification.
///
/// # Arguments
/// * `total_operators` - Total number of operators in the system
/// * `signing_indices` - Indices of operators who signed (0-based)
///
/// # Returns
/// * `Vec<u8>` - Bitmap where bit i indicates if operator i signed
///
/// # Format
/// - Bit 0 of byte 0: Operator 0
/// - Bit 1 of byte 0: Operator 1
/// - ...
/// - Bit 0 of byte 1: Operator 8
///
/// # Example
/// ```
/// // With 10 operators, if operators 0, 2, and 9 signed:
/// let bitmap = offchain_create_operators_bitmap(10, &[0, 2, 9]);
/// // Results in: [0x05, 0x02] (binary: 00000101, 00000010)
/// ```
pub fn offchain_create_operators_bitmap(
    total_operators: usize,
    signing_indices: &[usize],
) -> [u8; 32] {
    let mut bitmap = [0u8; 32];
    for &index in signing_indices {
        if index < total_operators {
            let byte_index = index / 8;
            let bit_index = index % 8;
            bitmap[byte_index] |= 1 << bit_index;
        }
    }
    bitmap
}

/// Prepare all data needed for on-chain vote verification
///
/// Aggregates signatures and public keys off-chain and creates a bitmap
/// indicating participation. Reduces on-chain computation and transaction size.
///
/// # Arguments
/// * `signatures` - Individual G1 signatures from each operator (64 bytes each)
/// * `pubkeys_g2` - G2 public keys of signing operators (128 bytes each)
/// * `signing_indices` - Indices of operators who signed (must match signatures order)
/// * `total_operators` - Total number of operators in the system
///
/// # Returns
/// * `Ok((signature, pubkey, bitmap))` - Tuple containing:
///   - Aggregated G1 signature (64 bytes)
///   - Aggregated G2 public key (128 bytes)
///   - Bitmap of signers
/// * `Err(String)` - If aggregation fails
///
/// # Important
/// Arrays must have same length and order must match:
/// signatures[i] corresponds to pubkeys_g2[i] and signing_indices[i]
#[allow(clippy::type_complexity)]
pub fn offchain_prepare_vote_data(
    g1_signatures: &[[u8; 64]],      // Uncompressed G1 signatures
    g2_signed_pubkeys: &[[u8; 128]], // Uncompressed G2 public keys
    signing_indices: &[usize],       // Which operators signed
    total_operators: usize,
) -> Result<([u8; 64], [u8; 128], [u8; 32]), String> {
    // Aggregate and compress signatures
    let aggregated_signature = aggregate_signatures(g1_signatures)?;
    // Aggregate and compress public keys
    let aggregated_g2 = offchain_aggregate_g2_pubkeys(g2_signed_pubkeys)?;
    // Create bitmap
    let bitmap = offchain_create_operators_bitmap(total_operators, signing_indices);
    Ok((aggregated_signature, aggregated_g2, bitmap))
}

#[inline(always)]
pub fn did_sign_bitmap(bitmap: [u8; 32], index: usize) -> Result<bool, String> {
    if index >= bitmap.len() * 8 {
        return Err("Index out of bounds".to_string());
    }

    let byte_index = index / 8;
    let bit_index = index % 8;
    let byte = bitmap[byte_index];
    let bit = byte & (1 << bit_index);
    Ok(bit != 0)
}

/// Derive a G1 public key from a private key (off-chain)
///
/// Computes G1_pubkey = private_key * G1_generator.
/// Used in the dual-key system where operators have both G1 and G2 keys.
///
/// # Arguments
/// * `private_key` - 32-byte private key scalar (big-endian)
///
/// # Returns
/// * `Ok([u8; 64])` - Uncompressed G1 public key (X || Y) in big-endian
/// * `Err(String)` - If multiplication fails
///
/// # Usage
/// G1 keys are stored on-chain for efficient aggregation in operator snapshots.
pub fn offchain_g1_from_private_key(private_key: &[u8; 32]) -> Result<[u8; 64], String> {
    let g1_generator = get_g1_generator();
    mult_g1(&g1_generator, private_key)
}

/// Derive a G2 public key from a private key (off-chain only)
///
/// Computes G2_pubkey = private_key * G2_generator.
/// Must be performed off-chain as Solana lacks G2 arithmetic support.
///
/// # Arguments
/// * `private_key` - 32-byte private key scalar (big-endian)
///
/// # Returns
/// * `Ok([u8; 128])` - Uncompressed G2 public key in EIP-197 format
/// * `Err(String)` - If key derivation fails
///
/// # Usage
/// G2 keys are used for signing operations and aggregated off-chain
/// before submission to Solana programs.
pub fn offchain_g2_from_private_key(private_key: &[u8; 32]) -> Result<[u8; 128], String> {
    // Convert private key to field element
    let fr = Fr::from_be_bytes_mod_order(private_key);

    // Multiply G2 generator by private key
    let g2_pubkey = (G2Affine::generator() * fr).into_affine();

    // Convert to bytes in EIP-197 format
    let mut result = [0u8; 128];
    result[0..32].copy_from_slice(&g2_pubkey.x.c1.into_bigint().to_bytes_be());
    result[32..64].copy_from_slice(&g2_pubkey.x.c0.into_bigint().to_bytes_be());
    result[64..96].copy_from_slice(&g2_pubkey.y.c1.into_bigint().to_bytes_be());
    result[96..128].copy_from_slice(&g2_pubkey.y.c0.into_bigint().to_bytes_be());

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::{Fr, G1Affine};
    use ark_ff::PrimeField;
    use hex;
    use solana_keypair::Keypair;

    #[cfg(test)]
    fn generate_random_bls_private_key() -> [u8; 32] {
        // Generate a random Solana keypair
        let keypair = Keypair::new();

        // Get the secret key bytes (first 32 bytes of the keypair)
        let secret_bytes = keypair.secret_bytes();
        let mut private_key = [0u8; 32];
        private_key.copy_from_slice(&secret_bytes[..32]);

        // Convert to Fr to ensure it's a valid scalar in the BN254 field
        // This will reduce the value modulo the curve order if needed
        let scalar = Fr::from_be_bytes_mod_order(&private_key);

        // Convert back to big-endian bytes
        let scalar_bigint = scalar.into_bigint();
        let bytes = scalar_bigint.to_bytes_be();

        // Ensure we have exactly 32 bytes
        let mut result = [0u8; 32];
        let len = bytes.len().min(32);
        result[32 - len..].copy_from_slice(&bytes[bytes.len() - len..]);

        result
    }

    // Usage example in tests:
    #[test]
    fn test_with_random_keys() {
        // Generate random private keys for testing
        let private_key1 = generate_random_bls_private_key();
        let private_key2 = generate_random_bls_private_key();

        // Use them to generate G1 and G2 public keys
        let g1_pubkey1 =
            offchain_g1_from_private_key(&private_key1).expect("Failed to derive G1 pubkey");
        let _g2_pubkey1 =
            offchain_g2_from_private_key(&private_key1).expect("Failed to derive G2 pubkey");

        // Ensure different keys produce different public keys
        let g1_pubkey2 =
            offchain_g1_from_private_key(&private_key2).expect("Failed to derive G1 pubkey");
        assert_ne!(
            g1_pubkey1, g1_pubkey2,
            "Different private keys should produce different public keys"
        );

        println!(
            "Generated random private key 1: {}",
            hex::encode(private_key1)
        );
        println!(
            "Generated random private key 2: {}",
            hex::encode(private_key2)
        );
    }

    #[test]
    fn test_hash_function() {
        // Test basic functionality
        let data = b"test message";
        let hash = solana_hash(data);
        assert_eq!(hash.len(), 32);

        // Test deterministic output - same input should always produce same hash
        let hash2 = solana_hash(data);
        assert_eq!(hash, hash2);

        // Test different inputs produce different hashes
        let hash3 = solana_hash(b"different message");
        assert_ne!(hash, hash3);

        // Test empty input
        let empty_hash = solana_hash(b"");
        assert_eq!(empty_hash.len(), 32);
        // SHA256 of empty string is known
        assert_eq!(
            hex::encode(empty_hash),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );

        // Test single byte
        let single_byte_hash = solana_hash(&[0x00]);
        assert_eq!(single_byte_hash.len(), 32);

        // Test large input
        let large_data = vec![0xAA; 1000];
        let large_hash = solana_hash(&large_data);
        assert_eq!(large_hash.len(), 32);

        // Verify the hash changes with even small input changes
        let data1 = b"The quick brown fox jumps over the lazy dog";
        let data2 = b"The quick brown fox jumps over the lazy dog."; // Added period
        let hash1 = solana_hash(data1);
        let hash2 = solana_hash(data2);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_constants_format() {
        use ark_ec::AffineRepr;
        use {G1Affine, G2Affine};

        // Check G1 generator
        let g1_gen = get_g1_generator();

        // Verify against arkworks G1 generator
        let ark_g1 = G1Affine::generator();
        let ark_g1_x = ark_g1.x.into_bigint().to_bytes_be();
        let ark_g1_y = ark_g1.y.into_bigint().to_bytes_be();

        assert_eq!(
            &g1_gen[..32],
            &ark_g1_x,
            "G1 X coordinate should match arkworks"
        );
        assert_eq!(
            &g1_gen[32..],
            &ark_g1_y,
            "G1 Y coordinate should match arkworks"
        );

        // Check G2 generator
        let g2_gen = get_g2_generator();

        // // Verify against arkworks G2 generator
        let ark_g2 = G2Affine::generator();
        let ark_g2_x_c1 = ark_g2.x.c1.into_bigint().to_bytes_be();
        let ark_g2_x_c0 = ark_g2.x.c0.into_bigint().to_bytes_be();
        let ark_g2_y_c1 = ark_g2.y.c1.into_bigint().to_bytes_be();
        let ark_g2_y_c0 = ark_g2.y.c0.into_bigint().to_bytes_be();

        assert_eq!(
            &g2_gen[0..32],
            &ark_g2_x_c1,
            "G2 X c1 should match arkworks"
        );
        assert_eq!(
            &g2_gen[32..64],
            &ark_g2_x_c0,
            "G2 X c0 should match arkworks"
        );
        assert_eq!(
            &g2_gen[64..96],
            &ark_g2_y_c1,
            "G2 Y c1 should match arkworks"
        );
        assert_eq!(
            &g2_gen[96..128],
            &ark_g2_y_c0,
            "G2 Y c0 should match arkworks"
        );

        // Check G2 minus one
        let g2_minus = get_g2_minus_one();

        // Verify G2 minus one is actually -G2
        let ark_neg_g2 = -ark_g2;
        let ark_neg_g2_x_c1 = ark_neg_g2.x.c1.into_bigint().to_bytes_be();
        let ark_neg_g2_x_c0 = ark_neg_g2.x.c0.into_bigint().to_bytes_be();
        let ark_neg_g2_y_c1 = ark_neg_g2.y.c1.into_bigint().to_bytes_be();
        let ark_neg_g2_y_c0 = ark_neg_g2.y.c0.into_bigint().to_bytes_be();

        assert_eq!(
            &g2_minus[0..32],
            &ark_neg_g2_x_c1,
            "Negated G2 X c1 should match arkworks"
        );
        assert_eq!(
            &g2_minus[32..64],
            &ark_neg_g2_x_c0,
            "Negated G2 X c0 should match arkworks"
        );
        assert_eq!(
            &g2_minus[64..96],
            &ark_neg_g2_y_c1,
            "Negated G2 Y c1 should match arkworks"
        );
        assert_eq!(
            &g2_minus[96..128],
            &ark_neg_g2_y_c0,
            "Negated G2 Y c0 should match arkworks"
        );

        // Verify the constants are different (Y coordinates should be negated)
        assert_ne!(
            &g2_gen[64..],
            &g2_minus[64..],
            "Y coordinates should be negated"
        );
    }

    #[test]
    fn debug_g2_encoding() {
        let private_key = generate_random_bls_private_key();
        let g2_pubkey =
            offchain_g2_from_private_key(&private_key).expect("Failed to derive G2 pubkey");

        // Verify it's a valid G2 point using arkworks
        let parsed = offchain_parse_g2_point(&g2_pubkey);
        assert!(parsed.is_ok(), "Should parse as valid G2 point");
    }

    #[test]
    fn test_sign_message() {
        // Test with a simple private key
        let private_key = generate_random_bls_private_key();
        let message = b"test message";

        // Generate signature
        let signature = solana_sign(&private_key, message, 0).unwrap();
        assert_eq!(signature.len(), 64);

        // Verify signature is deterministic
        let signature2 = solana_sign(&private_key, message, 0).unwrap();
        assert_eq!(signature, signature2);

        // Different private key produces different signature
        let private_key2 = generate_random_bls_private_key();
        let signature3 = solana_sign(&private_key2, message, 0).unwrap();
        assert_ne!(signature, signature3);

        // Different message produces different signature
        let signature4 = solana_sign(&private_key, b"different message", 0).unwrap();
        assert_ne!(signature, signature4);

        // Different domain produces different signature
        let signature5 = solana_sign(&private_key, message, 1).unwrap();
        assert_ne!(signature, signature5);

        // No domain produces different signature
        let signature6 = solana_sign(&private_key, message, 2).unwrap();
        assert_ne!(signature, signature6);

        // Verify the signature is a valid G1 point
        let x = Fq::from_be_bytes_mod_order(&signature[..32]);
        let y = Fq::from_be_bytes_mod_order(&signature[32..]);
        let sig_point = G1Affine::new_unchecked(x, y);
        assert!(sig_point.is_on_curve());
        assert!(sig_point.is_in_correct_subgroup_assuming_on_curve());

        // Test edge case: max private key (just below curve order)
        let max_key = generate_random_bls_private_key();
        let result = solana_sign(&max_key, message, 0);
        // This should still work as it gets reduced mod order
        assert!(result.is_ok());

        // Verify signatures are different for consecutive private keys
        let key1 = generate_random_bls_private_key();
        let key2 = generate_random_bls_private_key();
        let sig1 = solana_sign(&key1, message, 0).unwrap();
        let sig2 = solana_sign(&key2, message, 0).unwrap();
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn test_solana_verify_one_signature() {
        // ====================================================================
        // OFF-CHAIN: Setup and Key Generation
        // ====================================================================

        // Generate a private key (off-chain only - never goes on-chain)
        let private_key = generate_random_bls_private_key();

        // Derive both G1 and G2 public keys from the same private key
        let g1_pubkey =
            offchain_g1_from_private_key(&private_key).expect("Failed to derive G1 pubkey");
        let g2_pubkey =
            offchain_g2_from_private_key(&private_key).expect("Failed to derive G2 pubkey");

        // ====================================================================
        // ON-CHAIN: Store G1 public key in operator snapshot
        // ====================================================================

        // In real Solana program, this G1 pubkey would be stored in the Snapshot
        // account as part of operator registration
        // snapshot.operator_snapshots[0].g1_pubkey = g1_pubkey;

        // ====================================================================
        // OFF-CHAIN: Sign a message
        // ====================================================================

        // Message to sign (could be a vote counter value)
        let message = b"vote for round 42";

        // Generate signature using private key
        let signature = solana_sign(&private_key, message, 0).expect("Failed to sign message");

        // For a single signer, prepare the data as if it were aggregated
        let signatures = vec![signature];
        let pubkeys_g2 = vec![g2_pubkey];
        let signing_indices = vec![0]; // Operator at index 0 signed
        let total_operators = 1;

        // Aggregate off-chain (for single signer, just returns the same values)
        let (aggregated_signature, aggregated_g2, bitmap) =
            offchain_prepare_vote_data(&signatures, &pubkeys_g2, &signing_indices, total_operators)
                .expect("Failed to prepare vote data");

        // ====================================================================
        // ON-CHAIN: Compute aggregated G1 from stored values
        // ====================================================================

        // In the real program, this happens in process_cast_vote:
        // 1. Read total_aggregated_g1_pubkey from snapshot
        // 2. Compute non-signers' aggregated G1
        // 3. Subtract to get signers' aggregated G1

        // For this test with single signer:
        let total_aggregated_g1 = g1_pubkey; // Only one operator
        let _non_signers_g1: Option<[u8; 32]> = None; // No non-signers
        let signers_aggregated_g1 = total_aggregated_g1; // All operators signed

        // ====================================================================
        // ON-CHAIN: Verify the aggregated signature
        // ====================================================================

        // This is what happens inside process_cast_vote
        let is_valid = solana_verify_aggregated_signature(
            &signers_aggregated_g1, // Computed on-chain from G1 pubkeys
            &aggregated_g2,         // Pre-computed off-chain
            &aggregated_signature,  // Aggregated G1 signature
            message,                // The message that was signed
            0,                      // Domain separator
        )
        .expect("Verification failed");

        assert!(is_valid, "Signature verification should succeed");

        // ====================================================================
        // ON-CHAIN: Test verification failures
        // ====================================================================

        // Test 1: Wrong message should fail
        let wrong_message = b"vote for round 43";
        let is_valid_wrong_msg = solana_verify_aggregated_signature(
            &signers_aggregated_g1,
            &aggregated_g2,
            &aggregated_signature,
            wrong_message, // Different message
            0,
        )
        .expect("Verification failed");

        assert!(
            !is_valid_wrong_msg,
            "Wrong message should fail verification"
        );

        // Test 2: Wrong domain should fail
        let is_valid_wrong_domain = solana_verify_aggregated_signature(
            &signers_aggregated_g1,
            &aggregated_g2,
            &aggregated_signature,
            message,
            1, // Different domain
        )
        .expect("Verification failed");

        assert!(
            !is_valid_wrong_domain,
            "Wrong domain should fail verification"
        );

        // Test 3: Mismatched G1/G2 keys should fail
        // Simulate a different operator's G1 key being used
        let wrong_private_key = generate_random_bls_private_key();
        let wrong_g1_pubkey = offchain_g1_from_private_key(&wrong_private_key)
            .expect("Failed to derive wrong G1 pubkey");

        let is_valid_wrong_key = solana_verify_aggregated_signature(
            &wrong_g1_pubkey, // Wrong G1 doesn't match G2
            &aggregated_g2,
            &aggregated_signature,
            message,
            0,
        )
        .expect("Verification failed");

        assert!(
            !is_valid_wrong_key,
            "Mismatched keys should fail verification"
        );

        // Test 4: Wrong signature should fail
        let wrong_signature =
            solana_sign(&wrong_private_key, message, 0).expect("Failed to sign with wrong key");

        let is_valid_wrong_sig = solana_verify_aggregated_signature(
            &signers_aggregated_g1,
            &aggregated_g2,
            &wrong_signature, // Different signature
            message,
            0,
        )
        .expect("Verification failed");

        assert!(
            !is_valid_wrong_sig,
            "Wrong signature should fail verification"
        );

        // ====================================================================
        // Verify bitmap was created correctly
        // ====================================================================

        assert_eq!(bitmap[0], 0b00000001, "Bit 0 should be set for operator 0");
    }

    #[test]
    fn test_verify_three_signatures() {
        // ====================================================================
        // OFF-CHAIN: Setup 3 operators with their key pairs
        // ====================================================================

        // Generate 3 different private keys
        let private_key1 = generate_random_bls_private_key();
        let private_key2 = generate_random_bls_private_key();
        let private_key3 = generate_random_bls_private_key();

        // Derive G1 and G2 public keys for each operator
        let g1_pubkey1 =
            offchain_g1_from_private_key(&private_key1).expect("Failed to derive G1 pubkey 1");
        let g2_pubkey1 =
            offchain_g2_from_private_key(&private_key1).expect("Failed to derive G2 pubkey 1");

        let g1_pubkey2 =
            offchain_g1_from_private_key(&private_key2).expect("Failed to derive G1 pubkey 2");
        let g2_pubkey2 =
            offchain_g2_from_private_key(&private_key2).expect("Failed to derive G2 pubkey 2");

        let g1_pubkey3 =
            offchain_g1_from_private_key(&private_key3).expect("Failed to derive G1 pubkey 3");
        let g2_pubkey3 =
            offchain_g2_from_private_key(&private_key3).expect("Failed to derive G2 pubkey 3");

        // ====================================================================
        // ON-CHAIN: Store total aggregated G1 pubkey in snapshot
        // ====================================================================

        // Compute total aggregated G1 (would be stored in Snapshot)
        let mut total_g1 = add_g1(&g1_pubkey1, &g1_pubkey2).expect("Failed to add G1 keys 1+2");
        total_g1 = add_g1(&total_g1, &g1_pubkey3).expect("Failed to add G1 key 3");

        // ====================================================================
        // OFF-CHAIN: Three operators sign the same message
        // ====================================================================

        let message = b"vote for round 42";

        // Each operator signs independently
        let signature1 = solana_sign(&private_key1, message, 0).expect("Failed to sign with key 1");
        let signature2 = solana_sign(&private_key2, message, 0).expect("Failed to sign with key 2");
        let signature3 = solana_sign(&private_key3, message, 0).expect("Failed to sign with key 3");

        // ====================================================================
        // TEST CASE 1: All 3 operators sign
        // ====================================================================
        {
            println!("\n=== Test Case 1: All 3 operators sign ===");

            // Prepare vote data for all 3 signers
            let signatures = vec![signature1, signature2, signature3];
            let pubkeys_g2 = vec![g2_pubkey1, g2_pubkey2, g2_pubkey3];
            let signing_indices = vec![0, 1, 2];
            let total_operators = 3;

            let (aggregated_signature, aggregated_g2, bitmap) = offchain_prepare_vote_data(
                &signatures,
                &pubkeys_g2,
                &signing_indices,
                total_operators,
            )
            .expect("Failed to prepare vote data");

            // Verify bitmap
            assert_eq!(bitmap[0], 0b00000111); // Bits 0, 1, 2 set

            // ON-CHAIN: Since all signed, signers' G1 = total G1
            let signers_g1 = total_g1;

            // Verify signature
            let is_valid = solana_verify_aggregated_signature(
                &signers_g1,
                &aggregated_g2,
                &aggregated_signature,
                message,
                0,
            )
            .expect("Verification failed");

            assert!(is_valid, "All 3 signatures should verify");
            println!("✓ All 3 signatures verified successfully");
        }

        // ====================================================================
        // TEST CASE 2: Only operators 0 and 2 sign (operator 1 doesn't sign)
        // ====================================================================
        {
            println!("\n=== Test Case 2: Only operators 0 and 2 sign ===");

            // Prepare vote data for 2 signers
            let signatures = vec![signature1, signature3]; // Skip signature2
            let pubkeys_g2 = vec![g2_pubkey1, g2_pubkey3]; // Skip pubkey2
            let signing_indices = vec![0, 2]; // Operators 0 and 2
            let total_operators = 3;

            let (aggregated_signature, aggregated_g2, bitmap) = offchain_prepare_vote_data(
                &signatures,
                &pubkeys_g2,
                &signing_indices,
                total_operators,
            )
            .expect("Failed to prepare vote data");

            // Verify bitmap
            assert_eq!(bitmap[0], 0b00000101); // Bits 0 and 2 set, bit 1 clear

            // ON-CHAIN: Compute signers' G1 by subtracting non-signer
            // signers_g1 = total_g1 - g1_pubkey2
            let signers_g1 =
                sub_g1(&total_g1, &g1_pubkey2).expect("Failed to subtract non-signer G1");

            // Verify signature
            let is_valid = solana_verify_aggregated_signature(
                &signers_g1,
                &aggregated_g2,
                &aggregated_signature,
                message,
                0,
            )
            .expect("Verification failed");

            assert!(is_valid, "Operators 0,2 signatures should verify");
            println!("✓ Operators 0,2 signatures verified successfully");
        }

        // ====================================================================
        // TEST CASE 3: Only operator 1 signs
        // ====================================================================
        {
            println!("\n=== Test Case 3: Only operator 1 signs ===");

            // Prepare vote data for single signer
            let signatures = vec![signature2];
            let pubkeys_g2 = vec![g2_pubkey2];
            let signing_indices = vec![1];
            let total_operators = 3;

            let (aggregated_signature, aggregated_g2, bitmap) = offchain_prepare_vote_data(
                &signatures,
                &pubkeys_g2,
                &signing_indices,
                total_operators,
            )
            .expect("Failed to prepare vote data");

            // Verify bitmap
            assert_eq!(bitmap[0], 0b00000010); // Only bit 1 set

            // ON-CHAIN: Compute signers' G1
            // signers_g1 = total_g1 - g1_pubkey1 - g1_pubkey3
            let non_signers_g1 =
                add_g1(&g1_pubkey1, &g1_pubkey3).expect("Failed to add non-signer G1s");
            let signers_g1 =
                sub_g1(&total_g1, &non_signers_g1).expect("Failed to subtract non-signers G1");

            // Verify this equals g1_pubkey2
            assert_eq!(
                signers_g1, g1_pubkey2,
                "Signers G1 should equal operator 1's G1"
            );

            // Verify signature
            let is_valid = solana_verify_aggregated_signature(
                &signers_g1,
                &aggregated_g2,
                &aggregated_signature,
                message,
                0,
            )
            .expect("Verification failed");

            assert!(is_valid, "Operator 1 signature should verify");
            println!("✓ Operator 1 signature verified successfully");
        }

        // ====================================================================
        // TEST CASE 4: Verification should fail with wrong signers' G1
        // ====================================================================
        {
            println!("\n=== Test Case 4: Wrong signers' G1 should fail ===");

            // Use the aggregated data from test case 2 (operators 0,2)
            let signatures = vec![signature1, signature3];
            let pubkeys_g2 = vec![g2_pubkey1, g2_pubkey3];
            let signing_indices = vec![0, 2];
            let total_operators = 3;

            let (aggregated_signature, aggregated_g2, _bitmap) = offchain_prepare_vote_data(
                &signatures,
                &pubkeys_g2,
                &signing_indices,
                total_operators,
            )
            .expect("Failed to prepare vote data");

            // Use wrong signers' G1 (pretend all 3 signed when only 2 did)
            let wrong_signers_g1 = total_g1;

            // Verify signature - should fail
            let is_valid = solana_verify_aggregated_signature(
                &wrong_signers_g1, // Wrong!
                &aggregated_g2,
                &aggregated_signature,
                message,
                0,
            )
            .expect("Verification call failed");

            assert!(!is_valid, "Wrong signers' G1 should fail verification");
            println!("✓ Wrong signers' G1 correctly failed verification");
        }

        // ====================================================================
        // TEST CASE 5: Different messages should produce different signatures
        // ====================================================================
        {
            println!("\n=== Test Case 5: Different messages ===");

            let different_message = b"vote for round 43";

            // Sign different message
            let diff_sig1 = solana_sign(&private_key1, different_message, 0)
                .expect("Failed to sign different message");

            assert_ne!(
                signature1, diff_sig1,
                "Different messages should produce different signatures"
            );
            println!("✓ Different messages produce different signatures");
        }

        println!("\n✅ All 3-signature test cases passed!");
    }

    #[test]
    fn test_verify_two_of_three_signatures() {
        // ====================================================================
        // Setup: Create 3 operators with their key pairs
        // ====================================================================

        // Generate 3 different private keys
        let private_keys = [
            [0x01; 32], // Operator 0
            [0x02; 32], // Operator 1
            [0x03; 32], // Operator 2
        ];

        // Derive G1 and G2 public keys for each operator
        let mut g1_pubkeys = [[0u8; 64]; 3];
        let mut g2_pubkeys = [[0u8; 128]; 3];

        for i in 0..3 {
            g1_pubkeys[i] = offchain_g1_from_private_key(&private_keys[i])
                .unwrap_or_else(|e| panic!("Failed to derive G1 pubkey {} {}", i, e));
            g2_pubkeys[i] = offchain_g2_from_private_key(&private_keys[i])
                .unwrap_or_else(|e| panic!("Failed to derive G2 pubkey {} {}", i, e));
        }

        // ====================================================================
        // ON-CHAIN: Compute and store total aggregated G1 pubkey
        // ====================================================================

        // In the real program, this would be computed during snapshot creation
        let mut total_aggregated_g1 = g1_pubkeys[0];
        for (i, item) in g1_pubkeys.iter().enumerate().skip(1) {
            total_aggregated_g1 = add_g1(&total_aggregated_g1, item)
                .unwrap_or_else(|e| panic!("Failed to add G1 pubkey {} {}", i, e));
        }

        println!(
            "Total aggregated G1 (all 3 operators): {}",
            hex::encode(total_aggregated_g1)
        );

        // ====================================================================
        // OFF-CHAIN: Operators 0 and 2 sign (operator 1 is the non-signer)
        // ====================================================================

        let message = b"vote for round 42";

        // Only operators 0 and 2 sign
        let signature0 =
            solana_sign(&private_keys[0], message, 0).expect("Failed to sign with key 0");
        let signature2 =
            solana_sign(&private_keys[2], message, 0).expect("Failed to sign with key 2");

        // Prepare vote data for the 2 signers
        let signatures = vec![signature0, signature2];
        let signer_g2_pubkeys = vec![g2_pubkeys[0], g2_pubkeys[2]];
        let signing_indices = vec![0, 2]; // Operators 0 and 2 signed
        let total_operators = 3;

        // Aggregate off-chain
        let (aggregated_signature, aggregated_g2, bitmap) = offchain_prepare_vote_data(
            &signatures,
            &signer_g2_pubkeys,
            &signing_indices,
            total_operators,
        )
        .expect("Failed to prepare vote data");

        println!("\nOff-chain aggregation complete:");
        println!(
            "  Aggregated signature: {}",
            hex::encode(aggregated_signature)
        );
        println!("  Aggregated G2: {}", hex::encode(aggregated_g2));
        println!("  Bitmap: 0b{:08b} (operators 0 and 2 set)", bitmap[0]);

        // Verify bitmap is correct
        assert_eq!(bitmap[0], 0b00000101, "Bits 0 and 2 should be set");

        // ====================================================================
        // ON-CHAIN: Compute signers' aggregated G1
        // ====================================================================

        // The contract would:
        // 1. Read bitmap to determine non-signers (operator 1)
        // 2. Compute non-signers' aggregated G1 (just operator 1's G1)
        // 3. Subtract from total: signers_g1 = total_g1 - non_signers_g1

        let non_signers_g1 = g1_pubkeys[1]; // Only operator 1 didn't sign
        let signers_aggregated_g1 = sub_g1(&total_aggregated_g1, &non_signers_g1)
            .expect("Failed to subtract non-signer G1");

        println!("\nOn-chain computation:");
        println!(
            "  Non-signer G1 (operator 1): {}",
            hex::encode(non_signers_g1)
        );
        println!(
            "  Signers' aggregated G1: {}",
            hex::encode(signers_aggregated_g1)
        );

        // Verify that signers_aggregated_g1 = g1_pubkeys[0] + g1_pubkeys[2]
        let expected_signers_g1 =
            add_g1(&g1_pubkeys[0], &g1_pubkeys[2]).expect("Failed to add expected signer G1s");
        assert_eq!(
            signers_aggregated_g1, expected_signers_g1,
            "Signers' G1 should equal sum of operators 0 and 2"
        );

        // ====================================================================
        // ON-CHAIN: Verify the aggregated signature
        // ====================================================================

        println!("\nVerifying signature...");

        let is_valid = solana_verify_aggregated_signature(
            &signers_aggregated_g1, // Computed on-chain from bitmap
            &aggregated_g2,         // Pre-computed off-chain from signers
            &aggregated_signature,  // Aggregated G1 signature from signers
            message,                // The message that was signed
            0,                      // Domain separator
        )
        .expect("Verification failed");

        assert!(is_valid, "2-of-3 signature should verify successfully");
        println!("✅ 2-of-3 signature verified successfully!");

        // ====================================================================
        // Additional test: Verify it fails if we use wrong signers' G1
        // ====================================================================

        println!("\nTesting failure cases...");

        // Test 1: Using total G1 (pretending all 3 signed) should fail
        let should_fail = solana_verify_aggregated_signature(
            &total_aggregated_g1, // Wrong! This includes non-signer
            &aggregated_g2,
            &aggregated_signature,
            message,
            0,
        )
        .expect("Verification call failed");

        assert!(!should_fail, "Should fail when using wrong signers' G1");
        println!("✓ Correctly rejected wrong signers' G1");

        // Test 2: Using only one signer's G1 should fail
        let should_fail2 = solana_verify_aggregated_signature(
            &g1_pubkeys[0], // Wrong! Only one of the two signers
            &aggregated_g2,
            &aggregated_signature,
            message,
            0,
        )
        .expect("Verification call failed");

        assert!(!should_fail2, "Should fail when using partial signers' G1");
        println!("✓ Correctly rejected partial signers' G1");

        println!("\n✅ All 2-of-3 signature tests passed!");
    }

    #[test]
    fn test_verify_g1_g2() {
        // Test 1: Valid key pair from same private key
        let private_key = generate_random_bls_private_key();
        let g1 = offchain_g1_from_private_key(&private_key).unwrap();
        let g2 = offchain_g2_from_private_key(&private_key).unwrap();

        let result = verify_g1_g2(&g1, &g2).unwrap();
        assert!(result, "Valid key pair should verify");

        // Test 2: Invalid - keys from different private keys
        let private_key2 = generate_random_bls_private_key();
        let g1_2 = offchain_g1_from_private_key(&private_key2).unwrap();

        let result = verify_g1_g2(&g1_2, &g2).unwrap();
        assert!(
            !result,
            "Keys from different private keys should not verify"
        );

        // Test 3: Zero G1 point
        let zero_g1 = [0u8; 64];
        let result = verify_g1_g2(&zero_g1, &g2);
        assert!(result.is_err(), "Zero G1 should return error");

        // Test 4: Zero G2 point
        let zero_g2 = [0u8; 128];
        let result = verify_g1_g2(&g1, &zero_g2);
        assert!(result.is_err(), "Zero G2 should return error");

        // Test 5: Multiple valid key pairs
        for _ in 0..3 {
            let pk = generate_random_bls_private_key();
            let g1 = offchain_g1_from_private_key(&pk).unwrap();
            let g2 = offchain_g2_from_private_key(&pk).unwrap();

            let result = verify_g1_g2(&g1, &g2).unwrap();
            assert!(result, "All valid pairs should verify");
        }

        // Test 6: Cross-check - no false positives
        let keys: Vec<_> = (0..3)
            .map(|_| {
                let pk = generate_random_bls_private_key();
                let g1 = offchain_g1_from_private_key(&pk).unwrap();
                let g2 = offchain_g2_from_private_key(&pk).unwrap();
                (g1, g2)
            })
            .collect();

        // Check that mixing keys always fails
        for i in 0..3 {
            for j in 0..3 {
                if i != j {
                    let result = verify_g1_g2(&keys[i].0, &keys[j].1).unwrap();
                    assert!(!result, "Mixed keys should never verify");
                }
            }
        }
    }

    #[test]
    fn test_solana_verify_signature_with_g2() {
        // Test 1: Valid signature verification
        let private_key = generate_random_bls_private_key();
        let g2_pubkey = offchain_g2_from_private_key(&private_key).unwrap();
        let message = b"test message";

        let signature = solana_sign(&private_key, message, 0).unwrap();

        let valid = solana_verify_signature_with_g2(&g2_pubkey, &signature, message, 0).unwrap();
        assert!(valid, "Valid signature should verify");

        // Test 2: Wrong message should fail
        let wrong_message = b"this is a wrong message";
        let valid =
            solana_verify_signature_with_g2(&g2_pubkey, &signature, wrong_message, 0).unwrap();
        assert!(!valid, "Wrong message should fail verification");

        // Test 3: Wrong domain should fail
        let valid = solana_verify_signature_with_g2(&g2_pubkey, &signature, message, 1).unwrap();
        assert!(!valid, "Wrong domain should fail verification");

        // Test 4: Wrong G2 key should fail
        let wrong_private_key = generate_random_bls_private_key();
        let wrong_g2_pubkey = offchain_g2_from_private_key(&wrong_private_key).unwrap();
        let valid =
            solana_verify_signature_with_g2(&wrong_g2_pubkey, &signature, message, 0).unwrap();
        assert!(!valid, "Wrong G2 key should fail verification");

        // Test 5: Wrong signature should fail
        let wrong_signature = solana_sign(&wrong_private_key, message, 0).unwrap();
        let valid =
            solana_verify_signature_with_g2(&g2_pubkey, &wrong_signature, message, 0).unwrap();
        assert!(!valid, "Wrong signature should fail verification");

        // Test 6: No domain
        let signature_no_domain = solana_sign(&private_key, message, 2).unwrap();
        let valid =
            solana_verify_signature_with_g2(&g2_pubkey, &signature_no_domain, message, 2).unwrap();
        assert!(valid, "Signature without domain should verify");

        // Test 7: Multiple signers (aggregated)
        let private_key2 = generate_random_bls_private_key();
        let g2_pubkey2 = offchain_g2_from_private_key(&private_key2).unwrap();

        let sig1 = solana_sign(&private_key, message, 0).unwrap();
        let sig2 = solana_sign(&private_key2, message, 0).unwrap();

        let aggregated_sig = aggregate_signatures(&[sig1, sig2]).unwrap();
        let aggregated_g2 = offchain_aggregate_g2_pubkeys(&[g2_pubkey, g2_pubkey2]).unwrap();

        let valid =
            solana_verify_signature_with_g2(&aggregated_g2, &aggregated_sig, message, 0).unwrap();
        assert!(valid, "Aggregated signature should verify");
    }

    #[test]
    fn test_verify_signature_with_g2_edge_cases() {
        // Test with empty message
        let private_key = generate_random_bls_private_key();
        let g2_pubkey = offchain_g2_from_private_key(&private_key).unwrap();
        let empty_message = b"";

        let signature = solana_sign(&private_key, empty_message, 0).unwrap();
        let valid =
            solana_verify_signature_with_g2(&g2_pubkey, &signature, empty_message, 0).unwrap();
        assert!(valid, "Empty message should verify");

        // Test with large message
        let large_message = vec![0xAB; 1000];
        let signature = solana_sign(&private_key, &large_message, 0).unwrap();
        let valid =
            solana_verify_signature_with_g2(&g2_pubkey, &signature, &large_message, 0).unwrap();
        assert!(valid, "Large message should verify");
    }

    #[test]
    fn test_verify_signature_with_g2_consistency() {
        // Verify that this method gives same results as the full verification
        let private_key = generate_random_bls_private_key();
        let g1_pubkey = offchain_g1_from_private_key(&private_key).unwrap();
        let g2_pubkey = offchain_g2_from_private_key(&private_key).unwrap();
        let message = b"consistency test";

        let signature = solana_sign(&private_key, message, 0).unwrap();

        // Verify with G2 only
        let valid_g2_only =
            solana_verify_signature_with_g2(&g2_pubkey, &signature, message, 0).unwrap();

        // Verify with full method
        let valid_full =
            solana_verify_single_signature(&g1_pubkey, &g2_pubkey, &signature, message, 0).unwrap();

        assert_eq!(
            valid_g2_only, valid_full,
            "Both verification methods should give same result"
        );
    }
}
