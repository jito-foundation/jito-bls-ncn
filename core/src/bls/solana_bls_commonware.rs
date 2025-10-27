use ark_bn254::{Fr as Scalar, G1Affine, G2Affine};
use ark_ec::AffineRepr;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use commonware_cryptography::{
    PublicKey as CommonwarePublicKey, Signature as CommonwareSignature, Signer, Verifier,
};

use bytes::buf::BufMut;
use bytes::Buf;
use commonware_codec::{Error, FixedSize, Read, Write};
use commonware_utils::{array::Array, hex};
use std::{
    fmt::{Debug, Display},
    hash::{Hash, Hasher},
    ops::Deref,
};

use crate::bls::{
    solana_bls::{solana_sign, solana_verify_signature_with_g2, solana_verify_single_signature},
    solana_bls_interface::{
        SolanaBN254G1, SolanaBN254G2, SolanaBN254Keypair, SolanaBN254Signature,
    },
};

// const DIGEST_LENGTH: usize = 32;
// const PRIVATE_KEY_LENGTH: usize = 32;
// const G1_LENGTH: usize = 32;
// const SIGNATURE_LENGTH: usize = G1_LENGTH;
// const G2_LENGTH: usize = 64;
// const PUBLIC_KEY_LENGTH: usize = G2_LENGTH;

// const DIGEST_LENGTH: usize = 32;
const PRIVATE_KEY_LENGTH: usize = 32;
const G1_LENGTH: usize = 64;
const SIGNATURE_LENGTH: usize = G1_LENGTH;
const G2_LENGTH: usize = 128;
const PUBLIC_KEY_LENGTH: usize = G2_LENGTH;

impl SolanaBN254Keypair {
    pub fn solana_sign(&self, message: &[u8], consensus_count: u64) -> SolanaBN254Signature {
        let consensus_bytes = consensus_count.to_le_bytes();
        let namespace = Some(consensus_bytes.as_slice());
        self.sign(namespace, message)
    }

    pub fn solana_verify(&self, message: &[u8], signature: &SolanaBN254Signature, consensus_count: u64) -> bool {
        let consensus_bytes = consensus_count.to_le_bytes();
        let namespace = Some(consensus_bytes.as_slice());
        self.verify(namespace, message, signature)
    }
}

impl Signer for SolanaBN254Keypair {
    type Signature = SolanaBN254Signature;
    type PublicKey = SolanaBN254G2;

    /// Namespace actually needs to be the consensus count in Option<Byte>format
    fn sign(&self, namespace: Option<&[u8]>, message: &[u8]) -> Self::Signature {

        if namespace.is_none() {
            panic!("Consensus count is required ( Namespace, with consensus count (u64) as be bytes) - use SolanaBN254Keypair::solana_sign");
        }

        let consensus_bytes = namespace.expect("Could not unwrap consensus bytes");
        let consensus_count = u64::from_le_bytes(
            consensus_bytes[..8].try_into().expect("slice with incorrect length")
        );

        let raw_signature =
            solana_sign(&self.private_key, message, consensus_count).expect("Could not sign");
        SolanaBN254Signature::new(&raw_signature).expect("Could not create signature")
    }

    fn public_key(&self) -> Self::PublicKey {
        self.public_key.g2
    }
}

impl Verifier for SolanaBN254Keypair {
    type Signature = SolanaBN254Signature;

    fn verify(
        &self,
        namespace: Option<&[u8]>,
        message: &[u8],
        signature: &Self::Signature,
    ) -> bool {
        let g1 = &self.public_key.g1.raw;
        let g2 = &self.public_key.g2.raw;

        if namespace.is_none() {
            panic!("Consensus count is required ( Namespace, with consensus count (u64) as be bytes) - use SolanaBN254Keypair::solana_verify");
        }

        let consensus_bytes = namespace.expect("Could not unwrap consensus bytes");
        let consensus_count = u64::from_le_bytes(
            consensus_bytes[..8].try_into().expect("slice with incorrect length")
        );

        solana_verify_single_signature(g1, g2, &signature.raw, message, consensus_count)
            .expect("Could not verify signature")
    }
}

impl Array for SolanaBN254Keypair {}

impl FixedSize for SolanaBN254Keypair {
    const SIZE: usize = PRIVATE_KEY_LENGTH;
}

impl Write for SolanaBN254Keypair {
    fn write(&self, buf: &mut impl BufMut) {
        self.private_key.write(buf);
    }
}

impl Read for SolanaBN254Keypair {
    type Cfg = ();

