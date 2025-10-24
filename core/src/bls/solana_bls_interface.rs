use ark_bn254::{Fq, Fq2, G1Projective, G2Affine, G2Projective};
use ark_ff::{One, PrimeField};
use serde_json;
use solana_bn254::compression::prelude::{
    alt_bn128_g1_compress, alt_bn128_g1_decompress, alt_bn128_g2_compress, alt_bn128_g2_decompress,
};
use solana_keypair::Keypair;

use crate::bls::solana_bls::{offchain_g1_from_private_key, offchain_g2_from_private_key};

pub type SolanaBN254Signature = SolanaBN254G1;
#[derive(Clone, Copy)]
pub struct SolanaBN254G1 {
    pub point: G1Projective,
    pub raw: [u8; 64],
    pub compressed: [u8; 32],
}

impl SolanaBN254G1 {
    pub fn new(bytes: &[u8; 64]) -> Result<Self, String> {
        let x = Fq::from_be_bytes_mod_order(&bytes[0..32]);
        let y = Fq::from_be_bytes_mod_order(&bytes[32..64]);
        let point = G1Projective::new(x, y, Fq::one());
        let compressed = alt_bn128_g1_compress(bytes)
            .map_err(|e| format!("Could not compress bytes: {:?}", e))?;

        Ok(Self {
            point,
            raw: *bytes,
            compressed,
        })
    }

    pub fn from_compressed(compressed: &[u8; 32]) -> Result<Self, String> {
        let decompressed = alt_bn128_g1_decompress(compressed)
            .map_err(|e| format!("Could not decompress bytes: {:?}", e))?;

        Self::new(&decompressed)
    }

    pub fn from_hex(string: &str) -> Result<Self, String> {
        let bytes =
            hex::decode(string).map_err(|e| format!("Could not decode hex string: {:?}", e))?;
        let bytes = bytes.try_into().map_err(|_| "Invalid length".to_string())?;

        Self::new(&bytes)
    }

    pub fn from_compressed_hex(string: &str) -> Result<Self, String> {
        let bytes =
            hex::decode(string).map_err(|e| format!("Could not decode hex string: {:?}", e))?;
        let bytes = bytes.try_into().map_err(|_| "Invalid length".to_string())?;

        Self::from_compressed(&bytes)
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.raw)
    }

    pub fn to_compressed_hex(&self) -> String {
        hex::encode(self.compressed)
    }
}

#[derive(Clone, Copy)]
pub struct SolanaBN254G2 {
    pub point: G2Projective,
    pub raw: [u8; 128],
    pub compressed: [u8; 64],
}

impl SolanaBN254G2 {
    pub fn new(bytes: &[u8; 128]) -> Result<Self, String> {
        // Byte layout: [x.c1, x.c0, y.c1, y.c0] (32 bytes each)
        let x_c1 = Fq::from_be_bytes_mod_order(&bytes[0..32]);
        let x_c0 = Fq::from_be_bytes_mod_order(&bytes[32..64]);
        let x = Fq2::new(x_c0, x_c1);

        let y_c1 = Fq::from_be_bytes_mod_order(&bytes[64..96]);
        let y_c0 = Fq::from_be_bytes_mod_order(&bytes[96..128]);
        let y = Fq2::new(y_c0, y_c1);

        let affine_point = G2Affine::new(x, y);
        if !affine_point.is_on_curve() {
            return Err("Point is not on the G2 curve".to_string());
        }

        let point = G2Projective::from(affine_point);

        let compressed = alt_bn128_g2_compress(bytes)
            .map_err(|e| format!("Could not compress bytes: {:?}", e))?;

        Ok(Self {
            point,
            raw: *bytes,
            compressed,
        })
    }

    pub fn from_compressed(compressed: &[u8; 64]) -> Result<Self, String> {
        let decompressed = alt_bn128_g2_decompress(compressed)
            .map_err(|e| format!("Could not decompress bytes: {:?}", e))?;

        Self::new(&decompressed)
    }

