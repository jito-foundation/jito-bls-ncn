use core::convert::TryFrom;
use core::fmt;
use core::mem::MaybeUninit;

// ---------------- PODU16 ------------------------

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct PodU16 {
    pub data: [u8; 2],
}

impl From<u16> for PodU16 {
    #[inline(always)]
    fn from(value: u16) -> Self {
        Self {
            data: value.to_le_bytes(),
        }
    }
}

impl From<PodU16> for u16 {
    #[inline(always)]
    fn from(pod: PodU16) -> Self {
        u16::from_le_bytes(pod.data)
    }
}

impl PodU16 {
    #[inline(always)]
    pub fn get(&self) -> u16 {
        u16::from_le_bytes(self.data)
    }
    #[inline(always)]
    pub fn set(&mut self, value: u16) {
        self.data = value.to_le_bytes();
    }
}

// ---------------- PODU32 ------------------------

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct PodU32 {
    pub data: [u8; 4],
}

impl From<u32> for PodU32 {
    #[inline(always)]
    fn from(value: u32) -> Self {
        Self {
            data: value.to_le_bytes(),
        }
    }
}

impl From<PodU32> for u32 {
    #[inline(always)]
    fn from(pod: PodU32) -> Self {
        u32::from_le_bytes(pod.data)
    }
}

impl PodU32 {
    #[inline(always)]
    pub fn get(&self) -> u32 {
        u32::from_le_bytes(self.data)
    }
    #[inline(always)]
    pub fn set(&mut self, value: u32) {
        self.data = value.to_le_bytes();
    }
}

// ---------------- PODU64 ------------------------

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct PodU64 {
    pub data: [u8; 8],
}

impl From<u64> for PodU64 {
    #[inline(always)]
    fn from(value: u64) -> Self {
        Self {
            data: value.to_le_bytes(),
        }
    }
}

impl From<PodU64> for u64 {
    #[inline(always)]
    fn from(pod: PodU64) -> Self {
        u64::from_le_bytes(pod.data)
    }
}

impl PodU64 {
    #[inline(always)]
    pub fn get(&self) -> u64 {
        u64::from_le_bytes(self.data)
    }
    #[inline(always)]
    pub fn set(&mut self, value: u64) {
        self.data = value.to_le_bytes();
    }
}

// ---------------- PODU128 ------------------------

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct PodU128 {
    pub data: [u8; 16],
}

impl From<u128> for PodU128 {
    #[inline(always)]
    fn from(value: u128) -> Self {
        Self {
            data: value.to_le_bytes(),
        }
    }
}

impl From<PodU128> for u128 {
    #[inline(always)]
    fn from(pod: PodU128) -> Self {
        u128::from_le_bytes(pod.data)
    }
}

impl PodU128 {
    #[inline(always)]
    pub fn get(&self) -> u128 {
        u128::from_le_bytes(self.data)
    }
    #[inline(always)]
    pub fn set(&mut self, value: u128) {
        self.data = value.to_le_bytes();
    }
}

// ---------------- PODBool ------------------------

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct PodBool {
    pub data: u8, // 0 = false, 1 = true (other values invalid by convention)
}

impl PodBool {
    pub const TRUE: Self = Self { data: 1 };
    pub const FALSE: Self = Self { data: 0 };

    #[inline(always)]
    pub fn is_valid(&self) -> bool {
        self.data == 0 || self.data == 1
    }

    #[inline(always)]
    pub fn get(&self) -> bool {
        self.data != 0
    }

    #[inline(always)]
    pub fn set(&mut self, value: bool) {
        self.data = value as u8;
    }
}

// Permissive: any nonzero -> true (cannot fail)
impl From<PodBool> for bool {
    #[inline(always)]
    fn from(p: PodBool) -> Self {
        p.data != 0
    }
}

impl From<bool> for PodBool {
    #[inline(always)]
    fn from(b: bool) -> Self {
        Self { data: b as u8 }
    }
}

