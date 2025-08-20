//! Software sealing of pointers in the CHERIoT-RTOS.

use core::cheri::timeout::Timeout;
use core::ffi::c_void;

unsafe extern "chericcallcc" {
    /// Dynamically create a new token.
    #[cheriot_compartment = "allocator"]
    #[link_name = "_Z13token_key_newv"]
    fn token_key_new() -> *const c_void;

    /// Dynamically create a new sealed value.
    /// The return value is the sealed capability, while `unsealed` will contain a pointer to the
    /// allocated memory region.
    #[cheriot_compartment = "allocator"]
    #[link_name = "_Z27token_sealed_unsealed_allocP7TimeoutU19__sealed_capabilityP24AllocatorCapabilityStateP10SKeyStructjPPv"]
    fn token_sealed_unsealed_alloc(
        timeout: &core::cheri::timeout::Timeout,
        heap_capability: *const c_void,
        key: *const c_void,
        sz: usize,
        unsealed: *mut *mut c_void,
    ) -> *mut c_void;
}

unsafe extern "C" {
    #[cheriot_static_sealed_value]
    static __default_malloc_capability: *const c_void;
}

/// A sealed capability.
pub struct SealedCapability<T>(*mut T);

impl<T> Clone for SealedCapability<T> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<T> core::fmt::Debug for SealedCapability<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "SealedCapability<{}>(opaque)", core::any::type_name_of_val(&self.0))
    }
}

/// A sealing key.
///
/// This kind of key can be used for software sealing only: passing this to
/// [`core::intrinsics::cheri::cheri_unseal`] will generate an invalid sealed pointer.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct SealingKey(pub(self) *const ());

impl core::fmt::Debug for SealingKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "sealing_key({:p})", self.0)
    }
}

impl SealingKey {
    /// Create a new sealing key.
    ///
    /// This function is guaranteed to complete unless the allocator has exhausted
    /// the total number of sealing keys possible (2^32 - 2^24). After this point,
    /// it will never succeed. A compartment that is granted access to this entry
    /// point is trusted not to exhaust this resource. If you wish to allow a
    /// compartment to seal objects, but do not wish to allow it to allocate new
    /// sealing keys, then you should insert a proxy compartment that guarantees
    /// that it will call this API once and return a single key to the caller.
    ///
    /// The return value from this is a capability with the permit-seal and
    /// permit-unseal permissions.  Callers may remove one or both of these
    /// permissions and delegate the resulting capability to allow other
    /// compartments to either seal or unseal the capabilities with this key.
    ///
    /// If the sealing keys have been exhausted then this will return
    /// [`Option::None`].  This API is guaranteed never to block.
    #[inline(always)]
    pub fn try_new() -> Option<Self> {
        let res = unsafe { token_key_new() };
        if res.is_null() { None } else { Some(SealingKey(res as *const ())) }
    }
}

/// Represents the result of software sealing.
#[derive(Debug)]
pub struct Seal<T> {
    /// The sealed capability this seal created.
    sealed: SealedCapability<T>,
    /// The reference to the unsealed value.
    unsealed: *mut T,
}

impl<T> Seal<T> {
    /// Create a new [`Seal`].
    #[inline(always)]
    pub fn try_new<G: Fn() -> T>(key: SealingKey, generator: G) -> Option<Self> {
        Self::try_new_with_timeout(key, generator, 0u32.into())
    }

    /// Create a new [`Self`] with the given `timeout`.
    #[inline(always)]
    pub fn try_new_with_timeout<G: Fn() -> T>(
        key: SealingKey,
        generator: G,
        timeout: Timeout,
    ) -> Option<Self> {
        let mut unsealed: *mut T = core::ptr::null_mut();
        let sz = size_of::<T>();
        let sealed = unsafe {
            token_sealed_unsealed_alloc(
                &timeout,
                __default_malloc_capability,
                key.0 as *const c_void,
                sz,
                &mut unsealed as *mut *mut T as *mut *mut c_void,
            )
        };

        if unsealed.is_null() || sealed.is_null() {
            None
        } else {
            let value = generator();
            unsafe {
                core::ptr::copy_nonoverlapping(&value as *const T, unsealed, sz);
            }
            let sealed = SealedCapability::<T>(sealed as *mut T);
            Some(Self { sealed, unsealed })
        }
    }

    /// Get the sealed pointer from the seal.
    pub fn sealed(&self) -> SealedCapability<T> {
        self.sealed.clone()
    }

    /// Get the unsealed pointer from the seal.
    pub fn unsealed(&self) -> *mut T {
        self.unsealed
    }
}