    pub fn from_hex(string: &str) -> Result<Self, String> {
        let bytes =
            hex::decode(string).map_err(|e| format!("Could not decode hex string: {:?}", e))?;
        let bytes = bytes.try_into().map_err(|_| "Invalid length".to_string())?;

        Self::new(&bytes)
    }

    pub fn from_compressed_hex(string: &str) -> Result<Self, String> {
        let bytes =
            hex::decode(string).map_err(|e| format!("Could not decode hex string: {:?}", e))?;
        let bytes = bytes.try_into().map_err(|_| "Invalid length".to_string())?;

        Self::from_compressed(&bytes)
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.raw)
    }

    pub fn to_compressed_hex(&self) -> String {
        hex::encode(self.compressed)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SolanaBN254PublicKey {
    pub g1: SolanaBN254G1,
    pub g2: SolanaBN254G2,
}

impl SolanaBN254PublicKey {
    pub fn new(g1: SolanaBN254G1, g2: SolanaBN254G2) -> Self {
        Self { g1, g2 }
    }

    /// Serialize public key to JSON string
    pub fn to_json(&self) -> Result<String, String> {
        let json_obj = serde_json::json!({
            "g1": {
                "raw": hex::encode(self.g1.raw),
                "compressed": hex::encode(self.g1.compressed)
            },
            "g2": {
                "raw": hex::encode(self.g2.raw),
                "compressed": hex::encode(self.g2.compressed)
            }
        });

        serde_json::to_string(&json_obj)
            .map_err(|e| format!("Failed to serialize to JSON: {:?}", e))
    }

    /// Serialize public key to pretty-printed JSON string
    pub fn to_json_pretty(&self) -> Result<String, String> {
        let json_obj = serde_json::json!({
            "g1": {
                "raw": hex::encode(self.g1.raw),
                "compressed": hex::encode(self.g1.compressed)
            },
            "g2": {
                "raw": hex::encode(self.g2.raw),
                "compressed": hex::encode(self.g2.compressed)
            }
        });

        serde_json::to_string_pretty(&json_obj)
            .map_err(|e| format!("Failed to serialize to JSON: {:?}", e))
    }

    /// Deserialize public key from JSON string
    pub fn from_json(json_str: &str) -> Result<Self, String> {
        let json_obj: serde_json::Value =
            serde_json::from_str(json_str).map_err(|e| format!("Failed to parse JSON: {:?}", e))?;

        let g1_raw_hex = json_obj["g1"]["raw"]
            .as_str()
            .ok_or("Missing or invalid g1.raw field")?;
        let g1 = SolanaBN254G1::from_hex(g1_raw_hex)?;

        let g2_raw_hex = json_obj["g2"]["raw"]
            .as_str()
            .ok_or("Missing or invalid g2.raw field")?;
        let g2 = SolanaBN254G2::from_hex(g2_raw_hex)?;

        Ok(Self::new(g1, g2))
    }
}

#[derive(Clone, Copy)]
pub struct SolanaBN254Keypair {
    pub private_key: [u8; 32],
    pub public_key: SolanaBN254PublicKey,
}

impl SolanaBN254Keypair {
    pub fn new(private_key: &[u8; 32]) -> Result<Self, String> {
        let g1_bytes = offchain_g1_from_private_key(private_key)?;
        let g1 = SolanaBN254G1::new(&g1_bytes)?;

        let g2_bytes = offchain_g2_from_private_key(private_key)?;
        let g2 = SolanaBN254G2::new(&g2_bytes)?;

        let public_key = SolanaBN254PublicKey::new(g1, g2);

        Ok(Self {
            private_key: *private_key,
            public_key,
        })
    }

    pub fn new_from_keypair(keypair: &Keypair) -> Result<Self, String> {
        let private_key = keypair.secret_bytes();
        Self::new(private_key)
    }

    pub fn new_unique() -> Result<Self, String> {
        let keypair = Keypair::new();
        let private_key = keypair.secret_bytes();
        Self::new(private_key)
    }

    pub fn from_hex(private_key: &str) -> Result<Self, String> {
        let private_key = hex::decode(private_key).map_err(|_| "Invalid private key")?;
        let private_key = private_key
            .try_into()
            .map_err(|_| "Invalid private key length")?;
        Self::new(&private_key)
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.private_key)
    }

    /// Serialize keypair to JSON string
    pub fn to_json(&self) -> Result<String, String> {
        let json_obj = serde_json::json!({
            "private_key": hex::encode(self.private_key),
            "public_key": {
                "g1": {
                    "raw": hex::encode(self.public_key.g1.raw),
                    "compressed": hex::encode(self.public_key.g1.compressed)
                },
                "g2": {
                    "raw": hex::encode(self.public_key.g2.raw),
                    "compressed": hex::encode(self.public_key.g2.compressed)
                }
            }
        });

        serde_json::to_string(&json_obj)
            .map_err(|e| format!("Failed to serialize to JSON: {:?}", e))
    }

    /// Serialize keypair to pretty-printed JSON string
    pub fn to_json_pretty(&self) -> Result<String, String> {
        let json_obj = serde_json::json!({
            "private_key": hex::encode(self.private_key),
            "public_key": {
                "g1": {
                    "raw": hex::encode(self.public_key.g1.raw),
                    "compressed": hex::encode(self.public_key.g1.compressed)
                },
                "g2": {
                    "raw": hex::encode(self.public_key.g2.raw),
                    "compressed": hex::encode(self.public_key.g2.compressed)
                }
            }
        });

        serde_json::to_string_pretty(&json_obj)
            .map_err(|e| format!("Failed to serialize to JSON: {:?}", e))
    }

    /// Deserialize keypair from JSON string
    pub fn from_json(json_str: &str) -> Result<Self, String> {
        let json_obj: serde_json::Value =
            serde_json::from_str(json_str).map_err(|e| format!("Failed to parse JSON: {:?}", e))?;

        let private_key_hex = json_obj["private_key"]
            .as_str()
            .ok_or("Missing or invalid private_key field")?;

        Self::from_hex(private_key_hex)
    }

    /// Write keypair to JSON file
    pub fn to_json_file(&self, path: &str) -> Result<(), String> {
        let json_str = self.to_json_pretty()?;
        std::fs::write(path, json_str).map_err(|e| format!("Failed to write to file: {:?}", e))
    }

    /// Read keypair from JSON file
    pub fn from_json_file(path: &str) -> Result<Self, String> {
        let json_str =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {:?}", e))?;
        Self::from_json(&json_str)
    }
}