// ---------------- PODOption<T> ( Also 1 byte aligned ) ------------------------
// //! PodOption<T>: 1-byte-aligned, zero-copy-friendly Option-like container.
#[repr(C)]
pub struct PodOption<T> {
    /// 0 = None, 1 = Some (other values invalid)
    tag: u8,
    /// Uninitialized when tag == 0
    value: MaybeUninit<T>,
}

impl<T> PodOption<T> {
    pub const NONE_TAG: u8 = 0;
    pub const SOME_TAG: u8 = 1;

    /// `None` (leaves value uninitialized)
    #[inline(always)]
    pub const fn none() -> Self {
        Self {
            tag: Self::NONE_TAG,
            value: MaybeUninit::uninit(),
        }
    }

    /// `Some(v)`
    #[inline(always)]
    pub const fn some(v: T) -> Self {
        Self {
            tag: Self::SOME_TAG,
            value: MaybeUninit::new(v),
        }
    }

    #[inline(always)]
    pub const fn is_some(&self) -> bool {
        self.tag == Self::SOME_TAG
    }
    #[inline(always)]
    pub const fn is_none(&self) -> bool {
        self.tag == Self::NONE_TAG
    }
    #[inline(always)]
    pub const fn tag(&self) -> u8 {
        self.tag
    }
    #[inline(always)]
    pub const fn is_valid_tag(&self) -> bool {
        self.tag == 0 || self.tag == 1
    }

    /// `Some(&T)` if present
    #[inline(always)]
    pub fn as_ref(&self) -> Option<&T> {
        if self.is_some() {
            // SAFETY: tag==1 ⇒ initialized
            Some(unsafe { self.value.assume_init_ref() })
        } else {
            None
        }
    }

    /// `Some(&mut T)` if present
    #[inline(always)]
    pub fn as_mut(&mut self) -> Option<&mut T> {
        if self.is_some() {
            // SAFETY: tag==1 ⇒ initialized
            Some(unsafe { self.value.assume_init_mut() })
        } else {
            None
        }
    }

    /// Set to `None` (does not zero bytes)
    #[inline(always)]
    pub fn set_none(&mut self) {
        self.tag = Self::NONE_TAG;
        self.value = MaybeUninit::uninit();
    }

    /// Set to `Some(v)`
    #[inline(always)]
    pub fn set_some(&mut self, v: T) {
        self.tag = Self::SOME_TAG;
        self.value = MaybeUninit::new(v);
    }

    /// Copy-out without changing tag (requires `T: Copy`)
    #[inline(always)]
    pub fn copied(&self) -> Option<T>
    where
        T: Copy,
    {
        if self.is_some() {
            Some(unsafe { *self.value.assume_init_ref() })
        } else {
            None
        }
    }

    /// Take value and leave `None` (requires `T: Copy`)
    #[inline(always)]
    pub fn take(&mut self) -> Option<T>
    where
        T: Copy,
    {
        if self.is_some() {
            self.tag = Self::NONE_TAG;
            Some(unsafe { self.value.assume_init() })
        } else {
            None
        }
    }
}

// Conversions
impl<T> From<Option<T>> for PodOption<T> {
    #[inline(always)]
    fn from(o: Option<T>) -> Self {
        match o {
            Some(v) => Self::some(v),
            None => Self::none(),
        }
    }
}

impl<T> TryFrom<PodOption<T>> for Option<T> {
    type Error = ();
    #[inline(always)]
    fn try_from(p: PodOption<T>) -> Result<Self, Self::Error> {
        match p.tag {
            PodOption::<T>::NONE_TAG => Ok(None),
            PodOption::<T>::SOME_TAG => Ok(Some(unsafe { p.value.assume_init() })),
            _ => Err(()),
        }
    }
}

// Traits
impl<T> Default for PodOption<T> {
    #[inline(always)]
    fn default() -> Self {
        Self::none()
    }
}

impl<T: Copy> Copy for PodOption<T> {}
impl<T: Copy> Clone for PodOption<T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: PartialEq> PartialEq for PodOption<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self.as_ref(), other.as_ref()) {
            (Some(a), Some(b)) => a == b,
            (None, None) => true,
            _ => false,
        }
    }
}