    fn read_cfg(buf: &mut impl Buf, _cfg: &()) -> Result<Self, Error> {
        let mut raw = <[u8; PRIVATE_KEY_LENGTH]>::read_cfg(buf, &())?;
        let dst: &[u8] = &mut raw;
        Scalar::deserialize_compressed(dst).expect("Wrong Private Key");
        let keypair = SolanaBN254Keypair::new(&raw).expect("Could not create Keypair");
        Ok(keypair)
    }
}

impl Hash for SolanaBN254Keypair {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.private_key.hash(state);
    }
}

impl PartialEq for SolanaBN254Keypair {
    fn eq(&self, other: &Self) -> bool {
        self.private_key == other.private_key
    }
}

impl Eq for SolanaBN254Keypair {}

impl Ord for SolanaBN254Keypair {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.private_key.cmp(&other.private_key)
    }
}

impl PartialOrd for SolanaBN254Keypair {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl AsRef<[u8]> for SolanaBN254Keypair {
    fn as_ref(&self) -> &[u8] {
        &self.private_key
    }
}

impl Deref for SolanaBN254Keypair {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.private_key
    }
}

impl From<Scalar> for SolanaBN254Keypair {
    fn from(key: Scalar) -> Self {
        let mut raw = [0u8; PRIVATE_KEY_LENGTH];
        key.serialize_compressed(&mut raw[..]).unwrap();
        SolanaBN254Keypair::new(&raw).expect("Could not cast into private key")
    }
}

impl TryFrom<&[u8]> for SolanaBN254Keypair {
    type Error = Error;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let raw: [u8; PRIVATE_KEY_LENGTH] = value.try_into().expect("Invalid Private Key Length");
        let keypair = SolanaBN254Keypair::new(&raw).expect("Could not cast into private key");

        Ok(keypair)
    }
}

impl TryFrom<&Vec<u8>> for SolanaBN254Keypair {
    type Error = Error;
    fn try_from(value: &Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(value.as_slice())
    }
}

impl TryFrom<Vec<u8>> for SolanaBN254Keypair {
    type Error = Error;
    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(value.as_slice())
    }
}

impl Debug for SolanaBN254Keypair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex(&self.private_key))
    }
}

impl Display for SolanaBN254Keypair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex(&self.private_key))
    }
}

impl SolanaBN254G2 {
    pub fn solana_verify_g2(&self, message: &[u8], signature: &SolanaBN254Signature, consensus_count: u64) -> bool {
        let consensus_bytes = consensus_count.to_le_bytes();
        let namespace = Some(consensus_bytes.as_slice());
        self.verify(namespace, message, signature)
    }
}

impl Array for SolanaBN254G2 {}

impl FixedSize for SolanaBN254G2 {
    const SIZE: usize = PUBLIC_KEY_LENGTH;
}

impl Write for SolanaBN254G2 {
    fn write(&self, buf: &mut impl BufMut) {
        self.raw.write(buf);
    }
}

impl Read for SolanaBN254G2 {
    type Cfg = ();

    fn read_cfg(buf: &mut impl Buf, _cfg: &()) -> Result<Self, Error> {
        let mut raw = <[u8; PUBLIC_KEY_LENGTH]>::read_cfg(buf, &())?;
        let dst: &[u8] = &mut raw;
        let _ = G2Affine::deserialize_compressed(dst).expect("Wrong Public Key");
        let g2 = SolanaBN254G2::new(&raw).expect("Could not create G2");
        Ok(g2)
    }
}

impl Hash for SolanaBN254G2 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl PartialEq for SolanaBN254G2 {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl Eq for SolanaBN254G2 {}

impl Ord for SolanaBN254G2 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.raw.cmp(&other.raw)
    }
}

impl PartialOrd for SolanaBN254G2 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl AsRef<[u8]> for SolanaBN254G2 {
    fn as_ref(&self) -> &[u8] {
        &self.raw
    }
}

impl Deref for SolanaBN254G2 {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.raw
    }
}

impl From<G2Affine> for SolanaBN254G2 {
    fn from(key: G2Affine) -> Self {
        let mut raw = [0u8; PUBLIC_KEY_LENGTH];
        key.serialize_compressed(&mut raw[..]).unwrap();
        SolanaBN254G2::new(&raw).expect("Could not create G2")
    }
}