#[cfg(test)]
mod tests {
    use crate::bls::solana_bls::{aggregate_signatures, offchain_parse_g2_point, solana_sign};

    use super::*;
    use ark_bn254::{Fr, G1Affine};
    use ark_ff::BigInteger;
    use ark_ff::PrimeField;
    use ark_serialize::CanonicalDeserialize;
    use ark_serialize::CanonicalSerialize;
    use hex;
    use solana_keypair::Keypair;

    // Bread (formerly BN254) crate imports
    use bn254::{
        aggregate_signatures as bread_aggregate_signatures, Bn254 as Bread,
        G1PublicKey as BreadG1PublicKey, PrivateKey as BreadPrivateKey,
        Signature as BreadSignature,
    };
    use commonware_cryptography::{Signer, Verifier};

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

    // --------------------- Bread (BN254) Package Tests ----------------------

    // Here's the fix for the failing test. The issue is that the Bread crate expects
    // private keys to be serialized in arkworks compressed format, not raw bytes.

    #[test]
    fn test_compatibility_with_bread_crate() {
        // Test data - create a valid scalar and serialize it properly
        let private_key_scalar = Fr::from(42u64); // Valid scalar
        let mut private_key_bytes = generate_random_bls_private_key();
        private_key_scalar
            .serialize_compressed(&mut private_key_bytes[..])
            .expect("Failed to serialize scalar");

        let message = b"test message for compatibility";
        let domain = b"TEST_DOMAIN_MESSAGE";

        // ====================================================================
        // Test 1: Private Key Compatibility
        // ====================================================================
        {
            println!("\n=== Testing Private Key Compatibility ===");

            // Create Bread private key using the serialized scalar
            let _bread_privkey = BreadPrivateKey::try_from(private_key_bytes.to_vec())
                .expect("Failed to create Bread private key");

            // Verify the scalar value matches
            let our_scalar = Fr::deserialize_compressed(&private_key_bytes[..])
                .expect("Failed to deserialize scalar");

            // The Bread crate stores the key internally, we can verify by deriving public keys
            println!("✓ Private key created successfully in both formats");
            println!("  Scalar value: {:?}", our_scalar);
        }

        // ====================================================================
        // Test 2: G1 Public Key Derivation and Compatibility
        // ====================================================================
        {
            println!("\n=== Testing G1 Public Key Compatibility ===");

            // For our implementation, we need the scalar in big-endian format
            let our_private_key_be_vec = private_key_scalar.into_bigint().to_bytes_be();
            let our_private_key_be: [u8; 32] = our_private_key_be_vec.try_into().unwrap();

            let our_g1_pubkey = offchain_g1_from_private_key(&our_private_key_be)
                .expect("Failed to derive G1 pubkey");

            // Create Bread signer and derive G1 pubkey
            let bread_signer =
                Bread::new(BreadPrivateKey::try_from(private_key_bytes.to_vec()).unwrap())
                    .expect("Failed to create Bread signer");
            let bread_g1_pubkey = bread_signer.public_g1();

            // Extract coordinates from our uncompressed format
            let our_x = Fq::from_be_bytes_mod_order(&our_g1_pubkey[0..32]);
            let our_y = Fq::from_be_bytes_mod_order(&our_g1_pubkey[32..64]);

            // Get coordinates from Bread G1PublicKey
            let bread_x_str = bread_g1_pubkey.get_x();
            let bread_y_str = bread_g1_pubkey.get_y();

            // Compare coordinates
            assert_eq!(
                our_x.to_string(),
                bread_x_str,
                "G1 X coordinates should match"
            );
            assert_eq!(
                our_y.to_string(),
                bread_y_str,
                "G1 Y coordinates should match"
            );

            println!("✓ G1 public keys match perfectly");
            println!("  X: {}", bread_x_str);
            println!("  Y: {}", bread_y_str);
        }

        // ====================================================================
        // Test 3: G2 Public Key Derivation and Compatibility
        // ====================================================================
        {
            println!("\n=== Testing G2 Public Key Compatibility ===");

            // Derive G2 pubkey using our implementation with big-endian bytes
            let our_private_key_be_vec = private_key_scalar.into_bigint().to_bytes_be();
            let our_private_key_be: [u8; 32] = our_private_key_be_vec.try_into().unwrap();
            let our_g2_pubkey = offchain_g2_from_private_key(&our_private_key_be)
                .expect("Failed to derive G2 pubkey");

            // Get G2 pubkey from Bread signer
            let bread_signer =
                Bread::new(BreadPrivateKey::try_from(private_key_bytes.to_vec()).unwrap())
                    .expect("Failed to create Bread signer");
            let bread_g2_pubkey = bread_signer.public_key();

            // Bread crate uses compressed format, we need to parse our uncompressed format
            // and potentially compress it for comparison
            let our_g2_point =
                offchain_parse_g2_point(&our_g2_pubkey).expect("Failed to parse our G2 point");

            // Serialize our G2 point to compressed format for comparison
            let mut our_g2_compressed = vec![0u8; 64]; // G2 compressed is 64 bytes
            our_g2_point
                .serialize_compressed(&mut our_g2_compressed[..])
                .expect("Failed to compress G2 point");

            // Compare the compressed representations
            assert_eq!(
                &our_g2_compressed[..],
                bread_g2_pubkey.as_ref(),
                "G2 public keys should match in compressed format"
            );

            println!("✓ G2 public keys match in compressed format");
            println!("  Compressed G2 (hex): {}", hex::encode(&our_g2_compressed));
        }

        // Signatures will not match as the hash function used in the signature generation process is different.
        // ====================================================================
        // Test 4: Signature Generation Compatibility
        // ====================================================================
        {
            println!("\n=== Testing Signature Generation Compatibility ===");

            // Generate signature using our implementation with big-endian private key
            let our_private_key_be_vec = private_key_scalar.into_bigint().to_bytes_be();
            let our_private_key_be: [u8; 32] = our_private_key_be_vec.try_into().unwrap();
            let our_signature = solana_sign(&our_private_key_be, message, Some(domain))
                .expect("Failed to generate signature");

            // Generate signature using Bread crate
            let bread_signer =
                Bread::new(BreadPrivateKey::try_from(private_key_bytes.to_vec()).unwrap())
                    .expect("Failed to create Bread signer");
            let bread_signature = bread_signer.sign(Some(domain), message);

            // Parse our signature to get the G1 point
            let our_sig_x = Fq::from_be_bytes_mod_order(&our_signature[0..32]);
            let our_sig_y = Fq::from_be_bytes_mod_order(&our_signature[32..64]);
            let our_sig_point = G1Affine::new_unchecked(our_sig_x, our_sig_y);

            // Serialize our signature to compressed format
            let mut our_sig_compressed = generate_random_bls_private_key().to_vec(); // G1 compressed is 32 bytes
            our_sig_point
                .serialize_compressed(&mut our_sig_compressed[..])
                .expect("Failed to compress signature");

            // Compare compressed signatures
            assert_eq!(
                &our_sig_compressed[..],
                bread_signature.as_ref(),
                "Signatures should match in compressed format"
            );

            println!("✓ Signatures match perfectly");
            println!(
                "  Compressed signature (hex): {}",
                hex::encode(&our_sig_compressed)
            );
        }

        // ====================================================================
        // Test 5: Cross-Verification (Sign with ours, verify with theirs)
        // ====================================================================
        {
            println!("\n=== Testing Cross-Verification ===");

            // Sign with our implementation
            let our_private_key_be_vec = private_key_scalar.into_bigint().to_bytes_be();
            let our_private_key_be: [u8; 32] = our_private_key_be_vec.try_into().unwrap();
            let our_signature = solana_sign(&our_private_key_be, message, Some(domain))
                .expect("Failed to generate signature");

            // Convert to compressed format for Bread crate
            let sig_x = Fq::from_be_bytes_mod_order(&our_signature[0..32]);
            let sig_y = Fq::from_be_bytes_mod_order(&our_signature[32..64]);
            let sig_point = G1Affine::new_unchecked(sig_x, sig_y);

            let mut sig_compressed = generate_random_bls_private_key().to_vec();
            sig_point
                .serialize_compressed(&mut sig_compressed[..])
                .expect("Failed to compress signature");

            // Create Bread signature object
            let bread_signature = BreadSignature::try_from(sig_compressed.as_slice())
                .expect("Failed to create Bread signature");

            // Get public key for verification
            let bread_signer =
                Bread::new(BreadPrivateKey::try_from(private_key_bytes.to_vec()).unwrap())
                    .expect("Failed to create Bread signer");
            let bread_pubkey = bread_signer.public_key();

            // Verify using Bread crate
            let is_valid = bread_pubkey.verify(Some(domain), message, &bread_signature);
            assert!(is_valid, "Cross-verification should succeed");

            println!("✓ Cross-verification successful: signed with ours, verified with theirs");
        }

        // ====================================================================
        // Test 6: Signature Aggregation Compatibility
        // ====================================================================
        {
            println!("\n=== Testing Signature Aggregation Compatibility ===");

            // Create multiple signatures with properly serialized private keys
            let scalars = [Fr::from(1u64), Fr::from(2u64), Fr::from(3u64)];

            let mut our_signatures = Vec::new();
            let mut bread_signatures = Vec::new();

            for scalar in &scalars {
                // Serialize scalar for Bread crate
                let mut serialized_key = generate_random_bls_private_key();
                scalar
                    .serialize_compressed(&mut serialized_key[..])
                    .expect("Failed to serialize key");

                // Convert to big-endian for our implementation
                let be_key_vec = scalar.into_bigint().to_bytes_be();
                let be_key: [u8; 32] = be_key_vec.try_into().unwrap();

                // Our signature
                let our_sig = solana_sign(&be_key, message, Some(domain))
                    .expect("Failed to generate signature");
                our_signatures.push(our_sig);

                // Bread signature
                let bread_privkey = BreadPrivateKey::try_from(serialized_key.to_vec())
                    .expect("Failed to create Bread private key");
                let bread_signer = Bread::new(bread_privkey).expect("Failed to create signer");
                let bread_sig = bread_signer.sign(Some(domain), message);
                bread_signatures.push(bread_sig);
            }

            // Aggregate using our implementation
            let our_agg_sig =
                aggregate_signatures(&our_signatures).expect("Failed to aggregate signatures");

            // Aggregate using Bread crate
            let bread_agg_sig = bread_aggregate_signatures(&bread_signatures)
                .expect("Failed to aggregate with Bread crate");

            // Convert our aggregated signature to compressed format
            let agg_x = Fq::from_be_bytes_mod_order(&our_agg_sig[0..32]);
            let agg_y = Fq::from_be_bytes_mod_order(&our_agg_sig[32..64]);
            let agg_point = G1Affine::new_unchecked(agg_x, agg_y);

            let mut our_agg_compressed = generate_random_bls_private_key().to_vec();
            agg_point
                .serialize_compressed(&mut our_agg_compressed[..])
                .expect("Failed to compress aggregated signature");

            // Compare aggregated signatures
            assert_eq!(
                &our_agg_compressed[..],
                bread_agg_sig.as_ref(),
                "Aggregated signatures should match"
            );

            println!("✓ Signature aggregation compatible between implementations");
        }

        // ====================================================================
        // Test 7: G1PublicKey creation from coordinates
        // ====================================================================
        {
            println!("\n=== Testing G1PublicKey coordinate construction ===");

            // Get a G1 point from our implementation
            let our_private_key_be_vec = private_key_scalar.into_bigint().to_bytes_be();
            let our_private_key_be: [u8; 32] = our_private_key_be_vec.try_into().unwrap();
            let our_g1 = offchain_g1_from_private_key(&our_private_key_be)
                .expect("Failed to derive G1 pubkey");

            // Extract coordinates
            let x = Fq::from_be_bytes_mod_order(&our_g1[0..32]);
            let y = Fq::from_be_bytes_mod_order(&our_g1[32..64]);

            // Create Bread G1PublicKey from coordinates
            let bread_g1 =
                BreadG1PublicKey::create_from_g1_coordinates(&x.to_string(), &y.to_string())
                    .expect("Failed to create G1PublicKey from coordinates");

            // Verify they match
            assert_eq!(bread_g1.get_x(), x.to_string(), "X coordinate should match");
            assert_eq!(bread_g1.get_y(), y.to_string(), "Y coordinate should match");

            println!("✓ G1PublicKey coordinate construction works correctly");
        }

        println!("\n✅ All compatibility tests passed!");
    }
}