impl<T: Eq> Eq for PodOption<T> {}

impl<T: fmt::Debug> fmt::Debug for PodOption<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(r) = self.as_ref() {
            f.debug_tuple("PodOption::Some").field(r).finish()
        } else {
            f.write_str("PodOption::None")
        }
    }
}

#[cfg(test)]
mod tests {

    use solana_pubkey::Pubkey;

    use super::*;
    use core::fmt::Debug;
    use core::mem::{align_of, size_of};

    // ---------- helpers ----------
    fn assert_copy_clone<T: Copy + Clone + PartialEq + Debug>(v: T) {
        let c = v;
        let cc = c;
        assert_eq!(v, c);
        assert_eq!(c, cc);
    }

    // ---------- size & align checks ----------
    #[test]
    fn size_align_basic() {
        assert_eq!(align_of::<u8>(), 1);
        assert_eq!(align_of::<PodU16>(), 1);
        assert_eq!(align_of::<PodU32>(), 1);
        assert_eq!(align_of::<PodU64>(), 1);
        assert_eq!(align_of::<PodU128>(), 1);
        assert_eq!(align_of::<PodBool>(), 1);

        assert_eq!(size_of::<PodU16>(), 2);
        assert_eq!(size_of::<PodU32>(), 4);
        assert_eq!(size_of::<PodU64>(), 8);
        assert_eq!(size_of::<PodU128>(), 16);
        assert_eq!(size_of::<PodBool>(), 1);

        // PodOption<T> layout: [tag: u8 | value: T-bytes]
        assert_eq!(align_of::<PodOption<u8>>(), 1);
        assert_eq!(size_of::<PodOption<u8>>(), 1 + size_of::<u8>());
        assert_eq!(align_of::<PodOption<PodU64>>(), 1);
        assert_eq!(size_of::<PodOption<PodU64>>(), 1 + size_of::<PodU64>());
        assert_eq!(align_of::<PodOption<Pubkey>>(), 1);
        assert_eq!(size_of::<PodOption<Pubkey>>(), 1 + size_of::<Pubkey>());
    }

    // ---------- PodU* round-trips ----------
    #[test]
    fn podu16_roundtrip() {
        let vals = [0u16, 1, 0x1234, u16::MAX];
        for &v in &vals {
            let p = PodU16::from(v);
            assert_eq!(p.get(), v);
            let back: u16 = p.into();
            assert_eq!(back, v);

            let mut m = PodU16::default();
            m.set(v);
            assert_eq!(m.get(), v);
        }
    }

    #[test]
    fn podu32_roundtrip() {
        let vals = [0u32, 1, 0x12_34_56_78, u32::MAX];
        for &v in &vals {
            let p = PodU32::from(v);
            assert_eq!(p.get(), v);
            let back: u32 = p.into();
            assert_eq!(back, v);

            let mut m = PodU32::default();
            m.set(v);
            assert_eq!(m.get(), v);
        }
    }

    #[test]
    fn podu64_roundtrip() {
        let vals = [0u64, 1, 0x0123_4567_89AB_CDEF, u64::MAX];
        for &v in &vals {
            let p = PodU64::from(v);
            assert_eq!(p.get(), v);
            let back: u64 = p.into();
            assert_eq!(back, v);

            let mut m = PodU64::default();
            m.set(v);
            assert_eq!(m.get(), v);
        }
    }

    #[test]
    fn podu128_roundtrip() {
        let vals = [
            0u128,
            1,
            0x0123_4567_89AB_CDEF_0011_2233_4455_6677u128,
            u128::MAX,
        ];
        for &v in &vals {
            let p = PodU128::from(v);
            assert_eq!(p.get(), v);
            let back: u128 = p.into();
            assert_eq!(back, v);

            let mut m = PodU128::default();
            m.set(v);
            assert_eq!(m.get(), v);
        }
    }