impl TryFrom<&[u8]> for SolanaBN254G2 {
    type Error = Error;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let raw: [u8; PUBLIC_KEY_LENGTH] =
            TryInto::<[u8; PUBLIC_KEY_LENGTH]>::try_into(value).expect("Invalid Public Key Length");
        let key = G2Affine::deserialize_compressed(value).expect("Invalid Public Key");
        if !key.is_in_correct_subgroup_assuming_on_curve() || !key.is_on_curve() || key.is_zero() {
            return Err(Error::InvalidUsize);
        }
        let g2 = SolanaBN254G2::new(&raw).expect("Could not create G2");
        Ok(g2)
    }
}

impl TryFrom<&Vec<u8>> for SolanaBN254G2 {
    type Error = Error;
    fn try_from(value: &Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(value.as_slice())
    }
}

impl TryFrom<Vec<u8>> for SolanaBN254G2 {
    type Error = Error;
    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(value.as_slice())
    }
}

impl Debug for SolanaBN254G2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex(&self.raw))
    }
}

impl Display for SolanaBN254G2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex(&self.raw))
    }
}

impl Verifier for SolanaBN254G2 {
    type Signature = SolanaBN254Signature;

    fn verify(
        &self,
        namespace: Option<&[u8]>,
        message: &[u8],
        signature: &Self::Signature,
    ) -> bool {

        if namespace.is_none() {
            panic!("Consensus count is required ( Namespace, with consensus count (u64) as be bytes) - use SolanaBN254G2::solana_verify_g2");
        }

        let consensus_bytes = namespace.expect("Could not unwrap consensus bytes");
        let consensus_count = u64::from_le_bytes(
            consensus_bytes[..8].try_into().expect("slice with incorrect length")
        );

        solana_verify_signature_with_g2(&self.raw, &signature.raw, message, consensus_count)
            .expect("Could not verify")
    }
}

impl CommonwarePublicKey for SolanaBN254G2 {}

impl CommonwareSignature for SolanaBN254Signature {}

impl Array for SolanaBN254Signature {}

impl FixedSize for SolanaBN254Signature {
    const SIZE: usize = SIGNATURE_LENGTH;
}

impl Write for SolanaBN254Signature {
    fn write(&self, buf: &mut impl BufMut) {
        self.raw.write(buf);
    }
}

impl Read for SolanaBN254Signature {
    type Cfg = ();

    fn read_cfg(buf: &mut impl Buf, _cfg: &()) -> Result<Self, Error> {
        let mut raw = <[u8; SIGNATURE_LENGTH]>::read_cfg(buf, &())?;
        let dst: &[u8] = &mut raw;
        let _ = G1Affine::deserialize_compressed(dst).expect("Wrong Signature");
        let g1 = SolanaBN254G1::new(&raw).expect("Could not create G1");
        Ok(g1)
    }
}

impl Hash for SolanaBN254Signature {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl PartialEq for SolanaBN254Signature {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}
impl Eq for SolanaBN254Signature {}
impl Ord for SolanaBN254Signature {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.raw.cmp(&other.raw)
    }
}
impl PartialOrd for SolanaBN254Signature {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl AsRef<[u8]> for SolanaBN254Signature {
    fn as_ref(&self) -> &[u8] {
        &self.raw
    }
}

impl Deref for SolanaBN254Signature {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.raw
    }
}

impl From<G1Affine> for SolanaBN254Signature {
    fn from(sig: G1Affine) -> Self {
        let mut raw = [0u8; SIGNATURE_LENGTH];
        sig.serialize_compressed(&mut raw[..]).unwrap();
        SolanaBN254G1::new(&raw).expect("Could not create G1")
    }
}

impl TryFrom<&[u8]> for SolanaBN254Signature {
    type Error = Error;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let raw: [u8; SIGNATURE_LENGTH] =
            TryInto::<[u8; SIGNATURE_LENGTH]>::try_into(value).expect("Invalid Signature Length");
        let sig = G1Affine::deserialize_compressed(value).expect("Invalid Signature");
        if !sig.is_in_correct_subgroup_assuming_on_curve() || !sig.is_on_curve() || sig.is_zero() {
            return Err(Error::InvalidBool);
        }
        let g1 = SolanaBN254G1::new(&raw).expect("Could not create G1");
        Ok(g1)
    }
}

impl TryFrom<&Vec<u8>> for SolanaBN254Signature {
    type Error = Error;
    fn try_from(value: &Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(value.as_slice())
    }
}

impl TryFrom<Vec<u8>> for SolanaBN254Signature {
    type Error = Error;
    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(value.as_slice())
    }
}

impl Debug for SolanaBN254Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex(&self.raw))
    }
}

impl Display for SolanaBN254Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex(&self.raw))
    }
}

// TODO Tests
