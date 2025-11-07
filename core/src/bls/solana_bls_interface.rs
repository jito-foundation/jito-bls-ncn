use ark_bn254::{Fq, Fq2, G1Projective, G2Affine, G2Projective};
use ark_ff::{One, PrimeField};
use serde_json;
use solana_bn254::compression::prelude::{
    alt_bn128_g1_compress, alt_bn128_g1_decompress, alt_bn128_g2_compress, alt_bn128_g2_decompress,
};
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;

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

// ------------------------------------------------------------------------
// ---- TEST FIXTURES
// ------------------------------------------------------------------------

pub struct TestBlsOperator {
    pub bls_keypair: SolanaBN254Keypair,
    pub bls_public_key: SolanaBN254PublicKey,
    pub solana_operator: Pubkey,
}

impl TestBlsOperator {
    pub fn new(bls_keypair: SolanaBN254Keypair, solana_operator: Pubkey) -> Self {
        Self {
            bls_keypair,
            bls_public_key: bls_keypair.public_key,
            solana_operator,
        }
    }
}

pub struct TestBlsVault {
    pub solana_vault: Pubkey,
}

impl TestBlsVault {
    pub fn new(solana_vault: Pubkey) -> Self {
        Self { solana_vault }
    }
}

pub struct TestBlsOrchestrator {
    pub bls_keypair: SolanaBN254Keypair,
    pub bls_public_key: SolanaBN254PublicKey,
}

impl TestBlsOrchestrator {
    pub fn new(bls_keypair: SolanaBN254Keypair) -> Self {
        Self {
            bls_keypair,
            bls_public_key: bls_keypair.public_key,
        }
    }
}

pub struct TestBlsNcn {
    pub rpc_url: String,
    pub solana_test_keypair: Keypair,
    pub solana_ncn: Pubkey,
    pub bls_orchestrator: TestBlsOrchestrator,
    pub test_operators: Vec<TestBlsOperator>,
    pub test_vaults: Vec<TestBlsVault>,
}

impl TestBlsNcn {
    pub fn new(
        rpc_url: String,
        solana_test_keypair: Keypair,
        solana_ncn: Pubkey,
        bls_orchestrator: TestBlsOrchestrator,
        test_operators: Vec<TestBlsOperator>,
        test_vaults: Vec<TestBlsVault>,
    ) -> Self {
        Self {
            rpc_url,
            solana_test_keypair,
            solana_ncn,
            bls_orchestrator,
            test_operators,
            test_vaults,
        }
    }

    pub fn to_json(&self) -> Result<String, String> {
        let operators_json: Result<Vec<_>, String> = self
            .test_operators
            .iter()
            .map(|op| {
                Ok(serde_json::json!({
                    "bls_keypair": serde_json::from_str::<serde_json::Value>(&op.bls_keypair.to_json()?)
                        .map_err(|e| format!("Failed to parse operator keypair: {:?}", e))?,
                    "bls_public_key": serde_json::from_str::<serde_json::Value>(&op.bls_public_key.to_json()?)
                        .map_err(|e| format!("Failed to parse operator public key: {:?}", e))?,
                    "solana_operator": op.solana_operator.to_string()
                }))
            })
            .collect();

        let vaults_json: Vec<_> = self
            .test_vaults
            .iter()
            .map(|vault| {
                serde_json::json!({
                    "solana_vault": vault.solana_vault.to_string()
                })
            })
            .collect();

        let json_obj = serde_json::json!({
            "rpc_url": self.rpc_url,
            "solana_test_keypair": hex::encode(self.solana_test_keypair.secret_bytes()),
            "solana_ncn": self.solana_ncn.to_string(),
            "bls_orchestrator": {
                "bls_keypair": serde_json::from_str::<serde_json::Value>(&self.bls_orchestrator.bls_keypair.to_json()?)
                    .map_err(|e| format!("Failed to parse orchestrator keypair: {:?}", e))?,
                "bls_public_key": serde_json::from_str::<serde_json::Value>(&self.bls_orchestrator.bls_public_key.to_json()?)
                    .map_err(|e| format!("Failed to parse orchestrator public key: {:?}", e))?
            },
            "test_operators": operators_json?,
            "test_vaults": vaults_json
        });

        serde_json::to_string_pretty(&json_obj)
            .map_err(|e| format!("Failed to serialize to JSON: {:?}", e))
    }