    // ---------- PodBool ----------
    #[test]
    fn podbool_roundtrip_and_valid() {
        let mut b = PodBool::from(false);
        assert!(b.is_valid());
        assert!(!bool::from(b));
        assert!(!b.get());

        b.set(true);
        assert!(b.is_valid());
        assert!(bool::from(b));
        assert!(b.get());

        let t = PodBool::TRUE;
        let f = PodBool::FALSE;
        assert!(t.is_valid() && f.is_valid());
        assert!(bool::from(t));
        assert!(!bool::from(f));

        // Non-zero is true by From<PodBool> semantics
        let weird = PodBool { data: 255 };
        assert!(bool::from(weird));
        // But validity check flags it
        assert!(!weird.is_valid());
    }

    // ---------- PodOption<u8> ----------
    #[test]
    fn podoption_u8_basic() {
        let mut o: PodOption<u8> = PodOption::none();
        assert!(o.is_none());
        assert_eq!(o.tag(), 0);
        assert!(o.as_ref().is_none());
        assert_eq!(o.copied(), None);

        o.set_some(7);
        assert!(o.is_some());
        assert_eq!(o.tag(), 1);
        assert_eq!(o.as_ref(), Some(&7));
        assert_eq!(o.copied(), Some(7));

        let t = o.take();
        assert_eq!(t, Some(7));
        assert!(o.is_none());
    }

    // ---------- PodOption<PodU64> ----------
    #[test]
    fn podoption_podu64_basic() {
        let mut o: PodOption<PodU64> = PodOption::none();
        assert!(o.is_none());

        o.set_some(PodU64::from(42u64));
        assert!(o.is_some());
        let as_u64 = o.copied().map(u64::from);
        assert_eq!(as_u64, Some(42));

        // Clone/Copy works when T: Copy
        assert_copy_clone(o);

        let taken = o.take().map(u64::from);
        assert_eq!(taken, Some(42));
        assert!(o.is_none());
    }

    // ---------- PodOption<TestPubkey> ----------
    #[test]
    fn podoption_pubkey_basic() {
        let pk: Pubkey = Pubkey::new_from_array([0xABu8; 32]);
        let o: PodOption<Pubkey> = PodOption::some(pk);
        assert!(o.is_some());
        assert_eq!(o.as_ref().unwrap().to_bytes()[0], 0xAB);
        assert_eq!(o.as_ref().unwrap().to_bytes()[31], 0xAB);
    }

    // ---------- Conversions with Option<T> ----------
    #[test]
    fn podoption_from_into_option() {
        let a: PodOption<u8> = PodOption::from(Some(9u8));
        assert!(a.is_some());
        let b: Option<u8> = Option::try_from(a).unwrap();
        assert_eq!(b, Some(9));

        let c: PodOption<PodU64> = PodOption::from(None::<PodU64>);
        assert!(c.is_none());
        let d: Option<PodU64> = Option::try_from(c).unwrap();
        assert_eq!(d, None);
    }

    // ---------- TryFrom error path with invalid tag ----------
    #[test]
    fn podoption_invalid_tag_error() {
        // Build a value with tag=2 via a raw layout mirror.
        #[repr(C)]
        struct Raw<T> {
            tag: u8,
            value: MaybeUninit<T>,
        }

        let raw = Raw::<u8> {
            tag: 2,
            value: MaybeUninit::uninit(),
        };
        let poison: PodOption<u8> = unsafe { core::mem::transmute(raw) };
        let res: Result<Option<u8>, ()> = Option::try_from(poison);
        assert!(res.is_err());
    }

    // ---------- Debug impl sanity ----------
    #[test]
    fn debug_impl() {
        let none_u8: PodOption<u8> = PodOption::none();
        let some_u8: PodOption<u8> = PodOption::some(3);

        let dn = format!("{:?}", none_u8);
        let ds = format!("{:?}", some_u8);

        assert!(dn.contains("None"));
        assert!(ds.contains("Some"));
        assert!(ds.contains('3'));
    }
}
