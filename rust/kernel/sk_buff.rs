// SPDX-License-Identifier: GPL-2.0

// Copyright (c) 2025 Boris Astardzhiev <boris.astardzhiev@gmail.com>

//! sk_buffs.
//!
//! C header: [`include/linux/skbuff.h`](srctree/include/net/skbuff.h)

use crate::{
    // bindings, pr_info,
    alloc::Flags,
    net_device::NetDevice,
    prelude::{EINVAL, ENOMEM},
    types::{ARef, AlwaysRefCounted, Opaque},
};
use core::{marker::PhantomData, ptr};
use kernel::error::Result;

/// Wraps the kernel's `struct sk_buff`. Thread safe.
///
/// This structure represents the Rust abstraction for a C `struct sk_buff`. This
/// implementation abstracts the usage of an already existing C `struct sk_buff` within Rust
/// code that we get passed from the C side.
#[repr(transparent)]
pub struct SkBuff<'a> {
    inner: Opaque<bindings::sk_buff>,
    _marker: PhantomData<&'a ()>,
}

impl<'a> SkBuff<'a> {
    /// Returns a raw pointer to the inner C struct.
    #[inline]
    pub fn as_ptr(&self) -> *mut bindings::sk_buff {
        self.inner.get()
    }

    /// Creates a reference to a [`SkBuff`] from a valid pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `ptr` is valid and remains valid for the lifetime of the
    /// returned [`SkBuff`] reference.
    pub unsafe fn from_ptr(ptr: *const bindings::sk_buff) -> ARef<Self> {
        // SAFETY: The safety requirements guarantee the validity of the dereference, while the
        // `SkBuff` type being transparent makes the cast ok.
        unsafe { ARef::from_raw(ptr::NonNull::new_unchecked(ptr.cast::<Self>() as _)) }
    }

    #[allow(dead_code)]
    /// Get the packet type of this `sk_buff`.
    pub fn get_pkt_type(&self) -> Result<PacketType> {
        // SAFETY: Since the inner pointer is valid grab the packet type.
        let pkt_type = unsafe {
            (*self.inner.get())
                .__bindgen_anon_4
                .__bindgen_anon_1
                .as_ref()
                .pkt_type() as u32
        };
        match pkt_type {
            bindings::PACKET_HOST => Ok(PacketType::Host),
            bindings::PACKET_BROADCAST => Ok(PacketType::Broadcast),
            bindings::PACKET_MULTICAST => Ok(PacketType::Multicast),
            bindings::PACKET_OTHERHOST => Ok(PacketType::OtherHost),
            bindings::PACKET_OUTGOING => Ok(PacketType::Outgoing),
            bindings::PACKET_LOOPBACK => Ok(PacketType::Loopback),
            _ => Err(EINVAL),
        }
    }

    /// Get the network device in this `sk_buff`.
    pub fn get_dev(&self) -> Option<ARef<NetDevice>> {
        let skb = self.as_ptr();
        // SAFETY: Try to make a `NetDevice` out of the raw pointer in the `sk_buff`.
        NetDevice::from_raw(unsafe {
            (*skb)
                .__bindgen_anon_1
                .__bindgen_anon_1
                .__bindgen_anon_1
                .dev
        })
    }

    /// Set the network device in this `sk_buff`.
    pub fn set_dev(&self, dev: &NetDevice) {
        let skb = self.as_ptr();
        // SAFETY: Since the inner pointer is valid set the passed device.
        unsafe {
            (*skb)
                .__bindgen_anon_1
                .__bindgen_anon_1
                .__bindgen_anon_1
                .dev = dev.as_ptr();
        }
    }

    #[allow(dead_code)]
    /// Create a deep clone of this `sk_buff`.
    pub fn deep_clone<'b>(&'a self, flags: Flags) -> Result<ARef<SkBuff<'b>>> {
        let ptr = self.as_ptr();
        unsafe {
            // SAFETY: The safety requirement insures that ptr is valid.
            let nptr = bindings::skb_copy(ptr, flags.as_raw());
            if nptr.is_null() {
                return Err(ENOMEM);
            }
            Ok(SkBuff::from_ptr(nptr))
        }
    }

    #[allow(dead_code)]
    /// Create a shallow clone of this `sk_buff`.
    pub fn shallow_clone(&'a self, flags: Flags) -> Result<ARef<SkBuff<'a>>> {
        let ptr = self.as_ptr();
        unsafe {
            // SAFETY: The safety requirement insures that ptr is valid.
            let nptr = bindings::skb_clone(ptr, flags.as_raw());
            if nptr.is_null() {
                return Err(ENOMEM);
            }
            Ok(SkBuff::from_ptr(nptr))
        }
    }

    #[allow(dead_code)]
    /// Returns the remaining data in the buffer's first segment.
    pub fn head_data(&'a self) -> &'a [u8] {
        // SAFETY: The existence of a shared reference means that the refcount is nonzero.
        let headlen = unsafe { bindings::skb_headlen(self.as_ptr()) };
        let len = headlen.try_into().unwrap_or(0);
        // SAFETY: The existence of a shared reference means `self.inner` is valid.
        let data = unsafe { core::ptr::addr_of!((*self.as_ptr()).data).read() };
        // SAFETY: The `struct sk_buff` conventions guarantee that at least `skb_headlen(skb)` bytes
        // are valid from `skb->data`.
        unsafe { core::slice::from_raw_parts(data, len) }
    }
}

#[derive(Debug, PartialEq)]
/// Packet Types.
pub enum PacketType {
    /// To us.
    Host,
    /// To all.
    Broadcast,
    /// To group.
    Multicast,
    /// To someone else.
    OtherHost,
    /// Outgoing of any type.
    Outgoing,
    /// MC/BRD frame looped back.
    Loopback,
}

// SAFETY: Instances of `SkBuff` are always reference-counted.
unsafe impl<'a> AlwaysRefCounted for SkBuff<'a> {
    #[inline]
    fn inc_ref(&self) {
        // pr_info!("Incrementing ref({:p}) to sk_buff!\n", (*self).as_ptr());
        // SAFETY: The existence of a shared reference means that the refcount is nonzero.
        unsafe { bindings::skb_get(self.as_ptr()) };
    }

    #[inline]
    unsafe fn dec_ref(obj: ptr::NonNull<SkBuff<'_>>) {
        let ptr = obj.cast().as_ptr();
        // pr_info!("Decrementing ref({:p}) to sk_buff!\n", ptr);
        // SAFETY: The safety requirements guarantee that the refcount is non-zero.
        unsafe { bindings::kfree_skb(ptr) }
    }
}