    pub fn from_json(json_str: &str) -> Result<Self, String> {
        let json_obj: serde_json::Value =
            serde_json::from_str(json_str).map_err(|e| format!("Failed to parse JSON: {:?}", e))?;

        let rpc_url = json_obj["rpc_url"]
            .as_str()
            .ok_or("Missing or invalid rpc_url field")?;

        let keypair_hex = json_obj["solana_test_keypair"]
            .as_str()
            .ok_or("Missing or invalid solana_test_keypair field")?;
        let keypair_bytes = hex::decode(keypair_hex)
            .map_err(|e| format!("Failed to decode keypair hex: {:?}", e))?;
        let keypair_array: [u8; 32] = keypair_bytes
            .try_into()
            .map_err(|_| "Invalid keypair length, expected 32 bytes".to_string())?;
        let solana_test_keypair = Keypair::new_from_array(keypair_array);

        let solana_ncn = json_obj["solana_ncn"]
            .as_str()
            .ok_or("Missing or invalid solana_ncn field")?
            .parse::<Pubkey>()
            .map_err(|e| format!("Failed to parse solana_ncn pubkey: {:?}", e))?;

        let orchestrator_keypair = SolanaBN254Keypair::from_json(
            &serde_json::to_string(&json_obj["bls_orchestrator"]["bls_keypair"])
                .map_err(|e| format!("Failed to serialize orchestrator keypair: {:?}", e))?,
        )?;

        let orchestrator_public_key = SolanaBN254PublicKey::from_json(
            &serde_json::to_string(&json_obj["bls_orchestrator"]["bls_public_key"])
                .map_err(|e| format!("Failed to serialize orchestrator public key: {:?}", e))?,
        )?;

        let bls_orchestrator = TestBlsOrchestrator {
            bls_keypair: orchestrator_keypair,
            bls_public_key: orchestrator_public_key,
        };

        let test_operators = json_obj["test_operators"]
            .as_array()
            .ok_or("Missing or invalid test_operators field")?
            .iter()
            .map(|op| {
                let bls_keypair = SolanaBN254Keypair::from_json(
                    &serde_json::to_string(&op["bls_keypair"])
                        .map_err(|e| format!("Failed to serialize operator keypair: {:?}", e))?,
                )?;
                let bls_public_key = SolanaBN254PublicKey::from_json(
                    &serde_json::to_string(&op["bls_public_key"])
                        .map_err(|e| format!("Failed to serialize operator public key: {:?}", e))?,
                )?;
                let solana_operator = op["solana_operator"]
                    .as_str()
                    .ok_or("Missing or invalid solana_operator field")?
                    .parse::<Pubkey>()
                    .map_err(|e| format!("Failed to parse solana_operator pubkey: {:?}", e))?;

                Ok(TestBlsOperator {
                    bls_keypair,
                    bls_public_key,
                    solana_operator,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;

        let test_vaults = json_obj["test_vaults"]
            .as_array()
            .ok_or("Missing or invalid test_vaults field")?
            .iter()
            .map(|vault| {
                let solana_vault = vault["solana_vault"]
                    .as_str()
                    .ok_or("Missing or invalid solana_vault field")?
                    .parse::<Pubkey>()
                    .map_err(|e| format!("Failed to parse solana_vault pubkey: {:?}", e))?;

                Ok(TestBlsVault { solana_vault })
            })
            .collect::<Result<Vec<_>, String>>()?;

        Ok(Self::new(
            rpc_url.to_string(),
            solana_test_keypair,
            solana_ncn,
            bls_orchestrator,
            test_operators,
            test_vaults,
        ))
    }

    pub fn to_json_file(&self, path: &str) -> Result<(), String> {
        let json_str = self.to_json()?;
        std::fs::write(path, json_str).map_err(|e| format!("Failed to write to file: {:?}", e))
    }

    pub fn from_json_file(path: &str) -> Result<Self, String> {
        let json_str =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read file: {:?}", e))?;
        Self::from_json(&json_str)
    }
}
